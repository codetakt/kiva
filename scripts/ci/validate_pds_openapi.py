#!/usr/bin/env python3
"""Validate Kiva PDS OpenAPI contracts."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import yaml


REQUIRED_PATHS = {
    "/pds/consent-grants": {"get", "post"},
    "/pds/consent-revocations": {"post"},
    "/pds/data-products": {"get", "post"},
    "/pds/data-product-reads": {"post"},
    "/pds/provider-bindings": {"get", "post"},
    "/pds/record-inventory": {"get"},
    "/pds/metadata-exports": {"get"},
    "/pds/data-portability-exports": {"get"},
    "/pds/lifecycle-events": {"get", "post"},
    "/pds/retention-policies": {"get", "post"},
    "/pds/lifecycle-cases": {"get", "post"},
    "/pds/governance-profiles": {"get", "post"},
    "/pds/access-checks": {"post"},
    "/pds/access-audit-events": {"get"},
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
    "AccessAuditEvent",
    "AccessAuditEventList",
    "AccessCheckRequest",
    "AccessCheckResponse",
    "ConsentGrant",
    "ConsentGrantList",
    "ConsentGrantRequest",
    "ConsentRevocationRequest",
    "ConfidentialMechanismKind",
    "DataClass",
    "DataProduct",
    "DataProductList",
    "DataProductReadRequest",
    "DataProductReadResponse",
    "DataProductRequest",
    "GovernanceProfile",
    "GovernanceProfileKind",
    "GovernanceProfileList",
    "GovernanceProfileRequest",
    "LegalBasis",
    "LifecycleCase",
    "LifecycleCaseList",
    "LifecycleCaseRequest",
    "LifecycleCaseStatus",
    "LifecycleCaseType",
    "LifecycleEvent",
    "LifecycleEventList",
    "LifecycleEventRequest",
    "LifecycleEventType",
    "LifecycleResourceKind",
    "MetadataExport",
    "PdsSubjectAgent",
    "PortabilityExport",
    "PortabilityStatement",
    "ProviderBinding",
    "ProviderBindingList",
    "ProviderBindingRequest",
    "RecordInventory",
    "ReleaseArtifactKind",
    "ReleaseArtifactRef",
    "RetentionPolicy",
    "RetentionPolicyList",
    "RetentionPolicyRequest",
    "SecretComputeProviderCapability",
    "SecretComputeProviderEvidenceGate",
    "SecretComputeProviderKind",
    "SecretComputeProviderProfile",
    "SecretComputeProviderReleaseGate",
    "SecretComputeProviderRuntimeClass",
    "StatementInventoryRecord",
    "TombstonePublicationRule",
}
FORBIDDEN_AUDIT_FIELDS = {
    "aggregate",
    "aggregateValue",
    "noiseValue",
    "rawStatement",
    "reconstructedValue",
    "result",
    "share",
    "shares",
    "signature",
    "signedEnvelope",
    "statement",
}
RAW_PAYLOAD_SCHEMA_NAMES = {
    "AccessAuditEvent",
    "DataProduct",
    "DataProductReadResponse",
    "GovernanceProfile",
    "GovernanceProfileList",
    "LifecycleCase",
    "LifecycleCaseList",
    "LifecycleEvent",
    "MetadataExport",
    "ProviderBinding",
    "ProviderBindingList",
    "RecordInventory",
    "RetentionPolicy",
    "RetentionPolicyList",
    "StatementInventoryRecord",
}
REQUIRED_DATA_PRODUCT_SOURCE_KINDS = {
    "threshold-release",
    "differential-privacy-release",
    "shamir-threshold-reconstruction",
}
REQUIRED_TOMBSTONE_PUBLICATION_RULES = {
    "metadata-only",
    "subject-visible",
    "public-redacted",
}
REQUIRED_LIFECYCLE_CASE_STATUSES = {
    "open",
    "under-review",
    "resolved",
    "rejected",
    "tombstoned",
}
REQUIRED_METADATA_EXPORT_FIELDS = {
    "retentionPolicies",
    "lifecycleCases",
}
REQUIRED_OPERATION_SCOPES = {
    "getPdsConsentGrants": ["pds:read"],
    "postPdsConsentGrant": ["pds:delegate"],
    "postPdsConsentRevocation": ["pds:revoke"],
    "getPdsDataProducts": ["pds:read"],
    "postPdsDataProduct": ["pds:write"],
    "postPdsDataProductRead": ["pds:read"],
    "getPdsProviderBindings": ["pds:read"],
    "postPdsProviderBinding": ["pds:write"],
    "getPdsRecordInventory": ["pds:read"],
    "getPdsMetadataExports": ["pds:read"],
    "getPdsDataPortabilityExports": ["pds:read"],
    "getPdsLifecycleEvents": ["pds:read"],
    "postPdsLifecycleEvent": ["pds:write"],
    "getPdsRetentionPolicies": ["pds:read"],
    "postPdsRetentionPolicy": ["pds:write"],
    "getPdsLifecycleCases": ["pds:read"],
    "postPdsLifecycleCase": ["pds:write"],
    "getPdsGovernanceProfiles": ["pds:read"],
    "postPdsGovernanceProfile": ["pds:write"],
    "postPdsAccessCheck": ["pds:read"],
    "getPdsAccessAuditEvents": ["pds:audit"],
}


class ContractError(RuntimeError):
    """Raised when the PDS OpenAPI contract violates a Kiva invariant."""


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
    """Require each PDS operation to declare its exact resource-server scope."""
    operation_id = operation.get("operationId")
    if not isinstance(operation_id, str) or not operation_id:
        raise ContractError(f"{label} is missing operationId")
    expected = REQUIRED_OPERATION_SCOPES.get(operation_id)
    if expected is None:
        raise ContractError(f"{label} has no expected PDS scope mapping: {operation_id}")
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


def validate_contract(document: dict[str, Any]) -> None:
    """Validate Kiva PDS OpenAPI invariants."""
    if document.get("openapi") != "3.1.0":
        raise ContractError("OpenAPI version must be 3.1.0")

    contract = require_mapping(document.get("x-kiva-contract"), "x-kiva-contract")
    if contract.get("area") != "personal-data-store":
        raise ContractError("x-kiva-contract.area must be personal-data-store")
    if contract.get("status") != "Kiva platform extension, not part of the IEEE xAPI LRS surface.":
        raise ContractError("x-kiva-contract.status must mark the PDS API as a platform extension")

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
        raise ContractError(f"missing PDS schemas: {joined}")

    for schema_name in RAW_PAYLOAD_SCHEMA_NAMES:
        schema = require_mapping(schemas.get(schema_name), f"components.schemas.{schema_name}")
        forbidden = FORBIDDEN_AUDIT_FIELDS.intersection(
            schema_properties(schema, f"components.schemas.{schema_name}")
        )
        if forbidden:
            joined = ", ".join(sorted(forbidden))
            raise ContractError(f"{schema_name} exposes forbidden payload fields: {joined}")

    source_kind_schema = require_mapping(
        schemas.get("DataProductSourceKind"),
        "components.schemas.DataProductSourceKind",
    )
    source_kind_enum = source_kind_schema.get("enum")
    if not isinstance(source_kind_enum, list):
        raise ContractError("DataProductSourceKind.enum must be a list")
    missing_source_kinds = REQUIRED_DATA_PRODUCT_SOURCE_KINDS.difference(source_kind_enum)
    if missing_source_kinds:
        joined = ", ".join(sorted(missing_source_kinds))
        raise ContractError(f"DataProductSourceKind is missing values: {joined}")

    artifact_kind_schema = require_mapping(
        schemas.get("ReleaseArtifactKind"),
        "components.schemas.ReleaseArtifactKind",
    )
    artifact_kind_enum = artifact_kind_schema.get("enum")
    if not isinstance(artifact_kind_enum, list):
        raise ContractError("ReleaseArtifactKind.enum must be a list")
    missing_artifact_kinds = REQUIRED_DATA_PRODUCT_SOURCE_KINDS.difference(artifact_kind_enum)
    if missing_artifact_kinds:
        joined = ", ".join(sorted(missing_artifact_kinds))
        raise ContractError(f"ReleaseArtifactKind is missing values: {joined}")

    data_product_schema = require_mapping(
        schemas.get("DataProduct"),
        "components.schemas.DataProduct",
    )
    data_product_fields = schema_properties(data_product_schema, "components.schemas.DataProduct")
    if "releaseArtifact" not in data_product_fields:
        raise ContractError("DataProduct must expose releaseArtifact")

    tombstone_schema = require_mapping(
        schemas.get("TombstonePublicationRule"),
        "components.schemas.TombstonePublicationRule",
    )
    tombstone_enum = tombstone_schema.get("enum")
    if not isinstance(tombstone_enum, list):
        raise ContractError("TombstonePublicationRule.enum must be a list")
    missing_tombstone_rules = REQUIRED_TOMBSTONE_PUBLICATION_RULES.difference(tombstone_enum)
    if missing_tombstone_rules:
        joined = ", ".join(sorted(missing_tombstone_rules))
        raise ContractError(f"TombstonePublicationRule is missing values: {joined}")

    lifecycle_status_schema = require_mapping(
        schemas.get("LifecycleCaseStatus"),
        "components.schemas.LifecycleCaseStatus",
    )
    lifecycle_status_enum = lifecycle_status_schema.get("enum")
    if not isinstance(lifecycle_status_enum, list):
        raise ContractError("LifecycleCaseStatus.enum must be a list")
    missing_lifecycle_statuses = REQUIRED_LIFECYCLE_CASE_STATUSES.difference(
        lifecycle_status_enum,
    )
    if missing_lifecycle_statuses:
        joined = ", ".join(sorted(missing_lifecycle_statuses))
        raise ContractError(f"LifecycleCaseStatus is missing values: {joined}")

    metadata_export_schema = require_mapping(
        schemas.get("MetadataExport"),
        "components.schemas.MetadataExport",
    )
    metadata_export_fields = schema_properties(
        metadata_export_schema,
        "components.schemas.MetadataExport",
    )
    missing_export_fields = REQUIRED_METADATA_EXPORT_FIELDS.difference(metadata_export_fields)
    if missing_export_fields:
        joined = ", ".join(sorted(missing_export_fields))
        raise ContractError(f"MetadataExport is missing fields: {joined}")


def main() -> int:
    """Run the PDS OpenAPI validator."""
    parser = argparse.ArgumentParser()
    parser.add_argument("contract", type=Path)
    args = parser.parse_args()

    validate_contract(load_yaml(args.contract))
    print(f"validated {args.contract}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
