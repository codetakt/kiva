#!/usr/bin/env python3
"""Validate CI CheckCatalog/GatePlan normal-form policy and compatibility view."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.ci_normal_form import (
    CI_NORMAL_FORM_SYNC_POLICY,
    validate_check_catalog,
    validate_evidence_check_source_compatibility,
    validate_gate_plan,
)
from kiva_ci.manifest import load_yaml, require_bool, require_mapping, require_string, validation_error


DEFAULT_POLICY = Path("spec/ci/evidence-check-normal-form-policy.source.yaml")
EXPECTED_SYNC_POLICY = CI_NORMAL_FORM_SYNC_POLICY
EXPECTED_NORMAL_FORM_SOURCES = {
    "check_catalog": {
        "source": "spec/ci/evidence-check-catalog.source.yaml",
        "kind": "kiva-ci-evidence-check-catalog-source",
    },
    "gate_plan": {
        "source": "spec/ci/evidence-gate-plan.source.yaml",
        "kind": "kiva-ci-evidence-gate-plan-source",
    },
}


class EvidenceCheckNormalFormError(RuntimeError):
    """Raised when CI evidence-check normal-form validation fails."""


def rooted(root: Path, path: Path) -> Path:
    """Return a path relative to root unless it is already absolute."""
    return path if path.is_absolute() else root / path


def registry_sources(root: Path, registry_path: Path) -> set[str]:
    """Return source paths declared in the source-ledger schema registry."""
    registry = load_yaml(rooted(root, registry_path), EvidenceCheckNormalFormError)
    rows = registry.get("schemas")
    if not isinstance(rows, list):
        raise validation_error(
            EvidenceCheckNormalFormError,
            f"{registry_path}.schemas must be a list",
        )
    sources: set[str] = set()
    for index, row_value in enumerate(rows):
        row = require_mapping(row_value, f"{registry_path}.schemas[{index}]", EvidenceCheckNormalFormError)
        sources.add(
            require_string(
                row.get("source"),
                f"{registry_path}.schemas[{index}].source",
                EvidenceCheckNormalFormError,
            )
        )
    return sources


def validate_policy_envelope(policy: dict[str, Any], policy_path: Path) -> None:
    """Validate the common policy envelope."""
    if policy.get("schema_version") != 1:
        raise validation_error(EvidenceCheckNormalFormError, f"{policy_path}.schema_version must be 1")
    if policy.get("kind") != "kiva-ci-evidence-normal-form-policy-source":
        raise validation_error(
            EvidenceCheckNormalFormError,
            f"{policy_path}.kind must be kiva-ci-evidence-normal-form-policy-source",
        )
    if policy.get("status") != "active":
        raise validation_error(EvidenceCheckNormalFormError, f"{policy_path}.status must be active")
    if policy.get("transition_stage") != "normal-form-primary":
        raise validation_error(EvidenceCheckNormalFormError, f"{policy_path}.transition_stage must be normal-form-primary")


def validate_drift_policy(policy: dict[str, Any], policy_path: Path) -> None:
    """Validate drift-policy constants that make the transition enforceable."""
    drift_policy = require_mapping(
        policy.get("drift_policy"),
        f"{policy_path}.drift_policy",
        EvidenceCheckNormalFormError,
    )
    expected = {
        "enforcement": "compatibility-view-matches-normal-form",
        "synchronized_by": EXPECTED_SYNC_POLICY,
        "authoritative_behavior": "check-catalog-and-gate-plan",
        "retirement_gate": "evidence-check-source-groups-remain-byte-for-byte-compatibility-view",
    }
    for key, value in expected.items():
        actual = require_string(
            drift_policy.get(key),
            f"{policy_path}.drift_policy.{key}",
            EvidenceCheckNormalFormError,
        )
        if actual != value:
            raise validation_error(
                EvidenceCheckNormalFormError,
                f"{policy_path}.drift_policy.{key} must be {value}",
            )


def validate_compatibility_view_lifecycle(policy: dict[str, Any], policy_path: Path) -> None:
    """Validate evidence-checks.source.yaml is generated-only compatibility."""
    lifecycle = require_mapping(
        policy.get("compatibility_view_lifecycle"),
        f"{policy_path}.compatibility_view_lifecycle",
        EvidenceCheckNormalFormError,
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
            EvidenceCheckNormalFormError,
        )
        if actual is not value:
            raise validation_error(
                EvidenceCheckNormalFormError,
                f"{policy_path}.compatibility_view_lifecycle.{key} must be {value}",
            )
    review_role = require_string(
        lifecycle.get("review_role"),
        f"{policy_path}.compatibility_view_lifecycle.review_role",
        EvidenceCheckNormalFormError,
    )
    if review_role != "generated-compatibility-view":
        raise validation_error(
            EvidenceCheckNormalFormError,
            f"{policy_path}.compatibility_view_lifecycle.review_role must be generated-compatibility-view",
        )


def policy_normal_form_sources(policy: dict[str, Any], policy_path: Path) -> dict[str, str]:
    """Return policy-declared normal-form source paths keyed by table."""
    normal_form_sources = require_mapping(
        policy.get("normal_form_sources"),
        f"{policy_path}.normal_form_sources",
        EvidenceCheckNormalFormError,
    )
    if set(normal_form_sources) != set(EXPECTED_NORMAL_FORM_SOURCES):
        raise validation_error(
            EvidenceCheckNormalFormError,
            f"{policy_path}.normal_form_sources keys must be {sorted(EXPECTED_NORMAL_FORM_SOURCES)}",
        )
    paths: dict[str, str] = {}
    for key, expected in EXPECTED_NORMAL_FORM_SOURCES.items():
        row = require_mapping(
            normal_form_sources.get(key),
            f"{policy_path}.normal_form_sources.{key}",
            EvidenceCheckNormalFormError,
        )
        source = require_string(
            row.get("source"),
            f"{policy_path}.normal_form_sources.{key}.source",
            EvidenceCheckNormalFormError,
        )
        kind = require_string(
            row.get("kind"),
            f"{policy_path}.normal_form_sources.{key}.kind",
            EvidenceCheckNormalFormError,
        )
        if source != expected["source"]:
            raise validation_error(
                EvidenceCheckNormalFormError,
                f"{policy_path}.normal_form_sources.{key}.source must be {expected['source']}",
            )
        if kind != expected["kind"]:
            raise validation_error(
                EvidenceCheckNormalFormError,
                f"{policy_path}.normal_form_sources.{key}.kind must be {expected['kind']}",
            )
        paths[key] = source
    return paths


def validate_compatibility_view_declaration(
    compatibility_view: dict[str, Any],
    compatibility_view_path: Path,
    group_source_path: Path,
    paths_by_key: dict[str, str],
) -> None:
    """Validate the compatibility view points at the same normal-form sources as the policy."""
    declared_group_source = require_string(
        compatibility_view.get("group_source"),
        f"{compatibility_view_path}.group_source",
        EvidenceCheckNormalFormError,
    )
    if declared_group_source != group_source_path.as_posix():
        raise validation_error(
            EvidenceCheckNormalFormError,
            f"{compatibility_view_path}.group_source must be {group_source_path.as_posix()}",
        )
    declared_catalog = require_string(
        compatibility_view.get("check_catalog_source"),
        f"{compatibility_view_path}.check_catalog_source",
        EvidenceCheckNormalFormError,
    )
    if declared_catalog != paths_by_key["check_catalog"]:
        raise validation_error(
            EvidenceCheckNormalFormError,
            f"{compatibility_view_path}.check_catalog_source must be {paths_by_key['check_catalog']}",
        )
    declared_gate_plan = require_string(
        compatibility_view.get("gate_plan_source"),
        f"{compatibility_view_path}.gate_plan_source",
        EvidenceCheckNormalFormError,
    )
    if declared_gate_plan != paths_by_key["gate_plan"]:
        raise validation_error(
            EvidenceCheckNormalFormError,
            f"{compatibility_view_path}.gate_plan_source must be {paths_by_key['gate_plan']}",
        )


def validate_primary_normal_form(
    root: Path,
    compatibility_view_path: Path,
    group_source_path: Path,
    paths_by_key: dict[str, str],
) -> list[str]:
    """Validate primary CI normal-form ledgers and compatibility source view."""
    catalog_path = Path(paths_by_key["check_catalog"])
    gate_plan_path = Path(paths_by_key["gate_plan"])
    catalog = load_yaml(rooted(root, catalog_path), EvidenceCheckNormalFormError)
    gate_plan = load_yaml(rooted(root, gate_plan_path), EvidenceCheckNormalFormError)
    checks = validate_check_catalog(catalog, catalog_path, EvidenceCheckNormalFormError)
    gate_groups = validate_gate_plan(gate_plan, gate_plan_path, EvidenceCheckNormalFormError)
    if catalog.get("compatibility_view") != compatibility_view_path.as_posix():
        raise validation_error(
            EvidenceCheckNormalFormError,
            f"{catalog_path}.compatibility_view must be {compatibility_view_path.as_posix()}",
        )
    if gate_plan.get("compatibility_view") != compatibility_view_path.as_posix():
        raise validation_error(
            EvidenceCheckNormalFormError,
            f"{gate_plan_path}.compatibility_view must be {compatibility_view_path.as_posix()}",
        )
    if gate_plan.get("group_source") != group_source_path.as_posix():
        raise validation_error(
            EvidenceCheckNormalFormError,
            f"{gate_plan_path}.group_source must be {group_source_path.as_posix()}",
        )
    compatibility_view = load_yaml(rooted(root, compatibility_view_path), EvidenceCheckNormalFormError)
    validate_compatibility_view_declaration(
        compatibility_view,
        compatibility_view_path,
        group_source_path,
        paths_by_key,
    )
    validate_evidence_check_source_compatibility(
        compatibility_view,
        compatibility_view_path,
        checks,
        gate_groups,
        EvidenceCheckNormalFormError,
    )
    return [catalog_path.as_posix(), gate_plan_path.as_posix()]


def validate_normal_form_policy(root: Path, policy_path: Path) -> list[str]:
    """Validate the CI evidence-check normal-form policy and declared compatibility view."""
    policy = load_yaml(rooted(root, policy_path), EvidenceCheckNormalFormError)
    validate_policy_envelope(policy, policy_path)
    validate_drift_policy(policy, policy_path)
    validate_compatibility_view_lifecycle(policy, policy_path)
    compatibility_view_path = Path(
        require_string(policy.get("compatibility_view"), f"{policy_path}.compatibility_view", EvidenceCheckNormalFormError)
    )
    group_source_path = Path(
        require_string(policy.get("group_source"), f"{policy_path}.group_source", EvidenceCheckNormalFormError)
    )
    if policy.get("generator") != "scripts/ci/generate_evidence_check_catalog.py":
        raise validation_error(EvidenceCheckNormalFormError, f"{policy_path}.generator is not current")
    if policy.get("validator") != "scripts/ci/validate_evidence_check_normal_form.py":
        raise validation_error(EvidenceCheckNormalFormError, f"{policy_path}.validator is not current")
    paths_by_key = policy_normal_form_sources(policy, policy_path)
    validated = validate_primary_normal_form(
        root,
        compatibility_view_path,
        group_source_path,
        paths_by_key,
    )
    registry_path = Path(
        require_string(policy.get("schema_registry"), f"{policy_path}.schema_registry", EvidenceCheckNormalFormError)
    )
    registered_sources = registry_sources(root, registry_path)
    required_sources = {policy_path.as_posix(), *paths_by_key.values()}
    missing = sorted(required_sources.difference(registered_sources))
    if missing:
        raise validation_error(
            EvidenceCheckNormalFormError,
            f"{registry_path} is missing CI normal-form policy sources: {missing}",
        )
    return validated


def main() -> int:
    """Validate CI evidence-check normal-form policy and compatibility view."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--policy", type=Path, default=DEFAULT_POLICY)
    args = parser.parse_args()

    root = args.root.resolve()
    validated = validate_normal_form_policy(root, args.policy)
    print(f"validated CI evidence-check normal-form policy: {len(validated)} sources/views")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
