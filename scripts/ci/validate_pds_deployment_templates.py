#!/usr/bin/env python3
"""Validate Kiva PDS deployment templates and runbook catalogs."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import yaml
from jsonschema import Draft202012Validator, FormatChecker


DEFAULT_SCHEMA = Path("spec/pds/schemas/pds-deployment-template-catalog.schema.json")
DEFAULT_CONTRACT = Path("spec/openapi/kiva-pds.yaml")
REQUIRED_DEPLOYMENT_KINDS = {
    "learner-principal",
    "school-district",
    "multi-agency",
}
REQUIRED_RUNBOOK_REFS = {
    "runbook:key-rotation",
    "runbook:policy-change",
    "runbook:consent-incident",
    "runbook:audit-export",
    "runbook:emergency-access",
}
REQUIRED_PROHIBITED_PAYLOADS = {
    "raw xAPI Statement JSON",
}
KNOWN_CONFIDENTIAL_MECHANISMS = {
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


class PdsDeploymentTemplateError(RuntimeError):
    """Raised when PDS deployment templates violate Kiva invariants."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise PdsDeploymentTemplateError(f"{path} root must be a mapping")
    return value


def load_json(path: Path) -> dict[str, Any]:
    """Load a JSON mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise PdsDeploymentTemplateError(f"{path} root must be a mapping")
    return value


def format_validation_error(error: Any) -> str:
    """Format a JSON Schema validation error with a stable path."""
    path = "$"
    for part in error.absolute_path:
        if isinstance(part, int):
            path += f"[{part}]"
        else:
            path += f".{part}"
    return f"- {path}: {error.message}"


def validate_schema(instance: dict[str, Any], schema_path: Path) -> None:
    """Validate the catalog against its JSON Schema."""
    schema = load_json(schema_path)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(validator.iter_errors(instance), key=lambda error: list(error.absolute_path))
    if errors:
        details = "\n".join(format_validation_error(error) for error in errors)
        raise PdsDeploymentTemplateError(f"deployment template catalog violates {schema_path}:\n{details}")


def pds_operation_ids(contract: Path) -> set[str]:
    """Return operationIds from the PDS OpenAPI contract."""
    document = load_yaml(contract)
    paths = document.get("paths")
    if not isinstance(paths, dict):
        raise PdsDeploymentTemplateError(f"{contract}.paths must be a mapping")
    operation_ids: set[str] = set()
    for path_item in paths.values():
        if not isinstance(path_item, dict):
            continue
        for operation in path_item.values():
            if not isinstance(operation, dict):
                continue
            operation_id = operation.get("operationId")
            if isinstance(operation_id, str) and operation_id:
                operation_ids.add(operation_id)
    return operation_ids


def require_unique(values: list[str], label: str) -> None:
    """Require a list of strings to be unique."""
    if len(values) != len(set(values)):
        raise PdsDeploymentTemplateError(f"{label} must be unique")


def validate_role_separation(template: dict[str, Any]) -> None:
    """Validate public-sector separated-duty subjects."""
    role_subjects = template["governance_profile"]["role_subjects"]
    values = [value for value in role_subjects.values() if isinstance(value, str)]
    if len(values) != len(set(values)):
        raise PdsDeploymentTemplateError(f"{template['id']} role subjects must be pairwise distinct")


def validate_runbooks(template: dict[str, Any]) -> None:
    """Validate runbook references and approving-role boundaries."""
    template_id = template["id"]
    runbook_refs = template["governance_profile"]["runbook_refs"]
    runbooks = template["operator_runbooks"]
    runbook_ids = [runbook["id"] for runbook in runbooks]
    require_unique(runbook_refs, f"{template_id}.governance_profile.runbook_refs")
    require_unique(runbook_ids, f"{template_id}.operator_runbooks.id")
    if set(runbook_refs) != set(runbook_ids):
        raise PdsDeploymentTemplateError(
            f"{template_id} runbook_refs must exactly match operator_runbooks ids",
        )
    missing_required = REQUIRED_RUNBOOK_REFS.difference(runbook_refs)
    if missing_required:
        joined = ", ".join(sorted(missing_required))
        raise PdsDeploymentTemplateError(f"{template_id} is missing required runbooks: {joined}")
    for runbook in runbooks:
        if runbook["owner_role"] in runbook["approving_roles"]:
            raise PdsDeploymentTemplateError(
                f"{template_id}.{runbook['id']} owner_role must not approve its own runbook",
            )


def validate_confidential_mechanisms(template: dict[str, Any]) -> list[str]:
    """Validate enabled and disabled confidential mechanism declarations."""
    template_id = template["id"]
    mechanisms = template["confidential_mechanisms"]
    enabled = set(mechanisms["enabled"])
    disabled = set(mechanisms["disabled"])
    overlap = enabled & disabled
    if overlap:
        joined = ", ".join(sorted(overlap))
        raise PdsDeploymentTemplateError(
            f"{template_id} enables and disables the same mechanisms: {joined}",
        )
    classified = enabled | disabled
    missing = KNOWN_CONFIDENTIAL_MECHANISMS - classified
    if missing:
        joined = ", ".join(sorted(missing))
        raise PdsDeploymentTemplateError(
            f"{template_id} must classify confidential mechanisms: {joined}",
        )
    unknown = classified - KNOWN_CONFIDENTIAL_MECHANISMS
    if unknown:
        joined = ", ".join(sorted(unknown))
        raise PdsDeploymentTemplateError(
            f"{template_id} declares unknown confidential mechanisms: {joined}",
        )
    return sorted(enabled)


def validate_practical_workload_profile(template: dict[str, Any]) -> None:
    """Validate optional practical workload profile references."""
    profile = template.get("practical_workload_profile")
    if profile is None:
        return
    if not isinstance(profile, str) or not profile:
        raise PdsDeploymentTemplateError(
            f"{template['id']}.practical_workload_profile must be a non-empty path",
        )
    if not Path(profile).is_file():
        raise PdsDeploymentTemplateError(
            f"{template['id']}.practical_workload_profile does not exist: {profile}",
        )


def validate_legal_policies(
    template: dict[str, Any],
    operation_ids: set[str],
) -> None:
    """Validate legal policy examples and their contract references."""
    template_id = template["id"]
    policy_ids = [policy["id"] for policy in template["legal_policy_examples"]]
    require_unique(policy_ids, f"{template_id}.legal_policy_examples.id")
    for policy in template["legal_policy_examples"]:
        retention = policy["retention_policy"]
        if retention["review_interval_days"] > retention["retention_period_days"]:
            raise PdsDeploymentTemplateError(
                f"{template_id}.{policy['id']} review_interval_days exceeds retention_period_days",
            )
        if retention["tombstone_after_days"] < retention["retention_period_days"]:
            raise PdsDeploymentTemplateError(
                f"{template_id}.{policy['id']} tombstone_after_days is before retention_period_days",
            )
        if retention["data_class"] not in policy["data_classes"]:
            raise PdsDeploymentTemplateError(
                f"{template_id}.{policy['id']} retention data_class is not declared",
            )
        missing_operations = sorted(set(policy["permitted_operations"]) - operation_ids)
        if missing_operations:
            joined = ", ".join(missing_operations)
            raise PdsDeploymentTemplateError(
                f"{template_id}.{policy['id']} references unknown PDS operations: {joined}",
            )
        missing_payloads = REQUIRED_PROHIBITED_PAYLOADS.difference(policy["prohibited_payloads"])
        if missing_payloads:
            joined = ", ".join(sorted(missing_payloads))
            raise PdsDeploymentTemplateError(
                f"{template_id}.{policy['id']} must prohibit payloads: {joined}",
            )


def validate_catalog(
    catalog: dict[str, Any],
    *,
    schema: Path,
    contract: Path,
) -> dict[str, Any]:
    """Validate the deployment template catalog and return a report."""
    validate_schema(catalog, schema)
    operation_ids = pds_operation_ids(contract)
    templates = catalog["templates"]
    template_ids = [template["id"] for template in templates]
    require_unique(template_ids, "templates.id")

    deployment_kinds = {template["deployment_kind"] for template in templates}
    missing_kinds = REQUIRED_DEPLOYMENT_KINDS.difference(deployment_kinds)
    if missing_kinds:
        joined = ", ".join(sorted(missing_kinds))
        raise PdsDeploymentTemplateError(f"catalog is missing deployment kinds: {joined}")

    for template in templates:
        if template["deployment_kind"] != template["governance_profile"]["profile_kind"]:
            raise PdsDeploymentTemplateError(
                f"{template['id']} deployment_kind must match governance profile_kind",
        )
        validate_role_separation(template)
        validate_confidential_mechanisms(template)
        validate_practical_workload_profile(template)
        validate_runbooks(template)
        validate_legal_policies(template, operation_ids)

    return {
        "schema_version": 1,
        "kind": "pds-deployment-template-validation",
        "status": "passed",
        "catalog": "spec/pds/deployment-templates.yaml",
        "template_count": len(templates),
        "templates": sorted(template_ids),
        "deployment_kinds": sorted(deployment_kinds),
        "required_runbooks": sorted(REQUIRED_RUNBOOK_REFS),
        "confidential_mechanisms": {
            template["id"]: validate_confidential_mechanisms(template) for template in templates
        },
    }


def main() -> int:
    """Run the PDS deployment template validator."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--catalog", type=Path, default=Path("spec/pds/deployment-templates.yaml"))
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    report = validate_catalog(load_yaml(args.catalog), schema=args.schema, contract=args.contract)
    if args.json_report is not None:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    print(
        "validated PDS deployment templates: "
        f"{report['template_count']} templates, kinds={','.join(report['deployment_kinds'])}",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
