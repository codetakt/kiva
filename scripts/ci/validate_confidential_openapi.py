#!/usr/bin/env python3
"""Validate Kiva confidential-computation OpenAPI contracts."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import yaml


REQUIRED_PATHS = {
    "/confidential/additive-aggregates": {"post"},
    "/confidential/additive-share-signing-keys": {"get", "post"},
    "/confidential/additive-share-signing-key-rotations": {"post"},
    "/confidential/additive-share-signing-key-revocations": {"post"},
    "/confidential/additive-share-submissions": {"get", "post"},
    "/confidential/additive-share-replay-records": {"get"},
    "/confidential/threshold-release-requests": {"get", "post"},
    "/confidential/threshold-release-approvals": {"post"},
    "/confidential/threshold-releases": {"post"},
    "/confidential/shamir-threshold-reconstructions": {"get", "post"},
    "/confidential/differential-privacy-budgets": {"get", "post"},
    "/confidential/differential-privacy-budget-spends": {"post"},
    "/confidential/differential-privacy-composition-summaries": {"get"},
    "/confidential/differential-privacy-releases": {"get", "post"},
    "/confidential/differential-privacy-sampled-releases": {"post"},
    "/confidential/jobs": {"get", "post"},
    "/confidential/job-transitions": {"post"},
    "/confidential/job-audit-events": {"get"},
    "/confidential/participants": {"get", "post"},
    "/confidential/participant-transitions": {"post"},
    "/confidential/participant-audit-events": {"get"},
}

HTTP_METHODS = {"get", "put", "post", "delete", "head", "patch", "options"}
REQUIRED_COMPONENTS = {
    "securitySchemes",
    "parameters",
    "requestBodies",
    "responses",
    "schemas",
}
REQUIRED_SCHEMAS = {
    "AdditiveAggregateRequest",
    "AdditiveAggregateResponse",
    "ReplayRecordMetadata",
    "ShareSigningKeyMetadata",
    "SubmittedShareMetadata",
    "ThresholdReleaseMetadata",
    "ThresholdReleaseRequest",
    "ThresholdReleaseResponse",
    "ShamirShare",
    "VerifiableShamirShare",
    "ShamirThresholdReconstruction",
    "ShamirThresholdReconstructionMetadata",
    "ShamirThresholdReconstructionMetadataList",
    "ShamirThresholdReconstructionResponse",
    "DifferentialPrivacyBudget",
    "DifferentialPrivacyBudgetSpend",
    "DifferentialPrivacyRelease",
    "DifferentialPrivacySampledRelease",
    "DifferentialPrivacyBudgetMetadata",
    "DifferentialPrivacyBudgetMetadataList",
    "DifferentialPrivacyCompositionSummary",
    "DifferentialPrivacyCompositionSummaryList",
    "DifferentialPrivacyReleaseMetadata",
    "DifferentialPrivacyReleaseMetadataList",
    "ConfidentialJobRequest",
    "ConfidentialJobTransitionRequest",
    "ConfidentialJobMetadata",
    "ConfidentialJobMetadataList",
    "ConfidentialJobAuditEvent",
    "ConfidentialJobAuditEventList",
    "ConfidentialParticipantStatus",
    "ConfidentialParticipantRole",
    "ConfidentialParticipantRequest",
    "ConfidentialParticipantTransitionRequest",
    "ConfidentialParticipantMetadata",
    "ConfidentialParticipantMetadataList",
    "ConfidentialParticipantAuditEvent",
    "ConfidentialParticipantAuditEventList",
}
FORBIDDEN_METADATA_FIELDS = {
    "aggregate",
    "aggregateValue",
    "commitment",
    "proof",
    "share",
    "signature",
    "signedEnvelope",
    "signed_envelope",
    "openingNonce",
    "opening_nonce",
    "transportProof",
    "value",
}
REQUIRED_OPERATION_SCOPES = {
    "postConfidentialAdditiveAggregate": ["confidential:compute"],
    "getConfidentialShareSigningKeys": ["confidential:read"],
    "postConfidentialShareSigningKey": ["confidential:write"],
    "postConfidentialShareSigningKeyRotation": ["confidential:write"],
    "postConfidentialShareSigningKeyRevocation": ["confidential:write"],
    "getConfidentialAdditiveShareSubmissions": ["confidential:read"],
    "postConfidentialAdditiveShareSubmission": ["confidential:write"],
    "getConfidentialAdditiveShareReplayRecords": ["confidential:read"],
    "getConfidentialThresholdReleaseRequests": ["confidential:read"],
    "postConfidentialThresholdReleaseRequest": ["confidential:write"],
    "postConfidentialThresholdReleaseApproval": ["confidential:release"],
    "postConfidentialThresholdRelease": ["confidential:release"],
    "getConfidentialShamirThresholdReconstructions": ["confidential:read"],
    "postConfidentialShamirThresholdReconstruction": ["confidential:compute"],
    "getConfidentialDifferentialPrivacyBudgets": ["confidential:read"],
    "postConfidentialDifferentialPrivacyBudget": ["confidential:write"],
    "postConfidentialDifferentialPrivacyBudgetSpend": ["confidential:write"],
    "getConfidentialDifferentialPrivacyCompositionSummaries": ["confidential:read"],
    "getConfidentialDifferentialPrivacyReleases": ["confidential:read"],
    "postConfidentialDifferentialPrivacyRelease": ["confidential:release"],
    "postConfidentialDifferentialPrivacySampledRelease": ["confidential:release"],
    "getConfidentialJobs": ["confidential:read"],
    "postConfidentialJob": ["confidential:write"],
    "postConfidentialJobTransition": ["confidential:write"],
    "getConfidentialJobAuditEvents": ["confidential:read"],
    "getConfidentialParticipants": ["confidential:read"],
    "postConfidentialParticipant": ["confidential:write"],
    "postConfidentialParticipantTransition": ["confidential:write"],
    "getConfidentialParticipantAuditEvents": ["confidential:read"],
}
REQUIRED_DP_RELEASE_METADATA_FIELDS = {
    "adjacencyModel",
}
REQUIRED_DP_COMPOSITION_SUMMARY_FIELDS = {
    "advancedCompositionRule",
    "advancedCompositionDeltaMicrounits",
    "advancedCompositionEpsilonMicrounits",
    "releaseEpsilonSquareSumMicrounits2",
}
REQUIRED_SHAMIR_RECONSTRUCTION_METADATA_FIELDS = {
    "verificationProfile",
    "verifiedShareCount",
    "complaintCount",
}
REQUIRED_CONFIDENTIAL_JOB_MECHANISMS = {
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


class ContractError(RuntimeError):
    """Raised when the confidential OpenAPI contract violates a Kiva invariant."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise ContractError("OpenAPI document root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a contract error."""
    if not isinstance(value, dict):
        raise ContractError(f"{label} must be a mapping")
    return value


def require_success_response(operation: dict[str, Any], label: str) -> None:
    """Require each operation to define a 200 or 204 success response."""
    responses = require_mapping(operation.get("responses"), f"{label}.responses")
    if "200" not in responses and "204" not in responses:
        raise ContractError(f"{label} must define a 200 or 204 success response")


def require_operation_scope(operation: dict[str, Any], label: str) -> None:
    """Require each confidential operation to declare its resource-server scope."""
    operation_id = operation.get("operationId")
    if not isinstance(operation_id, str) or not operation_id:
        raise ContractError(f"{label} is missing operationId")
    expected = REQUIRED_OPERATION_SCOPES.get(operation_id)
    if expected is None:
        raise ContractError(f"{label} has no expected confidential scope mapping: {operation_id}")
    scopes = operation.get("x-kiva-required-scopes")
    if scopes != expected:
        raise ContractError(
            f"{label}.x-kiva-required-scopes must be exactly {expected!r}",
        )


def schema_properties(schema: dict[str, Any], label: str) -> set[str]:
    """Return a schema's property names."""
    properties = schema.get("properties")
    if not isinstance(properties, dict):
        raise ContractError(f"{label}.properties must be a mapping")
    return set(properties)


