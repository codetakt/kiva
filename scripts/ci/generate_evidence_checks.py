#!/usr/bin/env python3
"""Generate the CI evidence-check manifest from its source ledger."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.ci_normal_form import (
    manifest_groups_from_normal_form,
    required_groups_from_gate_plan,
    validate_check_catalog,
    validate_evidence_check_source_compatibility,
    validate_gate_plan,
)
from kiva_ci.evidence import validate_evidence_check_manifest_metadata
from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string
from kiva_ci.projection import projection_specs_from_normal_form, validate_projection_spec_compatibility_view
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_SOURCE = Path("spec/ci/evidence-checks.source.yaml")
DEFAULT_OUTPUT = Path("spec/ci/evidence-checks.yaml")
DEFAULT_GROUP_SOURCE = Path("spec/ci/evidence-check-groups.source.yaml")
DEFAULT_PROJECTION_SPEC = Path("spec/architecture/projection-specs.source.yaml")
DEFAULT_CHECK_CATALOG = Path("spec/ci/evidence-check-catalog.source.yaml")
DEFAULT_GATE_PLAN = Path("spec/ci/evidence-gate-plan.source.yaml")


class EvidenceChecksGenerationError(RuntimeError):
    """Raised when the evidence-check manifest cannot be generated."""


def validate_group_source(group_source: dict[str, Any], source_path: Path) -> None:
    """Validate the CI group metadata ledger used by GatePlan."""
    if group_source.get("schema_version") != 1:
        raise EvidenceChecksGenerationError(f"{source_path}.schema_version must be 1")
    if group_source.get("kind") != "kiva-ci-evidence-check-groups-source":
        raise EvidenceChecksGenerationError(
            f"{source_path}.kind must be kiva-ci-evidence-check-groups-source"
        )
    if group_source.get("status") != "active":
        raise EvidenceChecksGenerationError(f"{source_path}.status must be active")
    seen: set[str] = set()
    for index, raw_group in enumerate(
        require_list(group_source.get("groups"), f"{source_path}.groups", EvidenceChecksGenerationError)
    ):
        group = require_mapping(
            raw_group,
            f"{source_path}.groups[{index}]",
            EvidenceChecksGenerationError,
        )
        group_id = require_string(
            group.get("id"),
            f"{source_path}.groups[{index}].id",
            EvidenceChecksGenerationError,
        )
        if group_id in seen:
            raise EvidenceChecksGenerationError(f"duplicate CI evidence group id: {group_id}")
        seen.add(group_id)
        require_string(
            group.get("nix_gate"),
            f"{source_path}.groups[{index}].nix_gate",
            EvidenceChecksGenerationError,
        )
        require_string(
            group.get("claim_boundary"),
            f"{source_path}.groups[{index}].claim_boundary",
            EvidenceChecksGenerationError,
        )
    if not seen:
        raise EvidenceChecksGenerationError(f"{source_path}.groups must not be empty")


def projection_check_refs(root: Path, projection_spec: dict[str, Any], source_path: Path) -> dict[tuple[str, str], list[dict[str, str]]]:
    """Return ProjectionSpec rows keyed by referenced CI group/check id."""
    refs: dict[tuple[str, str], list[dict[str, str]]] = {}
    projections = projection_specs_from_normal_form(
        root,
        projection_spec,
        source_path,
        EvidenceChecksGenerationError,
    )
    validate_projection_spec_compatibility_view(
        projection_spec,
        source_path,
        projections,
        EvidenceChecksGenerationError,
    )
    for source_key, raw_projection in projections.items():
        projection = require_mapping(
            raw_projection,
            f"{source_path}.projections.{source_key}",
            EvidenceChecksGenerationError,
        )
        if "projection_checks" not in projection:
            continue
        projection_id = require_string(
            projection.get("projection_id"),
            f"{source_path}.projections.{source_key}.projection_id",
            EvidenceChecksGenerationError,
        )
        projection_kind = require_string(
            projection.get("projection_kind"),
            f"{source_path}.projections.{source_key}.projection_kind",
            EvidenceChecksGenerationError,
        )
        for check_index, raw_check in enumerate(
            require_list(
                projection.get("projection_checks"),
                f"{source_path}.projections.{source_key}.projection_checks",
                EvidenceChecksGenerationError,
            )
        ):
            check = require_mapping(
                raw_check,
                f"{source_path}.projections.{source_key}.projection_checks[{check_index}]",
                EvidenceChecksGenerationError,
            )
            key = (
                require_string(check.get("group"), f"{source_key}.projection_checks[{check_index}].group", EvidenceChecksGenerationError),
                require_string(check.get("check_id"), f"{source_key}.projection_checks[{check_index}].check_id", EvidenceChecksGenerationError),
            )
            refs.setdefault(key, []).append(
                {
                    "projection_source": str(source_key),
                    "projection_id": projection_id,
                    "projection_kind": projection_kind,
                }
            )
    return {
        key: sorted(value, key=lambda item: item["projection_source"])
        for key, value in refs.items()
    }


def apply_projection_spec_refs(
    document: dict[str, Any],
    refs: dict[tuple[str, str], list[dict[str, str]]],
) -> None:
    """Annotate generated CI checks with ProjectionSpec provenance."""
    for raw_group in require_list(document.get("groups"), "groups", EvidenceChecksGenerationError):
        group = require_mapping(raw_group, "groups[]", EvidenceChecksGenerationError)
        group_id = require_string(group.get("id"), "groups[].id", EvidenceChecksGenerationError)
        for raw_check in require_list(group.get("checks"), f"{group_id}.checks", EvidenceChecksGenerationError):
            check = require_mapping(raw_check, f"{group_id}.checks[]", EvidenceChecksGenerationError)
            check_id = require_string(check.get("id"), f"{group_id}.checks[].id", EvidenceChecksGenerationError)
            projection_refs = refs.get((group_id, check_id), [])
            if projection_refs:
                check["metadata_source"] = "projection-spec"
                check["projection_spec_refs"] = projection_refs
            else:
                check["metadata_source"] = "evidence-check-source"


def evidence_check_manifest_document(root: Path, source: dict[str, Any], source_path: Path) -> dict[str, Any]:
    """Return the generated evidence-check manifest document."""
    if source.get("schema_version") != 1:
        raise EvidenceChecksGenerationError(f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-ci-evidence-check-source":
        raise EvidenceChecksGenerationError(f"{source_path}.kind must be kiva-ci-evidence-check-source")
    group_source_path = Path(
        require_string(
            source.get("group_source", DEFAULT_GROUP_SOURCE.as_posix()),
            f"{source_path}.group_source",
            EvidenceChecksGenerationError,
        )
    )
    group_source = load_yaml(group_source_path, EvidenceChecksGenerationError)
    validate_group_source(group_source, group_source_path)
    catalog_path = Path(
        require_string(
            source.get("check_catalog_source", DEFAULT_CHECK_CATALOG.as_posix()),
            f"{source_path}.check_catalog_source",
            EvidenceChecksGenerationError,
        )
    )
    gate_plan_path = Path(
        require_string(
            source.get("gate_plan_source", DEFAULT_GATE_PLAN.as_posix()),
            f"{source_path}.gate_plan_source",
            EvidenceChecksGenerationError,
        )
    )
    catalog = load_yaml(root / catalog_path, EvidenceChecksGenerationError)
    gate_plan = load_yaml(root / gate_plan_path, EvidenceChecksGenerationError)
    checks = validate_check_catalog(catalog, catalog_path, EvidenceChecksGenerationError)
    gate_groups = validate_gate_plan(gate_plan, gate_plan_path, EvidenceChecksGenerationError)
    validate_evidence_check_source_compatibility(
        source,
        source_path,
        checks,
        gate_groups,
        EvidenceChecksGenerationError,
    )
    document = {
        key: value
        for key, value in source.items()
        if key not in {"groups", "required_groups"}
    }
    document["kind"] = "kiva-ci-evidence-check-manifest"
    document["group_source"] = group_source_path.as_posix()
    document["check_catalog_source"] = catalog_path.as_posix()
    document["gate_plan_source"] = gate_plan_path.as_posix()
    document["required_groups"] = required_groups_from_gate_plan(gate_groups, EvidenceChecksGenerationError)
    document["groups"] = manifest_groups_from_normal_form(checks, gate_groups, EvidenceChecksGenerationError)
    projection_spec_path = Path(
        require_string(
            source.get("projection_spec_source", DEFAULT_PROJECTION_SPEC.as_posix()),
            f"{source_path}.projection_spec_source",
            EvidenceChecksGenerationError,
        )
    )
    projection_spec = load_yaml(root / projection_spec_path, EvidenceChecksGenerationError)
    document["projection_spec_source"] = projection_spec_path.as_posix()
    apply_projection_spec_refs(document, projection_check_refs(root, projection_spec, projection_spec_path))
    validate_evidence_check_manifest_metadata(
        document,
        DEFAULT_OUTPUT.as_posix(),
        EvidenceChecksGenerationError,
    )
    return document


def render_evidence_checks(root: Path, source_path: Path = DEFAULT_SOURCE) -> str:
    """Render expected evidence-check manifest YAML."""
    source = load_yaml(root / source_path, EvidenceChecksGenerationError)
    document = evidence_check_manifest_document(root, source, source_path)
    return dump_yaml(document)


def main() -> int:
    """Generate or validate the CI evidence-check manifest."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    rendered = render_evidence_checks(root, args.source)
    command_hint = "scripts/ci/generate_evidence_checks.py"
    try:
        write_or_check_generated_artifacts(
            [GeneratedArtifact(root / args.out, rendered)],
            check=args.check,
            command_hint=command_hint,
            error_cls=EvidenceChecksGenerationError,
        )
    except EvidenceChecksGenerationError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated evidence-check manifest: {args.out}")
        return 0
    print(f"generated evidence-check manifest: {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
