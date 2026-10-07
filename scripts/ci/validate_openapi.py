#!/usr/bin/env python3
"""Validate Kiva OpenAPI contracts with repository-specific invariants."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import yaml


DEFAULT_REQUIREMENT_CATALOG = Path("spec/requirements/ieee-39274-1-1-2025.yaml")

REQUIRED_PATHS = {
    "/statements": {"put", "post", "get", "head"},
    "/activities/state": {"put", "post", "get", "delete", "head"},
    "/agents": {"get", "head"},
    "/activities": {"get", "head"},
    "/agents/profile": {"put", "post", "get", "delete", "head"},
    "/activities/profile": {"put", "post", "get", "delete", "head"},
    "/about": {"get", "head"},
}

HTTP_METHODS = {"get", "put", "post", "delete", "head", "patch", "options"}
XAPI_VERSION_HEADER = "X-Experience-API-Version"

REQUIRED_COMPONENTS = {
    "securitySchemes",
    "headers",
    "parameters",
    "requestBodies",
    "responses",
    "schemas",
}
REQUIRED_OPERATION_SCOPES = {
    "putStatement": ["xapi:write"],
    "postStatements": ["xapi:write"],
    "getStatements": ["xapi:read"],
    "headStatements": ["xapi:read"],
    "putStateDocument": ["xapi:write"],
    "postStateDocument": ["xapi:write"],
    "getStateDocuments": ["xapi:read"],
    "deleteStateDocuments": ["xapi:write"],
    "headStateDocuments": ["xapi:read"],
    "getAgentPerson": ["xapi:read"],
    "headAgentPerson": ["xapi:read"],
    "getActivity": ["xapi:read"],
    "headActivity": ["xapi:read"],
    "putAgentProfile": ["xapi:write"],
    "postAgentProfile": ["xapi:write"],
    "getAgentProfiles": ["xapi:read"],
    "deleteAgentProfile": ["xapi:write"],
    "headAgentProfile": ["xapi:read"],
    "putActivityProfile": ["xapi:write"],
    "postActivityProfile": ["xapi:write"],
    "getActivityProfiles": ["xapi:read"],
    "deleteActivityProfile": ["xapi:write"],
    "headActivityProfile": ["xapi:read"],
}


class ContractError(RuntimeError):
    """Raised when the OpenAPI contract violates a Kiva invariant."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise ContractError("OpenAPI document root must be a mapping")
    return value


