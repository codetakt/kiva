#!/usr/bin/env python3
"""Generate joined PDS plane packages from operation-owned PDS ledgers."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_PLANES = Path("spec/pds/core-planes.source.yaml")
DEFAULT_INVARIANTS = Path("spec/pds/invariants.source.yaml")
DEFAULT_STATE_MACHINES = Path("spec/pds/state-machines.source.yaml")
DEFAULT_RESOURCE_FAMILIES = Path("generated/architecture/pds-resource-families.yaml")
DEFAULT_OUTPUT = Path("generated/architecture/pds-plane-packages.yaml")


class PdsPlanePackageError(RuntimeError):
    """Raised when PDS plane packages cannot be generated."""


def rows_by_id(document: dict[str, Any], field: str, label: str) -> dict[str, dict[str, Any]]:
    """Return unique rows keyed by id."""
    rows: dict[str, dict[str, Any]] = {}
    for index, raw_row in enumerate(require_list(document.get(field), f"{label}.{field}", PdsPlanePackageError)):
        row = require_mapping(raw_row, f"{label}.{field}[{index}]", PdsPlanePackageError)
        row_id = require_string(row.get("id"), f"{label}.{field}[{index}].id", PdsPlanePackageError)
        if row_id in rows:
            raise PdsPlanePackageError(f"duplicate {label} id: {row_id}")
        rows[row_id] = row
    return rows


def referenced_rows(refs: Any, rows: dict[str, dict[str, Any]], label: str) -> list[dict[str, Any]]:
    """Resolve a list of ids to source rows."""
    resolved: list[dict[str, Any]] = []
    for index, value in enumerate(require_list(refs, label, PdsPlanePackageError)):
        row_id = require_string(value, f"{label}[{index}]", PdsPlanePackageError)
        row = rows.get(row_id)
        if row is None:
            raise PdsPlanePackageError(f"{label} references missing row: {row_id}")
        resolved.append(row)
    return resolved


def document(
    root: Path,
    planes_path: Path,
    invariants_path: Path,
    state_machines_path: Path,
    resource_families_path: Path,
    output_path: Path,
) -> dict[str, Any]:
    """Return the generated PDS plane package index."""
    planes = load_yaml(root / planes_path, PdsPlanePackageError)
    invariants = rows_by_id(load_yaml(root / invariants_path, PdsPlanePackageError), "invariants", "invariants")
    state_machines = rows_by_id(load_yaml(root / state_machines_path, PdsPlanePackageError), "machines", "state_machines")
    resource_families = rows_by_id(
        load_yaml(root / resource_families_path, PdsPlanePackageError),
        "resource_families",
        "resource_families",
    )
    packages: list[dict[str, Any]] = []
    for index, raw_plane in enumerate(require_list(planes.get("planes"), "planes.planes", PdsPlanePackageError)):
        plane = require_mapping(raw_plane, f"planes.planes[{index}]", PdsPlanePackageError)
        plane_id = require_string(plane.get("id"), f"planes.planes[{index}].id", PdsPlanePackageError)
        plane_invariants = referenced_rows(plane.get("invariant_refs"), invariants, f"plane.{plane_id}.invariant_refs")
        plane_machines = referenced_rows(plane.get("state_machine_refs"), state_machines, f"plane.{plane_id}.state_machine_refs")
        plane_families = referenced_rows(plane.get("resource_family_refs"), resource_families, f"plane.{plane_id}.resource_family_refs")
        packages.append(
            {
                "id": plane_id,
                "package_type": "PdsPlanePackage",
                "core_concept": require_string(plane.get("core_concept"), f"plane.{plane_id}.core_concept", PdsPlanePackageError),
                "responsibilities": list(require_list(plane.get("owns"), f"plane.{plane_id}.owns", PdsPlanePackageError)),
                "milestone_refs": list(require_list(plane.get("milestone_refs"), f"plane.{plane_id}.milestone_refs", PdsPlanePackageError)),
                "invariants": [
                    {
                        "id": row["id"],
                        "rule": row["rule"],
                        "required_checks": list(row.get("required_checks", [])),
                    }
                    for row in plane_invariants
                ],
                "state_machines": [
                    {
                        "id": row["id"],
                        "states": list(row.get("states", [])),
                        "terminal_states": list(row.get("terminal_states", [])),
                        "transition_count": len(require_list(row.get("transitions"), f"state_machine.{row['id']}.transitions", PdsPlanePackageError)),
                    }
                    for row in plane_machines
                ],
                "resource_families": [
                    {
                        "id": row["id"],
                        "description": row["description"],
                        "operation_count": len(require_list(row.get("operations"), f"resource_family.{row['id']}.operations", PdsPlanePackageError)),
                        "implementation_file_count": len(require_list(row.get("implementation_files"), f"resource_family.{row['id']}.implementation_files", PdsPlanePackageError)),
                    }
                    for row in plane_families
                ],
                "implementation_anchors": list(require_list(plane.get("implementation_anchors"), f"plane.{plane_id}.implementation_anchors", PdsPlanePackageError)),
                "required_checks": list(require_list(plane.get("required_checks"), f"plane.{plane_id}.required_checks", PdsPlanePackageError)),
            }
        )
    return {
        "schema_version": 1,
        "kind": "kiva-pds-plane-package-index",
        "status": "active",
        "description": "Generated PdsPlanePackage review view joining operation-owned planes, invariants, state machines, resource families, implementation anchors, and checks.",
        "generated_from": [
            planes_path.as_posix(),
            invariants_path.as_posix(),
            state_machines_path.as_posix(),
            resource_families_path.as_posix(),
        ],
        "generated_artifacts": {"plane_packages": output_path.as_posix()},
        "package_count": len(packages),
        "packages": packages,
    }


def rendered_artifacts(
    root: Path,
    planes_path: Path,
    invariants_path: Path,
    state_machines_path: Path,
    resource_families_path: Path,
    output_path: Path,
) -> list[GeneratedArtifact]:
    """Return generated PDS plane package artifacts."""
    return [
        GeneratedArtifact(
            root / output_path,
            dump_yaml(document(root, planes_path, invariants_path, state_machines_path, resource_families_path, output_path)),
        )
    ]


def main() -> int:
    """Generate or check the PDS plane package index."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--planes", type=Path, default=DEFAULT_PLANES)
    parser.add_argument("--invariants", type=Path, default=DEFAULT_INVARIANTS)
    parser.add_argument("--state-machines", type=Path, default=DEFAULT_STATE_MACHINES)
    parser.add_argument("--resource-families", type=Path, default=DEFAULT_RESOURCE_FAMILIES)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    write_or_check_generated_artifacts(
        rendered_artifacts(root, args.planes, args.invariants, args.state_machines, args.resource_families, args.out),
        check=args.check,
        command_hint="scripts/ci/generate_pds_plane_packages.py",
        error_cls=PdsPlanePackageError,
    )
    print(f"{'validated' if args.check else 'generated'} PDS plane packages: {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
