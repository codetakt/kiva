#!/usr/bin/env python3
"""Validate Kiva's CC15 external production assurance readiness profile."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import yaml


DEFAULT_PROFILE = Path("spec/confidential/external-production-assurance-readiness.yaml")
PROFILE = "kiva-cc15-external-production-assurance-readiness-v1"
SELECTION_ID = "kiva-cc8-multi-agency-aggregate-v1"
TEMPLATE_ID = "multi-agency-research-aggregate-v1"
ENABLED_MECHANISMS = {
    "additive-secure-aggregation",
    "differential-privacy-release",
    "shamir-vss-admission",
}
BOUNDED_MECHANISMS = {
    "psi-cohort-hash-token",
    "tee-portable-attestation",
}
REVIEW_GATED_DISABLED_MECHANISMS = {
    "psi-oprf-dh-voprf",
    "threshold-frost-signing",
    "tee-aws-nitro-attestation",
    "fhe-sum-feasibility",
    "fhe-sum-runtime",
}
REQUIRED_CI_FLAGS = (
    "clean_git_required_for_release_candidate",
    "live_postgres_required_for_release_candidate",
    "formal_verification_regression_required",
    "pds_evidence_required",
    "conformance_evidence_required",
    "release_evidence_set_validation_required",
)
REQUIRED_BUNDLE_FLAGS = (
    "signed_checksums_required",
    "toolchain_versions_required",
    "database_migration_state_required",
    "openapi_contracts_required",
    "review_outcomes_required_for_production_claim",
)
REQUIRED_CLAIM_FALSE = (
    "production_security_claim_allowed",
    "legal_privacy_certification",
    "ieee_xapi_certification",
    "complete_formal_verification",
    "production_mpc",
    "malicious_secure_psi",
    "production_fhe",
    "production_hardware_backed_tee",
    "production_threshold_cryptography",
)
REQUIRED_OPERATIONAL_SECTIONS = {
    "vulnerability_intake": "## Vulnerability Intake",
    "patch_slas": "## Patch SLAs",
    "cryptographic_deprecation_policy": "## Cryptographic Deprecation Policy",
    "key_compromise_response": "## Key Compromise Response",
    "audit_export_retention": "## Audit Export Retention",
    "incident_disclosure_workflow": "## Incident Disclosure Workflow",
}
REQUIRED_RELEASE_TERMS = (
    "require_clean_git",
    "live_http",
    "postgres",
    "kani_suite == \"regression\"",
)


class ExternalProductionAssuranceReadinessError(RuntimeError):
    """Raised when the CC15 readiness profile is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise ExternalProductionAssuranceReadinessError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise ExternalProductionAssuranceReadinessError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a non-empty list or raise a validation error."""
    if not isinstance(value, list) or not value:
        raise ExternalProductionAssuranceReadinessError(f"{label} must be a non-empty list")
    return value


def require_string_list(value: Any, label: str) -> list[str]:
    """Require a non-empty list of unique strings."""
    values = require_list(value, label)
    if not all(isinstance(item, str) and item for item in values):
        raise ExternalProductionAssuranceReadinessError(
            f"{label} must contain only non-empty strings",
        )
    if len(values) != len(set(values)):
        raise ExternalProductionAssuranceReadinessError(f"{label} must not contain duplicates")
    return values


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality with a stable error message."""
    if actual != expected:
        raise ExternalProductionAssuranceReadinessError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def require_path(path_value: Any, label: str) -> Path:
    """Require a repository path to exist."""
    if not isinstance(path_value, str) or not path_value:
        raise ExternalProductionAssuranceReadinessError(f"{label} must be a non-empty path")
    path = Path(path_value)
    if not path.exists():
        raise ExternalProductionAssuranceReadinessError(f"{label} does not exist: {path}")
    return path


