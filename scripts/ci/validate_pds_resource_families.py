#!/usr/bin/env python3
"""Validate PDS resource-family ownership against OpenAPI and routes."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

from kiva_ci.manifest import (
    load_json,
    load_yaml,
    relative_path,
    require_existing_path,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
)
from kiva_ci.ledger import rows_by_unique_string_field


DEFAULT_MANIFEST = Path("generated/architecture/pds-resource-families.yaml")
HTTP_METHODS = {"get", "put", "post", "delete", "patch", "head", "options", "trace"}
ROUTE_CONTEXT_ANCHORS = {
    "pds": ("let pds_router = Router::new()", ".with_state(pds_state)"),
    "pds-application": (
        "let application_router = Router::new()",
        ".with_state(application_state)",
    ),
}
PATH_CONSTANT_REGEX = re.compile(
    r'pub\(crate\)\s+const\s+(?P<name>PDS_[A-Z_]+_PATH):\s+&str\s+=\s+"(?P<path>[^"]+)";'
)


class PdsResourceFamilyError(RuntimeError):
    """Raised when the PDS resource-family ledger drifts."""


def local_ref_name(ref: str, prefix: str) -> str | None:
    """Return the local reference suffix if ref uses the expected prefix."""
    if ref.startswith(prefix):
        return ref.removeprefix(prefix)
    return None


def resolve_local_ref(document: dict[str, Any], ref: str) -> Any:
    """Resolve a local JSON pointer reference in an OpenAPI document."""
    if not ref.startswith("#/"):
        raise PdsResourceFamilyError(f"only local OpenAPI refs are supported: {ref}")
    node: Any = document
    for part in ref.removeprefix("#/").split("/"):
        if not isinstance(node, dict) or part not in node:
            raise PdsResourceFamilyError(f"OpenAPI ref cannot be resolved: {ref}")
        node = node[part]
    return node


def collect_schema_refs(
    document: dict[str, Any],
    node: Any,
    seen_refs: set[str] | None = None,
) -> set[str]:
    """Collect component schema references reachable from a node."""
    if seen_refs is None:
        seen_refs = set()
    if isinstance(node, dict):
        raw_ref = node.get("$ref")
        if isinstance(raw_ref, str):
            schema_name = local_ref_name(raw_ref, "#/components/schemas/")
            if schema_name is not None:
                return {schema_name}
            if raw_ref in seen_refs:
                return set()
            seen_refs.add(raw_ref)
            return collect_schema_refs(document, resolve_local_ref(document, raw_ref), seen_refs)
        refs: set[str] = set()
        for value in node.values():
            refs.update(collect_schema_refs(document, value, seen_refs))
        return refs
    if isinstance(node, list):
        refs: set[str] = set()
        for value in node:
            refs.update(collect_schema_refs(document, value, seen_refs))
        return refs
    return set()


def operation_request_schema_refs(document: dict[str, Any], operation: dict[str, Any]) -> set[str]:
    """Return schema refs used by an operation request body."""
    if "requestBody" not in operation:
        return set()
    return collect_schema_refs(document, operation["requestBody"])


def operation_success_response_schema_refs(
    document: dict[str, Any],
    operation: dict[str, Any],
) -> set[str]:
    """Return schema refs used by 2xx operation responses."""
    responses = require_mapping(
        operation.get("responses"),
        "operation.responses",
        PdsResourceFamilyError,
    )
    refs: set[str] = set()
    for status, response in responses.items():
        status_text = str(status)
        if not status_text.startswith("2"):
            continue
        refs.update(collect_schema_refs(document, response))
    return refs


def collect_pds_openapi_operations(document: dict[str, Any]) -> dict[tuple[str, str], dict[str, Any]]:
    """Return PDS OpenAPI operations keyed by path and lower-case method."""
    operations: dict[tuple[str, str], dict[str, Any]] = {}
    paths = require_mapping(document.get("paths"), "OpenAPI paths", PdsResourceFamilyError)
    for raw_path, path_item in paths.items():
        if not isinstance(raw_path, str) or not raw_path.startswith("/pds/"):
            continue
        path_mapping = require_mapping(path_item, f"paths.{raw_path}", PdsResourceFamilyError)
        for method, operation in path_mapping.items():
            method_lower = str(method).lower()
            if method_lower not in HTTP_METHODS:
                continue
            operations[(raw_path, method_lower)] = require_mapping(
                operation,
                f"paths.{raw_path}.{method_lower}",
                PdsResourceFamilyError,
            )
    return operations


def collect_pds_operation_catalog_entries(
    root: Path,
    raw_catalog_path: str,
    source_contract: Path,
) -> dict[str, dict[str, Any]]:
    """Return PDS operation-catalog entries keyed by OpenAPI operationId."""
    catalog_path = require_existing_path(
        root,
        raw_catalog_path,
        "operation_catalog",
        PdsResourceFamilyError,
    )
    catalog = load_yaml(root / catalog_path, PdsResourceFamilyError)
    entries: dict[str, dict[str, Any]] = {}
    for index, entry_value in enumerate(
        require_list(catalog.get("entries"), f"{raw_catalog_path}.entries", PdsResourceFamilyError)
    ):
        entry = require_mapping(
            entry_value,
            f"{raw_catalog_path}.entries[{index}]",
            PdsResourceFamilyError,
        )
        if entry.get("surface") != "pds":
            continue
        openapi_contract = require_string(
            entry.get("openapi_contract"),
            f"{raw_catalog_path}.entries[{index}].openapi_contract",
            PdsResourceFamilyError,
        )
        if openapi_contract != str(source_contract):
            continue
        operation_id = require_string(
            entry.get("operation_id"),
            f"{raw_catalog_path}.entries[{index}].operation_id",
            PdsResourceFamilyError,
        )
        if operation_id in entries:
            raise PdsResourceFamilyError(
                f"{raw_catalog_path} duplicates PDS operationId: {operation_id}"
            )
        entries[operation_id] = entry
    return entries


def collect_http_operation_catalog_entries(
    root: Path,
    raw_catalog_path: str,
    source_contract: Path,
) -> dict[tuple[str, str], dict[str, Any]]:
    """Return generated HTTP operation-catalog entries keyed by path and method."""
    catalog_path = require_existing_path(
        root,
        raw_catalog_path,
        "http_operation_catalog",
        PdsResourceFamilyError,
    )
    catalog = load_json(root / catalog_path, PdsResourceFamilyError)
    if catalog.get("kind") != "kiva-http-operation-catalog-evidence":
        raise PdsResourceFamilyError(
            f"{raw_catalog_path}.kind must be kiva-http-operation-catalog-evidence"
        )
    generated_from = require_string_list(
        catalog.get("generated_from"),
        f"{raw_catalog_path}.generated_from",
        PdsResourceFamilyError,
    )
    if str(source_contract) not in generated_from:
        raise PdsResourceFamilyError(
            f"{raw_catalog_path}.generated_from must include {source_contract}"
        )
    entries: dict[tuple[str, str], dict[str, Any]] = {}
    for index, entry_value in enumerate(
        require_list(catalog.get("operations"), f"{raw_catalog_path}.operations", PdsResourceFamilyError)
    ):
        entry = require_mapping(
            entry_value,
            f"{raw_catalog_path}.operations[{index}]",
            PdsResourceFamilyError,
        )
        openapi_contract = require_string(
            entry.get("openapi_contract"),
            f"{raw_catalog_path}.operations[{index}].openapi_contract",
            PdsResourceFamilyError,
        )
        if openapi_contract != str(source_contract):
            continue
        path = require_string(
            entry.get("path"),
            f"{raw_catalog_path}.operations[{index}].path",
            PdsResourceFamilyError,
        )
        method = require_string(
            entry.get("method"),
            f"{raw_catalog_path}.operations[{index}].method",
            PdsResourceFamilyError,
        ).lower()
        operation_id = require_string(
            entry.get("operation_id"),
            f"{raw_catalog_path}.operations[{index}].operation_id",
            PdsResourceFamilyError,
        )
        if method not in HTTP_METHODS:
            raise PdsResourceFamilyError(
                f"{raw_catalog_path}.operations[{index}].method is invalid: {method}"
            )
        key = (path, method)
        if key in entries:
            raise PdsResourceFamilyError(
                f"{raw_catalog_path} duplicates PDS HTTP operation: {path} {method}"
            )
        entry["operation_id"] = operation_id
        entries[key] = entry
    return entries


def split_top_level_args(value: str) -> list[str]:
    """Split a Rust macro/function argument list at top-level commas."""
    args: list[str] = []
    start = 0
    depth = 0
    for index, char in enumerate(value):
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
        elif char == "," and depth == 0:
            args.append(value[start:index].strip())
            start = index + 1
    tail = value[start:].strip()
    if tail:
        args.append(tail)
    return args


def matching_paren(text: str, open_paren: int) -> int:
    """Return the matching close paren for one Rust call."""
    depth = 0
    for index in range(open_paren, len(text)):
        char = text[index]
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                return index
    raise PdsResourceFamilyError("unterminated .route(...) call")


def extract_route_handlers(route_block: str) -> dict[tuple[str, str], str]:
    """Return route handlers keyed by path constant and lower-case method."""
    handlers: dict[tuple[str, str], str] = {}
    offset = 0
    while True:
        start = route_block.find(".route(", offset)
        if start < 0:
            break
        open_paren = start + len(".route")
        close_paren = matching_paren(route_block, open_paren)
        route_args = split_top_level_args(route_block[open_paren + 1 : close_paren])
        if len(route_args) < 2:
            raise PdsResourceFamilyError(f"route call has too few arguments: {route_args}")
        path_constant = route_args[0]
        chain = route_args[1]
        for method, handler in re.findall(r"\b(get|post|put|delete|patch)\s*\(\s*([A-Za-z0-9_]+)\s*\)", chain):
            key = (path_constant, method.lower())
            if key in handlers:
                raise PdsResourceFamilyError(
                    f"duplicate route handler for {path_constant} {method}"
                )
            handlers[key] = handler
        offset = close_paren + 1
    return handlers


def extract_route_contexts(route_text: str) -> dict[str, str]:
    """Return route builder blocks keyed by route context id."""
    contexts: dict[str, str] = {}
    for context_id, (start_anchor, end_anchor) in ROUTE_CONTEXT_ANCHORS.items():
        start = route_text.find(start_anchor)
        if start < 0:
            raise PdsResourceFamilyError(
                f"route context {context_id} missing start anchor: {start_anchor}"
            )
        end = route_text.find(end_anchor, start)
        if end < 0:
            raise PdsResourceFamilyError(
                f"route context {context_id} missing end anchor: {end_anchor}"
            )
        contexts[context_id] = route_text[start : end + len(end_anchor)]
    return contexts


def parse_path_constants(path_constants_text: str) -> dict[str, str]:
    """Parse PDS path constants from Rust source."""
    constants = {
        match.group("name"): match.group("path")
        for match in PATH_CONSTANT_REGEX.finditer(path_constants_text)
    }
    if not constants:
        raise PdsResourceFamilyError("no PDS path constants were parsed")
    return constants


def invert_path_constants(path_constants: dict[str, str]) -> dict[str, str]:
    """Return path constants keyed by HTTP path."""
    inverted: dict[str, str] = {}
    for constant, path in path_constants.items():
        if path in inverted:
            raise PdsResourceFamilyError(f"duplicate PDS path constant for path: {path}")
        inverted[path] = constant
    return inverted


def operation_key(operation: dict[str, Any], family_id: str, index: int) -> tuple[str, str]:
    """Validate and return an operation key."""
    raw_path = require_string(
        operation.get("path"),
        f"{family_id}.operations[{index}].path",
        PdsResourceFamilyError,
    )
    method = require_string(
        operation.get("method"),
        f"{family_id}.operations[{index}].method",
        PdsResourceFamilyError,
    ).lower()
    if not raw_path.startswith("/pds/"):
        raise PdsResourceFamilyError(f"{family_id}.operations[{index}].path must start with /pds/")
    if method not in HTTP_METHODS:
        raise PdsResourceFamilyError(f"{family_id}.operations[{index}].method is not supported: {method}")
    return (raw_path, method)


def validate_operation(
    root: Path,
    document: dict[str, Any],
    openapi_operations: dict[tuple[str, str], dict[str, Any]],
    route_handlers: dict[str, dict[tuple[str, str], str]],
    path_constants: dict[str, str],
    path_constants_by_path: dict[str, str],
    operation: dict[str, Any],
    family_id: str,
    index: int,
) -> tuple[tuple[str, str], dict[str, Any]]:
    """Validate one resource-family operation."""
    key = operation_key(operation, family_id, index)
    if key not in openapi_operations:
        raise PdsResourceFamilyError(
            f"{family_id}.operations[{index}] missing from OpenAPI: {key[0]} {key[1]}"
        )

    openapi_operation = openapi_operations[key]

    request_refs = operation_request_schema_refs(document, openapi_operation)
    response_refs = operation_success_response_schema_refs(document, openapi_operation)
    components = require_mapping(
        document.get("components"),
        "OpenAPI components",
        PdsResourceFamilyError,
    )
    schemas = require_mapping(
        components.get("schemas"),
        "OpenAPI components.schemas",
        PdsResourceFamilyError,
    )
    for schema_name in sorted(request_refs.union(response_refs)):
        if schema_name not in schemas:
            raise PdsResourceFamilyError(
                f"{family_id}.operations[{index}] OpenAPI schema is not in components: {schema_name}"
            )

    if "operation_id" in operation and openapi_operation.get("operationId") != require_string(
        operation.get("operation_id"),
        f"{family_id}.operations[{index}].operation_id",
        PdsResourceFamilyError,
    ):
        raise PdsResourceFamilyError(
            f"{family_id}.operations[{index}] operation_id does not match OpenAPI"
        )
    if "request_schemas" in operation:
        declared_request_schemas = set(
            require_string_list(
                operation.get("request_schemas", []),
                f"{family_id}.operations[{index}].request_schemas",
                PdsResourceFamilyError,
                allow_empty=True,
            )
        )
    else:
        declared_request_schemas = request_refs
    if request_refs != declared_request_schemas:
        raise PdsResourceFamilyError(
            f"{family_id}.operations[{index}] request schema drift: "
            f"declared={sorted(declared_request_schemas)} openapi={sorted(request_refs)}"
        )

    if "response_schemas" in operation:
        declared_response_schemas = set(
            require_string_list(
                operation.get("response_schemas", []),
                f"{family_id}.operations[{index}].response_schemas",
                PdsResourceFamilyError,
                allow_empty=True,
            )
        )
    else:
        declared_response_schemas = response_refs
    if response_refs != declared_response_schemas:
        raise PdsResourceFamilyError(
            f"{family_id}.operations[{index}] 2xx response schema drift: "
            f"declared={sorted(declared_response_schemas)} openapi={sorted(response_refs)}"
        )

    derived_path_constant = path_constants_by_path.get(key[0])
    if derived_path_constant is None:
        raise PdsResourceFamilyError(f"{family_id}.operations[{index}] has no path constant")
    path_constant = operation.get("path_constant", derived_path_constant)
    path_constant = require_string(
        path_constant,
        f"{family_id}.operations[{index}].path_constant",
        PdsResourceFamilyError,
    )
    if path_constant != derived_path_constant:
        raise PdsResourceFamilyError(
            f"{family_id}.operations[{index}] path_constant drift: "
            f"declared={path_constant} derived={derived_path_constant}"
        )
    if path_constants.get(path_constant) != key[0]:
        raise PdsResourceFamilyError(
            f"{family_id}.operations[{index}] path_constant {path_constant} does not map to {key[0]}"
        )

    route_context = require_string(
        operation.get("route_context"),
        f"{family_id}.operations[{index}].route_context",
        PdsResourceFamilyError,
    )
    if route_context not in route_handlers:
        raise PdsResourceFamilyError(
            f"{family_id}.operations[{index}] has unknown route_context: {route_context}"
        )
    route_key = (path_constant, key[1])
    derived_handler = route_handlers[route_context].get(route_key)
    if derived_handler is None:
        raise PdsResourceFamilyError(
            f"{family_id}.operations[{index}] route is not in {route_context}: "
            f"{path_constant} {key[1]}"
        )
    handler = operation.get("handler", derived_handler)
    handler = require_string(
        handler,
        f"{family_id}.operations[{index}].handler",
        PdsResourceFamilyError,
    )
    if handler != derived_handler:
        raise PdsResourceFamilyError(
            f"{family_id}.operations[{index}] handler drift: "
            f"declared={handler} derived={derived_handler}"
        )
    required_scopes = require_string_list(
        openapi_operation.get("x-kiva-required-scopes", []),
        f"{family_id}.operations[{index}].x-kiva-required-scopes",
        PdsResourceFamilyError,
    )

    # Keep handler modules present as real implementation, not just imported names.
    handler_search_root = root / "crates/kiva-server/src/surfaces/pds/handlers"
    if not any(handler in path.read_text(encoding="utf-8") for path in handler_search_root.glob("*.rs")):
        raise PdsResourceFamilyError(
            f"{family_id}.operations[{index}] handler implementation is missing: {handler}"
        )
    return key, {
        "operation_id": openapi_operation.get("operationId"),
        "path_constant": path_constant,
        "handler": handler,
        "request_schemas": sorted(request_refs),
        "response_schemas": sorted(response_refs),
        "required_scopes": sorted(required_scopes),
        "route_context": route_context,
    }


def validate_application_operation_catalog_policy(
    root: Path,
    manifest: dict[str, Any],
    source_contract: Path,
    operation_reports: dict[tuple[str, str], dict[str, Any]],
) -> dict[str, int]:
    """Validate PDS operations against HTTP and application operation catalogs."""
    raw_catalog = require_string(
        manifest.get("operation_catalog"),
        "operation_catalog",
        PdsResourceFamilyError,
    )
    raw_http_catalog = require_string(
        manifest.get("http_operation_catalog"),
        "http_operation_catalog",
        PdsResourceFamilyError,
    )
    policy = require_mapping(
        manifest.get("application_operation_catalog_policy"),
        "application_operation_catalog_policy",
        PdsResourceFamilyError,
    )
    source_of_truth = require_string(
        policy.get("source_of_truth"),
        "application_operation_catalog_policy.source_of_truth",
        PdsResourceFamilyError,
    )
    if not source_of_truth:
        raise PdsResourceFamilyError("application_operation_catalog_policy.source_of_truth is empty")
    http_source_of_truth = require_string(
        policy.get("http_operation_catalog_source"),
        "application_operation_catalog_policy.http_operation_catalog_source",
        PdsResourceFamilyError,
    )
    if not http_source_of_truth:
        raise PdsResourceFamilyError("application_operation_catalog_policy.http_operation_catalog_source is empty")
    lifecycle_coverage = require_string(
        policy.get("pds_operation_lifecycle_coverage"),
        "application_operation_catalog_policy.pds_operation_lifecycle_coverage",
        PdsResourceFamilyError,
    )
    if not lifecycle_coverage:
        raise PdsResourceFamilyError("application_operation_catalog_policy.pds_operation_lifecycle_coverage is empty")
    required_contexts = set(
        require_string_list(
            policy.get("catalog_required_route_contexts"),
            "application_operation_catalog_policy.catalog_required_route_contexts",
            PdsResourceFamilyError,
        )
    )
    required_methods = set(
        require_string_list(
            policy.get("catalog_required_methods"),
            "application_operation_catalog_policy.catalog_required_methods",
            PdsResourceFamilyError,
        )
    )
    operation_catalog_owns = require_string_list(
        policy.get("operation_catalog_owns"),
        "application_operation_catalog_policy.operation_catalog_owns",
        PdsResourceFamilyError,
    )
    if not {"operation_id", "required_scopes"}.issubset(operation_catalog_owns):
        raise PdsResourceFamilyError(
            "application_operation_catalog_policy.operation_catalog_owns must include "
            "operation_id and required_scopes"
        )
    resource_family_owns = require_string_list(
        policy.get("resource_family_ledger_owns"),
        "application_operation_catalog_policy.resource_family_ledger_owns",
        PdsResourceFamilyError,
    )
    if not {"resource_family", "route_context", "implementation_files"}.issubset(resource_family_owns):
        raise PdsResourceFamilyError(
            "application_operation_catalog_policy.resource_family_ledger_owns must include "
            "resource_family, route_context, and implementation_files"
        )

    http_catalog_entries = collect_http_operation_catalog_entries(root, raw_http_catalog, source_contract)
    catalog_entries = collect_pds_operation_catalog_entries(root, raw_catalog, source_contract)
    checked = 0
    required_operation_ids: set[str] = set()
    http_catalog_checked = 0
    for (path, method), report in operation_reports.items():
        operation_id = require_string(
            report.get("operation_id"),
            f"{path} {method}.operation_id",
            PdsResourceFamilyError,
        )
        route_context = require_string(
            report.get("route_context"),
            f"{path} {method}.route_context",
            PdsResourceFamilyError,
        )
        http_catalog_entry = http_catalog_entries.get((path, method))
        if http_catalog_entry is None:
            raise PdsResourceFamilyError(
                f"{operation_id} must be present in {raw_http_catalog}: {path} {method}"
            )
        if http_catalog_entry["operation_id"] != operation_id:
            raise PdsResourceFamilyError(
                f"{path} {method} HTTP operation catalog drift: "
                f"catalog={http_catalog_entry['operation_id']} openapi={operation_id}"
            )
        http_catalog_scopes = sorted(
            require_string_list(
                http_catalog_entry.get("required_scopes", []),
                f"{operation_id}.http_required_scopes",
                PdsResourceFamilyError,
                allow_empty=True,
            )
        )
        if http_catalog_scopes != report["required_scopes"]:
            raise PdsResourceFamilyError(
                f"{operation_id} HTTP catalog scope drift: "
                f"catalog={http_catalog_scopes} openapi={report['required_scopes']}"
            )
        http_catalog_checked += 1
        if route_context not in required_contexts:
            continue
        if method not in required_methods:
            continue
        catalog_entry = catalog_entries.get(operation_id)
        if catalog_entry is None:
            raise PdsResourceFamilyError(
                f"{operation_id} must be present in {raw_catalog} for {route_context} {method}"
            )
        catalog_scopes = sorted(
            require_string_list(
                catalog_entry.get("required_scopes"),
                f"{operation_id}.required_scopes",
                PdsResourceFamilyError,
            )
        )
        if catalog_scopes != report["required_scopes"]:
            raise PdsResourceFamilyError(
                f"{operation_id} scope drift: catalog={catalog_scopes} "
                f"openapi={report['required_scopes']}"
            )
        required_operation_ids.add(operation_id)
        checked += 1

    catalog_only = sorted(set(catalog_entries).difference(required_operation_ids))
    if catalog_only:
        raise PdsResourceFamilyError(
            f"{raw_catalog} contains PDS application workflow entries not required by "
            "the resource-family ledger: " + ", ".join(catalog_only)
        )
    return {
        "application_workflow_operation_checks": checked,
        "http_operation_catalog_checks": http_catalog_checked,
    }


def validate_boilerplate_policy(
    manifest: dict[str, Any],
    family_id: str,
    implementation_files: list[str],
) -> int:
    """Validate that a PDS family declares its server implementation roles."""
    policy = require_mapping(
        manifest.get("server_resource_family_boilerplate_policy"),
        "server_resource_family_boilerplate_policy",
        PdsResourceFamilyError,
    )
    role_matrix = require_mapping(
        policy.get("family_role_matrix"),
        "server_resource_family_boilerplate_policy.family_role_matrix",
        PdsResourceFamilyError,
    )
    required_roles = require_string_list(
        role_matrix.get(family_id),
        f"server_resource_family_boilerplate_policy.family_role_matrix.{family_id}",
        PdsResourceFamilyError,
    )
    if len(required_roles) != len(set(required_roles)):
        raise PdsResourceFamilyError(
            f"server_resource_family_boilerplate_policy.family_role_matrix.{family_id} "
            "must not contain duplicate roles"
        )
    role_prefixes = require_mapping(
        policy.get("role_prefixes"),
        "server_resource_family_boilerplate_policy.role_prefixes",
        PdsResourceFamilyError,
    )
    checked = 0
    for role in required_roles:
        prefix = require_string(
            role_prefixes.get(role),
            f"server_resource_family_boilerplate_policy.role_prefixes.{role}",
            PdsResourceFamilyError,
        )
        if not any(path.startswith(prefix) for path in implementation_files):
            raise PdsResourceFamilyError(
                f"{family_id}.implementation_files missing server role {role}: {prefix}"
            )
        checked += 1
    return checked


def validate_resource_family_profile_model(
    root: Path,
    manifest: dict[str, Any],
    manifest_path: Path,
) -> None:
    """Validate the PDS resource-family profile model projected into the ledger."""
    model = require_mapping(
        manifest.get("resource_family_profile_model"),
        "resource_family_profile_model",
        PdsResourceFamilyError,
    )
    if model.get("root_type") != "PdsResourceFamilyProfile":
        raise PdsResourceFamilyError(
            "resource_family_profile_model.root_type must be PdsResourceFamilyProfile"
        )
    generated_ledger = require_string(
        model.get("generated_ledger"),
        "resource_family_profile_model.generated_ledger",
        PdsResourceFamilyError,
    )
    if generated_ledger != manifest_path.as_posix():
        raise PdsResourceFamilyError(
            "resource_family_profile_model.generated_ledger must match the manifest path"
        )
    source_family_ledger = require_string(
        model.get("source_family_ledger"),
        "resource_family_profile_model.source_family_ledger",
        PdsResourceFamilyError,
    )
    if source_family_ledger != "spec/architecture/pds-resource-families.source.yaml":
        raise PdsResourceFamilyError(
            "resource_family_profile_model.source_family_ledger must be the PDS family source"
        )
    require_existing_path(
        root,
        source_family_ledger,
        "resource_family_profile_model.source_family_ledger",
        PdsResourceFamilyError,
    )
    source_policy = require_string(
        model.get("source_policy"),
        "resource_family_profile_model.source_policy",
        PdsResourceFamilyError,
    )
    if source_policy != "spec/architecture/surface-implementation.policy.yaml#/pds_resource_family_projection":
        raise PdsResourceFamilyError(
            "resource_family_profile_model.source_policy must point to the PDS projection policy"
        )
    role_policy = require_mapping(
        manifest.get("server_resource_family_boilerplate_policy"),
        "server_resource_family_boilerplate_policy",
        PdsResourceFamilyError,
    )
    role_prefixes = require_mapping(
        role_policy.get("role_prefixes"),
        "server_resource_family_boilerplate_policy.role_prefixes",
        PdsResourceFamilyError,
    )
    required_roles = set(
        require_string_list(
            model.get("required_roles"),
            "resource_family_profile_model.required_roles",
            PdsResourceFamilyError,
        )
    )
    if required_roles != set(role_prefixes):
        raise PdsResourceFamilyError(
            "resource_family_profile_model.required_roles must match role_prefixes"
        )
    require_string(
        model.get("rule"),
        "resource_family_profile_model.rule",
        PdsResourceFamilyError,
    )


def validate_boilerplate_role_matrix(manifest: dict[str, Any], family_ids: set[str]) -> None:
    """Validate that the role matrix has one entry for each PDS family."""
    policy = require_mapping(
        manifest.get("server_resource_family_boilerplate_policy"),
        "server_resource_family_boilerplate_policy",
        PdsResourceFamilyError,
    )
    role_matrix = require_mapping(
        policy.get("family_role_matrix"),
        "server_resource_family_boilerplate_policy.family_role_matrix",
        PdsResourceFamilyError,
    )
    actual = {str(family_id) for family_id in role_matrix}
    missing = sorted(family_ids.difference(actual))
    extra = sorted(actual.difference(family_ids))
    if missing:
        raise PdsResourceFamilyError(
            "server_resource_family_boilerplate_policy.family_role_matrix is missing: "
            + ", ".join(missing)
        )
    if extra:
        raise PdsResourceFamilyError(
            "server_resource_family_boilerplate_policy.family_role_matrix has unused entries: "
            + ", ".join(extra)
        )


def expected_boilerplate_projection(
    manifest: dict[str, Any],
    family_implementation_files: dict[str, list[str]],
) -> dict[str, Any]:
    """Return the role projection implied by the boilerplate policy."""
    policy = require_mapping(
        manifest.get("server_resource_family_boilerplate_policy"),
        "server_resource_family_boilerplate_policy",
        PdsResourceFamilyError,
    )
    role_matrix = require_mapping(
        policy.get("family_role_matrix"),
        "server_resource_family_boilerplate_policy.family_role_matrix",
        PdsResourceFamilyError,
    )
    role_prefixes = require_mapping(
        policy.get("role_prefixes"),
        "server_resource_family_boilerplate_policy.role_prefixes",
        PdsResourceFamilyError,
    )
    families: list[dict[str, Any]] = []
    for family_id in family_implementation_files:
        implementation_files = family_implementation_files[family_id]
        roles: list[dict[str, Any]] = []
        for role in require_string_list(
            role_matrix.get(family_id),
            f"server_resource_family_boilerplate_policy.family_role_matrix.{family_id}",
            PdsResourceFamilyError,
        ):
            prefix = require_string(
                role_prefixes.get(role),
                f"server_resource_family_boilerplate_policy.role_prefixes.{role}",
                PdsResourceFamilyError,
            )
            matched_files = sorted(path for path in implementation_files if path.startswith(prefix))
            roles.append(
                {
                    "role": role,
                    "prefix": prefix,
                    "matched_files": matched_files,
                    "implemented": bool(matched_files),
                }
            )
        families.append({"family": family_id, "roles": roles})
    return {
        "source_policy": "server_resource_family_boilerplate_policy",
        "families": families,
    }


def validate_boilerplate_projection(
    manifest: dict[str, Any],
    family_implementation_files: dict[str, list[str]],
) -> None:
    """Validate the generated PDS family boilerplate projection."""
    projection = require_mapping(
        manifest.get("server_resource_family_boilerplate_projection"),
        "server_resource_family_boilerplate_projection",
        PdsResourceFamilyError,
    )
    expected = expected_boilerplate_projection(manifest, family_implementation_files)
    if projection != expected:
        raise PdsResourceFamilyError(
            "server_resource_family_boilerplate_projection drifts from policy and resource_families"
        )


def validate_surface_operation_projection_basis(
    manifest: dict[str, Any],
    expected_role: str,
) -> int:
    """Validate common SurfaceOperation projection provenance."""
    basis = require_mapping(
        manifest.get("projection_basis"),
        "projection_basis",
        PdsResourceFamilyError,
    )
    if basis.get("root_type") != "SurfaceOperation":
        raise PdsResourceFamilyError("projection_basis.root_type must be SurfaceOperation")
    if basis.get("projection_role") != expected_role:
        raise PdsResourceFamilyError(
            f"projection_basis.projection_role must be {expected_role}"
        )
    if require_string_list(
        basis.get("operation_key_fields"),
        "projection_basis.operation_key_fields",
        PdsResourceFamilyError,
    ) != ["openapi_contract", "operation_id"]:
        raise PdsResourceFamilyError(
            "projection_basis.operation_key_fields must be openapi_contract, operation_id"
        )
    if require_string_list(
        basis.get("route_key_fields"),
        "projection_basis.route_key_fields",
        PdsResourceFamilyError,
    ) != ["openapi_contract", "method", "path"]:
        raise PdsResourceFamilyError(
            "projection_basis.route_key_fields must be openapi_contract, method, path"
        )
    joined_facets = set(
        require_string_list(
            basis.get("joined_facets"),
            "projection_basis.joined_facets",
            PdsResourceFamilyError,
        )
    )
    required_facets = {"openapi_contract", "method", "path", "operation_id", "pds_resource_family", "pds_route_context"}
    if not required_facets.issubset(joined_facets):
        raise PdsResourceFamilyError(
            "projection_basis.joined_facets must include PDS route ownership facets"
        )
    operation_count = basis.get("operation_count")
    if not isinstance(operation_count, int) or operation_count <= 0:
        raise PdsResourceFamilyError("projection_basis.operation_count must be a positive integer")
    return operation_count


def validate_resource_families(root: Path, manifest_path: Path) -> dict[str, Any]:
    """Validate the PDS resource-family ledger."""
    manifest = load_yaml(root / manifest_path, PdsResourceFamilyError)
    if manifest.get("schema_version") != 1:
        raise PdsResourceFamilyError("schema_version must be 1")
    if manifest.get("kind") != "kiva-pds-resource-family-ledger":
        raise PdsResourceFamilyError("kind must be kiva-pds-resource-family-ledger")
    if manifest.get("status") != "active":
        raise PdsResourceFamilyError("status must be active")
    surface_operation_count = validate_surface_operation_projection_basis(
        manifest,
        "pds-route-context-and-handler-ownership-projection",
    )
    validate_resource_family_profile_model(root, manifest, manifest_path)

    source_contract = require_existing_path(
        root,
        require_string(manifest.get("source_contract"), "source_contract", PdsResourceFamilyError),
        "source_contract",
        PdsResourceFamilyError,
    )
    route_file = require_existing_path(
        root,
        require_string(manifest.get("route_file"), "route_file", PdsResourceFamilyError),
        "route_file",
        PdsResourceFamilyError,
    )
    path_constants_file = require_existing_path(
        root,
        require_string(
            manifest.get("path_constants_file"),
            "path_constants_file",
            PdsResourceFamilyError,
        ),
        "path_constants_file",
        PdsResourceFamilyError,
    )

    openapi = load_yaml(root / source_contract, PdsResourceFamilyError)
    openapi_operations = collect_pds_openapi_operations(openapi)
    route_contexts = extract_route_contexts((root / route_file).read_text(encoding="utf-8"))
    route_handlers = {
        context_id: extract_route_handlers(context_text)
        for context_id, context_text in route_contexts.items()
    }
    path_constants = parse_path_constants((root / path_constants_file).read_text(encoding="utf-8"))
    path_constants_by_path = invert_path_constants(path_constants)

    families = rows_by_unique_string_field(
        manifest.get("resource_families"),
        label="resource_families",
        error_cls=PdsResourceFamilyError,
        duplicate_label="resource family id",
    )
    if not families:
        raise PdsResourceFamilyError("resource_families must not be empty")
    seen_operations: dict[tuple[str, str], str] = {}
    operation_reports: dict[tuple[str, str], dict[str, Any]] = {}
    checked_files: set[str] = set()
    boilerplate_checks = 0
    family_implementation_files: dict[str, list[str]] = {}

    for family_id, family in families.items():
        require_string(family.get("description"), f"{family_id}.description", PdsResourceFamilyError)
        implementation_files = require_string_list(
            family.get("implementation_files"),
            f"{family_id}.implementation_files",
            PdsResourceFamilyError,
        )
        family_implementation_files[family_id] = implementation_files
        boilerplate_checks += validate_boilerplate_policy(manifest, family_id, implementation_files)
        for raw_file in implementation_files:
            require_existing_path(
                root,
                raw_file,
                f"{family_id}.implementation_files",
                PdsResourceFamilyError,
            )
            checked_files.add(raw_file)
        operations = require_list(
            family.get("operations"),
            f"{family_id}.operations",
            PdsResourceFamilyError,
        )
        if not operations:
            raise PdsResourceFamilyError(f"{family_id}.operations must not be empty")
        for operation_index, operation_value in enumerate(operations):
            operation = require_mapping(
                operation_value,
                f"{family_id}.operations[{operation_index}]",
                PdsResourceFamilyError,
            )
            key, derived = validate_operation(
                root,
                openapi,
                openapi_operations,
                route_handlers,
                path_constants,
                path_constants_by_path,
                operation,
                family_id,
                operation_index,
            )
            if key in seen_operations:
                raise PdsResourceFamilyError(
                    f"PDS operation is owned more than once: {key[0]} {key[1]}"
                )
            seen_operations[key] = family_id
            operation_reports[key] = derived

    validate_boilerplate_role_matrix(manifest, set(families))
    validate_boilerplate_projection(manifest, family_implementation_files)

    missing = sorted(set(openapi_operations).difference(seen_operations))
    extra = sorted(set(seen_operations).difference(openapi_operations))
    if missing:
        raise PdsResourceFamilyError(
            "OpenAPI PDS operations missing from resource-family ledger: "
            + ", ".join(f"{path} {method}" for path, method in missing)
        )
    if extra:
        raise PdsResourceFamilyError(
            "Resource-family operations not present in OpenAPI: "
            + ", ".join(f"{path} {method}" for path, method in extra)
        )
    catalog_checks = validate_application_operation_catalog_policy(
        root,
        manifest,
        source_contract,
        operation_reports,
    )

    return {
        "manifest": str(manifest_path),
        "family_count": len(families),
        "operation_count": len(seen_operations),
        "surface_operation_count": surface_operation_count,
        "implementation_file_count": len(checked_files),
        **catalog_checks,
        "server_boilerplate_checks": boilerplate_checks,
        "derived_fields": [
            "operation_id",
            "path_constant",
            "handler",
            "request_schemas",
            "response_schemas",
        ],
    }


def parse_args() -> argparse.Namespace:
    """Parse CLI arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path.cwd(),
        help="Repository root. Defaults to the current working directory.",
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        default=DEFAULT_MANIFEST,
        help="PDS resource-family manifest path relative to --root.",
    )
    parser.add_argument(
        "--json-report",
        type=Path,
        help="Optional path for a JSON validation summary.",
    )
    return parser.parse_args()


def main() -> int:
    """CLI entrypoint."""
    args = parse_args()
    root = args.root.resolve()
    manifest_path = relative_path(str(args.manifest), "--manifest", PdsResourceFamilyError)
    try:
        report = validate_resource_families(root, manifest_path)
    except PdsResourceFamilyError as error:
        print(f"PDS resource-family validation failed: {error}", file=sys.stderr)
        return 1

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
