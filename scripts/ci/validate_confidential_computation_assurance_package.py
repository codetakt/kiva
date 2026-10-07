#!/usr/bin/env python3
"""Validate Kiva's CC7 confidential computation assurance package."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import yaml


DEFAULT_PACKAGE = Path("spec/confidential/confidential-computation-assurance-package.yaml")


class ConfidentialComputationAssurancePackageError(RuntimeError):
    """Raised when the CC7 package manifest is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise ConfidentialComputationAssurancePackageError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise ConfidentialComputationAssurancePackageError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a list or raise a validation error."""
    if not isinstance(value, list):
        raise ConfidentialComputationAssurancePackageError(f"{label} must be a list")
    return value


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality with a stable error message."""
    if actual != expected:
        raise ConfidentialComputationAssurancePackageError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def require_path(path_value: Any, label: str) -> Path:
    """Require a repository path to exist."""
    if not isinstance(path_value, str) or not path_value:
        raise ConfidentialComputationAssurancePackageError(f"{label} must be a non-empty path")
    path = Path(path_value)
    if not path.exists():
        raise ConfidentialComputationAssurancePackageError(f"{label} does not exist: {path}")
    return path


def require_string_list(value: Any, label: str) -> list[str]:
    """Require a non-empty list of strings."""
    values = require_list(value, label)
    if not values or not all(isinstance(item, str) and item for item in values):
        raise ConfidentialComputationAssurancePackageError(
            f"{label} must be a non-empty list of strings",
        )
    if len(values) != len(set(values)):
        raise ConfidentialComputationAssurancePackageError(f"{label} must not contain duplicates")
    return values


def require_string(value: Any, label: str) -> str:
    """Require a non-empty string."""
    if not isinstance(value, str) or not value:
        raise ConfidentialComputationAssurancePackageError(
            f"{label} must be a non-empty string",
        )
    return value


def require_bool(value: Any, label: str) -> bool:
    """Require a boolean value."""
    if not isinstance(value, bool):
        raise ConfidentialComputationAssurancePackageError(
            f"{label} must be a boolean",
        )
    return value


def validation_policy(package: dict[str, Any]) -> dict[str, Any]:
    """Return the package-owned validation policy."""
    policy = require_mapping(package.get("validation_policy"), "validation_policy")
    required_fields = (
        "required_claim_true",
        "required_claim_false",
        "expected_mechanism_statuses",
        "required_operator_coverage",
        "operator_runbook_headings",
        "required_nix_checks",
        "required_evidence_bundles",
        "required_validators",
    )
    for field in required_fields:
        if field not in policy:
            raise ConfidentialComputationAssurancePackageError(
                f"validation_policy.{field} is required",
            )
    return policy


def resolve_document_ref(ref: str) -> dict[str, Any]:
    """Resolve a repository-local YAML document reference."""
    if "#/" not in ref:
        raise ConfidentialComputationAssurancePackageError(
            f"assurance_source_model_ref must use a local JSON pointer: {ref}",
        )
    raw_path, pointer = ref.split("#/", 1)
    document = load_yaml(require_path(raw_path, "assurance_source_model_ref.document"))
    node: Any = document
    for part in pointer.split("/"):
        if not isinstance(node, dict) or part not in node:
            raise ConfidentialComputationAssurancePackageError(
                f"assurance_source_model_ref cannot be resolved: {ref}",
            )
        node = node[part]
    return require_mapping(node, "assurance_source_model_ref.target")


def validate_claim_boundary(package: dict[str, Any], policy: dict[str, Any]) -> None:
    """Validate package-level claim and non-claim language."""
    boundary = require_mapping(package.get("claim_boundary"), "claim_boundary")
    for key in require_string_list(policy.get("required_claim_true"), "validation_policy.required_claim_true"):
        require_equal(require_bool(boundary.get(key), f"claim_boundary.{key}"), True, f"claim_boundary.{key}")
    for key in require_string_list(policy.get("required_claim_false"), "validation_policy.required_claim_false"):
        require_equal(require_bool(boundary.get(key), f"claim_boundary.{key}"), False, f"claim_boundary.{key}")


