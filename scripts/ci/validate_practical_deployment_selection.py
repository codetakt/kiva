#!/usr/bin/env python3
"""Validate Kiva's CC8 practical confidential deployment selection."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import yaml


DEFAULT_SELECTION = Path("spec/confidential/practical-deployment-workload-selection.yaml")
KNOWN_MECHANISMS = {
    "additive-secure-aggregation",
    "differential-privacy-release",
    "shamir-vss-admission",
    "threshold-frost-signing",
    "psi-cohort-hash-token",
    "psi-oprf-dh-voprf",
    "tee-portable-attestation",
    "tee-aws-nitro-attestation",
    "fhe-sum-feasibility",
    "fhe-sum-runtime",
}
REQUIRED_SUCCESS_METRICS = {
    "privacy",
    "utility",
    "latency",
    "cost",
    "operator_effort",
    "auditability",
}
REQUIRED_THREAT_SECTIONS = {
    "adversary_model",
    "collusion_assumptions",
    "role_separation",
    "data_minimization_boundary",
    "accepted_residual_risks",
}
REQUIRED_COMPLETED_ASSURANCE_MILESTONES = {
    "CC9",
    "CC10",
    "CC15",
}
REQUIRED_WORKLOAD_STATUS = "selected-for-practical-profile"


class PracticalDeploymentSelectionError(RuntimeError):
    """Raised when the CC8 deployment selection is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise PracticalDeploymentSelectionError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise PracticalDeploymentSelectionError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a list or raise a validation error."""
    if not isinstance(value, list) or not value:
        raise PracticalDeploymentSelectionError(f"{label} must be a non-empty list")
    return value


def require_string_list(value: Any, label: str) -> list[str]:
    """Require a non-empty list of unique strings."""
    values = require_list(value, label)
    if not all(isinstance(item, str) and item for item in values):
        raise PracticalDeploymentSelectionError(f"{label} must contain only non-empty strings")
    if len(values) != len(set(values)):
        raise PracticalDeploymentSelectionError(f"{label} must not contain duplicates")
    return values


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality."""
    if actual != expected:
        raise PracticalDeploymentSelectionError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def require_path(path_value: Any, label: str) -> Path:
    """Require a repository path to exist."""
    if not isinstance(path_value, str) or not path_value:
        raise PracticalDeploymentSelectionError(f"{label} must be a non-empty path")
    path = Path(path_value)
    if not path.exists():
        raise PracticalDeploymentSelectionError(f"{label} does not exist: {path}")
    return path


def find_template(catalog: dict[str, Any], template_id: str) -> dict[str, Any]:
    """Return the deployment template with the requested id."""
    for template_value in require_list(catalog.get("templates"), "deployment_templates.templates"):
        template = require_mapping(template_value, "deployment_templates.templates[]")
        if template.get("id") == template_id:
            return template
    raise PracticalDeploymentSelectionError(f"deployment template is missing {template_id!r}")


