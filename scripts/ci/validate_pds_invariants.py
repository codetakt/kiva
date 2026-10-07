#!/usr/bin/env python3
"""Validate the PDS invariant source ledger."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.evidence import load_evidence_check_manifest
from kiva_ci.manifest import (
    load_yaml,
    mapping_by_unique_string_key,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
    validation_error,
)


DEFAULT_SOURCE = Path("spec/pds/invariants.source.yaml")


class PdsInvariantError(RuntimeError):
    """Raised when the PDS invariant ledger is inconsistent."""


def check_ids(manifest: dict[str, Any]) -> set[str]:
    """Return evidence-check ids from the generated manifest."""
    ids: set[str] = set()
    for group_index, raw_group in enumerate(require_list(manifest.get("groups"), "groups", PdsInvariantError)):
        group = require_mapping(raw_group, f"groups[{group_index}]", PdsInvariantError)
        group_id = require_string(group.get("id"), f"groups[{group_index}].id", PdsInvariantError)
        for check_index, raw_check in enumerate(
            require_list(group.get("checks"), f"groups.{group_id}.checks", PdsInvariantError)
        ):
            check = require_mapping(raw_check, f"groups.{group_id}.checks[{check_index}]", PdsInvariantError)
            ids.add(require_string(check.get("id"), f"{group_id}.checks[{check_index}].id", PdsInvariantError))
    return ids


def validate_envelope(source: dict[str, Any], source_path: Path) -> None:
    """Validate invariant source envelope and model declaration."""
    if source.get("schema_version") != 1:
        raise validation_error(PdsInvariantError, f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-pds-invariants-source":
        raise validation_error(PdsInvariantError, f"{source_path}.kind must be kiva-pds-invariants-source")
    if source.get("status") != "active":
        raise validation_error(PdsInvariantError, f"{source_path}.status must be active")
    model = require_mapping(source.get("invariant_model"), f"{source_path}.invariant_model", PdsInvariantError)
    if model.get("root_type") != "PdsInvariant":
        raise validation_error(PdsInvariantError, f"{source_path}.invariant_model.root_type must be PdsInvariant")
    if model.get("parent_root_type") != "PdsCorePlane":
        raise validation_error(PdsInvariantError, f"{source_path}.invariant_model.parent_root_type must be PdsCorePlane")


def plane_ids(core_planes: dict[str, Any]) -> set[str]:
    """Return PDS core-plane ids."""
    rows = mapping_by_unique_string_key(
        core_planes.get("planes"),
        key="id",
        label="core_planes.planes",
        error_cls=PdsInvariantError,
    )
    return set(rows)


def validate_invariant(
    root: Path,
    invariant: dict[str, Any],
    known_planes: set[str],
    known_checks: set[str],
) -> set[str]:
    """Validate one invariant row and return referenced plane ids."""
    invariant_id = require_string(invariant.get("id"), "invariant.id", PdsInvariantError)
    require_string(invariant.get("rule"), f"{invariant_id}.rule", PdsInvariantError)
    refs = set(
        require_string_list(invariant.get("plane_refs"), f"{invariant_id}.plane_refs", PdsInvariantError)
    )
    unknown_planes = sorted(refs.difference(known_planes))
    if unknown_planes:
        raise validation_error(PdsInvariantError, f"{invariant_id} references unknown planes: {unknown_planes}")
    combined_text = ""
    for anchor in require_string_list(
        invariant.get("implementation_anchors"),
        f"{invariant_id}.implementation_anchors",
        PdsInvariantError,
    ):
        path = root / anchor
        if not path.exists():
            raise validation_error(PdsInvariantError, f"{invariant_id} missing implementation anchor: {anchor}")
        if path.is_file():
            combined_text += path.read_text(encoding="utf-8")
    for term in require_string_list(
        invariant.get("required_terms"),
        f"{invariant_id}.required_terms",
        PdsInvariantError,
    ):
        if term not in combined_text:
            raise validation_error(PdsInvariantError, f"{invariant_id} missing required term: {term}")
    for term in require_string_list(
        invariant.get("forbidden_terms", []),
        f"{invariant_id}.forbidden_terms",
        PdsInvariantError,
        allow_empty=True,
    ):
        if term in combined_text:
            raise validation_error(PdsInvariantError, f"{invariant_id} contains forbidden term: {term}")
    for check_id in require_string_list(
        invariant.get("required_checks"),
        f"{invariant_id}.required_checks",
        PdsInvariantError,
    ):
        if check_id not in known_checks:
            raise validation_error(PdsInvariantError, f"{invariant_id} references missing check: {check_id}")
    return refs


def validate_pds_invariants(root: Path, source_path: Path) -> tuple[int, int]:
    """Validate the PDS invariant ledger."""
    source = load_yaml(root / source_path, PdsInvariantError)
    validate_envelope(source, source_path)
    core_planes_path = Path(require_string(source.get("core_planes"), f"{source_path}.core_planes", PdsInvariantError))
    evidence_manifest_path = Path(
        require_string(source.get("evidence_manifest"), f"{source_path}.evidence_manifest", PdsInvariantError)
    )
    core_planes = load_yaml(root / core_planes_path, PdsInvariantError)
    manifest = load_evidence_check_manifest(root / evidence_manifest_path, PdsInvariantError)
    known_planes = plane_ids(core_planes)
    expected_planes = set(
        require_string_list(
            require_mapping(source.get("invariant_model"), "invariant_model", PdsInvariantError).get("required_planes"),
            "invariant_model.required_planes",
            PdsInvariantError,
        )
    )
    if known_planes != expected_planes:
        raise validation_error(PdsInvariantError, "PDS invariant required_planes must match core plane ids")
    known_checks = check_ids(manifest)
    invariants = mapping_by_unique_string_key(
        source.get("invariants"),
        key="id",
        label="invariants",
        error_cls=PdsInvariantError,
    )
    required_ids = {
        "pds-subject-scope",
        "pds-consent-revocation-fail-closed",
        "pds-metadata-only-inventory-export",
        "pds-access-decision-audit-recorded",
        "pds-release-artifact-availability",
        "pds-lifecycle-terminal-immutability",
        "pds-provider-binding-metadata-only",
    }
    if set(invariants) != required_ids:
        raise validation_error(PdsInvariantError, f"PDS invariant ids must be {sorted(required_ids)}")
    covered_planes: set[str] = set()
    for invariant in invariants.values():
        covered_planes.update(validate_invariant(root, invariant, known_planes, known_checks))
    missing_planes = sorted(known_planes.difference(covered_planes))
    if missing_planes:
        raise validation_error(PdsInvariantError, f"PDS invariants do not cover planes: {missing_planes}")
    return len(invariants), len(covered_planes)


def main() -> int:
    """Run PDS invariant validation."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    args = parser.parse_args()

    invariant_count, plane_count = validate_pds_invariants(args.root.resolve(), args.source)
    print(f"validated PDS invariants: {invariant_count} invariants, {plane_count} planes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