def validate_assurance_source_model(package: dict[str, Any], package_path: Path) -> None:
    """Validate the package's pointer to the design-backbone source model."""
    ref = require_string(package.get("assurance_source_model_ref"), "assurance_source_model_ref")
    section = resolve_document_ref(ref)
    expected_package_path = package_path
    if package_path.is_absolute():
        try:
            expected_package_path = package_path.relative_to(Path.cwd())
        except ValueError:
            expected_package_path = package_path
    declared_package_path = require_string(
        section.get("package"),
        "assurance_source_model_ref.target.package",
    )
    require_equal(
        declared_package_path,
        expected_package_path.as_posix(),
        "assurance_source_model_ref.target.package",
    )
    require_equal(
        require_string(section.get("package_role"), "assurance_source_model_ref.target.package_role"),
        "claim-boundary-package",
        "assurance_source_model_ref.target.package_role",
    )
    for key in ("evidence_registry", "verified_kernel_ledger"):
        require_path(
            require_string(section.get(key), f"assurance_source_model_ref.target.{key}"),
            f"assurance_source_model_ref.target.{key}",
        )
    for key in (
        "evidence_registry_role",
        "verified_kernel_ledger_role",
        "typed_evidence_owner",
        "verification_boundary_owner",
    ):
        require_string(section.get(key), f"assurance_source_model_ref.target.{key}")
    require_string_list(
        section.get("reference_direction"),
        "assurance_source_model_ref.target.reference_direction",
    )


def validate_mechanisms(package: dict[str, Any], policy: dict[str, Any]) -> list[str]:
    """Validate selected mechanism entries and their evidence paths."""
    expected_mechanisms = require_mapping(
        policy.get("expected_mechanism_statuses"),
        "validation_policy.expected_mechanism_statuses",
    )
    if not expected_mechanisms:
        raise ConfidentialComputationAssurancePackageError(
            "validation_policy.expected_mechanism_statuses must not be empty",
        )
    for mechanism_id, status in expected_mechanisms.items():
        if not isinstance(mechanism_id, str) or not mechanism_id:
            raise ConfidentialComputationAssurancePackageError(
                "validation_policy.expected_mechanism_statuses keys must be non-empty strings",
            )
        require_string(
            status,
            f"validation_policy.expected_mechanism_statuses.{mechanism_id}",
        )
    mechanisms = require_list(package.get("selected_mechanisms"), "selected_mechanisms")
    seen: set[str] = set()
    for mechanism_value in mechanisms:
        mechanism = require_mapping(mechanism_value, "selected_mechanisms[]")
        mechanism_id = mechanism.get("id")
        if mechanism_id not in expected_mechanisms:
            raise ConfidentialComputationAssurancePackageError(
                f"unexpected selected mechanism: {mechanism_id!r}",
            )
        if mechanism_id in seen:
            raise ConfidentialComputationAssurancePackageError(
                f"duplicate selected mechanism: {mechanism_id}",
            )
        seen.add(mechanism_id)
        require_equal(
            mechanism.get("claim_status"),
            expected_mechanisms[mechanism_id],
            f"{mechanism_id}.claim_status",
        )
        if not isinstance(mechanism.get("milestone"), str) or not mechanism["milestone"].startswith("CC"):
            raise ConfidentialComputationAssurancePackageError(
                f"{mechanism_id}.milestone must reference a CC milestone",
            )
        require_string_list(mechanism.get("non_claims"), f"{mechanism_id}.non_claims")
        evidence = require_mapping(mechanism.get("evidence"), f"{mechanism_id}.evidence")
        if len(evidence) < 5:
            raise ConfidentialComputationAssurancePackageError(
                f"{mechanism_id}.evidence must contain at least five artifacts",
            )
        for evidence_key, evidence_path in evidence.items():
            if isinstance(evidence_path, str):
                require_path(evidence_path, f"{mechanism_id}.evidence.{evidence_key}")
        if "assurance_profile" in mechanism:
            assurance_profile = require_path(
                mechanism["assurance_profile"],
                f"{mechanism_id}.assurance_profile",
            )
            profile = load_yaml(assurance_profile)
            if profile.get("status") not in {"published", "review-gated"}:
                raise ConfidentialComputationAssurancePackageError(
                    f"{mechanism_id}.assurance_profile.status must be published or review-gated",
                )
        if "job_contract" in mechanism:
            contract = load_yaml(require_path(mechanism["job_contract"], f"{mechanism_id}.job_contract"))
            contract_status = contract.get("status")
            if contract_status not in {
                "published",
                "implemented-core-contract",
                "review-gated",
                "review-gated-runtime-boundary",
            }:
                raise ConfidentialComputationAssurancePackageError(
                    f"{mechanism_id}.job_contract.status must be published or "
                    f"implemented-core-contract or review-gated, got {contract_status!r}",
                )
    missing = sorted(set(expected_mechanisms) - seen)
    if missing:
        raise ConfidentialComputationAssurancePackageError(
            "selected_mechanisms is missing: " + ", ".join(missing),
        )
    return sorted(seen)


