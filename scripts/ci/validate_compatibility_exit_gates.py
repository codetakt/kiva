#!/usr/bin/env python3
"""Validate every retained compatibility surface has an executable exit gate."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string


DEFAULT_LEDGER = Path("spec/architecture/compatibility-surfaces.yaml")
REQUIRED_ENTRY_FIELDS = {
    "id",
    "owner",
    "status",
    "reason",
    "migration_target",
    "expansion_policy",
    "retirement_milestone",
    "retirement_condition",
    "retirement_check",
    "allowed_owner_exports",
    "forbidden_call_site_check",
}


class CompatibilityExitGateError(RuntimeError):
    """Raised when compatibility exit-gate validation fails."""


def validate_entry(entry: dict[str, Any], label: str, root: Path) -> None:
    """Validate one compatibility entry has a complete exit contract."""
    missing = sorted(field for field in REQUIRED_ENTRY_FIELDS if field not in entry)
    if missing:
        raise CompatibilityExitGateError(f"{label} missing fields: {missing}")
    owner = require_string(entry.get("owner"), f"{label}.owner", CompatibilityExitGateError)
    status = require_string(entry.get("status"), f"{label}.status", CompatibilityExitGateError)
    if status == "removed":
        return
    if status != "retained":
        raise CompatibilityExitGateError(f"{label}.status must be retained or removed")
    if not (root / owner).exists():
        raise CompatibilityExitGateError(f"{label}.owner does not exist: {owner}")
    if require_string(entry.get("next_status"), f"{label}.next_status", CompatibilityExitGateError) != "deletion-ready":
        raise CompatibilityExitGateError(f"{label}.next_status must be deletion-ready")
    retirement_check = require_mapping(entry.get("retirement_check"), f"{label}.retirement_check", CompatibilityExitGateError)
    check_type = require_string(retirement_check.get("type"), f"{label}.retirement_check.type", CompatibilityExitGateError)
    if check_type != "owner-anchor-present":
        raise CompatibilityExitGateError(f"{label}.retirement_check.type must be owner-anchor-present")
    anchors = require_list(retirement_check.get("anchors"), f"{label}.retirement_check.anchors", CompatibilityExitGateError)
    if not anchors:
        raise CompatibilityExitGateError(f"{label}.retirement_check.anchors must not be empty")
    forbidden = require_mapping(entry.get("forbidden_call_site_check"), f"{label}.forbidden_call_site_check", CompatibilityExitGateError)
    if require_string(forbidden.get("type"), f"{label}.forbidden_call_site_check.type", CompatibilityExitGateError) != "rust-use-absent":
        raise CompatibilityExitGateError(f"{label}.forbidden_call_site_check.type must be rust-use-absent")
    if not require_list(forbidden.get("search_roots"), f"{label}.forbidden_call_site_check.search_roots", CompatibilityExitGateError):
        raise CompatibilityExitGateError(f"{label}.forbidden_call_site_check.search_roots must not be empty")
    if not require_list(forbidden.get("forbidden_patterns"), f"{label}.forbidden_call_site_check.forbidden_patterns", CompatibilityExitGateError):
        raise CompatibilityExitGateError(f"{label}.forbidden_call_site_check.forbidden_patterns must not be empty")


def validate(root: Path, ledger_path: Path) -> int:
    """Validate the compatibility ledger exit gates."""
    ledger = load_yaml(root / ledger_path, CompatibilityExitGateError)
    if ledger.get("kind") != "kiva-architecture-compatibility-surface-ledger":
        raise CompatibilityExitGateError(f"{ledger_path}.kind must be kiva-architecture-compatibility-surface-ledger")
    entries = require_list(ledger.get("entries"), f"{ledger_path}.entries", CompatibilityExitGateError)
    seen: set[str] = set()
    retained_count = 0
    for index, raw_entry in enumerate(entries):
        entry = require_mapping(raw_entry, f"{ledger_path}.entries[{index}]", CompatibilityExitGateError)
        entry_id = require_string(entry.get("id"), f"{ledger_path}.entries[{index}].id", CompatibilityExitGateError)
        if entry_id in seen:
            raise CompatibilityExitGateError(f"duplicate compatibility entry id: {entry_id}")
        seen.add(entry_id)
        validate_entry(entry, f"{ledger_path}.entries[{index}]", root)
        if entry.get("status") == "retained":
            retained_count += 1
    return retained_count


def main() -> int:
    """Run compatibility exit-gate validation."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--ledger", type=Path, default=DEFAULT_LEDGER)
    args = parser.parse_args()

    count = validate(args.root.resolve(), args.ledger)
    print(f"validated compatibility exit gates: {count} retained surfaces")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