def require_path_or_paths(section: dict[str, Any], label: str) -> list[Path]:
    """Require a section with either path or paths entries."""
    require_equal(section.get("required"), True, f"{label}.required")
    paths: list[Path] = []
    if "path" in section:
        paths.append(require_path(section["path"], f"{label}.path"))
    if "paths" in section:
        for index, path_value in enumerate(require_string_list(section["paths"], f"{label}.paths")):
            paths.append(require_path(path_value, f"{label}.paths[{index}]"))
    if not paths:
        raise ExternalProductionAssuranceReadinessError(f"{label} must reference at least one path")
    return paths


def validate_scope(profile: dict[str, Any]) -> dict[str, list[str]]:
    """Validate that CC15 is bound to the exact CC8 deployment profile."""
    scope = require_mapping(profile.get("release_candidate_scope"), "release_candidate_scope")
    selection_path = require_path(scope.get("deployment_selection"), "release_candidate_scope.deployment_selection")
    selection = load_yaml(selection_path)
    require_equal(selection.get("selection_id"), SELECTION_ID, "selection.selection_id")
    require_equal(scope.get("selection_id"), SELECTION_ID, "release_candidate_scope.selection_id")
    require_equal(scope.get("selected_template_id"), TEMPLATE_ID, "release_candidate_scope.selected_template_id")
    selected_profile = require_mapping(selection.get("selected_deployment_profile"), "selection.selected_deployment_profile")
    require_equal(selected_profile.get("template_id"), TEMPLATE_ID, "selection.selected_deployment_profile.template_id")
    require_equal(
        scope.get("production_claim_allowed_before_external_review"),
        False,
        "release_candidate_scope.production_claim_allowed_before_external_review",
    )

    mechanisms = require_mapping(scope.get("selected_mechanisms"), "release_candidate_scope.selected_mechanisms")
    enabled = set(
        require_string_list(
            mechanisms.get("enabled_for_first_practical_workload"),
            "release_candidate_scope.selected_mechanisms.enabled_for_first_practical_workload",
        ),
    )
    bounded = set(
        require_string_list(
            mechanisms.get("bounded_assurance_only"),
            "release_candidate_scope.selected_mechanisms.bounded_assurance_only",
        ),
    )
    disabled = set(
        require_string_list(
            mechanisms.get("review_gated_available_not_enabled"),
            "release_candidate_scope.selected_mechanisms.review_gated_available_not_enabled",
        ),
    )
    require_equal(enabled, ENABLED_MECHANISMS, "release_candidate_scope.enabled mechanisms")
    require_equal(bounded, BOUNDED_MECHANISMS, "release_candidate_scope.bounded mechanisms")
    require_equal(disabled, REVIEW_GATED_DISABLED_MECHANISMS, "release_candidate_scope.review-gated mechanisms")

    selection_mechanisms = require_mapping(selection.get("mechanism_selection"), "selection.mechanism_selection")
    require_equal(
        set(selection_mechanisms.get("enabled_for_first_practical_workload", [])),
        enabled,
        "selection enabled mechanisms",
    )
    require_equal(
        set(selection_mechanisms.get("bounded_assurance_only", [])),
        bounded,
        "selection bounded mechanisms",
    )
    require_equal(
        set(selection_mechanisms.get("disabled_for_first_practical_workload", [])),
        disabled,
        "selection disabled mechanisms",
    )
    return {
        "enabled": sorted(enabled),
        "bounded_assurance_only": sorted(bounded),
        "review_gated_available_not_enabled": sorted(disabled),
    }


