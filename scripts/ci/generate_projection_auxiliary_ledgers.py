#!/usr/bin/env python3
"""Render the legacy ProjectionSpec compatibility sidecar from normal-form ledgers."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string
from kiva_ci.projection import (
    projection_specs_from_normal_form,
    require_envelope,
    validate_projection_spec_compatibility_view,
)
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_SOURCE = Path("spec/architecture/projection-specs.source.yaml")
DEFAULT_ARTIFACTS = Path("spec/architecture/projection-artifacts.source.yaml")
DEFAULT_RUNTIME = Path("spec/architecture/projection-runtime.source.yaml")
DEFAULT_CHECKS = Path("spec/architecture/projection-check-bindings.source.yaml")


class ProjectionAuxiliaryLedgerError(RuntimeError):
    """Raised when ProjectionSpec auxiliary ledgers cannot be generated."""


def projection_rows(source: dict[str, Any], source_path: Path) -> dict[str, dict[str, Any]]:
    """Return ProjectionSpec rows keyed by ArchitectureGraph source id."""
    require_envelope(
        source,
        path=source_path,
        kind="kiva-architecture-projection-spec-source",
        error_cls=ProjectionAuxiliaryLedgerError,
    )
    return {
        str(source_key): require_mapping(
            raw_projection,
            f"{source_path}.projections.{source_key}",
            ProjectionAuxiliaryLedgerError,
        )
        for source_key, raw_projection in require_mapping(
            source.get("projections"),
            f"{source_path}.projections",
            ProjectionAuxiliaryLedgerError,
        ).items()
    }


def optional_string_list(value: Any, label: str) -> list[str]:
    """Return an optional list of strings from a ProjectionSpec row."""
    if value is None:
        return []
    return [str(item) for item in require_list(value, label, ProjectionAuxiliaryLedgerError)]


def projection_identity(source_key: str, projection: dict[str, Any]) -> dict[str, str]:
    """Return the stable identity fields for one projection."""
    return {
        "projection_source": source_key,
        "projection_id": require_string(
            projection.get("projection_id"),
            f"projections.{source_key}.projection_id",
            ProjectionAuxiliaryLedgerError,
        ),
        "projection_kind": require_string(
            projection.get("projection_kind"),
            f"projections.{source_key}.projection_kind",
            ProjectionAuxiliaryLedgerError,
        ),
    }


def artifacts_ledger(source_path: Path, rows: dict[str, dict[str, Any]]) -> dict[str, Any]:
    """Return the normalized ProjectionArtifact ledger."""
    artifacts = []
    for source_key, projection in rows.items():
        generated_artifacts = optional_string_list(
            projection.get("generated_artifacts"),
            f"projections.{source_key}.generated_artifacts",
        )
        projection_targets = optional_string_list(
            projection.get("projection_targets"),
            f"projections.{source_key}.projection_targets",
        )
        projection_exports = optional_string_list(
            projection.get("projection_exports"),
            f"projections.{source_key}.projection_exports",
        )
        bundle = str(projection.get("bundle", ""))
        index = str(projection.get("index", ""))
        if not (generated_artifacts or projection_targets or projection_exports or bundle or index):
            continue
        row = {
            **projection_identity(source_key, projection),
            "generated_artifacts": generated_artifacts,
            "projection_targets": projection_targets,
            "projection_exports": projection_exports,
        }
        if bundle:
            row["bundle"] = bundle
        if index:
            row["index"] = index
        artifacts.append(row)
    return {
        "schema_version": 1,
        "kind": "kiva-projection-artifacts-source",
        "status": "active",
        "description": "Normalized ProjectionArtifact rows for the ProjectionRuntime normal form.",
        "projection_spec_source": source_path.as_posix(),
        "synchronization_policy": "normal-form-primary-with-projection-spec-compatibility-view",
        "artifacts": artifacts,
    }


def runtime_ledger(source_path: Path, rows: dict[str, dict[str, Any]], generator_plan: dict[str, Any]) -> dict[str, Any]:
    """Return the normalized ProjectionRuntime ledger."""
    runtime_rows = []
    for source_key, projection in rows.items():
        runtime_rows.append(
            {
                **projection_identity(source_key, projection),
                "source": str(projection.get("source", "")),
                "strategy": str(projection.get("strategy", "")),
                "projection_sources": optional_string_list(
                    projection.get("projection_sources"),
                    f"projections.{source_key}.projection_sources",
                ),
                "projection_source_refs": optional_string_list(
                    projection.get("projection_source_refs"),
                    f"projections.{source_key}.projection_source_refs",
                ),
                "projection_generator": str(projection.get("projection_generator", "")),
                "validator": str(projection.get("validator", "")),
            }
        )
    return {
        "schema_version": 1,
        "kind": "kiva-projection-runtime-source",
        "status": "active",
        "description": "Normalized ProjectionRuntime rows for the ProjectionRuntime normal form.",
        "generator_plan": generator_plan,
        "projection_spec_source": source_path.as_posix(),
        "synchronization_policy": "normal-form-primary-with-projection-spec-compatibility-view",
        "runtime": runtime_rows,
    }


def check_bindings_ledger(source_path: Path, rows: dict[str, dict[str, Any]]) -> dict[str, Any]:
    """Return the normalized ProjectionCheckBinding ledger."""
    bindings = []
    for source_key, projection in rows.items():
        identity = projection_identity(source_key, projection)
        for index, raw_check in enumerate(
            require_list(
                projection.get("projection_checks", []),
                f"projections.{source_key}.projection_checks",
                ProjectionAuxiliaryLedgerError,
            )
        ):
            check = require_mapping(
                raw_check,
                f"projections.{source_key}.projection_checks[{index}]",
                ProjectionAuxiliaryLedgerError,
            )
            bindings.append(
                {
                    **identity,
                    "group": require_string(
                        check.get("group"),
                        f"projections.{source_key}.projection_checks[{index}].group",
                        ProjectionAuxiliaryLedgerError,
                    ),
                    "check_id": require_string(
                        check.get("check_id"),
                        f"projections.{source_key}.projection_checks[{index}].check_id",
                        ProjectionAuxiliaryLedgerError,
                    ),
                }
            )
    return {
        "schema_version": 1,
        "kind": "kiva-projection-check-bindings-source",
        "status": "active",
        "description": "Normalized ProjectionCheckBinding rows for the ProjectionRuntime normal form.",
        "projection_spec_source": source_path.as_posix(),
        "synchronization_policy": "normal-form-primary-with-projection-spec-compatibility-view",
        "bindings": bindings,
    }


def rendered_auxiliary_artifacts(root: Path, source_path: Path) -> list[GeneratedArtifact]:
    """Return all rendered ProjectionSpec auxiliary ledgers."""
    source = load_yaml(root / source_path, ProjectionAuxiliaryLedgerError)
    rows = projection_rows(source, source_path)
    current_runtime = load_yaml(root / DEFAULT_RUNTIME, ProjectionAuxiliaryLedgerError)
    generator_plan = require_mapping(
        current_runtime.get("generator_plan"),
        f"{DEFAULT_RUNTIME}.generator_plan",
        ProjectionAuxiliaryLedgerError,
    )
    return [
        GeneratedArtifact(root / DEFAULT_ARTIFACTS, dump_yaml(artifacts_ledger(source_path, rows))),
        GeneratedArtifact(root / DEFAULT_RUNTIME, dump_yaml(runtime_ledger(source_path, rows, generator_plan))),
        GeneratedArtifact(root / DEFAULT_CHECKS, dump_yaml(check_bindings_ledger(source_path, rows))),
    ]


def projection_spec_compatibility_document(root: Path, source_path: Path) -> dict[str, Any]:
    """Return ProjectionSpec compatibility view rendered from normal-form ledgers."""
    source = load_yaml(root / source_path, ProjectionAuxiliaryLedgerError)
    require_envelope(
        source,
        path=source_path,
        kind="kiva-architecture-projection-spec-source",
        error_cls=ProjectionAuxiliaryLedgerError,
    )
    projections = projection_specs_from_normal_form(
        root,
        source,
        source_path,
        ProjectionAuxiliaryLedgerError,
    )
    document = {
        key: value
        for key, value in source.items()
        if key != "projections"
    }
    document["description"] = (
        "Generated legacy compatibility ProjectionSpec view for generated artifacts, "
        "projection inputs, validators, and CI checks. DesignUniverse owns the "
        "ArchitectureGraph source projection; normal-form ProjectionRuntime "
        "ledgers own source-to-projection behavior."
    )
    document["legacy_sidecar_lifecycle"] = {
        "human_editable": False,
        "generated_only": True,
        "canonical_source": False,
        "authoritative_behavior": "projection-normal-form-ledgers",
        "retention_reason": "compatibility-drift-check",
    }
    document["projections"] = projections
    return document


def rendered_artifacts(root: Path, source_path: Path) -> list[GeneratedArtifact]:
    """Return the rendered ProjectionSpec compatibility view."""
    return [
        GeneratedArtifact(
            root / source_path,
            dump_yaml(projection_spec_compatibility_document(root, source_path)),
        )
    ]


def main() -> int:
    """Generate or validate normalized ProjectionSpec auxiliary ledgers."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument(
        "--auxiliary-ledgers",
        action="store_true",
        help="Legacy bootstrap only: render normal-form auxiliary ledgers from the retained ProjectionSpec sidecar.",
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    artifacts = (
        rendered_auxiliary_artifacts(root, args.source)
        if args.auxiliary_ledgers
        else rendered_artifacts(root, args.source)
    )
    command_hint = "scripts/ci/generate_projection_auxiliary_ledgers.py"
    write_or_check_generated_artifacts(
        artifacts,
        check=args.check,
        command_hint=command_hint,
        error_cls=ProjectionAuxiliaryLedgerError,
    )
    verb = "validated" if args.check else "generated"
    target = (
        f"ProjectionSpec auxiliary ledgers: {DEFAULT_ARTIFACTS}, {DEFAULT_RUNTIME}, {DEFAULT_CHECKS}"
        if args.auxiliary_ledgers
        else f"ProjectionSpec compatibility view: {args.source}"
    )
    print(f"{verb} {target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
