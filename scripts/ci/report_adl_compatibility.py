#!/usr/bin/env python3
"""Report ADL xAPI LRS conformance suite compatibility gaps."""

from __future__ import annotations

import argparse
import json
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


DEFAULT_COMPATIBILITY_MAP = Path("spec/conformance/adl-compatibility-map.yaml")
VALID_RELATIONSHIPS = {"equivalent", "partial", "absent", "obsolete"}
COVERAGE_RELATIONSHIPS = {"equivalent", "partial", "absent"}
REPORT_SCHEMA_VERSION = 1


def load_requirements(path: Path) -> list[dict[str, Any]]:
    """Load ordered requirement mappings from the catalog."""
    catalog = load_yaml(path)
    requirements = require_list(catalog.get("requirements"), "requirements")
    loaded: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, requirement in enumerate(requirements):
        requirement_map = require_mapping(requirement, f"requirements[{index}]")
        requirement_id = requirement_map.get("id")
        if not isinstance(requirement_id, str) or not requirement_id:
            raise ConformanceError(f"requirements[{index}].id must be a non-empty string")
        if requirement_id in seen:
            raise ConformanceError(f"duplicate requirement id: {requirement_id}")
        seen.add(requirement_id)
        loaded.append(requirement_map)
    return loaded


def build_report(
    map_path: Path,
    catalog_path: Path,
    adl_root: Path | None,
) -> dict[str, Any]:
    """Build a JSON-serializable ADL compatibility gap report."""
    requirements = load_requirements(catalog_path)
    requirement_by_id = {requirement["id"]: requirement for requirement in requirements}
    document = validate_compatibility_map(map_path, set(requirement_by_id), adl_root)
    mappings = [
        require_mapping(mapping, f"mappings[{index}]")
        for index, mapping in enumerate(require_list(document.get("mappings"), "mappings"))
    ]

    classified = classify_requirements(requirements, mappings)
    return {
        "schema_version": REPORT_SCHEMA_VERSION,
        "standard": document.get("standard"),
        "compatibility_map": str(map_path),
        "requirement_catalog": str(catalog_path),
        "adl_suite": document.get("adl_suite"),
        "adl_source_verification": adl_source_verification(adl_root),
        "totals": {
            "requirements": len(requirements),
            "mappings": len(mappings),
            "equivalent": len(classified["equivalent"]),
            "partial": len(classified["partial"]),
            "absent": len(classified["absent"]),
            "unclassified": len(classified["unclassified"]),
        },
        "mappings_by_relationship": count_mappings_by_relationship(mappings),
        "requirements": {
            relationship: [
                requirement_row(requirement_id, requirement_by_id, mappings)
                for requirement_id in sorted(requirement_ids)
            ]
            for relationship, requirement_ids in classified.items()
        },
    }


def validate_compatibility_map(
    path: Path,
    requirement_ids: set[str],
    adl_root: Path | None,
) -> dict[str, Any]:
    """Validate the ADL compatibility map and return its document."""
    document = load_yaml(path)
    if document.get("requirement_catalog") != str(DEFAULT_REQUIREMENT_CATALOG):
        raise ConformanceError(f"requirement_catalog must be {DEFAULT_REQUIREMENT_CATALOG}")

    adl_suite = require_mapping(document.get("adl_suite"), "adl_suite")
    for key in ("id", "upstream", "test_root", "role", "license"):
        if not isinstance(adl_suite.get(key), str) or not adl_suite[key]:
            raise ConformanceError(f"adl_suite.{key} must be a non-empty string")

    mapping_ids: set[str] = set()
    mapped_requirements: set[str] = set()
    mappings = require_list(document.get("mappings"), "mappings")
    if not mappings:
        raise ConformanceError("mappings must not be empty")

    for index, mapping in enumerate(mappings):
        mapping_map = require_mapping(mapping, f"mappings[{index}]")
        mapping_id = require_non_empty_string(mapping_map.get("id"), f"mappings[{index}].id")
        if mapping_id in mapping_ids:
            raise ConformanceError(f"duplicate mapping id: {mapping_id}")
        mapping_ids.add(mapping_id)

        relationship = require_non_empty_string(
            mapping_map.get("relationship"),
            f"{mapping_id}.relationship",
        )
        if relationship not in VALID_RELATIONSHIPS:
            raise ConformanceError(
                f"{mapping_id}.relationship must be one of {sorted(VALID_RELATIONSHIPS)}"
            )
        if not isinstance(mapping_map.get("rationale"), str) or not mapping_map["rationale"]:
            raise ConformanceError(f"{mapping_id}.rationale must be a non-empty string")

        sources = validate_adl_sources(mapping_id, mapping_map, adl_root)
        kiva_requirements = validate_kiva_requirements(mapping_id, mapping_map, requirement_ids)
        if relationship in {"equivalent", "partial"} and not sources:
            raise ConformanceError(f"{mapping_id} must include ADL sources")
        if relationship in COVERAGE_RELATIONSHIPS and not kiva_requirements:
            raise ConformanceError(f"{mapping_id} must include Kiva requirements")
        mapped_requirements.update(kiva_requirements)

    missing = requirement_ids.difference(mapped_requirements)
    if missing:
        missing_ids = ", ".join(sorted(missing))
        raise ConformanceError(f"requirements missing from ADL compatibility map: {missing_ids}")
    return document