def validate_external_review_inputs(profile: dict[str, Any]) -> list[str]:
    """Validate external review input references."""
    inputs = require_mapping(profile.get("external_review_inputs"), "external_review_inputs")
    required_sections = {
        "selected_mechanism_package",
        "provider_contract",
        "threat_model",
        "code",
        "cryptographic_assumptions",
        "deployment_templates",
        "operator_runbooks",
        "openapi_contracts",
        "formal_models",
    }
    missing = sorted(required_sections - set(inputs))
    if missing:
        raise ExternalProductionAssuranceReadinessError(
            "external_review_inputs is missing: " + ", ".join(missing),
        )
    reviewed_paths: list[str] = []
    for section_name in sorted(required_sections):
        for path in require_path_or_paths(
            require_mapping(inputs[section_name], f"external_review_inputs.{section_name}"),
            f"external_review_inputs.{section_name}",
        ):
            reviewed_paths.append(path.as_posix())
    return sorted(reviewed_paths)


def validate_ci_requirements(profile: dict[str, Any]) -> None:
    """Validate release-candidate CI evidence requirements."""
    section = require_mapping(profile.get("ci_evidence_requirements"), "ci_evidence_requirements")
    for flag in REQUIRED_CI_FLAGS:
        require_equal(section.get(flag), True, f"ci_evidence_requirements.{flag}")
    workflows = require_string_list(section.get("workflows"), "ci_evidence_requirements.workflows")
    workflow_text = "\n".join(
        require_path(workflow, f"ci_evidence_requirements.workflows[{workflow}]").read_text(
            encoding="utf-8",
        )
        for workflow in workflows
    )
    for check in require_string_list(section.get("nix_checks"), "ci_evidence_requirements.nix_checks"):
        if check not in workflow_text:
            raise ExternalProductionAssuranceReadinessError(
                f"required Nix check is not referenced by workflows: {check}",
            )
    for validator in require_string_list(section.get("validators"), "ci_evidence_requirements.validators"):
        require_path(validator, f"ci_evidence_requirements.validators[{validator}]")

    release_validator = Path("scripts/ci/validate_release_evidence_set.py").read_text(encoding="utf-8")
    for term in REQUIRED_RELEASE_TERMS:
        if term not in release_validator:
            raise ExternalProductionAssuranceReadinessError(
                f"release evidence validator is missing required term: {term}",
            )


def validate_operational_controls(profile: dict[str, Any]) -> None:
    """Validate operational control sections and runbook headings."""
    controls = require_mapping(profile.get("operational_controls"), "operational_controls")
    runbook_path = require_path(
        require_mapping(profile.get("evidence"), "evidence").get("operator_runbook"),
        "evidence.operator_runbook",
    )
    runbook_text = runbook_path.read_text(encoding="utf-8")
    for control, heading in REQUIRED_OPERATIONAL_SECTIONS.items():
        section = require_mapping(controls.get(control), f"operational_controls.{control}")
        require_equal(section.get("required"), True, f"operational_controls.{control}.required")
        require_equal(section.get("runbook_section"), heading, f"operational_controls.{control}.runbook_section")
        if heading not in runbook_text:
            raise ExternalProductionAssuranceReadinessError(
                f"operator runbook is missing heading for {control}: {heading}",
            )
    if int(require_mapping(controls["patch_slas"], "operational_controls.patch_slas").get("critical_hours", 0)) > 24:
        raise ExternalProductionAssuranceReadinessError("critical patch SLA must be 24 hours or less")
    if int(require_mapping(controls["audit_export_retention"], "operational_controls.audit_export_retention").get("minimum_days", 0)) < 540:
        raise ExternalProductionAssuranceReadinessError("audit export retention must be at least 540 days")


def validate_release_bundle(profile: dict[str, Any]) -> None:
    """Validate release-candidate evidence-bundle requirements."""
    bundle = require_mapping(profile.get("release_candidate_evidence_bundle"), "release_candidate_evidence_bundle")
    for flag in REQUIRED_BUNDLE_FLAGS:
        require_equal(bundle.get(flag), True, f"release_candidate_evidence_bundle.{flag}")
    for path_key in (
        "release_evidence_validator",
        "conformance_generator",
        "pds_evidence_generator",
        "release_workflow",
    ):
        require_path(bundle.get(path_key), f"release_candidate_evidence_bundle.{path_key}")


