#!/usr/bin/env python3
"""Report Kiva conformance discharge coverage."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from validate_conformance import (
    DEFAULT_REQUIREMENT_CATALOG,
    load_yaml,
    require_list,
    require_mapping,
    validate_discharge_plan,
)

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
            raise RuntimeError(f"requirements[{index}].id must be a non-empty string")
        if requirement_id in seen:
            raise RuntimeError(f"duplicate requirement id: {requirement_id}")
        seen.add(requirement_id)
        loaded.append(requirement_map)
    return loaded


def build_report(plan_path: Path, catalog_path: Path) -> dict[str, Any]:
    """Build a JSON-serializable coverage report."""
    requirements = load_requirements(catalog_path)
    requirement_ids = {requirement["id"] for requirement in requirements}
    validate_discharge_plan(plan_path, requirement_ids)

    plan = load_yaml(plan_path)
    groups = [
        require_mapping(group, f"discharge_groups[{index}]")
        for index, group in enumerate(require_list(plan.get("discharge_groups"), "discharge_groups"))
    ]

    active_by_requirement = coverage_by_status(groups, "active")
    planned_by_requirement = coverage_by_status(groups, "planned")
    rows = []
    for requirement in requirements:
        requirement_id = requirement["id"]
        active_groups = active_by_requirement.get(requirement_id, [])
        planned_groups = planned_by_requirement.get(requirement_id, [])
        status = "active" if active_groups else "planned"
        rows.append(
            {
                "id": requirement_id,
                "clause": requirement.get("clause"),
                "level": requirement.get("level"),
                "area": requirement.get("area"),
                "projection": requirement.get("projection"),
                "summary": requirement.get("summary"),
                "status": status,
                "active_groups": active_groups,
                "planned_groups": planned_groups,
            }
        )

    active = [row for row in rows if row["status"] == "active"]
    planned_only = [row for row in rows if row["status"] == "planned"]
    return {
        "schema_version": REPORT_SCHEMA_VERSION,
        "standard": plan.get("standard"),
        "requirement_catalog": str(catalog_path),
        "discharge_plan": str(plan_path),
        "totals": {
            "requirements": len(rows),
            "active": len(active),
            "planned_only": len(planned_only),
        },
        "active_by_method": count_active_methods(groups),
        "planned_only": planned_only,
        "requirements": rows,
    }


def coverage_by_status(groups: list[dict[str, Any]], status: str) -> dict[str, list[str]]:
    """Return requirement ids mapped to discharge group ids for a status."""
    coverage: dict[str, list[str]] = {}
    for group in groups:
        if group.get("status") != status:
            continue
        group_id = group["id"]
        for requirement_id in require_list(group.get("requirements"), f"{group_id}.requirements"):
            coverage.setdefault(requirement_id, []).append(group_id)
    return coverage


def count_active_methods(groups: list[dict[str, Any]]) -> dict[str, int]:
    """Count unique active requirements by discharge method."""
    methods: dict[str, set[str]] = {}
    for group in groups:
        if group.get("status") != "active":
            continue
        method = group["method"]
        group_id = group["id"]
        requirements = require_list(group.get("requirements"), f"{group_id}.requirements")
        methods.setdefault(method, set()).update(requirements)
    return {method: len(requirements) for method, requirements in sorted(methods.items())}


def print_markdown(report: dict[str, Any]) -> None:
    """Print a compact Markdown coverage report."""
    totals = report["totals"]
    print(f"# {report['standard']} Conformance Coverage")
    print()
    print(f"- Requirement catalog: `{report['requirement_catalog']}`")
    print(f"- Discharge plan: `{report['discharge_plan']}`")
    print(f"- Requirements: {totals['requirements']}")
    print(f"- Active evidence: {totals['active']}")
    print(f"- Planned only: {totals['planned_only']}")
    print()
    print("## Active Evidence By Method")
    print()
    for method, count in report["active_by_method"].items():
        print(f"- `{method}`: {count}")
    print()
    print("## Planned-Only Requirements")
    print()
    print("| Requirement | Clause | Area | Projection | Summary |")
    print("| --- | --- | --- | --- | --- |")
    for row in report["planned_only"]:
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
    """Run the coverage reporter."""
    parser = argparse.ArgumentParser()
    parser.add_argument("plan", type=Path)
    parser.add_argument("--requirements", type=Path, default=DEFAULT_REQUIREMENT_CATALOG)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    report = build_report(args.plan, args.requirements)
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_markdown(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