def adl_source_verification(adl_root: Path | None) -> dict[str, Any]:
    """Return metadata about the optional local ADL source checkout."""
    if adl_root is None:
        return {
            "checked": False,
            "root": None,
            "exists": False,
            "git_commit": None,
            "git_dirty": None,
        }
    return {
        "checked": True,
        "root": str(adl_root),
        "exists": adl_root.is_dir(),
        "git_commit": git_output(adl_root, "rev-parse", "HEAD"),
        "git_dirty": bool(git_output(adl_root, "status", "--porcelain"))
        if (adl_root / ".git").exists()
        else None,
    }


def git_output(path: Path, *args: str) -> str | None:
    """Return git command output for a path, or None outside a git checkout."""
    if not (path / ".git").exists():
        return None
    return subprocess.check_output(["git", "-C", str(path), *args], text=True).strip()


def require_non_empty_string(value: Any, label: str) -> str:
    """Return a non-empty string or raise a conformance error."""
    if not isinstance(value, str) or not value:
        raise ConformanceError(f"{label} must be a non-empty string")
    return value


def validate_adl_sources(
    mapping_id: str,
    mapping: dict[str, Any],
    adl_root: Path | None,
) -> list[str]:
    """Validate ADL source declarations and optionally source paths."""
    sources: list[str] = []
    for index, source in enumerate(require_list(mapping.get("adl_sources"), f"{mapping_id}.adl_sources")):
        source_map = require_mapping(source, f"{mapping_id}.adl_sources[{index}]")
        source_path = require_non_empty_string(
            source_map.get("path"),
            f"{mapping_id}.adl_sources[{index}].path",
        )
        if source_path.startswith("/"):
            raise ConformanceError(f"{mapping_id}.adl_sources[{index}].path must be relative")
        if adl_root is not None and not (adl_root / source_path).is_file():
            raise ConformanceError(f"{mapping_id} references missing ADL source: {source_path}")
        sources.append(source_path)
    return sources


def validate_kiva_requirements(
    mapping_id: str,
    mapping: dict[str, Any],
    requirement_ids: set[str],
) -> list[str]:
    """Validate Kiva requirement references."""
    kiva_requirements: list[str] = []
    seen: set[str] = set()
    for index, requirement in enumerate(
        require_list(mapping.get("kiva_requirements"), f"{mapping_id}.kiva_requirements")
    ):
        if not isinstance(requirement, str) or not requirement:
            raise ConformanceError(
                f"{mapping_id}.kiva_requirements[{index}] must be a non-empty string"
            )
        if requirement not in requirement_ids:
            raise ConformanceError(f"{mapping_id} references unknown requirement: {requirement}")
        if requirement in seen:
            raise ConformanceError(f"{mapping_id} repeats requirement: {requirement}")
        seen.add(requirement)
        kiva_requirements.append(requirement)
    return kiva_requirements