def validate_openapi_and_persistence(package: dict[str, Any]) -> None:
    """Validate OpenAPI, generated schema, examples, and migration references."""
    section = require_mapping(package.get("openapi_and_persistence"), "openapi_and_persistence")
    for key, path_value in section.items():
        require_path(path_value, f"openapi_and_persistence.{key}")


def validate_ci(package: dict[str, Any], policy: dict[str, Any]) -> None:
    """Validate CI workflow, Nix check, validator, and evidence-bundle references."""
    section = require_mapping(package.get("verification_ci"), "verification_ci")
    workflows = require_string_list(section.get("workflows"), "verification_ci.workflows")
    for workflow in workflows:
        require_path(workflow, f"verification_ci.workflows[{workflow}]")

    nix_checks = set(require_string_list(section.get("nix_checks"), "verification_ci.nix_checks"))
    required_nix_checks = set(
        require_string_list(policy.get("required_nix_checks"), "validation_policy.required_nix_checks")
    )
    missing_checks = sorted(required_nix_checks - nix_checks)
    if missing_checks:
        raise ConfidentialComputationAssurancePackageError(
            "verification_ci.nix_checks is missing: " + ", ".join(missing_checks),
        )

    validators = require_string_list(section.get("validators"), "verification_ci.validators")
    required_validators = set(
        require_string_list(policy.get("required_validators"), "validation_policy.required_validators")
    )
    missing_validators = sorted(required_validators - set(validators))
    if missing_validators:
        raise ConfidentialComputationAssurancePackageError(
            "verification_ci.validators is missing: " + ", ".join(missing_validators),
        )
    for validator in validators:
        require_path(validator, f"verification_ci.validators[{validator}]")

    bundles = set(require_string_list(section.get("evidence_bundles"), "verification_ci.evidence_bundles"))
    required_evidence_bundles = set(
        require_string_list(
            policy.get("required_evidence_bundles"),
            "validation_policy.required_evidence_bundles",
        )
    )
    missing_bundles = sorted(required_evidence_bundles - bundles)
    if missing_bundles:
        raise ConfidentialComputationAssurancePackageError(
            "verification_ci.evidence_bundles is missing: " + ", ".join(missing_bundles),
        )

    workflow_text = "\n".join(Path(workflow).read_text(encoding="utf-8") for workflow in workflows)
    for check in required_nix_checks:
        if check not in workflow_text:
            raise ConfidentialComputationAssurancePackageError(
                f"required Nix check is not referenced by workflows: {check}",
            )
    for bundle in required_evidence_bundles:
        if bundle not in workflow_text:
            raise ConfidentialComputationAssurancePackageError(
                f"required evidence bundle is not referenced by workflows: {bundle}",
            )

    flake_checks = Path("nix/flake/checks.nix").read_text(encoding="utf-8")
    manifest_text = Path("spec/ci/evidence-checks.yaml").read_text(encoding="utf-8")
    if "run_evidence_checks.py" not in flake_checks:
        raise ConfidentialComputationAssurancePackageError(
            "confidential-computation-assurance Nix check must run the evidence-check manifest runner",
        )
    if "validate_confidential_computation_assurance_package.py" not in manifest_text:
        raise ConfidentialComputationAssurancePackageError(
            "confidential-computation-assurance manifest must run the CC7 package validator",
        )
    if "validate_assurance_evidence_registry.py" not in manifest_text:
        raise ConfidentialComputationAssurancePackageError(
            "confidential-computation-assurance manifest must run the assurance evidence registry validator",
        )
    if "validate_confidential_verified_kernels.py" not in manifest_text:
        raise ConfidentialComputationAssurancePackageError(
            "confidential-computation-assurance manifest must run the verified-kernel validator",
        )