def validate_metadata_schema(schemas: dict[str, Any], name: str) -> None:
    """Require metadata schemas not to expose share material."""
    schema = require_mapping(schemas.get(name), f"components.schemas.{name}")
    fields = schema_properties(schema, f"components.schemas.{name}")
    forbidden = FORBIDDEN_METADATA_FIELDS.intersection(fields)
    if forbidden:
        joined = ", ".join(sorted(forbidden))
        raise ContractError(f"{name} exposes forbidden share payload fields: {joined}")


def validate_required_schema_fields(
    schemas: dict[str, Any],
    name: str,
    required_fields: set[str],
) -> None:
    """Require a schema to list expected fields as required properties."""
    schema = require_mapping(schemas.get(name), f"components.schemas.{name}")
    required = schema.get("required")
    if not isinstance(required, list):
        raise ContractError(f"{name}.required must be a list")
    missing = required_fields.difference(required)
    if missing:
        joined = ", ".join(sorted(missing))
        raise ContractError(f"{name} is missing required fields: {joined}")


def validate_confidential_job_mechanism_enum(schemas: dict[str, Any]) -> None:
    """Require all supported confidential job mechanisms to remain in OpenAPI."""
    schema = require_mapping(
        schemas.get("ConfidentialJobMechanism"),
        "components.schemas.ConfidentialJobMechanism",
    )
    enum = schema.get("enum")
    if not isinstance(enum, list):
        raise ContractError("ConfidentialJobMechanism.enum must be a list")
    actual = set(enum)
    if actual != REQUIRED_CONFIDENTIAL_JOB_MECHANISMS:
        raise ContractError(
            "ConfidentialJobMechanism enum drifted: "
            f"expected {sorted(REQUIRED_CONFIDENTIAL_JOB_MECHANISMS)!r}, "
            f"got {sorted(actual)!r}",
        )