def classify_requirements(
    requirements: list[dict[str, Any]],
    mappings: list[dict[str, Any]],
) -> dict[str, set[str]]:
    """Classify requirements by strongest ADL relationship."""
    equivalent: set[str] = set()
    partial: set[str] = set()
    absent: set[str] = set()
    for mapping in mappings:
        relationship = mapping["relationship"]
        mapped_requirements = set(mapping["kiva_requirements"])
        if relationship == "equivalent":
            equivalent.update(mapped_requirements)
        elif relationship == "partial":
            partial.update(mapped_requirements)
        elif relationship == "absent":
            absent.update(mapped_requirements)

    partial.difference_update(equivalent)
    absent.difference_update(equivalent)
    absent.difference_update(partial)
    all_requirement_ids = {requirement["id"] for requirement in requirements}
    unclassified = all_requirement_ids.difference(equivalent, partial, absent)
    return {
        "equivalent": equivalent,
        "partial": partial,
        "absent": absent,
        "unclassified": unclassified,
    }


def count_mappings_by_relationship(mappings: list[dict[str, Any]]) -> dict[str, int]:
    """Count mapping records by relationship."""
    counts = {relationship: 0 for relationship in sorted(VALID_RELATIONSHIPS)}
    for mapping in mappings:
        counts[mapping["relationship"]] += 1
    return counts


def requirement_row(
    requirement_id: str,
    requirement_by_id: dict[str, dict[str, Any]],
    mappings: list[dict[str, Any]],
) -> dict[str, Any]:
    """Build a report row for a requirement."""
    requirement = requirement_by_id[requirement_id]
    return {
        "id": requirement_id,
        "clause": requirement.get("clause"),
        "area": requirement.get("area"),
        "summary": requirement.get("summary"),
        "mappings": [
            mapping["id"]
            for mapping in mappings
            if requirement_id in set(mapping["kiva_requirements"])
        ],
    }


def print_markdown(report: dict[str, Any]) -> None:
    """Print a compact Markdown ADL compatibility gap report."""
    totals = report["totals"]
    print(f"# {report['standard']} ADL Compatibility Gap Report")
    print()
    print(f"- Compatibility map: `{report['compatibility_map']}`")
    print(f"- Requirement catalog: `{report['requirement_catalog']}`")
    print(f"- ADL suite: `{report['adl_suite']['id']}`")
    print(f"- ADL source checked: {report['adl_source_verification']['checked']}")
    if report["adl_source_verification"]["root"] is not None:
        print(f"- ADL source root: `{report['adl_source_verification']['root']}`")
    if report["adl_source_verification"]["git_commit"] is not None:
        print(f"- ADL source commit: `{report['adl_source_verification']['git_commit']}`")
        print(f"- ADL source dirty: {report['adl_source_verification']['git_dirty']}")
    print(f"- Requirements: {totals['requirements']}")
    print(f"- Equivalent ADL coverage: {totals['equivalent']}")
    print(f"- Partial ADL coverage: {totals['partial']}")
    print(f"- Explicit ADL gaps: {totals['absent']}")
    print(f"- Unclassified: {totals['unclassified']}")
    print()
    print("## Mapping Records By Relationship")
    print()
    for relationship, count in report["mappings_by_relationship"].items():
        print(f"- `{relationship}`: {count}")
    print_requirement_table("Partial ADL Coverage", report["requirements"]["partial"])
    print_requirement_table("Explicit ADL Gaps", report["requirements"]["absent"])
    print_requirement_table("Unclassified Requirements", report["requirements"]["unclassified"])


def print_requirement_table(title: str, rows: list[dict[str, Any]]) -> None:
    """Print requirement rows as a Markdown table."""
    print()
    print(f"## {title}")
    print()
    print("| Requirement | Clause | Area | Mappings | Summary |")
    print("| --- | --- | --- | --- | --- |")
    for row in rows:
        print(
            "| {id} | {clause} | {area} | {mappings} | {summary} |".format(
                id=row["id"],
                clause=row.get("clause") or "",
                area=row.get("area") or "",
                mappings=escape_markdown_table_text(", ".join(row.get("mappings") or [])),
                summary=escape_markdown_table_text(row.get("summary") or ""),
            )
        )


def escape_markdown_table_text(value: str) -> str:
    """Escape Markdown table-sensitive characters."""
    return value.replace("|", "\\|").replace("\n", " ")


def main() -> int:
    """Run the ADL compatibility reporter."""
    parser = argparse.ArgumentParser()
    parser.add_argument("map", type=Path, nargs="?", default=DEFAULT_COMPATIBILITY_MAP)
    parser.add_argument("--requirements", type=Path, default=DEFAULT_REQUIREMENT_CATALOG)
    parser.add_argument("--adl-root", type=Path)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    report = build_report(args.map, args.requirements, args.adl_root)
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_markdown(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