def validate_operator_readiness(package: dict[str, Any], policy: dict[str, Any]) -> None:
    """Validate operator runbook coverage."""
    section = require_mapping(package.get("operator_readiness"), "operator_readiness")
    runbook_path = require_path(section.get("runbook"), "operator_readiness.runbook")
    coverage = require_mapping(section.get("coverage"), "operator_readiness.coverage")
    for key in require_string_list(
        policy.get("required_operator_coverage"),
        "validation_policy.required_operator_coverage",
    ):
        require_equal(
            require_bool(coverage.get(key), f"operator_readiness.coverage.{key}"),
            True,
            f"operator_readiness.coverage.{key}",
        )

    runbook_text = runbook_path.read_text(encoding="utf-8")
    headings = require_mapping(
        policy.get("operator_runbook_headings"),
        "validation_policy.operator_runbook_headings",
    )
    for key, heading_value in headings.items():
        if not isinstance(key, str) or not key:
            raise ConfidentialComputationAssurancePackageError(
                "validation_policy.operator_runbook_headings keys must be non-empty strings",
            )
        heading = require_string(
            heading_value,
            f"validation_policy.operator_runbook_headings.{key}",
        )
        if heading not in runbook_text:
            raise ConfidentialComputationAssurancePackageError(
                f"operator runbook is missing {heading!r} for {key}",
            )


def validate_deployment_templates(package: dict[str, Any], mechanism_ids: list[str]) -> dict[str, list[str]]:
    """Validate deployment templates declare enabled and disabled mechanisms."""
    section = require_mapping(package.get("deployment_templates"), "deployment_templates")
    catalog_path = require_path(section.get("catalog"), "deployment_templates.catalog")
    require_path(section.get("schema"), "deployment_templates.schema")
    require_equal(
        section.get("required_mechanism_field"),
        "confidential_mechanisms",
        "deployment_templates.required_mechanism_field",
    )
    catalog = load_yaml(catalog_path)
    declared: dict[str, list[str]] = {}
    for template_value in require_list(catalog.get("templates"), "deployment_templates.catalog.templates"):
        template = require_mapping(template_value, "deployment template")
        template_id = str(template.get("id"))
        mechanisms = require_mapping(template.get("confidential_mechanisms"), f"{template_id}.confidential_mechanisms")
        enabled = set(require_string_list(mechanisms.get("enabled"), f"{template_id}.confidential_mechanisms.enabled"))
        disabled = set(require_string_list(mechanisms.get("disabled"), f"{template_id}.confidential_mechanisms.disabled"))
        overlap = sorted(enabled & disabled)
        if overlap:
            raise ConfidentialComputationAssurancePackageError(
                f"{template_id} enables and disables mechanisms: {', '.join(overlap)}",
            )
        missing = sorted(set(mechanism_ids) - enabled - disabled)
        if missing:
            raise ConfidentialComputationAssurancePackageError(
                f"{template_id} does not classify mechanisms: {', '.join(missing)}",
            )
        declared[template_id] = sorted(enabled)
    return declared