def validate_contract(document: dict[str, Any]) -> None:
    """Validate Kiva confidential-computation OpenAPI invariants."""
    if document.get("openapi") != "3.1.0":
        raise ContractError("OpenAPI version must be 3.1.0")

    contract = require_mapping(document.get("x-kiva-contract"), "x-kiva-contract")
    if contract.get("area") != "confidential-computation":
        raise ContractError("x-kiva-contract.area must be confidential-computation")

    paths = require_mapping(document.get("paths"), "paths")
    components = require_mapping(document.get("components"), "components")
    missing_components = REQUIRED_COMPONENTS.difference(components)
    if missing_components:
        joined = ", ".join(sorted(missing_components))
        raise ContractError(f"missing OpenAPI component sections: {joined}")

    for path, methods in REQUIRED_PATHS.items():
        path_item = require_mapping(paths.get(path), f"paths.{path}")
        missing_methods = methods.difference(path_item)
        if missing_methods:
            joined = ", ".join(sorted(missing_methods))
            raise ContractError(f"{path} is missing methods: {joined}")

    for path, path_item in paths.items():
        path_map = require_mapping(path_item, f"paths.{path}")
        for method, operation in path_map.items():
            if method not in HTTP_METHODS:
                continue
            operation_map = require_mapping(operation, f"{path}.{method}")
            require_operation_scope(operation_map, f"{path}.{method}")
            require_success_response(operation_map, f"{path}.{method}")

    schemas = require_mapping(components.get("schemas"), "components.schemas")
    missing_schemas = REQUIRED_SCHEMAS.difference(schemas)
    if missing_schemas:
        joined = ", ".join(sorted(missing_schemas))
        raise ContractError(f"missing confidential schemas: {joined}")

    for metadata_schema in (
        "ReplayRecordMetadata",
        "ShareSigningKeyMetadata",
        "SubmittedShareMetadata",
        "ThresholdReleaseMetadata",
        "ShamirThresholdReconstructionMetadata",
        "DifferentialPrivacyBudgetMetadata",
        "DifferentialPrivacyReleaseMetadata",
        "ConfidentialJobMetadata",
        "ConfidentialJobAuditEvent",
        "ConfidentialParticipantMetadata",
        "ConfidentialParticipantAuditEvent",
    ):
        validate_metadata_schema(schemas, metadata_schema)

    validate_required_schema_fields(
        schemas,
        "DifferentialPrivacyReleaseMetadata",
        REQUIRED_DP_RELEASE_METADATA_FIELDS,
    )
    validate_required_schema_fields(
        schemas,
        "DifferentialPrivacyCompositionSummary",
        REQUIRED_DP_COMPOSITION_SUMMARY_FIELDS,
    )
    validate_required_schema_fields(
        schemas,
        "ShamirThresholdReconstructionMetadata",
        REQUIRED_SHAMIR_RECONSTRUCTION_METADATA_FIELDS,
    )
    validate_confidential_job_mechanism_enum(schemas)


def main() -> int:
    """Run the confidential OpenAPI validator."""
    parser = argparse.ArgumentParser()
    parser.add_argument("contract", type=Path)
    args = parser.parse_args()

    validate_contract(load_yaml(args.contract))
    print(f"validated {args.contract}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