def validate_selected_profile(selection: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    """Validate the selected deployment profile and return selection/template."""
    selected = require_mapping(selection.get("selected_deployment_profile"), "selected_deployment_profile")
    template_id = str(selected.get("template_id"))
    if "required_next_milestones" in selected:
        raise PracticalDeploymentSelectionError(
            "selected_deployment_profile.required_next_milestones is stale; "
            "use completed_assurance_milestones",
        )
    completed = set(
        require_string_list(
            selected.get("completed_assurance_milestones"),
            "selected_deployment_profile.completed_assurance_milestones",
        ),
    )
    require_equal(
        completed,
        REQUIRED_COMPLETED_ASSURANCE_MILESTONES,
        "selected_deployment_profile.completed_assurance_milestones",
    )
    update = require_mapping(selection.get("deployment_template_update"), "deployment_template_update")
    catalog_path = require_path(update.get("catalog"), "deployment_template_update.catalog")
    catalog = load_yaml(catalog_path)
    template = find_template(catalog, template_id)

    require_equal(update.get("selected_template_id"), template_id, "deployment_template_update.selected_template_id")
    require_equal(template.get("deployment_kind"), selected.get("deployment_kind"), "selected_deployment_profile.deployment_kind")
    require_equal(
        template.get("jurisdiction_profile"),
        selected.get("jurisdiction_profile"),
        "selected_deployment_profile.jurisdiction_profile",
    )
    governance = require_mapping(template.get("governance_profile"), f"{template_id}.governance_profile")
    require_equal(
        governance.get("tenant_partition"),
        selected.get("tenant_partition"),
        "selected_deployment_profile.tenant_partition",
    )
    require_equal(
        governance.get("subject_partition"),
        selected.get("subject_partition"),
        "selected_deployment_profile.subject_partition",
    )
    required_field = str(update.get("required_field"))
    require_equal(required_field, "practical_workload_profile", "deployment_template_update.required_field")
    require_equal(
        template.get(required_field),
        update.get("required_value"),
        f"{template_id}.{required_field}",
    )
    require_path(update.get("required_value"), "deployment_template_update.required_value")
    return selected, template


def validate_workloads(selection: dict[str, Any]) -> list[str]:
    """Validate concrete workload selections."""
    workload_ids: list[str] = []
    for workload_value in require_list(selection.get("workloads"), "workloads"):
        workload = require_mapping(workload_value, "workloads[]")
        workload_id = str(workload.get("id"))
        if not workload_id:
            raise PracticalDeploymentSelectionError("workload.id must be non-empty")
        workload_ids.append(workload_id)
        require_equal(workload.get("status"), REQUIRED_WORKLOAD_STATUS, f"{workload_id}.status")
        require_string_list(workload.get("input_record_classes"), f"{workload_id}.input_record_classes")
        local = require_mapping(workload.get("local_preprocessing"), f"{workload_id}.local_preprocessing")
        require_equal(local.get("raw_xapi_leaves_custodian"), False, f"{workload_id}.raw_xapi_leaves_custodian")
        aggregation = require_mapping(workload.get("aggregation_function"), f"{workload_id}.aggregation_function")
        if aggregation.get("family") not in {"additive-sum", "paired-additive-sum"}:
            raise PracticalDeploymentSelectionError(f"{workload_id}.aggregation_function.family is unsupported")
        require_equal(aggregation.get("bounded_input"), True, f"{workload_id}.aggregation_function.bounded_input")
        require_equal(
            aggregation.get("requires_all_participants"),
            True,
            f"{workload_id}.aggregation_function.requires_all_participants",
        )
        release = require_mapping(workload.get("release"), f"{workload_id}.release")
        if int(release.get("minimum_population", 0)) < 2:
            raise PracticalDeploymentSelectionError(f"{workload_id}.release.minimum_population must be at least 2")
        require_equal(release.get("raw_record_read"), False, f"{workload_id}.release.raw_record_read")
        require_mapping(workload.get("utility_targets"), f"{workload_id}.utility_targets")
    if len(workload_ids) != len(set(workload_ids)):
        raise PracticalDeploymentSelectionError("workload ids must be unique")
    return workload_ids


def validate_threat_model(selection: dict[str, Any]) -> None:
    """Validate threat model and data minimization sections."""
    threat_model = require_mapping(selection.get("threat_model"), "threat_model")
    missing = sorted(REQUIRED_THREAT_SECTIONS - set(threat_model))
    if missing:
        raise PracticalDeploymentSelectionError(
            "threat_model is missing sections: " + ", ".join(missing),
        )
    require_string_list(threat_model["adversary_model"], "threat_model.adversary_model")
    collusion = require_mapping(threat_model["collusion_assumptions"], "threat_model.collusion_assumptions")
    require_string_list(collusion.get("tolerated"), "threat_model.collusion_assumptions.tolerated")
    require_string_list(collusion.get("not_tolerated"), "threat_model.collusion_assumptions.not_tolerated")
    role_separation = require_mapping(threat_model["role_separation"], "threat_model.role_separation")
    if len(set(role_separation.values())) != len(role_separation):
        raise PracticalDeploymentSelectionError("threat_model.role_separation values must be distinct")
    minimization = require_mapping(
        threat_model["data_minimization_boundary"],
        "threat_model.data_minimization_boundary",
    )
    require_equal(
        minimization.get("raw_xapi_statement_json_leaves_custodian"),
        False,
        "data_minimization.raw_xapi_statement_json_leaves_custodian",
    )
    require_equal(
        minimization.get("raw_actor_identifier_leaves_custodian"),
        False,
        "data_minimization.raw_actor_identifier_leaves_custodian",
    )
    require_string_list(threat_model["accepted_residual_risks"], "threat_model.accepted_residual_risks")


def validate_mechanism_selection(selection: dict[str, Any], template: dict[str, Any]) -> dict[str, list[str]]:
    """Validate mechanism classification and production-promotion candidates."""
    mechanism_selection = require_mapping(selection.get("mechanism_selection"), "mechanism_selection")
    enabled = set(
        require_string_list(
            mechanism_selection.get("enabled_for_first_practical_workload"),
            "mechanism_selection.enabled_for_first_practical_workload",
        ),
    )
    bounded = set(
        require_string_list(
            mechanism_selection.get("bounded_assurance_only"),
            "mechanism_selection.bounded_assurance_only",
        ),
    )
    disabled = set(
        require_string_list(
            mechanism_selection.get("disabled_for_first_practical_workload"),
            "mechanism_selection.disabled_for_first_practical_workload",
        ),
    )
    overlap = (enabled & bounded) | (enabled & disabled) | (bounded & disabled)
    if overlap:
        raise PracticalDeploymentSelectionError(
            "mechanism classifications overlap: " + ", ".join(sorted(overlap)),
        )
    classified = enabled | bounded | disabled
    require_equal(classified, KNOWN_MECHANISMS, "mechanism_selection classified mechanism set")

    template_mechanisms = require_mapping(template.get("confidential_mechanisms"), "template.confidential_mechanisms")
    template_enabled = set(require_string_list(template_mechanisms.get("enabled"), "template.confidential_mechanisms.enabled"))
    template_disabled = set(require_string_list(template_mechanisms.get("disabled"), "template.confidential_mechanisms.disabled"))
    if not enabled <= template_enabled:
        raise PracticalDeploymentSelectionError(
            "first practical mechanisms must be enabled in the selected template",
        )
    if not bounded <= template_enabled:
        raise PracticalDeploymentSelectionError(
            "bounded assurance mechanisms must be enabled in the selected template",
        )
    if not disabled <= template_disabled:
        raise PracticalDeploymentSelectionError(
            "disabled practical mechanisms must be disabled in the selected template",
        )

    candidates = require_list(
        mechanism_selection.get("production_promotion_candidates"),
        "mechanism_selection.production_promotion_candidates",
    )
    candidate_ids: list[str] = []
    for candidate_value in candidates:
        candidate = require_mapping(candidate_value, "mechanism_selection.production_promotion_candidates[]")
        mechanism = str(candidate.get("mechanism"))
        if mechanism not in KNOWN_MECHANISMS:
            raise PracticalDeploymentSelectionError(f"unknown production candidate mechanism: {mechanism}")
        if not str(candidate.get("milestone", "")).startswith("CC"):
            raise PracticalDeploymentSelectionError(f"{mechanism}.milestone must be a CC milestone")
        if not isinstance(candidate.get("decision"), str) or not candidate["decision"]:
            raise PracticalDeploymentSelectionError(f"{mechanism}.decision must be non-empty")
        candidate_ids.append(mechanism)
    return {
        "enabled": sorted(enabled),
        "bounded_assurance_only": sorted(bounded),
        "disabled": sorted(disabled),
        "production_candidates": sorted(candidate_ids),
    }


def validate_success_metrics(selection: dict[str, Any]) -> None:
    """Validate privacy, utility, latency, cost, effort, and audit metrics."""
    metrics = require_mapping(selection.get("success_metrics"), "success_metrics")
    missing = sorted(REQUIRED_SUCCESS_METRICS - set(metrics))
    if missing:
        raise PracticalDeploymentSelectionError(
            "success_metrics is missing: " + ", ".join(missing),
        )
    privacy = require_mapping(metrics["privacy"], "success_metrics.privacy")
    if int(privacy.get("minimum_population", 0)) < 2:
        raise PracticalDeploymentSelectionError("success_metrics.privacy.minimum_population must be at least 2")
    require_equal(privacy.get("raw_record_publication_allowed"), False, "success_metrics.privacy.raw_record_publication_allowed")
    for section_name in REQUIRED_SUCCESS_METRICS - {"privacy"}:
        require_mapping(metrics[section_name], f"success_metrics.{section_name}")


def validate_package_reference(selection: dict[str, Any]) -> None:
    """Validate that the CC7 package references the CC8 profile."""
    package_update = require_mapping(selection.get("cc7_package_update"), "cc7_package_update")
    package_path = require_path(package_update.get("package"), "cc7_package_update.package")
    package = load_yaml(package_path)
    section_name = str(package_update.get("required_section"))
    require_equal(section_name, "practical_deployment_selection", "cc7_package_update.required_section")
    package_section = require_mapping(package.get(section_name), f"{package_path}.{section_name}")
    require_equal(
        package_section.get("profile"),
        package_update.get("selected_profile"),
        "confidential package practical profile",
    )
    require_equal(
        package_section.get("selected_template"),
        selection["selected_deployment_profile"]["template_id"],
        "confidential package selected template",
    )
    require_equal(
        package_section.get("validator"),
        package_update.get("validator"),
        "confidential package practical validator",
    )
    require_path(package_update.get("selected_profile"), "cc7_package_update.selected_profile")
    require_path(package_update.get("validator"), "cc7_package_update.validator")


def validate_non_claims(selection: dict[str, Any]) -> None:
    """Validate that CC8 does not overstate production claims."""
    non_claims = require_mapping(selection.get("non_claims"), "non_claims")
    for key, value in non_claims.items():
        require_equal(value, False, f"non_claims.{key}")


def validate_selection(selection: dict[str, Any]) -> dict[str, Any]:
    """Validate the CC8 selection and return a report."""
    require_equal(selection.get("schema_version"), 1, "schema_version")
    require_equal(selection.get("kind"), "practical-confidential-deployment-selection", "kind")
    require_equal(selection.get("status"), "published", "status")
    require_equal(selection.get("milestone"), "CC8", "milestone")
    selected, template = validate_selected_profile(selection)
    workloads = validate_workloads(selection)
    validate_threat_model(selection)
    mechanisms = validate_mechanism_selection(selection, template)
    validate_success_metrics(selection)
    validate_package_reference(selection)
    validate_non_claims(selection)
    return {
        "schema_version": 1,
        "kind": "practical-deployment-selection-validation",
        "status": "passed",
        "selection_id": selection["selection_id"],
        "selected_template": selected["template_id"],
        "completed_assurance_milestones": sorted(REQUIRED_COMPLETED_ASSURANCE_MILESTONES),
        "workloads": workloads,
        "mechanisms": mechanisms,
    }


def main() -> int:
    """Run the CC8 validator."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selection", type=Path, default=DEFAULT_SELECTION)
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    report = validate_selection(load_yaml(args.selection))
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