def validate_practical_deployment_selection(package: dict[str, Any]) -> None:
    """Validate the CC8 practical deployment-selection reference."""
    section = require_mapping(
        package.get("practical_deployment_selection"),
        "practical_deployment_selection",
    )
    require_equal(section.get("milestone"), "CC8", "practical_deployment_selection.milestone")
    profile_path = require_path(section.get("profile"), "practical_deployment_selection.profile")
    require_path(section.get("validator"), "practical_deployment_selection.validator")
    profile = load_yaml(profile_path)
    require_equal(profile.get("status"), "published", "practical_deployment_selection.profile.status")
    require_equal(profile.get("milestone"), "CC8", "practical_deployment_selection.profile.milestone")
    selected = require_mapping(
        profile.get("selected_deployment_profile"),
        "practical_deployment_selection.profile.selected_deployment_profile",
    )
    require_equal(
        section.get("selected_template"),
        selected.get("template_id"),
        "practical_deployment_selection.selected_template",
    )


def validate_production_job_lifecycle(package: dict[str, Any]) -> None:
    """Validate the CC9 confidential job lifecycle reference."""
    section = require_mapping(package.get("production_job_lifecycle"), "production_job_lifecycle")
    require_equal(section.get("milestone"), "CC9", "production_job_lifecycle.milestone")
    profile_path = require_path(section.get("profile"), "production_job_lifecycle.profile")
    require_path(section.get("validator"), "production_job_lifecycle.validator")
    require_path(section.get("openapi"), "production_job_lifecycle.openapi")
    require_path(section.get("migration"), "production_job_lifecycle.migration")
    profile = load_yaml(profile_path)
    require_equal(profile.get("status"), "published", "production_job_lifecycle.profile.status")
    require_equal(profile.get("milestone"), "CC9", "production_job_lifecycle.profile.milestone")


def validate_production_key_participant_lifecycle(package: dict[str, Any]) -> None:
    """Validate the CC10 confidential key/participant lifecycle reference."""
    section = require_mapping(
        package.get("production_key_participant_lifecycle"),
        "production_key_participant_lifecycle",
    )
    require_equal(
        section.get("milestone"),
        "CC10",
        "production_key_participant_lifecycle.milestone",
    )
    profile_path = require_path(
        section.get("profile"),
        "production_key_participant_lifecycle.profile",
    )
    require_path(section.get("validator"), "production_key_participant_lifecycle.validator")
    require_path(section.get("openapi"), "production_key_participant_lifecycle.openapi")
    require_path(section.get("migration"), "production_key_participant_lifecycle.migration")
    profile = load_yaml(profile_path)
    require_equal(
        profile.get("status"),
        "published",
        "production_key_participant_lifecycle.profile.status",
    )
    require_equal(
        profile.get("milestone"),
        "CC10",
        "production_key_participant_lifecycle.profile.milestone",
    )


def validate_production_psi_upgrade(package: dict[str, Any]) -> None:
    """Validate the CC11 review-gated production PSI upgrade reference."""
    section = require_mapping(package.get("production_psi_upgrade"), "production_psi_upgrade")
    require_equal(section.get("milestone"), "CC11", "production_psi_upgrade.milestone")
    profile_path = require_path(section.get("profile"), "production_psi_upgrade.profile")
    require_path(section.get("job_contract"), "production_psi_upgrade.job_contract")
    require_path(section.get("vectors"), "production_psi_upgrade.vectors")
    require_path(section.get("benchmarks"), "production_psi_upgrade.benchmarks")
    require_path(section.get("interop"), "production_psi_upgrade.interop")
    require_path(section.get("validator"), "production_psi_upgrade.validator")
    require_path(section.get("openapi"), "production_psi_upgrade.openapi")
    require_path(section.get("tamarin_model"), "production_psi_upgrade.tamarin_model")
    profile = load_yaml(profile_path)
    require_equal(profile.get("status"), "review-gated", "production_psi_upgrade.profile.status")
    require_equal(profile.get("milestone"), "CC11", "production_psi_upgrade.profile.milestone")
    review = require_mapping(
        profile.get("external_crypto_review"),
        "production_psi_upgrade.profile.external_crypto_review",
    )
    require_equal(
        review.get("required_before_production_claim"),
        True,
        "production_psi_upgrade.profile.external_crypto_review.required_before_production_claim",
    )
    require_equal(
        review.get("production_claim_allowed"),
        False,
        "production_psi_upgrade.profile.external_crypto_review.production_claim_allowed",
    )