def validate_review_status(profile: dict[str, Any]) -> None:
    """Validate external review status and prohibited production claims."""
    review = require_mapping(profile.get("external_review_status"), "external_review_status")
    require_equal(
        review.get("external_review_required_before_production_claim"),
        True,
        "external_review_status.external_review_required_before_production_claim",
    )
    require_equal(
        review.get("external_review_completed_in_repository"),
        False,
        "external_review_status.external_review_completed_in_repository",
    )
    require_equal(
        review.get("review_outcomes_bound_to_release_candidate"),
        False,
        "external_review_status.review_outcomes_bound_to_release_candidate",
    )

    claim_language = require_mapping(profile.get("public_claim_language"), "public_claim_language")
    allowed = require_string_list(claim_language.get("allowed_claims"), "public_claim_language.allowed_claims")
    prohibited = require_string_list(claim_language.get("prohibited_claims"), "public_claim_language.prohibited_claims")
    if len(allowed) < 3 or len(prohibited) < 6:
        raise ExternalProductionAssuranceReadinessError("public claim language is too narrow")
    flags = require_mapping(claim_language.get("claim_flags"), "public_claim_language.claim_flags")
    for flag in REQUIRED_CLAIM_FALSE:
        require_equal(flags.get(flag), False, f"public_claim_language.claim_flags.{flag}")


def validate_traceability(profile: dict[str, Any]) -> None:
    """Validate CC15 compliance-matrix traceability."""
    traceability = require_mapping(profile.get("traceability"), "traceability")
    requirement_id = str(traceability.get("requirement_id"))
    require_equal(
        requirement_id,
        "confidential-external-production-assurance-readiness-001",
        "traceability.requirement_id",
    )
    matrix_path = require_path(traceability.get("compliance_matrix"), "traceability.compliance_matrix")
    require_path(traceability.get("milestone_ledger"), "traceability.milestone_ledger")
    matrix = load_yaml(matrix_path)
    requirements = require_list(matrix.get("requirements"), "compliance_matrix.requirements")
    present = {item.get("id") for item in requirements if isinstance(item, dict)}
    if requirement_id not in present:
        raise ExternalProductionAssuranceReadinessError(
            f"compliance matrix is missing {requirement_id}",
        )


def validate_evidence_paths(profile: dict[str, Any]) -> None:
    """Validate direct evidence references."""
    evidence = require_mapping(profile.get("evidence"), "evidence")
    for key, path_value in evidence.items():
        require_path(path_value, f"evidence.{key}")


def validate_profile(profile: dict[str, Any]) -> dict[str, Any]:
    """Validate the CC15 readiness profile and return a report payload."""
    require_equal(profile.get("schema_version"), 1, "schema_version")
    require_equal(profile.get("kind"), "external-production-assurance-readiness", "kind")
    require_equal(profile.get("status"), "review-ready", "status")
    require_equal(profile.get("milestone"), "CC15", "milestone")
    require_equal(profile.get("profile"), PROFILE, "profile")
    mechanisms = validate_scope(profile)
    reviewed_paths = validate_external_review_inputs(profile)
    validate_ci_requirements(profile)
    validate_operational_controls(profile)
    validate_release_bundle(profile)
    validate_review_status(profile)
    validate_traceability(profile)
    validate_evidence_paths(profile)
    return {
        "schema_version": 1,
        "kind": "external-production-assurance-readiness-validation",
        "status": "passed",
        "profile": profile["profile"],
        "selection_id": SELECTION_ID,
        "selected_template_id": TEMPLATE_ID,
        "mechanisms": mechanisms,
        "review_input_paths": reviewed_paths,
    }


def main() -> int:
    """Run the CC15 readiness validator."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", type=Path, default=DEFAULT_PROFILE)
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    report = validate_profile(load_yaml(args.profile))
    if args.json_report is not None:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    else:
        print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
