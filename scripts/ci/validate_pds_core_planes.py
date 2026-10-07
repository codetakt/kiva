#!/usr/bin/env python3
"""Validate the four-plane PDS core model."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.evidence import load_evidence_check_manifest
from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string, require_string_list, validation_error


DEFAULT_SOURCE = Path("spec/pds/core-planes.source.yaml")
DEFAULT_MILESTONES = Path("spec/pds/publication-milestones.yaml")
DEFAULT_EVIDENCE_MANIFEST = Path("spec/ci/evidence-checks.yaml")
DEFAULT_PDS_FAMILIES = Path("generated/architecture/pds-resource-families.yaml")
DEFAULT_PDS_INVARIANTS = Path("spec/pds/invariants.source.yaml")
DEFAULT_PDS_STATE_MACHINES = Path("spec/pds/state-machines.source.yaml")
DEFAULT_PDS_PLANE_PACKAGES = Path("generated/architecture/pds-plane-packages.yaml")
EXPECTED_PLANES = ["event", "inventory", "policy", "release"]


class PdsCorePlanesError(RuntimeError):
    """Raised when the PDS core-plane model is inconsistent."""


def check_ids(manifest: dict[str, Any]) -> set[str]:
    """Return all evidence-check ids from the generated manifest."""
    ids: set[str] = set()
    for group_index, raw_group in enumerate(require_list(manifest.get("groups"), "groups", PdsCorePlanesError)):
        group = require_mapping(raw_group, f"groups[{group_index}]", PdsCorePlanesError)
        group_id = require_string(group.get("id"), f"groups[{group_index}].id", PdsCorePlanesError)
        for check_index, raw_check in enumerate(
            require_list(group.get("checks"), f"groups.{group_id}.checks", PdsCorePlanesError)
        ):
            check = require_mapping(raw_check, f"groups.{group_id}.checks[{check_index}]", PdsCorePlanesError)
            ids.add(require_string(check.get("id"), f"groups.{group_id}.checks[{check_index}].id", PdsCorePlanesError))
    return ids


def milestone_ids(milestones: dict[str, Any], milestones_path: Path) -> set[str]:
    """Return milestone ids from the PDS publication milestone mirror."""
    if milestones.get("kind") != "pds-publication-milestone-mirror":
        raise validation_error(PdsCorePlanesError, f"{milestones_path}.kind must be pds-publication-milestone-mirror")
    ids: set[str] = set()
    for index, raw_milestone in enumerate(
        require_list(milestones.get("milestones"), f"{milestones_path}.milestones", PdsCorePlanesError)
    ):
        milestone = require_mapping(raw_milestone, f"{milestones_path}.milestones[{index}]", PdsCorePlanesError)
        milestone_id = require_string(milestone.get("id"), f"{milestones_path}.milestones[{index}].id", PdsCorePlanesError)
        if milestone_id in ids:
            raise validation_error(PdsCorePlanesError, f"duplicate PDS milestone id: {milestone_id}")
        ids.add(milestone_id)
    return ids


def pds_resource_family_ids(families: dict[str, Any], families_path: Path) -> set[str]:
    """Return generated PDS resource-family ids."""
    if families.get("kind") != "kiva-pds-resource-family-ledger":
        raise validation_error(PdsCorePlanesError, f"{families_path}.kind must be kiva-pds-resource-family-ledger")
    ids: set[str] = set()
    for index, raw_family in enumerate(
        require_list(families.get("resource_families"), f"{families_path}.resource_families", PdsCorePlanesError)
    ):
        family = require_mapping(raw_family, f"{families_path}.resource_families[{index}]", PdsCorePlanesError)
        family_id = require_string(family.get("id"), f"{families_path}.resource_families[{index}].id", PdsCorePlanesError)
        if family_id in ids:
            raise validation_error(PdsCorePlanesError, f"duplicate PDS resource family id: {family_id}")
        ids.add(family_id)
    return ids


def validate_envelope(source: dict[str, Any], source_path: Path) -> None:
    """Validate the source envelope and root model declaration."""
    if source.get("schema_version") != 1:
        raise validation_error(PdsCorePlanesError, f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-pds-core-planes-source":
        raise validation_error(PdsCorePlanesError, f"{source_path}.kind must be kiva-pds-core-planes-source")
    if source.get("status") != "active":
        raise validation_error(PdsCorePlanesError, f"{source_path}.status must be active")
    model = require_mapping(source.get("plane_model"), f"{source_path}.plane_model", PdsCorePlanesError)
    if model.get("root_type") != "PdsCorePlane":
        raise validation_error(PdsCorePlanesError, f"{source_path}.plane_model.root_type must be PdsCorePlane")
    planes = require_string_list(model.get("planes"), f"{source_path}.plane_model.planes", PdsCorePlanesError)
    if planes != EXPECTED_PLANES:
        raise validation_error(PdsCorePlanesError, f"{source_path}.plane_model.planes must be {EXPECTED_PLANES}")
    if model.get("invariant_source") != DEFAULT_PDS_INVARIANTS.as_posix():
        raise validation_error(
            PdsCorePlanesError,
            f"{source_path}.plane_model.invariant_source must be {DEFAULT_PDS_INVARIANTS.as_posix()}",
        )
    if model.get("state_machine_source") != DEFAULT_PDS_STATE_MACHINES.as_posix():
        raise validation_error(
            PdsCorePlanesError,
            f"{source_path}.plane_model.state_machine_source must be {DEFAULT_PDS_STATE_MACHINES.as_posix()}",
        )
    if model.get("package_projection") != DEFAULT_PDS_PLANE_PACKAGES.as_posix():
        raise validation_error(
            PdsCorePlanesError,
            f"{source_path}.plane_model.package_projection must be {DEFAULT_PDS_PLANE_PACKAGES.as_posix()}",
        )
    require_string(model.get("package_rule"), f"{source_path}.plane_model.package_rule", PdsCorePlanesError)


def invariant_refs_by_plane(
    invariants: dict[str, Any],
    invariants_path: Path,
) -> tuple[set[str], dict[str, list[str]]]:
    """Return invariant ids and their inverse mapping by PDS plane."""
    if invariants.get("kind") != "kiva-pds-invariants-source":
        raise validation_error(PdsCorePlanesError, f"{invariants_path}.kind must be kiva-pds-invariants-source")
    refs_by_plane: dict[str, list[str]] = {plane_id: [] for plane_id in EXPECTED_PLANES}
    invariant_ids: set[str] = set()
    for index, raw_invariant in enumerate(
        require_list(invariants.get("invariants"), f"{invariants_path}.invariants", PdsCorePlanesError)
    ):
        invariant = require_mapping(raw_invariant, f"{invariants_path}.invariants[{index}]", PdsCorePlanesError)
        invariant_id = require_string(
            invariant.get("id"),
            f"{invariants_path}.invariants[{index}].id",
            PdsCorePlanesError,
        )
        if invariant_id in invariant_ids:
            raise validation_error(PdsCorePlanesError, f"duplicate PDS invariant id: {invariant_id}")
        invariant_ids.add(invariant_id)
        for plane_id in require_string_list(
            invariant.get("plane_refs"),
            f"{invariants_path}.invariants[{index}].plane_refs",
            PdsCorePlanesError,
        ):
            if plane_id not in refs_by_plane:
                raise validation_error(PdsCorePlanesError, f"{invariant_id} references unknown PDS plane: {plane_id}")
            refs_by_plane[plane_id].append(invariant_id)
    return invariant_ids, refs_by_plane


def state_machine_refs_by_plane(
    state_machines: dict[str, Any],
    state_machines_path: Path,
) -> tuple[set[str], dict[str, list[str]]]:
    """Return state-machine ids and their inverse mapping by PDS plane."""
    if state_machines.get("kind") != "kiva-pds-state-machines-source":
        raise validation_error(PdsCorePlanesError, f"{state_machines_path}.kind must be kiva-pds-state-machines-source")
    refs_by_plane: dict[str, list[str]] = {plane_id: [] for plane_id in EXPECTED_PLANES}
    machine_ids: set[str] = set()
    for index, raw_machine in enumerate(
        require_list(state_machines.get("machines"), f"{state_machines_path}.machines", PdsCorePlanesError)
    ):
        machine = require_mapping(raw_machine, f"{state_machines_path}.machines[{index}]", PdsCorePlanesError)
        machine_id = require_string(
            machine.get("id"),
            f"{state_machines_path}.machines[{index}].id",
            PdsCorePlanesError,
        )
        if machine_id in machine_ids:
            raise validation_error(PdsCorePlanesError, f"duplicate PDS state-machine id: {machine_id}")
        machine_ids.add(machine_id)
        for plane_id in require_string_list(
            machine.get("plane_refs"),
            f"{state_machines_path}.machines[{index}].plane_refs",
            PdsCorePlanesError,
        ):
            if plane_id not in refs_by_plane:
                raise validation_error(PdsCorePlanesError, f"{machine_id} references unknown PDS plane: {plane_id}")
            refs_by_plane[plane_id].append(machine_id)
    return machine_ids, refs_by_plane


def validate_pds_core_planes(
    root: Path,
    source_path: Path,
    milestones_path: Path,
    manifest_path: Path,
    pds_families_path: Path = DEFAULT_PDS_FAMILIES,
    pds_invariants_path: Path = DEFAULT_PDS_INVARIANTS,
    pds_state_machines_path: Path = DEFAULT_PDS_STATE_MACHINES,
    pds_plane_packages_path: Path = DEFAULT_PDS_PLANE_PACKAGES,
) -> tuple[int, int]:
    """Validate PDS core-plane coverage."""
    source = load_yaml(root / source_path, PdsCorePlanesError)
    milestones = load_yaml(root / milestones_path, PdsCorePlanesError)
    manifest = load_evidence_check_manifest(root / manifest_path, PdsCorePlanesError)
    pds_families = load_yaml(root / pds_families_path, PdsCorePlanesError)
    pds_invariants = load_yaml(root / pds_invariants_path, PdsCorePlanesError)
    pds_state_machines = load_yaml(root / pds_state_machines_path, PdsCorePlanesError)
    pds_plane_packages = load_yaml(root / pds_plane_packages_path, PdsCorePlanesError)
    validate_envelope(source, source_path)

    known_milestones = milestone_ids(milestones, milestones_path)
    known_checks = check_ids(manifest)
    known_families = pds_resource_family_ids(pds_families, pds_families_path)
    known_invariants, expected_invariant_refs_by_plane = invariant_refs_by_plane(pds_invariants, pds_invariants_path)
    known_state_machines, expected_state_machine_refs_by_plane = state_machine_refs_by_plane(pds_state_machines, pds_state_machines_path)
    declared_plane_ids: list[str] = []
    covered_milestones: set[str] = set()
    covered_families: set[str] = set()
    covered_invariants: set[str] = set()
    covered_state_machines: set[str] = set()
    for index, raw_plane in enumerate(require_list(source.get("planes"), f"{source_path}.planes", PdsCorePlanesError)):
        plane = require_mapping(raw_plane, f"{source_path}.planes[{index}]", PdsCorePlanesError)
        plane_id = require_string(plane.get("id"), f"{source_path}.planes[{index}].id", PdsCorePlanesError)
        declared_plane_ids.append(plane_id)
        require_string(plane.get("core_concept"), f"{source_path}.planes[{index}].core_concept", PdsCorePlanesError)
        owns = require_string_list(plane.get("owns"), f"{source_path}.planes[{index}].owns", PdsCorePlanesError)
        if len(owns) < 2:
            raise validation_error(PdsCorePlanesError, f"{source_path}.planes[{index}].owns must contain at least two responsibilities")
        for anchor in require_string_list(
            plane.get("implementation_anchors"),
            f"{source_path}.planes[{index}].implementation_anchors",
            PdsCorePlanesError,
        ):
            if not (root / anchor).exists():
                raise validation_error(PdsCorePlanesError, f"{source_path}.planes.{plane_id} missing anchor: {anchor}")
        for check_id in require_string_list(
            plane.get("required_checks"),
            f"{source_path}.planes[{index}].required_checks",
            PdsCorePlanesError,
        ):
            if check_id not in known_checks:
                raise validation_error(PdsCorePlanesError, f"{source_path}.planes.{plane_id} references missing check: {check_id}")
        for family_id in require_string_list(
            plane.get("resource_family_refs"),
            f"{source_path}.planes[{index}].resource_family_refs",
            PdsCorePlanesError,
        ):
            if family_id not in known_families:
                raise validation_error(PdsCorePlanesError, f"{source_path}.planes.{plane_id} references missing PDS resource family: {family_id}")
            covered_families.add(family_id)
        invariant_refs = require_string_list(
            plane.get("invariant_refs"),
            f"{source_path}.planes[{index}].invariant_refs",
            PdsCorePlanesError,
        )
        unknown_invariants = sorted(set(invariant_refs).difference(known_invariants))
        if unknown_invariants:
            raise validation_error(
                PdsCorePlanesError,
                f"{source_path}.planes.{plane_id} references missing PDS invariants: {unknown_invariants}",
            )
        expected_invariant_refs = expected_invariant_refs_by_plane[plane_id]
        if invariant_refs != expected_invariant_refs:
            raise validation_error(
                PdsCorePlanesError,
                f"{source_path}.planes.{plane_id}.invariant_refs must be inverse of {pds_invariants_path}: {expected_invariant_refs}",
            )
        covered_invariants.update(invariant_refs)
        state_machine_refs = require_string_list(
            plane.get("state_machine_refs"),
            f"{source_path}.planes[{index}].state_machine_refs",
            PdsCorePlanesError,
        )
        unknown_state_machines = sorted(set(state_machine_refs).difference(known_state_machines))
        if unknown_state_machines:
            raise validation_error(
                PdsCorePlanesError,
                f"{source_path}.planes.{plane_id} references missing PDS state machines: {unknown_state_machines}",
            )
        expected_state_machine_refs = expected_state_machine_refs_by_plane[plane_id]
        if state_machine_refs != expected_state_machine_refs:
            raise validation_error(
                PdsCorePlanesError,
                f"{source_path}.planes.{plane_id}.state_machine_refs must be inverse of {pds_state_machines_path}: {expected_state_machine_refs}",
            )
        covered_state_machines.update(state_machine_refs)
        for milestone_id in require_string_list(
            plane.get("milestone_refs"),
            f"{source_path}.planes[{index}].milestone_refs",
            PdsCorePlanesError,
        ):
            if milestone_id not in known_milestones:
                raise validation_error(PdsCorePlanesError, f"{source_path}.planes.{plane_id} references missing milestone: {milestone_id}")
            covered_milestones.add(milestone_id)
    if declared_plane_ids != EXPECTED_PLANES:
        raise validation_error(PdsCorePlanesError, f"{source_path}.planes ids must be {EXPECTED_PLANES}")
    uncovered = sorted(known_milestones.difference(covered_milestones))
    if uncovered:
        raise validation_error(PdsCorePlanesError, f"PDS milestones are not mapped to core planes: {uncovered}")
    uncovered_families = sorted(known_families.difference(covered_families))
    if uncovered_families:
        raise validation_error(PdsCorePlanesError, f"PDS resource families are not mapped to core planes: {uncovered_families}")
    uncovered_invariants = sorted(known_invariants.difference(covered_invariants))
    if uncovered_invariants:
        raise validation_error(PdsCorePlanesError, f"PDS invariants are not mapped to core planes: {uncovered_invariants}")
    uncovered_state_machines = sorted(known_state_machines.difference(covered_state_machines))
    if uncovered_state_machines:
        raise validation_error(PdsCorePlanesError, f"PDS state machines are not mapped to core planes: {uncovered_state_machines}")
    if pds_plane_packages.get("kind") != "kiva-pds-plane-package-index":
        raise validation_error(PdsCorePlanesError, f"{pds_plane_packages_path}.kind must be kiva-pds-plane-package-index")
    package_ids = [
        require_string(
            require_mapping(row, f"{pds_plane_packages_path}.packages[{index}]", PdsCorePlanesError).get("id"),
            f"{pds_plane_packages_path}.packages[{index}].id",
            PdsCorePlanesError,
        )
        for index, row in enumerate(
            require_list(pds_plane_packages.get("packages"), f"{pds_plane_packages_path}.packages", PdsCorePlanesError)
        )
    ]
    if package_ids != EXPECTED_PLANES:
        raise validation_error(PdsCorePlanesError, f"{pds_plane_packages_path}.packages ids must be {EXPECTED_PLANES}")
    return len(declared_plane_ids), len(covered_milestones)


def main() -> int:
    """Validate PDS core planes."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--milestones", type=Path, default=DEFAULT_MILESTONES)
    parser.add_argument("--evidence-manifest", type=Path, default=DEFAULT_EVIDENCE_MANIFEST)
    parser.add_argument("--pds-families", type=Path, default=DEFAULT_PDS_FAMILIES)
    parser.add_argument("--pds-invariants", type=Path, default=DEFAULT_PDS_INVARIANTS)
    parser.add_argument("--pds-state-machines", type=Path, default=DEFAULT_PDS_STATE_MACHINES)
    parser.add_argument("--pds-plane-packages", type=Path, default=DEFAULT_PDS_PLANE_PACKAGES)
    args = parser.parse_args()

    plane_count, milestone_count = validate_pds_core_planes(
        args.root.resolve(),
        args.source,
        args.milestones,
        args.evidence_manifest,
        args.pds_families,
        args.pds_invariants,
        args.pds_state_machines,
        args.pds_plane_packages,
    )
    print(f"validated PDS core planes: {plane_count} planes, {milestone_count} milestones")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