def validate_production_tee_attestation(package: dict[str, Any]) -> None:
    """Validate the CC12 review-gated AWS Nitro TEE attestation reference."""
    section = require_mapping(package.get("production_tee_attestation"), "production_tee_attestation")
    require_equal(section.get("milestone"), "CC12", "production_tee_attestation.milestone")
    profile_path = require_path(section.get("profile"), "production_tee_attestation.profile")
    require_path(section.get("job_contract"), "production_tee_attestation.job_contract")
    require_path(section.get("vectors"), "production_tee_attestation.vectors")
    require_path(section.get("validator"), "production_tee_attestation.validator")
    require_path(section.get("openapi"), "production_tee_attestation.openapi")
    require_path(section.get("tamarin_model"), "production_tee_attestation.tamarin_model")
    profile = load_yaml(profile_path)
    require_equal(profile.get("status"), "review-gated", "production_tee_attestation.profile.status")
    require_equal(profile.get("milestone"), "CC12", "production_tee_attestation.profile.milestone")
    review = require_mapping(
        profile.get("external_security_review"),
        "production_tee_attestation.profile.external_security_review",
    )
    require_equal(
        review.get("required_before_production_claim"),
        True,
        "production_tee_attestation.profile.external_security_review.required_before_production_claim",
    )
    require_equal(
        review.get("production_claim_allowed"),
        False,
        "production_tee_attestation.profile.external_security_review.production_claim_allowed",
    )


def validate_production_threshold_cryptography(package: dict[str, Any]) -> None:
    """Validate the CC13 review-gated FROST threshold-signing reference."""
    section = require_mapping(
        package.get("production_threshold_cryptography"),
        "production_threshold_cryptography",
    )
    require_equal(
        section.get("milestone"),
        "CC13",
        "production_threshold_cryptography.milestone",
    )
    profile_path = require_path(section.get("profile"), "production_threshold_cryptography.profile")
    require_path(section.get("job_contract"), "production_threshold_cryptography.job_contract")
    require_path(section.get("vectors"), "production_threshold_cryptography.vectors")
    require_path(section.get("validator"), "production_threshold_cryptography.validator")
    require_path(section.get("openapi"), "production_threshold_cryptography.openapi")
    require_path(section.get("tamarin_model"), "production_threshold_cryptography.tamarin_model")
    profile = load_yaml(profile_path)
    require_equal(
        profile.get("status"),
        "review-gated",
        "production_threshold_cryptography.profile.status",
    )
    require_equal(
        profile.get("milestone"),
        "CC13",
        "production_threshold_cryptography.profile.milestone",
    )
    review = require_mapping(
        profile.get("external_cryptographic_review"),
        "production_threshold_cryptography.profile.external_cryptographic_review",
    )
    require_equal(
        review.get("required_before_production_claim"),
        True,
        "production_threshold_cryptography.profile.external_cryptographic_review.required_before_production_claim",
    )
    require_equal(
        review.get("production_threshold_cryptography_claim_allowed"),
        False,
        "production_threshold_cryptography.profile.external_cryptographic_review.production_threshold_cryptography_claim_allowed",
    )
    require_equal(
        review.get("malicious_participant_robustness_claim_allowed"),
        False,
        "production_threshold_cryptography.profile.external_cryptographic_review.malicious_participant_robustness_claim_allowed",
    )


