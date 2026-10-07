#!/usr/bin/env python3
"""Validate Kiva conformance discharge plans."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import yaml


DEFAULT_REQUIREMENT_CATALOG = Path("spec/requirements/ieee-39274-1-1-2025.yaml")

VALID_METHODS = {
    "external-adl",
    "http-test",
    "openapi",
    "property-kani",
    "tamarin",
    "upstream-contribution",
}
VALID_STATUSES = {"active", "planned"}


class ConformanceError(RuntimeError):
    """Raised when a conformance discharge plan is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise ConformanceError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a conformance error."""
    if not isinstance(value, dict):
        raise ConformanceError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a list or raise a conformance error."""
    if not isinstance(value, list):
        raise ConformanceError(f"{label} must be a list")
    return value


def load_requirement_ids(path: Path) -> set[str]:
    """Load requirement ids from the requirement catalog."""
    catalog = load_yaml(path)
    requirements = require_list(catalog.get("requirements"), "requirements")

    ids: set[str] = set()
    for index, requirement in enumerate(requirements):
        requirement_map = require_mapping(requirement, f"requirements[{index}]")
        requirement_id = requirement_map.get("id")
        if not isinstance(requirement_id, str) or not requirement_id:
            raise ConformanceError(f"requirements[{index}].id must be a non-empty string")
        if requirement_id in ids:
            raise ConformanceError(f"duplicate requirement id: {requirement_id}")
        ids.add(requirement_id)
    return ids


def validate_external_suites(document: dict[str, Any]) -> set[str]:
    """Validate external suite declarations and return suite ids."""
    suite_ids: set[str] = set()
    for index, suite in enumerate(require_list(document.get("external_suites"), "external_suites")):
        suite_map = require_mapping(suite, f"external_suites[{index}]")
        suite_id = suite_map.get("id")
        if not isinstance(suite_id, str) or not suite_id:
            raise ConformanceError(f"external_suites[{index}].id must be a non-empty string")
        if suite_id in suite_ids:
            raise ConformanceError(f"duplicate external suite id: {suite_id}")
        for key in ("role", "upstream", "observed_commit", "license"):
            if key not in suite_map:
                raise ConformanceError(f"{suite_id} is missing {key}")
        suite_ids.add(suite_id)
    return suite_ids


def validate_discharge_plan(path: Path, requirement_ids: set[str]) -> None:
    """Validate a conformance discharge plan against a requirement catalog."""
    document = load_yaml(path)

    if document.get("requirement_catalog") != str(DEFAULT_REQUIREMENT_CATALOG):
        raise ConformanceError(f"requirement_catalog must be {DEFAULT_REQUIREMENT_CATALOG}")

    suite_ids = validate_external_suites(document)
    covered: set[str] = set()

    groups = require_list(document.get("discharge_groups"), "discharge_groups")
    if not groups:
        raise ConformanceError("discharge_groups must not be empty")

    group_ids: set[str] = set()
    for index, group in enumerate(groups):
        group_map = require_mapping(group, f"discharge_groups[{index}]")
        group_id = group_map.get("id")
        if not isinstance(group_id, str) or not group_id:
            raise ConformanceError(f"discharge_groups[{index}].id must be a non-empty string")
        if group_id in group_ids:
            raise ConformanceError(f"duplicate discharge group id: {group_id}")
        group_ids.add(group_id)

        method = group_map.get("method")
        if method not in VALID_METHODS:
            raise ConformanceError(f"{group_id}.method must be one of {sorted(VALID_METHODS)}")
        status = group_map.get("status")
        if status not in VALID_STATUSES:
            raise ConformanceError(f"{group_id}.status must be one of {sorted(VALID_STATUSES)}")

        if method == "external-adl":
            external_suite = group_map.get("external_suite")
            if external_suite not in suite_ids:
                raise ConformanceError(f"{group_id}.external_suite must reference an external_suites id")

        evidence = require_list(group_map.get("evidence"), f"{group_id}.evidence")
        if not evidence:
            raise ConformanceError(f"{group_id}.evidence must not be empty")

        requirements = require_list(group_map.get("requirements"), f"{group_id}.requirements")
        if not requirements:
            raise ConformanceError(f"{group_id}.requirements must not be empty")
        for requirement_id in requirements:
            if not isinstance(requirement_id, str) or not requirement_id:
                raise ConformanceError(f"{group_id}.requirements entries must be non-empty strings")
            if requirement_id not in requirement_ids:
                raise ConformanceError(f"{group_id} references unknown requirement: {requirement_id}")
            covered.add(requirement_id)

    missing = requirement_ids.difference(covered)
    if missing:
        missing_ids = ", ".join(sorted(missing))
        raise ConformanceError(f"requirements missing from discharge plan: {missing_ids}")


def main() -> int:
    """Run the conformance plan validator."""
    parser = argparse.ArgumentParser()
    parser.add_argument("plan", type=Path)
    parser.add_argument("--requirements", type=Path, default=DEFAULT_REQUIREMENT_CATALOG)
    args = parser.parse_args()

    requirement_ids = load_requirement_ids(args.requirements)
    validate_discharge_plan(args.plan, requirement_ids)
    print(f"validated {args.plan}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
