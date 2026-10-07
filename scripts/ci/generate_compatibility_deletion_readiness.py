#!/usr/bin/env python3
"""Generate the compatibility-surface deletion-readiness report."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.compatibility import validate_compatibility_surface_ledgers
from kiva_ci.manifest import (
    load_yaml,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
)
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_SOURCE = Path("spec/architecture/compatibility-surfaces.yaml")
DEFAULT_OUTPUT = Path("generated/architecture/compatibility-deletion-readiness.yaml")


class CompatibilityDeletionReadinessError(RuntimeError):
    """Raised when compatibility deletion-readiness cannot be generated."""


def gate(
    gate_id: str,
    status: str,
    evidence: list[str] | None = None,
) -> dict[str, Any]:
    """Return a stable readiness gate row."""
    row: dict[str, Any] = {
        "id": gate_id,
        "status": status,
    }
    if evidence is not None:
        row["evidence"] = evidence
    return row


def owner_anchor_gate(root: Path, entry: dict[str, Any], entry_id: str) -> dict[str, Any]:
    """Return the owner-anchor gate for one retained compatibility surface."""
    owner = require_string(entry.get("owner"), f"{entry_id}.owner", CompatibilityDeletionReadinessError)
    owner_text = (root / owner).read_text(encoding="utf-8")
    retirement_check = require_mapping(
        entry.get("retirement_check"),
        f"{entry_id}.retirement_check",
        CompatibilityDeletionReadinessError,
    )
    anchors = require_string_list(
        retirement_check.get("anchors"),
        f"{entry_id}.retirement_check.anchors",
        CompatibilityDeletionReadinessError,
    )
    present = [anchor for anchor in anchors if anchor in owner_text]
    missing = sorted(set(anchors).difference(present))
    if missing:
        return gate("owner-anchor-present", "blocked", missing)
    return gate("owner-anchor-present", "passed", present)


def compatibility_entry_readiness(
    root: Path,
    entry: dict[str, Any],
    index: int,
) -> dict[str, Any]:
    """Project one compatibility-surface entry into deletion-readiness state."""
    entry_id = require_string(
        entry.get("id"),
        f"entries[{index}].id",
        CompatibilityDeletionReadinessError,
    )
    status = require_string(
        entry.get("status"),
        f"{entry_id}.status",
        CompatibilityDeletionReadinessError,
    )
    next_status = entry.get("next_status")
    if next_status is not None and not isinstance(next_status, str):
        raise CompatibilityDeletionReadinessError(f"{entry_id}.next_status must be a string")
    replacement_paths = require_string_list(
        entry.get("replacement_paths", []),
        f"{entry_id}.replacement_paths",
        CompatibilityDeletionReadinessError,
        allow_empty=True,
    )
    forbidden_check = require_mapping(
        entry.get("forbidden_call_site_check"),
        f"{entry_id}.forbidden_call_site_check",
        CompatibilityDeletionReadinessError,
    )
    search_roots = require_string_list(
        forbidden_check.get("search_roots"),
        f"{entry_id}.forbidden_call_site_check.search_roots",
        CompatibilityDeletionReadinessError,
    )
    forbidden_patterns = require_string_list(
        forbidden_check.get("forbidden_patterns"),
        f"{entry_id}.forbidden_call_site_check.forbidden_patterns",
        CompatibilityDeletionReadinessError,
    )

    blockers: list[str] = []
    readiness = "not-applicable"
    if status == "retained":
        readiness = "ready-to-delete" if next_status == "deletion-ready" else "blocked"
        if next_status != "deletion-ready":
            blockers.append("next_status is not deletion-ready")
    elif status == "deletion-ready":
        readiness = "ready-to-delete"
    elif status == "removed":
        readiness = "removed"
    else:
        blockers.append(f"unsupported status: {status}")
        readiness = "blocked"

    gates = (
        [gate("removed-surface-recorded", "passed")]
        if status == "removed"
        else [
            owner_anchor_gate(root, entry, entry_id),
            gate("allowed-owner-exports-confined", "passed"),
            gate("forbidden-call-sites-absent", "passed", search_roots),
            gate("replacement-paths-valid", "passed", replacement_paths),
        ]
    )
    if any(row["status"] != "passed" for row in gates) and readiness == "ready-to-delete":
        readiness = "blocked"
        blockers.extend(row["id"] for row in gates if row["status"] != "passed")

    return {
        "id": entry_id,
        "owner": require_string(entry.get("owner"), f"{entry_id}.owner", CompatibilityDeletionReadinessError),
        "status": status,
        "next_status": next_status,
        "readiness": readiness,
        "migration_target": require_string(
            entry.get("migration_target"),
            f"{entry_id}.migration_target",
            CompatibilityDeletionReadinessError,
        ),
        "retirement_milestone": require_string(
            entry.get("retirement_milestone"),
            f"{entry_id}.retirement_milestone",
            CompatibilityDeletionReadinessError,
        ),
        "retained_surface_count": len(
            require_string_list(
                entry.get("retained_surface"),
                f"{entry_id}.retained_surface",
                CompatibilityDeletionReadinessError,
            )
        ),
        "forbidden_pattern_count": len(forbidden_patterns),
        "blockers": blockers,
        "gates": gates,
    }


def deletion_readiness_document(root: Path, source_path: Path) -> dict[str, Any]:
    """Return the compatibility-surface deletion-readiness report."""
    validate_compatibility_surface_ledgers(
        root,
        {"compatibility_surface_ledgers": [source_path.as_posix()]},
        CompatibilityDeletionReadinessError,
    )
    source = load_yaml(root / source_path, CompatibilityDeletionReadinessError)
    if source.get("kind") != "kiva-architecture-compatibility-surface-ledger":
        raise CompatibilityDeletionReadinessError(
            f"{source_path}.kind must be kiva-architecture-compatibility-surface-ledger"
        )
    entries = [
        compatibility_entry_readiness(
            root,
            require_mapping(value, f"entries[{index}]", CompatibilityDeletionReadinessError),
            index,
        )
        for index, value in enumerate(
            require_list(source.get("entries"), f"{source_path}.entries", CompatibilityDeletionReadinessError)
        )
    ]
    ready_entries = [entry for entry in entries if entry["readiness"] == "ready-to-delete"]
    blocked_entries = [entry for entry in entries if entry["readiness"] == "blocked"]
    return {
        "schema_version": 1,
        "kind": "kiva-compatibility-deletion-readiness-report",
        "status": "active",
        "source_ledger": source_path.as_posix(),
        "summary": {
            "entry_count": len(entries),
            "retained_count": sum(1 for entry in entries if entry["status"] == "retained"),
            "deletion_ready_count": len(ready_entries),
            "blocked_count": len(blocked_entries),
            "removed_count": sum(1 for entry in entries if entry["status"] == "removed"),
        },
        "entries": entries,
    }


def render_deletion_readiness(root: Path, source_path: Path) -> str:
    """Render the expected deletion-readiness report YAML."""
    return dump_yaml(deletion_readiness_document(root, source_path))


def main() -> int:
    """Generate or validate the compatibility deletion-readiness report."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    rendered = render_deletion_readiness(root, args.source)
    try:
        write_or_check_generated_artifacts(
            [GeneratedArtifact(root / args.out, rendered)],
            check=args.check,
            command_hint="scripts/ci/generate_compatibility_deletion_readiness.py",
            error_cls=CompatibilityDeletionReadinessError,
        )
    except CompatibilityDeletionReadinessError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated compatibility deletion-readiness report: {args.out}")
        return 0
    print(f"generated compatibility deletion-readiness report: {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