def validate_production_fhe_integration(package: dict[str, Any]) -> None:
    """Validate the CC14 review-gated FHE runtime integration reference."""
    section = require_mapping(package.get("production_fhe_integration"), "production_fhe_integration")
    require_equal(section.get("milestone"), "CC14", "production_fhe_integration.milestone")
    profile_path = require_path(section.get("profile"), "production_fhe_integration.profile")
    require_path(section.get("job_contract"), "production_fhe_integration.job_contract")
    require_path(section.get("vectors"), "production_fhe_integration.vectors")
    require_path(section.get("validator"), "production_fhe_integration.validator")
    require_path(section.get("openapi"), "production_fhe_integration.openapi")
    require_path(section.get("tamarin_model"), "production_fhe_integration.tamarin_model")
    profile = load_yaml(profile_path)
    require_equal(profile.get("status"), "review-gated", "production_fhe_integration.profile.status")
    require_equal(profile.get("milestone"), "CC14", "production_fhe_integration.profile.milestone")
    review = require_mapping(
        profile.get("external_cryptographic_review"),
        "production_fhe_integration.profile.external_cryptographic_review",
    )
    require_equal(
        review.get("required_before_production_claim"),
        True,
        "production_fhe_integration.profile.external_cryptographic_review.required_before_production_claim",
    )
    require_equal(
        review.get("production_fhe_claim_allowed"),
        False,
        "production_fhe_integration.profile.external_cryptographic_review.production_fhe_claim_allowed",
    )
    require_equal(
        review.get("malicious_participant_robustness_claim_allowed"),
        False,
        "production_fhe_integration.profile.external_cryptographic_review.malicious_participant_robustness_claim_allowed",
    )


def validate_external_production_assurance(package: dict[str, Any]) -> None:
    """Validate the CC15 external production assurance readiness reference."""
    section = require_mapping(
        package.get("external_production_assurance"),
        "external_production_assurance",
    )
    require_equal(section.get("milestone"), "CC15", "external_production_assurance.milestone")
    profile_path = require_path(section.get("profile"), "external_production_assurance.profile")
    require_path(section.get("validator"), "external_production_assurance.validator")
    require_path(
        section.get("release_evidence_validator"),
        "external_production_assurance.release_evidence_validator",
    )
    require_path(section.get("architecture_note"), "external_production_assurance.architecture_note")
    require_path(section.get("operator_runbook"), "external_production_assurance.operator_runbook")
    require_path(section.get("release_workflow"), "external_production_assurance.release_workflow")
    deployment_profile = require_path(
        section.get("deployment_profile"),
        "external_production_assurance.deployment_profile",
    )
    require_equal(
        section.get("selected_template"),
        "multi-agency-research-aggregate-v1",
        "external_production_assurance.selected_template",
    )
    require_equal(
        section.get("production_claim_allowed_before_external_review"),
        False,
        "external_production_assurance.production_claim_allowed_before_external_review",
    )
    profile = load_yaml(profile_path)
    require_equal(profile.get("status"), "review-ready", "external_production_assurance.profile.status")
    require_equal(profile.get("milestone"), "CC15", "external_production_assurance.profile.milestone")
    review_status = require_mapping(
        profile.get("external_review_status"),
        "external_production_assurance.profile.external_review_status",
    )
    require_equal(
        review_status.get("external_review_required_before_production_claim"),
        True,
        "external_production_assurance.profile.external_review_status.external_review_required_before_production_claim",
    )
    require_equal(
        review_status.get("external_review_completed_in_repository"),
        False,
        "external_production_assurance.profile.external_review_status.external_review_completed_in_repository",
    )
    deployment = load_yaml(deployment_profile)
    require_equal(
        deployment.get("selection_id"),
        "kiva-cc8-multi-agency-aggregate-v1",
        "external_production_assurance.deployment_profile.selection_id",
    )


def validate_secret_compute_provider_contract(package: dict[str, Any]) -> None:
    """Validate the CC16 secret/confidential-compute provider contract reference."""
    section = require_mapping(
        package.get("secret_compute_provider_contract"),
        "secret_compute_provider_contract",
    )
    require_equal(section.get("milestone"), "CC16", "secret_compute_provider_contract.milestone")
    profile_path = require_path(section.get("profile"), "secret_compute_provider_contract.profile")
    require_path(section.get("validator"), "secret_compute_provider_contract.validator")
    require_path(section.get("architecture_note"), "secret_compute_provider_contract.architecture_note")
    require_path(section.get("rust_core"), "secret_compute_provider_contract.rust_core")
    require_equal(
        section.get("reference_provider_profile"),
        "kiva-reference-primitives-v1",
        "secret_compute_provider_contract.reference_provider_profile",
    )
    require_equal(
        section.get("production_claim_allowed_for_reference_provider"),
        False,
        "secret_compute_provider_contract.production_claim_allowed_for_reference_provider",
    )
    profile = load_yaml(profile_path)
    require_equal(profile.get("status"), "published", "secret_compute_provider_contract.profile.status")
    require_equal(profile.get("milestone"), "CC16", "secret_compute_provider_contract.profile.milestone")
    reference = require_mapping(
        profile.get("reference_provider"),
        "secret_compute_provider_contract.profile.reference_provider",
    )
    require_equal(
        reference.get("production_claim_allowed"),
        False,
        "secret_compute_provider_contract.profile.reference_provider.production_claim_allowed",
    )


