#!/usr/bin/env python3
"""Validate Projection normal-form compatibility view has no retired fields."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.manifest import load_yaml, require_bool, require_list, require_mapping, require_string, validation_error
from kiva_ci.projection import NORMAL_FORM_SYNC_POLICY


DEFAULT_POLICY = Path("spec/architecture/projection-normal-form-policy.source.yaml")
DEFAULT_PROJECTION_SPEC = Path("spec/architecture/projection-specs.source.yaml")
DEFAULT_ARTIFACTS = Path("spec/architecture/projection-artifacts.source.yaml")
DEFAULT_RUNTIME = Path("spec/architecture/projection-runtime.source.yaml")
DEFAULT_CHECK_BINDINGS = Path("spec/architecture/projection-check-bindings.source.yaml")
RETIRED_SYNC_POLICY = "-".join(("transitional", "mirror-from-projection-spec"))


class ProjectionTransitionCleanupError(RuntimeError):
    """Raised when stale Projection compatibility state remains."""


def rooted(root: Path, path: Path) -> Path:
    """Return a path relative to root unless it is already absolute."""
    return path if path.is_absolute() else root / path


def reject_stale_value(value: Any, label: str) -> None:
    """Reject retired compatibility constants and field names in a loaded YAML value."""
    if isinstance(value, dict):
        if "scalar_targets" in value:
            raise validation_error(
                ProjectionTransitionCleanupError,
                f"{label}.scalar_targets is retired; use bundle/index or generated_artifacts",
            )
        for optional_key in ("bundle", "index"):
            if value.get(optional_key) == "":
                raise validation_error(
                    ProjectionTransitionCleanupError,
                    f"{label}.{optional_key} must be omitted when empty",
                )
        for key, child in value.items():
            reject_stale_value(child, f"{label}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            reject_stale_value(child, f"{label}[{index}]")
    elif value == RETIRED_SYNC_POLICY:
        raise validation_error(
            ProjectionTransitionCleanupError,
            f"{label} still uses retired synchronization policy {RETIRED_SYNC_POLICY}",
        )


def validate_policy(policy: dict[str, Any], path: Path) -> None:
    """Validate normal-form policy is already in primary mode."""
    if policy.get("transition_stage") != "normal-form-primary":
        raise validation_error(
            ProjectionTransitionCleanupError,
            f"{path}.transition_stage must be normal-form-primary",
        )
    drift_policy = require_mapping(
        policy.get("drift_policy"),
        f"{path}.drift_policy",
        ProjectionTransitionCleanupError,
    )
    if drift_policy.get("synchronized_by") != NORMAL_FORM_SYNC_POLICY:
        raise validation_error(
            ProjectionTransitionCleanupError,
            f"{path}.drift_policy.synchronized_by must be {NORMAL_FORM_SYNC_POLICY}",
        )
    if drift_policy.get("enforcement") != "compatibility-view-matches-normal-form":
        raise validation_error(
            ProjectionTransitionCleanupError,
            f"{path}.drift_policy.enforcement must be compatibility-view-matches-normal-form",
        )


def validate_projection_spec(spec: dict[str, Any], path: Path) -> None:
    """Validate ProjectionSpec is explicitly a compatibility view."""
    auxiliary = require_mapping(
        spec.get("projection_auxiliary_sources"),
        f"{path}.projection_auxiliary_sources",
        ProjectionTransitionCleanupError,
    )
    if auxiliary.get("synchronization_policy") != NORMAL_FORM_SYNC_POLICY:
        raise validation_error(
            ProjectionTransitionCleanupError,
            f"{path}.projection_auxiliary_sources.synchronization_policy must be {NORMAL_FORM_SYNC_POLICY}",
        )
    description = require_string(spec.get("description"), f"{path}.description", ProjectionTransitionCleanupError)
    normalized_description = description.lower()
    if "compatibility projectionspec view" not in normalized_description:
        raise validation_error(
            ProjectionTransitionCleanupError,
            f"{path}.description must identify ProjectionSpec as a compatibility view",
        )
    lifecycle = require_mapping(
        spec.get("legacy_sidecar_lifecycle"),
        f"{path}.legacy_sidecar_lifecycle",
        ProjectionTransitionCleanupError,
    )
    expected_flags = {
        "human_editable": False,
        "generated_only": True,
        "canonical_source": False,
    }
    for field, expected in expected_flags.items():
        actual = require_bool(
            lifecycle.get(field),
            f"{path}.legacy_sidecar_lifecycle.{field}",
            ProjectionTransitionCleanupError,
        )
        if actual is not expected:
            raise validation_error(
                ProjectionTransitionCleanupError,
                f"{path}.legacy_sidecar_lifecycle.{field} must be {expected}",
            )
    if lifecycle.get("authoritative_behavior") != "projection-normal-form-ledgers":
        raise validation_error(
            ProjectionTransitionCleanupError,
            f"{path}.legacy_sidecar_lifecycle.authoritative_behavior must be projection-normal-form-ledgers",
        )
    if lifecycle.get("retention_reason") != "compatibility-drift-check":
        raise validation_error(
            ProjectionTransitionCleanupError,
            f"{path}.legacy_sidecar_lifecycle.retention_reason must be compatibility-drift-check",
        )


def validate_auxiliary_ledger(document: dict[str, Any], path: Path) -> int:
    """Validate one projection auxiliary ledger has only normal-form fields."""
    if document.get("synchronization_policy") != NORMAL_FORM_SYNC_POLICY:
        raise validation_error(
            ProjectionTransitionCleanupError,
            f"{path}.synchronization_policy must be {NORMAL_FORM_SYNC_POLICY}",
        )
    reject_stale_value(document, path.as_posix())
    for collection_name in ("artifacts", "runtime", "bindings"):
        if collection_name in document:
            return len(
                require_list(
                    document.get(collection_name),
                    f"{path}.{collection_name}",
                    ProjectionTransitionCleanupError,
                )
            )
    raise validation_error(
        ProjectionTransitionCleanupError,
        f"{path} must contain artifacts, runtime, or check_bindings rows",
    )


def validate_projection_transition_cleanup(
    root: Path,
    policy_path: Path,
    projection_spec_path: Path,
    auxiliary_paths: list[Path],
) -> int:
    """Validate projection normal-form promotion cleanup."""
    policy = load_yaml(rooted(root, policy_path), ProjectionTransitionCleanupError)
    projection_spec = load_yaml(rooted(root, projection_spec_path), ProjectionTransitionCleanupError)
    validate_policy(policy, policy_path)
    validate_projection_spec(projection_spec, projection_spec_path)
    reject_stale_value(policy, policy_path.as_posix())
    reject_stale_value(projection_spec, projection_spec_path.as_posix())
    row_count = 0
    for path in auxiliary_paths:
        document = load_yaml(rooted(root, path), ProjectionTransitionCleanupError)
        row_count += validate_auxiliary_ledger(document, path)
    return row_count


def main() -> int:
    """Run Projection transition-cleanup validation."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--policy", type=Path, default=DEFAULT_POLICY)
    parser.add_argument("--projection-spec", type=Path, default=DEFAULT_PROJECTION_SPEC)
    parser.add_argument(
        "--auxiliary",
        type=Path,
        action="append",
        default=[DEFAULT_ARTIFACTS, DEFAULT_RUNTIME, DEFAULT_CHECK_BINDINGS],
    )
    args = parser.parse_args()

    row_count = validate_projection_transition_cleanup(
        args.root.resolve(),
        args.policy,
        args.projection_spec,
        args.auxiliary,
    )
    print(f"validated Projection transition cleanup: {row_count} normal-form rows")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