def load_requirement_catalog(path: Path) -> dict[str, dict[str, Any]]:
    """Load and validate a Kiva requirement catalog."""
    catalog = load_yaml(path)
    requirements = catalog.get("requirements")
    if not isinstance(requirements, list):
        raise ContractError("requirement catalog must contain a requirements list")

    by_id: dict[str, dict[str, Any]] = {}
    for index, requirement in enumerate(requirements):
        requirement_map = require_mapping(requirement, f"requirements[{index}]")
        requirement_id = requirement_map.get("id")
        if not isinstance(requirement_id, str) or not requirement_id:
            raise ContractError(f"requirements[{index}].id must be a non-empty string")
        if requirement_id in by_id:
            raise ContractError(f"duplicate requirement id: {requirement_id}")
        for key in ("clause", "level", "summary", "projection"):
            if key not in requirement_map:
                raise ContractError(f"{requirement_id} is missing {key}")
        by_id[requirement_id] = requirement_map
    return by_id


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a contract error."""
    if not isinstance(value, dict):
        raise ContractError(f"{label} must be a mapping")
    return value


def resolve_ref(document: dict[str, Any], ref: str, label: str) -> Any:
    """Resolve a local JSON Pointer reference."""
    if not ref.startswith("#/"):
        raise ContractError(f"{label} must use a local reference")

    value: Any = document
    for raw_part in ref[2:].split("/"):
        part = raw_part.replace("~1", "/").replace("~0", "~")
        if not isinstance(value, dict) or part not in value:
            raise ContractError(f"{label} references missing path component: {part}")
        value = value[part]
    return value


def response_definition(
    document: dict[str, Any],
    response: dict[str, Any],
    label: str,
) -> dict[str, Any]:
    """Return a concrete response definition, resolving component references."""
    if "$ref" in response:
        resolved = resolve_ref(document, str(response["$ref"]), label)
        return require_mapping(resolved, label)
    return response


def require_xapi_version_header(
    document: dict[str, Any],
    response: dict[str, Any],
    label: str,
) -> None:
    """Require every operation response to declare X-Experience-API-Version."""
    concrete_response = response_definition(document, response, label)
    headers = require_mapping(concrete_response.get("headers"), f"{label}.headers")
    if XAPI_VERSION_HEADER not in headers:
        raise ContractError(f"{label} must define {XAPI_VERSION_HEADER}")


def require_operation_scope(operation: dict[str, Any], label: str) -> None:
    """Require non-/about xAPI operations to declare their Kiva scope gate."""
    operation_id = operation.get("operationId")
    if not isinstance(operation_id, str) or not operation_id:
        raise ContractError(f"{label} is missing operationId")
    expected = REQUIRED_OPERATION_SCOPES.get(operation_id)
    if expected is None:
        if operation.get("security") == []:
            return
        raise ContractError(f"{label} has no expected xAPI scope mapping: {operation_id}")
    scopes = operation.get("x-kiva-required-scopes")
    if scopes != expected:
        raise ContractError(
            f"{label}.x-kiva-required-scopes must be exactly {expected!r}",
        )


def collect_requirement_refs(value: Any) -> set[str]:
    """Collect all x-kiva-requirements entries from a document tree."""
    refs: set[str] = set()
    if isinstance(value, dict):
        raw_requirements = value.get("x-kiva-requirements", [])
        if raw_requirements is not None:
            if not isinstance(raw_requirements, list):
                raise ContractError("x-kiva-requirements must be a list")
            for requirement_id in raw_requirements:
                if not isinstance(requirement_id, str) or not requirement_id:
                    raise ContractError("x-kiva-requirements entries must be non-empty strings")
                refs.add(requirement_id)
        for child_value in value.values():
            refs.update(collect_requirement_refs(child_value))
    elif isinstance(value, list):
        for child_value in value:
            refs.update(collect_requirement_refs(child_value))
    return refs


def validate_requirement_traceability(
    document: dict[str, Any],
    requirements: dict[str, dict[str, Any]],
) -> None:
    """Validate that OpenAPI requirement references and catalog entries align."""
    refs = collect_requirement_refs(document)
    if not refs:
        raise ContractError("OpenAPI document must contain x-kiva-requirements references")

    unknown_refs = refs.difference(requirements)
    if unknown_refs:
        unknown = ", ".join(sorted(unknown_refs))
        raise ContractError(f"unknown x-kiva-requirements references: {unknown}")

    expected_refs = {
        requirement_id
        for requirement_id, requirement in requirements.items()
        if requirement.get("projection") in {"openapi", "openapi-and-assertion", "assertion"}
    }
    missing_refs = expected_refs.difference(refs)
    if missing_refs:
        missing = ", ".join(sorted(missing_refs))
        raise ContractError(f"catalog requirements not referenced by OpenAPI: {missing}")


def validate_contract(document: dict[str, Any], requirements: dict[str, dict[str, Any]]) -> None:
    """Validate high-value Kiva OpenAPI invariants."""
    if document.get("openapi") != "3.1.0":
        raise ContractError("OpenAPI version must be 3.1.0")

    paths = require_mapping(document.get("paths"), "paths")
    components = require_mapping(document.get("components"), "components")

    missing_components = REQUIRED_COMPONENTS.difference(components)
    if missing_components:
        missing = ", ".join(sorted(missing_components))
        raise ContractError(f"missing OpenAPI component sections: {missing}")

    for path, methods in REQUIRED_PATHS.items():
        path_item = require_mapping(paths.get(path), f"paths.{path}")
        missing_methods = methods.difference(path_item)
        if missing_methods:
            missing = ", ".join(sorted(missing_methods))
            raise ContractError(f"{path} is missing methods: {missing}")

    for path, path_item in paths.items():
        if not isinstance(path_item, dict):
            continue
        for method, operation in path_item.items():
            if method not in HTTP_METHODS:
                continue
            operation_map = require_mapping(operation, f"{path}.{method}")
            require_operation_scope(operation_map, f"{path}.{method}")
            responses = require_mapping(operation_map.get("responses"), f"{path}.{method}.responses")
            if "200" not in responses and "204" not in responses:
                raise ContractError(f"{path}.{method} must define a 200 or 204 success response")
            for status, response in responses.items():
                response_map = require_mapping(response, f"{path}.{method}.responses.{status}")
                require_xapi_version_header(document, response_map, f"{path}.{method}.responses.{status}")

    schemas = require_mapping(components.get("schemas"), "components.schemas")
    statement = require_mapping(schemas.get("Statement"), "components.schemas.Statement")
    required_statement_fields = set(statement.get("required", []))
    if not {"actor", "verb", "object"}.issubset(required_statement_fields):
        raise ContractError("Statement schema must require actor, verb, and object")

    actor = require_mapping(schemas.get("Actor"), "components.schemas.Actor")
    if "oneOf" not in actor:
        raise ContractError("Actor schema must distinguish Agent and Group variants")

    contract = require_mapping(document.get("x-kiva-contract"), "x-kiva-contract")
    if contract.get("standard") != "IEEE/ISO/IEC 39274-1-1-2025":
        raise ContractError("x-kiva-contract.standard must name IEEE/ISO/IEC 39274-1-1-2025")
    if contract.get("requirement-catalog") != str(DEFAULT_REQUIREMENT_CATALOG):
        raise ContractError(f"x-kiva-contract.requirement-catalog must be {DEFAULT_REQUIREMENT_CATALOG}")

    validate_requirement_traceability(document, requirements)


def main() -> int:
    """Run the contract validator."""
    parser = argparse.ArgumentParser()
    parser.add_argument("contract", type=Path)
    parser.add_argument("--requirements", type=Path, default=DEFAULT_REQUIREMENT_CATALOG)
    args = parser.parse_args()

    document = load_yaml(args.contract)
    requirements = load_requirement_catalog(args.requirements)
    validate_contract(document, requirements)
    print(f"validated {args.contract}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