def validate_traceability(package: dict[str, Any]) -> None:
    """Validate CC7 traceability references."""
    section = require_mapping(package.get("traceability"), "traceability")
    matrix_path = require_path(section.get("compliance_matrix"), "traceability.compliance_matrix")
    require_path(section.get("milestone_ledger"), "traceability.milestone_ledger")
    require_path(section.get("architecture_note"), "traceability.architecture_note")
    required_ids = set(require_string_list(section.get("required_requirement_ids"), "traceability.required_requirement_ids"))
    matrix = load_yaml(matrix_path)
    requirements = require_list(matrix.get("requirements"), "compliance_matrix.requirements")
    present_ids = {item.get("id") for item in requirements if isinstance(item, dict)}
    missing = sorted(required_ids - present_ids)
    if missing:
        raise ConfidentialComputationAssurancePackageError(
            "compliance matrix is missing: " + ", ".join(missing),
        )


def validate_public_claim_language(package: dict[str, Any]) -> None:
    """Validate public claim allowlist and denylist."""
    section = require_mapping(package.get("public_claim_language"), "public_claim_language")
    allowed = require_string_list(section.get("allowed_claims"), "public_claim_language.allowed_claims")
    prohibited = require_string_list(section.get("prohibited_claims"), "public_claim_language.prohibited_claims")
    if len(allowed) < 3:
        raise ConfidentialComputationAssurancePackageError("at least three allowed claims are required")
    if len(prohibited) < 6:
        raise ConfidentialComputationAssurancePackageError("at least six prohibited claims are required")


def validate_package(package: dict[str, Any], package_path: Path = DEFAULT_PACKAGE) -> dict[str, Any]:
    """Validate the CC7 package and return a report payload."""
    require_equal(package.get("schema_version"), 1, "schema_version")
    require_equal(package.get("kind"), "confidential-computation-assurance-package", "kind")
    require_equal(package.get("status"), "published", "status")
    require_equal(
        package.get("package_id"),
        "kiva-confidential-computation-assurance-cc7-v1",
        "package_id",
    )
    policy = validation_policy(package)
    validate_claim_boundary(package, policy)
    validate_assurance_source_model(package, package_path)
    mechanism_ids = validate_mechanisms(package, policy)
    validate_openapi_and_persistence(package)
    validate_ci(package, policy)
    validate_operator_readiness(package, policy)
    deployments = validate_deployment_templates(package, mechanism_ids)
    validate_practical_deployment_selection(package)
    validate_production_job_lifecycle(package)
    validate_production_key_participant_lifecycle(package)
    validate_production_psi_upgrade(package)
    validate_production_tee_attestation(package)
    validate_production_threshold_cryptography(package)
    validate_production_fhe_integration(package)
    validate_external_production_assurance(package)
    validate_secret_compute_provider_contract(package)
    validate_traceability(package)
    validate_public_claim_language(package)
    return {
        "schema_version": 1,
        "kind": "confidential-computation-assurance-package-validation",
        "status": "passed",
        "package_id": package["package_id"],
        "mechanisms": mechanism_ids,
        "deployment_templates": deployments,
    }


def main() -> int:
    """Run the CC7 package validator."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, default=DEFAULT_PACKAGE)
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    report = validate_package(load_yaml(args.package), args.package)
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
