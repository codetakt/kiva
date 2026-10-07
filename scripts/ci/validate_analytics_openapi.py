#!/usr/bin/env python3
"""Validate the governed analytics OpenAPI contract."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import yaml


EXPECTED_OPERATIONS = {
    ("/analytics/products", "get"): ("getAnalyticsProducts", {"200", "400", "401", "403"}),
    ("/analytics/products/{dataProductId}", "get"): (
        "getAnalyticsProduct",
        {"200", "400", "401", "403", "404"},
    ),
    ("/analytics/products/{dataProductId}/aggregate", "get"): (
        "getAnalyticsProductAggregate",
        {"200", "400", "401", "403", "404"},
    ),
}
REQUIRED_SCHEMAS = {
    "AnalyticsAggregate",
    "AnalyticsProduct",
    "AnalyticsProductPage",
    "ErrorResponse",
}
FORBIDDEN_FIELDS = {
    "actor",
    "aggregate",
    "aggregateValue",
    "authority",
    "mbox",
    "noise",
    "noiseValue",
    "openid",
    "owner",
    "ownerId",
    "rawRecord",
    "rawStatement",
    "reconstructedValue",
    "share",
    "shares",
    "signature",
    "signedEnvelope",
    "statement",
    "subject",
    "subjectId",
}
ALLOWED_RELEASE_KINDS = {
    "threshold-release",
    "differential-privacy-release",
}


class ContractError(RuntimeError):
    """Raised when the analytics contract violates a serving invariant."""


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or reject the malformed contract."""
    if not isinstance(value, dict):
        raise ContractError(f"{label} must be a mapping")
    return value


def load_yaml(path: Path) -> dict[str, Any]:
    """Load an OpenAPI mapping."""
    with path.open("r", encoding="utf-8") as handle:
        document = yaml.safe_load(handle)
    return require_mapping(document, "OpenAPI document")


def schema_properties(schema: dict[str, Any], label: str) -> set[str]:
    """Return a schema's property names."""
    return set(require_mapping(schema.get("properties"), f"{label}.properties"))


def validate_contract(document: dict[str, Any]) -> None:
    """Validate the governed aggregate serving boundary."""
    if document.get("openapi") != "3.1.0":
        raise ContractError("OpenAPI version must be 3.1.0")

    contract = require_mapping(document.get("x-kiva-contract"), "x-kiva-contract")
    if contract.get("area") != "governed-analytics":
        raise ContractError("x-kiva-contract.area must be governed-analytics")
    expected_status = "Kiva platform extension, not part of the IEEE xAPI LRS surface."
    if contract.get("status") != expected_status:
        raise ContractError("x-kiva-contract.status must separate analytics from xAPI")

    paths = require_mapping(document.get("paths"), "paths")
    observed = {
        (path, method)
        for path, path_item in paths.items()
        for method in require_mapping(path_item, f"paths.{path}")
        if method in {"get", "put", "post", "delete", "patch", "head", "options"}
    }
    if observed != set(EXPECTED_OPERATIONS):
        raise ContractError("analytics contract must expose exactly the three governed GET operations")

    for key, (operation_id, required_responses) in EXPECTED_OPERATIONS.items():
        path, method = key
        operation = require_mapping(paths[path][method], f"paths.{path}.{method}")
        if operation.get("operationId") != operation_id:
            raise ContractError(f"{path}.{method}.operationId must be {operation_id}")
        if operation.get("x-kiva-required-scopes") != ["analytics:read"]:
            raise ContractError(f"{path}.{method} must require exactly analytics:read")
        responses = require_mapping(operation.get("responses"), f"{path}.{method}.responses")
        missing = required_responses.difference(responses)
        if missing:
            raise ContractError(f"{path}.{method} is missing responses: {', '.join(sorted(missing))}")

    components = require_mapping(document.get("components"), "components")
    parameters = require_mapping(components.get("parameters"), "components.parameters")
    page_size = require_mapping(parameters.get("PageSize"), "components.parameters.PageSize")
    page_schema = require_mapping(page_size.get("schema"), "components.parameters.PageSize.schema")
    if page_schema.get("minimum") != 1 or page_schema.get("maximum") != 100:
        raise ContractError("PageSize must be bounded from 1 through 100")
    cursor = require_mapping(parameters.get("Cursor"), "components.parameters.Cursor")
    cursor_schema = require_mapping(cursor.get("schema"), "components.parameters.Cursor.schema")
    if cursor_schema.get("minLength") != 1 or cursor_schema.get("maxLength") != 128:
        raise ContractError("Cursor length must be bounded from 1 through 128")

    schemas = require_mapping(components.get("schemas"), "components.schemas")
    missing_schemas = REQUIRED_SCHEMAS.difference(schemas)
    if missing_schemas:
        raise ContractError(f"missing analytics schemas: {', '.join(sorted(missing_schemas))}")
    for name in REQUIRED_SCHEMAS:
        schema = require_mapping(schemas[name], f"components.schemas.{name}")
        forbidden = FORBIDDEN_FIELDS.intersection(schema_properties(schema, f"components.schemas.{name}"))
        if forbidden:
            raise ContractError(f"{name} exposes forbidden fields: {', '.join(sorted(forbidden))}")

    for name in ("AnalyticsProduct", "AnalyticsAggregate"):
        schema = require_mapping(schemas[name], f"components.schemas.{name}")
        properties = require_mapping(schema.get("properties"), f"components.schemas.{name}.properties")
        release_kind = require_mapping(properties.get("releaseKind"), f"{name}.releaseKind")
        values = release_kind.get("enum")
        if not isinstance(values, list) or set(values) != ALLOWED_RELEASE_KINDS:
            raise ContractError(f"{name}.releaseKind must be limited to admitted aggregate releases")


def main() -> int:
    """Run the analytics OpenAPI validator."""
    parser = argparse.ArgumentParser()
    parser.add_argument("contract", type=Path)
    args = parser.parse_args()
    validate_contract(load_yaml(args.contract))
    print(f"validated {args.contract}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
