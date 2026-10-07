"""Ownership overlay projection for Kiva public surface operations."""

from __future__ import annotations

from typing import Any, TypeVar

from kiva_ci.manifest import (
    ManifestError,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
    validation_error,
)


ErrorT = TypeVar("ErrorT", bound=RuntimeError)

OWNERSHIP_SOURCE_KIND = "kiva-operation-ownership-source"
HTTP_DERIVED_FIELDS = {"method", "path", "required_scopes"}
OWNERSHIP_KEY_FIELDS = ("openapi_contract", "operation_id")
PASSTHROUGH_CONTRACTS_FIELD = "derived_passthrough_contracts"
OWNED_OPTIONAL_FIELDS = (
    "application_operation_id",
    "application_surface",
    "workflow_label",
    "rust_workflow_kind",
    "workflow_family",
    "pds_resource_family",
    "pds_route_context",
)


def validate_operation_ownership_source(
    document: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> list[dict[str, Any]]:
    """Validate and return operation ownership overlay rows."""
    if document.get("schema_version") != 1:
        raise validation_error(error_cls, "operation_ownership.schema_version must be 1")
    if document.get("kind") != OWNERSHIP_SOURCE_KIND:
        raise validation_error(error_cls, f"operation_ownership.kind must be {OWNERSHIP_SOURCE_KIND}")
    if document.get("status") != "active":
        raise validation_error(error_cls, "operation_ownership.status must be active")
    key_fields = tuple(
        require_string_list(
            document.get("key_fields"),
            "operation_ownership.key_fields",
            error_cls,
        )
    )
    if key_fields != OWNERSHIP_KEY_FIELDS:
        raise validation_error(
            error_cls,
            f"operation_ownership.key_fields must be {list(OWNERSHIP_KEY_FIELDS)}",
        )
    owned_fields = tuple(
        require_string_list(
            document.get("owned_fields"),
            "operation_ownership.owned_fields",
            error_cls,
        )
    )
    if owned_fields != OWNED_OPTIONAL_FIELDS:
        raise validation_error(
            error_cls,
            f"operation_ownership.owned_fields must be {list(OWNED_OPTIONAL_FIELDS)}",
        )
    require_string_list(
        document.get(PASSTHROUGH_CONTRACTS_FIELD, []),
        f"operation_ownership.{PASSTHROUGH_CONTRACTS_FIELD}",
        error_cls,
        allow_empty=True,
    )

    rows: list[dict[str, Any]] = []
    seen: set[tuple[str, str]] = set()
    for index, value in enumerate(
        require_list(document.get("entries"), "operation_ownership.entries", error_cls)
    ):
        label = f"operation_ownership.entries[{index}]"
        entry = require_mapping(value, label, error_cls)
        forbidden = sorted(HTTP_DERIVED_FIELDS.intersection(entry))
        if forbidden:
            raise validation_error(error_cls, f"{label} contains OpenAPI-derived fields: {forbidden}")
        key = (
            require_string(entry.get("openapi_contract"), f"{label}.openapi_contract", error_cls),
            require_string(entry.get("operation_id"), f"{label}.operation_id", error_cls),
        )
        if key in seen:
            raise validation_error(error_cls, f"duplicate operation ownership key: {key}")
        seen.add(key)
        row = {
            "openapi_contract": key[0],
            "operation_id": key[1],
        }
        for field in OWNED_OPTIONAL_FIELDS:
            value = entry.get(field)
            if value is not None:
                row[field] = require_string(value, f"{label}.{field}", error_cls)
        rows.append(row)
    return rows


def derived_passthrough_contracts(
    document: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> set[str]:
    """Return contracts whose HTTP-only rows are projected from OpenAPI."""
    return set(
        require_string_list(
            document.get(PASSTHROUGH_CONTRACTS_FIELD, []),
            f"operation_ownership.{PASSTHROUGH_CONTRACTS_FIELD}",
            error_cls,
            allow_empty=True,
        )
    )


def project_surface_entries_from_ownership(
    ownership_source: dict[str, Any],
    http_operations: list[dict[str, Any]],
    error_cls: type[ErrorT] = ManifestError,
) -> list[dict[str, Any]]:
    """Join OpenAPI-derived HTTP operations with human-owned operation overlay rows."""
    ownership_rows = validate_operation_ownership_source(ownership_source, error_cls)
    passthrough_contracts = derived_passthrough_contracts(ownership_source, error_cls)
    ownership_by_key = {
        (entry["openapi_contract"], entry["operation_id"]): entry
        for entry in ownership_rows
    }
    http_by_key = {
        (operation["openapi_contract"], operation["operation_id"]): operation
        for operation in http_operations
    }
    ownership_only = sorted(set(ownership_by_key).difference(http_by_key))
    http_only = sorted(set(http_by_key).difference(ownership_by_key))
    if ownership_only:
        formatted = ", ".join(f"{contract}#{operation_id}" for contract, operation_id in ownership_only)
        raise validation_error(error_cls, "operation ownership entries missing from HTTP catalog: " + formatted)
    forbidden_http_only = [
        key for key in http_only if key[0] not in passthrough_contracts
    ]
    if forbidden_http_only:
        formatted = ", ".join(
            f"{contract}#{operation_id}" for contract, operation_id in forbidden_http_only
        )
        raise validation_error(error_cls, "HTTP operations missing from operation ownership source: " + formatted)

    entries: list[dict[str, Any]] = []
    for operation in http_operations:
        key = (operation["openapi_contract"], operation["operation_id"])
        ownership = ownership_by_key.get(key, {})
        entry: dict[str, Any] = {
            "openapi_contract": operation["openapi_contract"],
            "method": operation["method"],
            "path": operation["path"],
            "operation_id": operation["operation_id"],
            "required_scopes": list(operation["required_scopes"]),
        }
        for field in OWNED_OPTIONAL_FIELDS:
            if field in ownership:
                entry[field] = ownership[field]
        entries.append(entry)
    return entries
