#!/usr/bin/env python3
"""Report requirement traceability for bundled conformance case files."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from validate_conformance import (
    DEFAULT_REQUIREMENT_CATALOG,
    ConformanceError,
    load_yaml,
    require_list,
    require_mapping,
)


DEFAULT_CASE_ROOT = Path("tests/conformance/http")
DEFAULT_GAP_MAP = Path("spec/conformance/executable-gap-map.yaml")
REPORT_SCHEMA_VERSION = 1
STANDARD = "IEEE/ISO/IEC 39274-1-1-2025"
VALID_GAP_CLASSIFICATIONS = {"endpoint-profile-sensitive"}
VALID_ENDPOINT_PROFILES = {
    "deterministic-too-large-threshold",
    "deterministic-rate-limit-trigger",
}


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


def build_report(case_roots: list[Path], catalog_path: Path, gap_map_path: Path) -> dict[str, Any]:
    """Build a JSON-serializable conformance case matrix report."""
    roots = case_roots or [DEFAULT_CASE_ROOT]
    requirements = load_requirements(catalog_path)
    requirement_by_id = {requirement["id"]: requirement for requirement in requirements}
    suite_rows, case_rows = load_case_rows(roots, set(requirement_by_id))
    cases_by_requirement = index_cases_by_requirement(case_rows)
    requirement_rows = [
        requirement_row(requirement["id"], requirement_by_id, cases_by_requirement)
        for requirement in requirements
    ]
    uncovered = [row for row in requirement_rows if not row["cases"]]
    gap_rows = load_gap_rows(gap_map_path, requirement_by_id, {row["id"] for row in uncovered})
    gap_requirement_ids = {row["id"] for row in gap_rows}
    unclassified_uncovered = [row for row in uncovered if row["id"] not in gap_requirement_ids]
    destructive_cases = [row for row in case_rows if row["safety"]["destructive"]]
    endpoint_profile_required_cases = [row for row in case_rows if row["requires_endpoint_profiles"]]

    return {
        "schema_version": REPORT_SCHEMA_VERSION,
        "standard": STANDARD,
        "requirement_catalog": str(catalog_path),
        "executable_gap_map": str(gap_map_path),
        "case_roots": [str(root) for root in roots],
        "totals": {
            "requirements": len(requirement_rows),
            "suites": len(suite_rows),
            "cases": len(case_rows),
            "requirements_with_cases": len(requirement_rows) - len(uncovered),
            "requirements_without_cases": len(uncovered),
            "profile_sensitive_requirements": len(gap_rows),
            "unclassified_uncovered_requirements": len(unclassified_uncovered),
            "endpoint_profile_required_cases": len(endpoint_profile_required_cases),
            "destructive_cases": len(destructive_cases),
            "low_side_effect_cases": len(case_rows) - len(destructive_cases),
        },
        "suites": suite_rows,
        "cases": case_rows,
        "requirements": requirement_rows,
        "uncovered_requirements": uncovered,
        "profile_sensitive_requirements": gap_rows,
        "unclassified_uncovered_requirements": unclassified_uncovered,
        "endpoint_profile_required_cases": endpoint_profile_required_cases,
    }


def load_case_rows(
    roots: list[Path],
    requirement_ids: set[str],
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """Load suite and case traceability rows from YAML case files."""
    paths = collect_case_paths(roots)
    suite_rows: list[dict[str, Any]] = []
    case_rows: list[dict[str, Any]] = []
    seen_suites: set[str] = set()
    seen_cases: set[str] = set()

    for path in paths:
        document = load_yaml(path)
        suite = require_mapping(document.get("suite"), f"{path}.suite")
        suite_id = require_non_empty_string(suite.get("id"), f"{path}.suite.id")
        if suite_id in seen_suites:
            raise ConformanceError(f"duplicate suite id: {suite_id}")
        seen_suites.add(suite_id)
        if suite.get("standard") != STANDARD:
            raise ConformanceError(f"{path}.suite.standard must be {STANDARD}")

        cases = require_list(document.get("cases"), f"{path}.cases")
        if not cases:
            raise ConformanceError(f"{path}.cases must not be empty")
        suite_case_ids: list[str] = []
        for index, case in enumerate(cases):
            case_map = require_mapping(case, f"{path}.cases[{index}]")
            case_id = require_non_empty_string(case_map.get("id"), f"{path}.cases[{index}].id")
            if case_id in seen_cases:
                raise ConformanceError(f"duplicate case id: {case_id}")
            seen_cases.add(case_id)
            suite_case_ids.append(case_id)
            case_rows.append(case_row(path, suite_id, case_id, case_map, requirement_ids))

        suite_rows.append(
            {
                "id": suite_id,
                "path": str(path),
                "cases": suite_case_ids,
            }
        )

    return suite_rows, case_rows


def collect_case_paths(roots: list[Path]) -> list[Path]:
    """Collect YAML case paths from files or directories."""
    paths: list[Path] = []
    for root in roots:
        collect_case_path(root, paths)
    paths.sort()
    if not paths:
        raise ConformanceError("no YAML conformance case files found")
    return paths


def collect_case_path(path: Path, output: list[Path]) -> None:
    """Collect one YAML file or recurse through one directory."""
    if path.is_file():
        if path.suffix not in {".yaml", ".yml"}:
            raise ConformanceError(f"case file path must end with .yaml or .yml: {path}")
        output.append(path)
        return
    if not path.is_dir():
        raise ConformanceError(f"case path must be a file or directory: {path}")

    children = sorted(path.iterdir())
    for child in children:
        if child.is_dir() or child.suffix in {".yaml", ".yml"}:
            collect_case_path(child, output)


def require_non_empty_string(value: Any, label: str) -> str:
    """Return a non-empty string or raise a conformance error."""
    if not isinstance(value, str) or not value:
        raise ConformanceError(f"{label} must be a non-empty string")
    return value


def case_row(
    path: Path,
    suite_id: str,
    case_id: str,
    case: dict[str, Any],
    requirement_ids: set[str],
) -> dict[str, Any]:
    """Build and validate one case traceability row."""
    requirements = validate_case_requirements(path, case_id, case, requirement_ids)
    safety = validate_case_safety(path, case_id, case)
    requires_endpoint_profiles = validate_case_endpoint_profiles(path, case_id, case)
    return {
        "id": case_id,
        "suite": suite_id,
        "path": str(path),
        "requirements": requirements,
        "safety": safety,
        "requires_endpoint_profiles": requires_endpoint_profiles,
    }


def validate_case_requirements(
    path: Path,
    case_id: str,
    case: dict[str, Any],
    requirement_ids: set[str],
) -> list[str]:
    """Validate requirement references for one case."""
    requirements: list[str] = []
    seen: set[str] = set()
    for index, requirement in enumerate(require_list(case.get("requirements"), f"{case_id}.requirements")):
        if not isinstance(requirement, str) or not requirement:
            raise ConformanceError(f"{path} {case_id}.requirements[{index}] must be non-empty")
        if requirement not in requirement_ids:
            raise ConformanceError(f"{path} {case_id} references unknown requirement: {requirement}")
        if requirement in seen:
            raise ConformanceError(f"{path} {case_id} repeats requirement: {requirement}")
        seen.add(requirement)
        requirements.append(requirement)
    if not requirements:
        raise ConformanceError(f"{path} {case_id}.requirements must not be empty")
    return requirements


def validate_case_safety(path: Path, case_id: str, case: dict[str, Any]) -> dict[str, bool]:
    """Validate the case safety classification."""
    safety = require_mapping(case.get("safety"), f"{case_id}.safety")
    destructive = require_bool(safety.get("destructive"), f"{case_id}.safety.destructive")
    requires_isolated_lrs = require_bool(
        safety.get("requires_isolated_lrs"),
        f"{case_id}.safety.requires_isolated_lrs",
    )
    if destructive and not requires_isolated_lrs:
        raise ConformanceError(f"{path} {case_id} destructive cases must require an isolated LRS")
    return {
        "destructive": destructive,
        "requires_isolated_lrs": requires_isolated_lrs,
    }


def require_bool(value: Any, label: str) -> bool:
    """Return a bool or raise a conformance error."""
    if not isinstance(value, bool):
        raise ConformanceError(f"{label} must be a boolean")
    return value


def validate_case_endpoint_profiles(path: Path, case_id: str, case: dict[str, Any]) -> list[str]:
    """Validate endpoint profile requirements for one case."""
    profiles: list[str] = []
    seen: set[str] = set()
    raw_profiles = (
        require_list(
            case.get("requires_endpoint_profiles"),
            f"{case_id}.requires_endpoint_profiles",
        )
        if "requires_endpoint_profiles" in case
        else []
    )
    for index, profile in enumerate(raw_profiles):
        if not isinstance(profile, str) or not profile:
            raise ConformanceError(
                f"{path} {case_id}.requires_endpoint_profiles[{index}] must be non-empty"
            )
        if profile not in VALID_ENDPOINT_PROFILES:
            raise ConformanceError(
                f"{path} {case_id} references unsupported endpoint profile: {profile}"
            )
        if profile in seen:
            raise ConformanceError(f"{path} {case_id} repeats endpoint profile: {profile}")
        seen.add(profile)
        profiles.append(profile)
    return profiles


def index_cases_by_requirement(case_rows: list[dict[str, Any]]) -> dict[str, list[str]]:
    """Return requirement ids mapped to case ids."""
    cases_by_requirement: dict[str, list[str]] = {}
    for case in case_rows:
        for requirement in case["requirements"]:
            cases_by_requirement.setdefault(requirement, []).append(case["id"])
    return cases_by_requirement


def load_gap_rows(
    path: Path,
    requirement_by_id: dict[str, dict[str, Any]],
    uncovered_requirement_ids: set[str],
) -> list[dict[str, Any]]:
    """Load endpoint-profile-sensitive gap declarations."""
    document = load_yaml(path)
    if document.get("standard") != STANDARD:
        raise ConformanceError(f"{path}.standard must be {STANDARD}")
    if document.get("requirement_catalog") != str(DEFAULT_REQUIREMENT_CATALOG):
        raise ConformanceError(f"{path}.requirement_catalog must be {DEFAULT_REQUIREMENT_CATALOG}")

    rows: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, gap in enumerate(require_list(document.get("gaps"), f"{path}.gaps")):
        gap_map = require_mapping(gap, f"{path}.gaps[{index}]")
        requirement_id = require_non_empty_string(
            gap_map.get("requirement"),
            f"{path}.gaps[{index}].requirement",
        )
        if requirement_id in seen:
            raise ConformanceError(f"{path} repeats gap requirement: {requirement_id}")
        seen.add(requirement_id)
        if requirement_id not in requirement_by_id:
            raise ConformanceError(f"{path} references unknown requirement: {requirement_id}")
        if requirement_id not in uncovered_requirement_ids:
            raise ConformanceError(f"{path} gap has executable cases already: {requirement_id}")

        classification = require_non_empty_string(
            gap_map.get("classification"),
            f"{path}.gaps[{index}].classification",
        )
        if classification not in VALID_GAP_CLASSIFICATIONS:
            raise ConformanceError(
                f"{path} {requirement_id}.classification must be one of "
                f"{sorted(VALID_GAP_CLASSIFICATIONS)}"
            )
        rationale = require_non_empty_string(
            gap_map.get("rationale"),
            f"{path}.gaps[{index}].rationale",
        )
        required_profile = require_non_empty_string(
            gap_map.get("required_profile"),
            f"{path}.gaps[{index}].required_profile",
        )
        requirement = requirement_by_id[requirement_id]
        rows.append(
            {
                "id": requirement_id,
                "clause": requirement.get("clause"),
                "area": requirement.get("area"),
                "projection": requirement.get("projection"),
                "summary": requirement.get("summary"),
                "classification": classification,
                "required_profile": required_profile,
                "rationale": rationale,
            }
        )
    return rows


def requirement_row(
    requirement_id: str,
    requirement_by_id: dict[str, dict[str, Any]],
    cases_by_requirement: dict[str, list[str]],
) -> dict[str, Any]:
    """Build one requirement traceability row."""
    requirement = requirement_by_id[requirement_id]
    return {
        "id": requirement_id,
        "clause": requirement.get("clause"),
        "area": requirement.get("area"),
        "projection": requirement.get("projection"),
        "summary": requirement.get("summary"),
        "cases": sorted(cases_by_requirement.get(requirement_id, [])),
    }


def print_markdown(report: dict[str, Any]) -> None:
    """Print a compact Markdown case matrix report."""
    totals = report["totals"]
    print(f"# {report['standard']} Conformance Case Matrix")
    print()
    print(f"- Requirement catalog: `{report['requirement_catalog']}`")
    print(f"- Executable gap map: `{report['executable_gap_map']}`")
    print(f"- Case roots: {', '.join(f'`{root}`' for root in report['case_roots'])}")
    print(f"- Requirements: {totals['requirements']}")
    print(f"- Requirements with executable cases: {totals['requirements_with_cases']}")
    print(f"- Requirements without executable cases: {totals['requirements_without_cases']}")
    print(
        "- Non-executable endpoint-profile-sensitive requirements: "
        f"{totals['profile_sensitive_requirements']}"
    )
    print(f"- Unclassified uncovered requirements: {totals['unclassified_uncovered_requirements']}")
    print(f"- Endpoint-profile-required executable cases: {totals['endpoint_profile_required_cases']}")
    print(f"- Suites: {totals['suites']}")
    print(f"- Cases: {totals['cases']}")
    print(f"- Destructive cases: {totals['destructive_cases']}")
    print(f"- Low-side-effect cases: {totals['low_side_effect_cases']}")
    print()
    print("## Non-Executable Endpoint-Profile-Sensitive Requirements")
    print()
    print("| Requirement | Clause | Required Profile | Rationale |")
    print("| --- | --- | --- | --- |")
    for row in report["profile_sensitive_requirements"]:
        print(
            "| {id} | {clause} | {required_profile} | {rationale} |".format(
                id=row["id"],
                clause=row.get("clause") or "",
                required_profile=row["required_profile"],
                rationale=escape_markdown_table_text(row["rationale"]),
            )
        )
    print()
    print("## Endpoint-Profile-Required Executable Cases")
    print()
    print("| Case | Suite | Profiles | Requirements |")
    print("| --- | --- | --- | --- |")
    for row in report["endpoint_profile_required_cases"]:
        print(
            "| {id} | {suite} | {profiles} | {requirements} |".format(
                id=row["id"],
                suite=row["suite"],
                profiles=", ".join(
                    f"`{profile}`" for profile in row["requires_endpoint_profiles"]
                ),
                requirements=", ".join(f"`{requirement}`" for requirement in row["requirements"]),
            )
        )
    print()
    print("## Requirements Without Executable Cases")
    print()
    print("| Requirement | Clause | Area | Projection | Summary |")
    print("| --- | --- | --- | --- | --- |")
    for row in report["uncovered_requirements"]:
        print(
            "| {id} | {clause} | {area} | {projection} | {summary} |".format(
                id=row["id"],
                clause=row.get("clause") or "",
                area=row.get("area") or "",
                projection=row.get("projection") or "",
                summary=escape_markdown_table_text(row.get("summary") or ""),
            )
        )


def escape_markdown_table_text(value: str) -> str:
    """Escape Markdown table-sensitive characters."""
    return value.replace("|", "\\|").replace("\n", " ")


def main() -> int:
    """Run the case matrix reporter."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--case-root", type=Path, action="append", default=[])
    parser.add_argument("--requirements", type=Path, default=DEFAULT_REQUIREMENT_CATALOG)
    parser.add_argument("--gap-map", type=Path, default=DEFAULT_GAP_MAP)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    report = build_report(args.case_root, args.requirements, args.gap_map)
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_markdown(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
