#!/usr/bin/env python3
"""Generate normalized CI check catalog and gate-plan ledgers."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.ci_normal_form import CI_NORMAL_FORM_SYNC_POLICY
from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string
from kiva_ci.projection import require_envelope
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_SOURCE = Path("spec/ci/evidence-checks.source.yaml")
DEFAULT_GROUP_SOURCE = Path("spec/ci/evidence-check-groups.source.yaml")
DEFAULT_CATALOG = Path("spec/ci/evidence-check-catalog.source.yaml")
DEFAULT_GATE_PLAN = Path("spec/ci/evidence-gate-plan.source.yaml")


class EvidenceCheckCatalogError(RuntimeError):
    """Raised when CI check catalog generation fails."""


def evidence_groups(source: dict[str, Any], source_path: Path) -> list[dict[str, Any]]:
    """Return source CI groups with validation."""
    if source.get("schema_version") != 1:
        raise EvidenceCheckCatalogError(f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-ci-evidence-check-source":
        raise EvidenceCheckCatalogError(f"{source_path}.kind must be kiva-ci-evidence-check-source")
    return [
        require_mapping(group, f"{source_path}.groups[{index}]", EvidenceCheckCatalogError)
        for index, group in enumerate(
            require_list(source.get("groups"), f"{source_path}.groups", EvidenceCheckCatalogError)
        )
    ]


def group_metadata(group_source: dict[str, Any], source_path: Path) -> dict[str, dict[str, str]]:
    """Return required CI group metadata keyed by group id."""
    require_envelope(
        group_source,
        path=source_path,
        kind="kiva-ci-evidence-check-groups-source",
        error_cls=EvidenceCheckCatalogError,
    )
    rows: dict[str, dict[str, str]] = {}
    for index, raw_group in enumerate(
        require_list(group_source.get("groups"), f"{source_path}.groups", EvidenceCheckCatalogError)
    ):
        group = require_mapping(raw_group, f"{source_path}.groups[{index}]", EvidenceCheckCatalogError)
        group_id = require_string(group.get("id"), f"{source_path}.groups[{index}].id", EvidenceCheckCatalogError)
        if group_id in rows:
            raise EvidenceCheckCatalogError(f"duplicate CI group id: {group_id}")
        rows[group_id] = {
            "nix_gate": require_string(
                group.get("nix_gate"),
                f"{source_path}.groups[{index}].nix_gate",
                EvidenceCheckCatalogError,
            ),
            "claim_boundary": require_string(
                group.get("claim_boundary"),
                f"{source_path}.groups[{index}].claim_boundary",
                EvidenceCheckCatalogError,
            ),
        }
    return rows


def check_catalog(source_path: Path, groups: list[dict[str, Any]]) -> dict[str, Any]:
    """Return the normalized CI check catalog."""
    rows = []
    seen: set[tuple[str, str]] = set()
    for group_index, group in enumerate(groups):
        group_id = require_string(group.get("id"), f"groups[{group_index}].id", EvidenceCheckCatalogError)
        for check_index, raw_check in enumerate(
            require_list(group.get("checks"), f"groups[{group_index}].checks", EvidenceCheckCatalogError)
        ):
            check = require_mapping(
                raw_check,
                f"groups[{group_index}].checks[{check_index}]",
                EvidenceCheckCatalogError,
            )
            check_id = require_string(check.get("id"), f"{group_id}.checks[{check_index}].id", EvidenceCheckCatalogError)
            key = (group_id, check_id)
            if key in seen:
                raise EvidenceCheckCatalogError(f"duplicate CI check row: {group_id}/{check_id}")
            seen.add(key)
            row = {"group": group_id, **check}
            rows.append(row)
    return {
        "schema_version": 1,
        "kind": "kiva-ci-evidence-check-catalog-source",
        "status": "active",
        "description": "Primary CI CheckCatalog rows for registry-driven evidence execution.",
        "compatibility_view": source_path.as_posix(),
        "synchronization_policy": CI_NORMAL_FORM_SYNC_POLICY,
        "checks": rows,
    }


def gate_plan(
    source_path: Path,
    group_source_path: Path,
    groups: list[dict[str, Any]],
    metadata: dict[str, dict[str, str]],
) -> dict[str, Any]:
    """Return the normalized CI gate plan."""
    plan_groups = []
    seen: set[str] = set()
    for group_index, group in enumerate(groups):
        group_id = require_string(group.get("id"), f"groups[{group_index}].id", EvidenceCheckCatalogError)
        if group_id in seen:
            raise EvidenceCheckCatalogError(f"duplicate gate plan group id: {group_id}")
        seen.add(group_id)
        group_meta = metadata.get(group_id)
        if group_meta is None:
            raise EvidenceCheckCatalogError(f"gate plan group is not declared in group source: {group_id}")
        check_ids = [
            require_string(
                require_mapping(check, f"{group_id}.checks[{check_index}]", EvidenceCheckCatalogError).get("id"),
                f"{group_id}.checks[{check_index}].id",
                EvidenceCheckCatalogError,
            )
            for check_index, check in enumerate(
                require_list(group.get("checks"), f"groups[{group_index}].checks", EvidenceCheckCatalogError)
            )
        ]
        plan_groups.append(
            {
                "id": group_id,
                "description": require_string(
                    group.get("description"),
                    f"groups[{group_index}].description",
                    EvidenceCheckCatalogError,
                ),
                **group_meta,
                "check_order": check_ids,
            }
        )
    missing = sorted(set(metadata).difference(seen))
    if missing:
        raise EvidenceCheckCatalogError(f"group source has no gate-plan rows for groups: {missing}")
    return {
        "schema_version": 1,
        "kind": "kiva-ci-evidence-gate-plan-source",
        "status": "active",
        "description": "Primary CI GatePlan rows for evidence group order, Nix gates, and claim boundaries.",
        "compatibility_view": source_path.as_posix(),
        "group_source": group_source_path.as_posix(),
        "synchronization_policy": CI_NORMAL_FORM_SYNC_POLICY,
        "groups": plan_groups,
    }


def rendered_artifacts(root: Path, source_path: Path, group_source_path: Path) -> list[GeneratedArtifact]:
    """Return rendered CI catalog and gate-plan artifacts."""
    source = load_yaml(root / source_path, EvidenceCheckCatalogError)
    group_source = load_yaml(root / group_source_path, EvidenceCheckCatalogError)
    groups = evidence_groups(source, source_path)
    metadata = group_metadata(group_source, group_source_path)
    return [
        GeneratedArtifact(root / DEFAULT_CATALOG, dump_yaml(check_catalog(source_path, groups))),
        GeneratedArtifact(root / DEFAULT_GATE_PLAN, dump_yaml(gate_plan(source_path, group_source_path, groups, metadata))),
    ]


def main() -> int:
    """Generate or validate normalized CI catalog and gate-plan ledgers."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--group-source", type=Path, default=DEFAULT_GROUP_SOURCE)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    artifacts = rendered_artifacts(root, args.source, args.group_source)
    command_hint = "scripts/ci/generate_evidence_check_catalog.py"
    try:
        write_or_check_generated_artifacts(
            artifacts,
            check=args.check,
            command_hint=command_hint,
            error_cls=EvidenceCheckCatalogError,
        )
    except EvidenceCheckCatalogError as error:
        raise SystemExit(str(error)) from error
    verb = "validated" if args.check else "generated"
    print(f"{verb} CI check catalog and gate plan: {DEFAULT_CATALOG}, {DEFAULT_GATE_PLAN}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
