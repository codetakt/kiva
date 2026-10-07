#!/usr/bin/env python3
"""Validate ProjectionRuntime normal-form policy and compatibility view."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from generate_projection_auxiliary_ledgers import rendered_artifacts
from kiva_ci.manifest import load_yaml, require_bool, require_mapping, require_string, validation_error
from kiva_ci.projection import NORMAL_FORM_SYNC_POLICY, projection_specs_from_normal_form, validate_projection_spec_compatibility_view
from run_generators import load_generator_plan, topological_order


DEFAULT_POLICY = Path("spec/architecture/projection-normal-form-policy.source.yaml")
EXPECTED_AUXILIARY_KEYS = {
    "artifacts": {
        "source": "spec/architecture/projection-artifacts.source.yaml",
        "kind": "kiva-projection-artifacts-source",
    },
    "runtime": {
        "source": "spec/architecture/projection-runtime.source.yaml",
        "kind": "kiva-projection-runtime-source",
    },
    "check_bindings": {
        "source": "spec/architecture/projection-check-bindings.source.yaml",
        "kind": "kiva-projection-check-bindings-source",
    },
}


class ProjectionNormalFormError(RuntimeError):
    """Raised when ProjectionSpec normal-form validation fails."""


def rooted(root: Path, path: Path) -> Path:
    """Return a path relative to root unless it is already absolute."""
    return path if path.is_absolute() else root / path


def registry_sources(root: Path, registry_path: Path) -> set[str]:
    """Return source paths declared in the source-ledger schema registry."""
    registry = load_yaml(rooted(root, registry_path), ProjectionNormalFormError)
    rows = registry.get("schemas")
    if not isinstance(rows, list):
        raise validation_error(
            ProjectionNormalFormError,
            f"{registry_path}.schemas must be a list",
        )
    sources: set[str] = set()
    for index, row_value in enumerate(rows):
        row = require_mapping(row_value, f"{registry_path}.schemas[{index}]", ProjectionNormalFormError)
        sources.add(
            require_string(
                row.get("source"),
                f"{registry_path}.schemas[{index}].source",
                ProjectionNormalFormError,
            )
        )
    return sources


def validate_policy_envelope(policy: dict[str, Any], policy_path: Path) -> None:
    """Validate the common policy envelope."""
    if policy.get("schema_version") != 1:
        raise validation_error(ProjectionNormalFormError, f"{policy_path}.schema_version must be 1")
    if policy.get("kind") != "kiva-projection-normal-form-policy-source":
        raise validation_error(
            ProjectionNormalFormError,
            f"{policy_path}.kind must be kiva-projection-normal-form-policy-source",
        )
    if policy.get("status") != "active":
        raise validation_error(ProjectionNormalFormError, f"{policy_path}.status must be active")
    if policy.get("transition_stage") != "normal-form-primary":
        raise validation_error(ProjectionNormalFormError, f"{policy_path}.transition_stage must be normal-form-primary")
    if policy.get("target_stage") != "normal-form-primary":
        raise validation_error(ProjectionNormalFormError, f"{policy_path}.target_stage must be normal-form-primary")


def validate_drift_policy(policy: dict[str, Any], policy_path: Path) -> None:
    """Validate drift-policy constants that make the transition enforceable."""
    drift_policy = require_mapping(
        policy.get("drift_policy"),
        f"{policy_path}.drift_policy",
        ProjectionNormalFormError,
    )
    expected = {
        "enforcement": "compatibility-view-matches-normal-form",
        "synchronized_by": NORMAL_FORM_SYNC_POLICY,
        "authoritative_behavior": "projection-normal-form-ledgers",
        "retirement_gate": "projection-spec-inline-fields-remain-byte-for-byte-compatibility-view",
    }
    for key, value in expected.items():
        actual = require_string(
            drift_policy.get(key),
            f"{policy_path}.drift_policy.{key}",
            ProjectionNormalFormError,
        )
        if actual != value:
            raise validation_error(
                ProjectionNormalFormError,
                f"{policy_path}.drift_policy.{key} must be {value}",
            )


def validate_compatibility_view_lifecycle(policy: dict[str, Any], policy_path: Path) -> None:
    """Validate ProjectionSpec is declared as a generated-only compatibility view."""
    lifecycle = require_mapping(
        policy.get("compatibility_view_lifecycle"),
        f"{policy_path}.compatibility_view_lifecycle",
        ProjectionNormalFormError,
    )
    expected = {
        "human_editable": False,
        "generated_only": True,
        "canonical_source": False,
    }
    for key, value in expected.items():
        actual = require_bool(
            lifecycle.get(key),
            f"{policy_path}.compatibility_view_lifecycle.{key}",
            ProjectionNormalFormError,
        )
        if actual is not value:
            raise validation_error(
                ProjectionNormalFormError,
                f"{policy_path}.compatibility_view_lifecycle.{key} must be {value}",
            )
    review_role = require_string(
        lifecycle.get("review_role"),
        f"{policy_path}.compatibility_view_lifecycle.review_role",
        ProjectionNormalFormError,
    )
    if review_role != "generated-compatibility-view":
        raise validation_error(
            ProjectionNormalFormError,
            f"{policy_path}.compatibility_view_lifecycle.review_role must be generated-compatibility-view",
        )


def policy_auxiliary_sources(policy: dict[str, Any], policy_path: Path) -> dict[str, str]:
    """Return policy-declared auxiliary source paths keyed by normal-form table."""
    auxiliary_sources = require_mapping(
        policy.get("auxiliary_sources"),
        f"{policy_path}.auxiliary_sources",
        ProjectionNormalFormError,
    )
    if set(auxiliary_sources) != set(EXPECTED_AUXILIARY_KEYS):
        raise validation_error(
            ProjectionNormalFormError,
            f"{policy_path}.auxiliary_sources keys must be {sorted(EXPECTED_AUXILIARY_KEYS)}",
        )
    paths: dict[str, str] = {}
    for key, expected in EXPECTED_AUXILIARY_KEYS.items():
        row = require_mapping(
            auxiliary_sources.get(key),
            f"{policy_path}.auxiliary_sources.{key}",
            ProjectionNormalFormError,
        )
        source = require_string(
            row.get("source"),
            f"{policy_path}.auxiliary_sources.{key}.source",
            ProjectionNormalFormError,
        )
        kind = require_string(
            row.get("kind"),
            f"{policy_path}.auxiliary_sources.{key}.kind",
            ProjectionNormalFormError,
        )
        if source != expected["source"]:
            raise validation_error(
                ProjectionNormalFormError,
                f"{policy_path}.auxiliary_sources.{key}.source must be {expected['source']}",
            )
        if kind != expected["kind"]:
            raise validation_error(
                ProjectionNormalFormError,
                f"{policy_path}.auxiliary_sources.{key}.kind must be {expected['kind']}",
            )
        paths[key] = source
    return paths


def validate_promotion_criteria(policy: dict[str, Any], policy_path: Path) -> None:
    """Validate normal-form promotion criteria."""
    criteria = require_mapping(
        policy.get("promotion_criteria"),
        f"{policy_path}.promotion_criteria",
        ProjectionNormalFormError,
    )
    for key in [
        "compiler_reads_normal_form_ledgers_first",
        "inline_projection_fields_are_compatibility_view",
        "compatibility_view_is_byte_for_byte_current",
        "source_schema_registry_covers_normal_form",
    ]:
        if criteria.get(key) is not True:
            raise validation_error(
                ProjectionNormalFormError,
                f"{policy_path}.promotion_criteria.{key} must be true",
            )


def validate_projection_spec_declaration(
    projection_spec: dict[str, Any],
    projection_spec_path: Path,
    paths_by_key: dict[str, str],
) -> None:
    """Validate ProjectionSpec points at the same auxiliary sources as the policy."""
    auxiliary = require_mapping(
        projection_spec.get("projection_auxiliary_sources"),
        f"{projection_spec_path}.projection_auxiliary_sources",
        ProjectionNormalFormError,
    )
    if auxiliary.get("synchronization_policy") != NORMAL_FORM_SYNC_POLICY:
        raise validation_error(
            ProjectionNormalFormError,
            f"{projection_spec_path}.projection_auxiliary_sources.synchronization_policy must be {NORMAL_FORM_SYNC_POLICY}",
        )
    for key, path in paths_by_key.items():
        declared = require_string(
            auxiliary.get(key),
            f"{projection_spec_path}.projection_auxiliary_sources.{key}",
            ProjectionNormalFormError,
        )
        if declared != path:
            raise validation_error(
                ProjectionNormalFormError,
                f"{projection_spec_path}.projection_auxiliary_sources.{key} must be {path}",
            )


def validate_projection_spec_compatibility_artifact(root: Path, projection_spec_path: Path) -> str:
    """Validate committed ProjectionSpec is the generated compatibility view."""
    artifact = rendered_artifacts(root, projection_spec_path)[0]
    actual_path = artifact.path
    if not actual_path.exists():
        raise validation_error(
            ProjectionNormalFormError,
            f"{actual_path.relative_to(root)} is missing; run scripts/ci/generate_projection_auxiliary_ledgers.py",
        )
    actual_text = actual_path.read_text(encoding="utf-8")
    if actual_text != artifact.text:
        raise validation_error(
            ProjectionNormalFormError,
            f"{actual_path.relative_to(root)} drifted from normal-form projection ledgers; run scripts/ci/generate_projection_auxiliary_ledgers.py",
        )
    return actual_path.relative_to(root).as_posix()


def validate_auxiliary_sources(root: Path, projection_spec_path: Path, paths_by_key: dict[str, str]) -> list[str]:
    """Validate committed auxiliary ledgers are primary normal-form sources."""
    validated: list[str] = []
    for key, source in paths_by_key.items():
        actual_path = rooted(root, Path(source))
        relative_path = Path(source).as_posix()
        document = load_yaml(actual_path, ProjectionNormalFormError)
        expected = next(
            row for row in EXPECTED_AUXILIARY_KEYS.values() if row["source"] == relative_path
        )
        if document.get("kind") != expected["kind"]:
            raise validation_error(
                ProjectionNormalFormError,
                f"{relative_path}.kind must be {expected['kind']}",
            )
        if document.get("status") != "active":
            raise validation_error(ProjectionNormalFormError, f"{relative_path}.status must be active")
        if document.get("projection_spec_source") != projection_spec_path.as_posix():
            raise validation_error(
                ProjectionNormalFormError,
                f"{relative_path}.projection_spec_source must be {projection_spec_path.as_posix()}",
            )
        if document.get("synchronization_policy") != NORMAL_FORM_SYNC_POLICY:
            raise validation_error(
                ProjectionNormalFormError,
                f"{relative_path}.synchronization_policy must be {NORMAL_FORM_SYNC_POLICY}",
            )
        if key == "artifacts":
            for row_index, row_value in enumerate(document.get("artifacts", [])):
                if isinstance(row_value, dict) and (row_value.get("bundle") == "" or row_value.get("index") == ""):
                    raise validation_error(
                        ProjectionNormalFormError,
                        f"{relative_path}.artifacts[{row_index}] must omit empty bundle/index fields",
                    )
        validated.append(relative_path)
    return validated


def validate_normal_form_policy(root: Path, policy_path: Path) -> list[str]:
    """Validate the ProjectionRuntime normal-form policy and declared compatibility view."""
    policy = load_yaml(rooted(root, policy_path), ProjectionNormalFormError)
    validate_policy_envelope(policy, policy_path)
    validate_drift_policy(policy, policy_path)
    validate_compatibility_view_lifecycle(policy, policy_path)
    validate_promotion_criteria(policy, policy_path)
    projection_spec_path = Path(
        require_string(
            policy.get("compatibility_view"),
            f"{policy_path}.compatibility_view",
            ProjectionNormalFormError,
        )
    )
    if policy.get("generator") != "scripts/ci/generate_projection_auxiliary_ledgers.py":
        raise validation_error(ProjectionNormalFormError, f"{policy_path}.generator is not current")
    if policy.get("validator") != "scripts/ci/validate_projection_normal_form.py":
        raise validation_error(ProjectionNormalFormError, f"{policy_path}.validator is not current")
    paths_by_key = policy_auxiliary_sources(policy, policy_path)
    projection_spec = load_yaml(rooted(root, projection_spec_path), ProjectionNormalFormError)
    validate_projection_spec_declaration(projection_spec, projection_spec_path, paths_by_key)
    validated = validate_auxiliary_sources(root, projection_spec_path, paths_by_key)
    topological_order(load_generator_plan(root, Path(paths_by_key["runtime"])))
    normal_form_rows = projection_specs_from_normal_form(
        root,
        projection_spec,
        projection_spec_path,
        ProjectionNormalFormError,
    )
    validate_projection_spec_compatibility_view(
        projection_spec,
        projection_spec_path,
        normal_form_rows,
        ProjectionNormalFormError,
    )
    validated.append(validate_projection_spec_compatibility_artifact(root, projection_spec_path))
    registry_path = Path(
        require_string(policy.get("schema_registry"), f"{policy_path}.schema_registry", ProjectionNormalFormError)
    )
    registered_sources = registry_sources(root, registry_path)
    required_sources = {policy_path.as_posix(), *paths_by_key.values()}
    missing = sorted(required_sources.difference(registered_sources))
    if missing:
        raise validation_error(
            ProjectionNormalFormError,
            f"{registry_path} is missing normal-form policy sources: {missing}",
        )
    return validated


def main() -> int:
    """Validate ProjectionRuntime normal-form policy and compatibility view."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--policy", type=Path, default=DEFAULT_POLICY)
    args = parser.parse_args()

    root = args.root.resolve()
    validated = validate_normal_form_policy(root, args.policy)
    print(f"validated ProjectionRuntime normal-form policy: {len(validated)} sources/views")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
