#!/usr/bin/env python3
"""Audit the clause-indexed IEEE xAPI LRS requirement catalog."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path
from typing import Any

from validate_conformance import (
    DEFAULT_REQUIREMENT_CATALOG,
    ConformanceError,
    load_yaml,
    require_list,
    require_mapping,
)


REPORT_SCHEMA_VERSION = 1
STANDARD = "IEEE/ISO/IEC 39274-1-1-2025"
DEFAULT_REFERENCE_ROOT = Path(
    "/home/claude/Codex/opensource.ieee.org/xapi/xapi-base-standard-documentation"
)
DEFAULT_SOURCE_TRIAGE = Path("spec/conformance/ieee-source-clause-triage.yaml")
VALID_LEVELS = {"shall", "shall-not"}
VALID_PROJECTIONS = {"assertion", "openapi", "openapi-and-assertion"}
VALID_TRIAGE_CLASSIFICATIONS = {
    "composite-processing-section",
    "data-model-subsection",
    "non-normative-guidance",
    "parent-section",
    "recommendation",
    "security-guidance",
}
CLAUSE_PATTERN = re.compile(r"^4(?:\.\d+)*$")
HEADING_PATTERN = re.compile(r"^(#{1,6})\s+(4(?:\.\d+)*)\b\s*(.*)$")


def build_report(
    catalog_path: Path,
    reference_root: Path | None,
    require_reference_root: bool,
    source_triage_path: Path | None,
) -> dict[str, Any]:
    """Build a JSON-serializable audit report for the requirement catalog."""
    catalog = load_yaml(catalog_path)
    issues: list[str] = []

    standard = catalog.get("standard")
    if standard != STANDARD:
        issues.append(f"standard must be {STANDARD}")

    requirements = validate_requirements(catalog, issues)
    requirements_by_clause = index_requirements_by_clause(requirements)
    source = validate_source(catalog, reference_root, require_reference_root, issues)
    source_clauses = load_source_clauses(source, issues)
    source_clause_ids = {row["clause"] for row in source_clauses}
    triage = load_source_triage(source_triage_path, source_clause_ids, issues)
    clause_rows = build_clause_rows(
        requirements_by_clause,
        source_clauses,
        triage["clauses"],
        issues,
    )
    level_counts = count_by_key(requirements, "level")
    projection_counts = count_by_key(requirements, "projection")

    report = {
        "schema_version": REPORT_SCHEMA_VERSION,
        "standard": STANDARD,
        "status": "passed" if not issues else "failed",
        "requirement_catalog": str(catalog_path),
        "source": source,
        "totals": {
            "requirements": len(requirements),
            "clauses": len({requirement["clause"] for requirement in requirements}),
            "source_clauses": len(source_clauses),
            "source_clauses_with_requirements": sum(
                1 for row in clause_rows if row["requirements"]
            ),
            "source_clauses_triaged_without_requirements": sum(
                1 for row in clause_rows if row["triage"] is not None and not row["requirements"]
            ),
        },
        "levels": level_counts,
        "projections": projection_counts,
        "source_triage": triage["metadata"],
        "clauses": clause_rows,
        "issues": issues,
    }
    return report


def validate_source(
    catalog: dict[str, Any],
    reference_root: Path | None,
    require_reference_root: bool,
    issues: list[str],
) -> dict[str, Any]:
    """Validate the source declaration and optional local IEEE reference path."""
    source = require_mapping(catalog.get("source"), "source")
    repository = require_non_empty_string(source.get("repository"), "source.repository", issues)
    document = require_non_empty_string(source.get("document"), "source.document", issues)

    reference_checked = reference_root is not None
    reference_path = reference_root / document if reference_root is not None else None
    source_document_exists = reference_path.is_file() if reference_path is not None else False
    reference_git_commit = git_output(reference_root, "rev-parse", "HEAD") if reference_root else None
    reference_git_dirty = (
        bool(git_output(reference_root, "status", "--porcelain")) if reference_root else None
    )
    if require_reference_root and reference_root is None:
        issues.append("reference_root is required")
    if require_reference_root and reference_root is not None and not reference_root.is_dir():
        issues.append(f"reference_root does not exist: {reference_root}")
    if require_reference_root and reference_path is not None and not source_document_exists:
        issues.append(f"source document does not exist: {reference_path}")

    return {
        "repository": repository,
        "document": document,
        "reference_root": str(reference_root) if reference_root is not None else None,
        "reference_path": str(reference_path) if reference_path is not None else None,
        "reference_checked": reference_checked,
        "source_document_exists": source_document_exists,
        "git_commit": reference_git_commit,
        "git_dirty": reference_git_dirty,
    }


def validate_requirements(
    catalog: dict[str, Any],
    issues: list[str],
) -> list[dict[str, Any]]:
    """Validate requirement row shape and return normalized rows."""
    rows = require_list(catalog.get("requirements"), "requirements")
    requirements: list[dict[str, Any]] = []
    seen_ids: set[str] = set()

    for index, row in enumerate(rows):
        label = f"requirements[{index}]"
        requirement = require_mapping(row, label)
        requirement_id = require_non_empty_string(requirement.get("id"), f"{label}.id", issues)
        clause = require_non_empty_string(requirement.get("clause"), f"{label}.clause", issues)
        level = require_non_empty_string(requirement.get("level"), f"{label}.level", issues)
        area = require_non_empty_string(requirement.get("area"), f"{label}.area", issues)
        summary = require_non_empty_string(requirement.get("summary"), f"{label}.summary", issues)
        projection = require_non_empty_string(
            requirement.get("projection"),
            f"{label}.projection",
            issues,
        )

        if requirement_id in seen_ids:
            issues.append(f"duplicate requirement id: {requirement_id}")
        seen_ids.add(requirement_id)
        if not requirement_id.startswith("xapi-lrs-"):
            issues.append(f"{label}.id must start with xapi-lrs-")
        if not CLAUSE_PATTERN.match(clause):
            issues.append(f"{label}.clause must be an xAPI LRS clause under 4.x")
        if level not in VALID_LEVELS:
            issues.append(f"{label}.level must be one of {sorted(VALID_LEVELS)}")
        if projection not in VALID_PROJECTIONS:
            issues.append(f"{label}.projection must be one of {sorted(VALID_PROJECTIONS)}")

        requirements.append(
            {
                "id": requirement_id,
                "clause": clause,
                "level": level,
                "area": area,
                "summary": summary,
                "projection": projection,
            }
        )

    if not requirements:
        issues.append("requirements must not be empty")
    return requirements


def index_requirements_by_clause(
    requirements: list[dict[str, Any]],
) -> dict[str, list[dict[str, Any]]]:
    """Index normalized requirement rows by clause id."""
    by_clause: dict[str, list[dict[str, Any]]] = {}
    for requirement in requirements:
        by_clause.setdefault(requirement["clause"], []).append(requirement)
    return by_clause


def load_source_clauses(source: dict[str, Any], issues: list[str]) -> list[dict[str, Any]]:
    """Load clause heading ranges from the local IEEE source document."""
    if not source["reference_checked"] or not source["source_document_exists"]:
        return []

    source_path = Path(source["reference_path"])
    lines = source_path.read_text(encoding="utf-8").splitlines()
    headings: list[dict[str, Any]] = []
    for line_number, line in enumerate(lines, 1):
        match = HEADING_PATTERN.match(line)
        if match is None:
            continue
        title = match.group(3).strip()
        headings.append(
            {
                "clause": match.group(2),
                "title": title.lstrip(". ").strip(),
                "heading_level": len(match.group(1)),
                "line_start": line_number,
                "line_end": len(lines),
            }
        )

    seen: set[str] = set()
    for index, heading in enumerate(headings):
        clause = heading["clause"]
        if clause in seen:
            issues.append(f"source document repeats clause heading: {clause}")
        seen.add(clause)
        for later in headings[index + 1 :]:
            if later["heading_level"] <= heading["heading_level"]:
                heading["line_end"] = later["line_start"] - 1
                break
    return headings


def load_source_triage(
    path: Path | None,
    source_clause_ids: set[str],
    issues: list[str],
) -> dict[str, Any]:
    """Load source-clause triage for clauses without direct requirement rows."""
    if path is None:
        return {
            "metadata": {
                "path": None,
                "checked": False,
            },
            "clauses": {},
        }
    if not path.is_file():
        issues.append(f"source triage file does not exist: {path}")
        return {
            "metadata": {
                "path": str(path),
                "checked": False,
            },
            "clauses": {},
        }

    document = load_yaml(path)
    if document.get("schema_version") != 1:
        issues.append(f"{path}.schema_version must be 1")
    if document.get("standard") != STANDARD:
        issues.append(f"{path}.standard must be {STANDARD}")

    triage_by_clause: dict[str, dict[str, Any]] = {}
    for index, row in enumerate(require_list(document.get("triage"), f"{path}.triage")):
        label = f"{path}.triage[{index}]"
        row_map = require_mapping(row, label)
        clause = require_non_empty_string(row_map.get("clause"), f"{label}.clause", issues)
        classification = require_non_empty_string(
            row_map.get("classification"),
            f"{label}.classification",
            issues,
        )
        rationale = require_non_empty_string(row_map.get("rationale"), f"{label}.rationale", issues)
        covered_by = [
            value
            for value in require_list(row_map.get("covered_by"), f"{label}.covered_by")
            if isinstance(value, str) and value
        ]
        if len(covered_by) != len(require_list(row_map.get("covered_by"), f"{label}.covered_by")):
            issues.append(f"{label}.covered_by entries must be non-empty strings")
        if clause in triage_by_clause:
            issues.append(f"{path} repeats source triage clause: {clause}")
        if source_clause_ids and clause not in source_clause_ids:
            issues.append(f"{path} triages unknown source clause: {clause}")
        if classification not in VALID_TRIAGE_CLASSIFICATIONS:
            issues.append(
                f"{label}.classification must be one of {sorted(VALID_TRIAGE_CLASSIFICATIONS)}"
            )
        triage_by_clause[clause] = {
            "classification": classification,
            "rationale": rationale,
            "covered_by": covered_by,
        }

    return {
        "metadata": {
            "path": str(path),
            "checked": True,
            "triaged_clauses": len(triage_by_clause),
        },
        "clauses": triage_by_clause,
    }


def build_clause_rows(
    requirements_by_clause: dict[str, list[dict[str, Any]]],
    source_clauses: list[dict[str, Any]],
    triage_by_clause: dict[str, dict[str, Any]],
    issues: list[str],
) -> list[dict[str, Any]]:
    """Build source clause rows and validate catalog/source coverage."""
    source_clause_ids = {row["clause"] for row in source_clauses}
    for clause in requirements_by_clause:
        if source_clause_ids and clause not in source_clause_ids:
            issues.append(f"requirement catalog references missing source clause: {clause}")

    rows: list[dict[str, Any]] = []
    for source_clause in source_clauses:
        clause = source_clause["clause"]
        requirements = requirements_by_clause.get(clause, [])
        triage = triage_by_clause.get(clause)
        if not requirements and triage is None:
            issues.append(f"source clause lacks requirement rows or triage: {clause}")
        rows.append(
            {
                "clause": clause,
                "title": source_clause["title"],
                "line_start": source_clause["line_start"],
                "line_end": source_clause["line_end"],
                "requirements": [requirement["id"] for requirement in requirements],
                "triage": triage,
            }
        )

    return rows


def require_non_empty_string(value: Any, label: str, issues: list[str]) -> str:
    """Return a string value while recording structural issues."""
    if not isinstance(value, str) or not value:
        issues.append(f"{label} must be a non-empty string")
        return ""
    return value


def count_by_key(rows: list[dict[str, Any]], key: str) -> dict[str, int]:
    """Count row values by a string key."""
    counts: dict[str, int] = {}
    for row in rows:
        value = row[key]
        counts[value] = counts.get(value, 0) + 1
    return dict(sorted(counts.items()))


def git_output(path: Path | None, *args: str) -> str | None:
    """Return git command output for a path, or None outside a git checkout."""
    if path is None or not (path / ".git").exists():
        return None
    return subprocess.check_output(["git", "-C", str(path), *args], text=True).strip()


def print_markdown(report: dict[str, Any]) -> None:
    """Print a compact Markdown audit report."""
    source = report["source"]
    totals = report["totals"]
    print(f"# {report['standard']} Requirement Catalog Audit")
    print()
    print(f"- Requirement catalog: `{report['requirement_catalog']}`")
    print(f"- Status: `{report['status']}`")
    print(f"- Requirements: {totals['requirements']}")
    print(f"- Clauses: {totals['clauses']}")
    print(f"- Source repository: `{source['repository']}`")
    print(f"- Source document: `{source['document']}`")
    print(f"- Reference checked: {source['reference_checked']}")
    print(f"- Source document exists: {source['source_document_exists']}")
    if source["git_commit"] is not None:
        print(f"- Source git commit: `{source['git_commit']}`")
        print(f"- Source git dirty: {source['git_dirty']}")
    if source["reference_path"] is not None:
        print(f"- Reference path: `{source['reference_path']}`")
    print(f"- Source triage checked: {report['source_triage']['checked']}")
    if report["source_triage"]["path"] is not None:
        print(f"- Source triage: `{report['source_triage']['path']}`")
    print()
    print("## Levels")
    print()
    for level, count in report["levels"].items():
        print(f"- `{level}`: {count}")
    print()
    print("## Projections")
    print()
    for projection, count in report["projections"].items():
        print(f"- `{projection}`: {count}")
    print()
    print("## Issues")
    print()
    if report["issues"]:
        for issue in report["issues"]:
            print(f"- {issue}")
    else:
        print("- None")
    if report["clauses"]:
        print()
        print("## Source Clauses")
        print()
        print("| Clause | Lines | Requirements | Triage |")
        print("| --- | --- | --- | --- |")
        for row in report["clauses"]:
            triage = row["triage"]["classification"] if row["triage"] is not None else ""
            requirements = ", ".join(row["requirements"])
            print(
                "| {clause} | {line_start}-{line_end} | {requirements} | {triage} |".format(
                    clause=row["clause"],
                    line_start=row["line_start"],
                    line_end=row["line_end"],
                    requirements=escape_markdown_table_text(requirements),
                    triage=triage,
                )
            )


def escape_markdown_table_text(value: str) -> str:
    """Escape Markdown table-sensitive characters."""
    return value.replace("|", "\\|").replace("\n", " ")


def main() -> int:
    """Run the requirement catalog audit."""
    parser = argparse.ArgumentParser()
    parser.add_argument("catalog", type=Path, nargs="?", default=DEFAULT_REQUIREMENT_CATALOG)
    parser.add_argument("--reference-root", type=Path, default=DEFAULT_REFERENCE_ROOT)
    parser.add_argument("--no-reference-root", action="store_true")
    parser.add_argument("--require-reference-root", action="store_true")
    parser.add_argument("--source-triage", type=Path, default=DEFAULT_SOURCE_TRIAGE)
    parser.add_argument("--no-source-triage", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    reference_root = None if args.no_reference_root else args.reference_root
    source_triage = None if args.no_source_triage else args.source_triage
    report = build_report(
        args.catalog,
        reference_root,
        args.require_reference_root,
        source_triage,
    )
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_markdown(report)
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ConformanceError as error:
        raise SystemExit(f"error: {error}")
