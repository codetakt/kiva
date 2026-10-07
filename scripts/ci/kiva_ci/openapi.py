"""OpenAPI projection helpers shared by Kiva CI validators."""

from __future__ import annotations

from pathlib import Path
from typing import Any, TypeVar

from .manifest import (
    ManifestError,
    load_yaml,
    require_mapping,
    require_string,
    require_string_list,
    validation_error,
)


ErrorT = TypeVar("ErrorT", bound=RuntimeError)

HTTP_METHOD_ORDER = ["get", "put", "post", "delete", "patch", "head", "options", "trace"]
HTTP_METHODS = set(HTTP_METHOD_ORDER)


def optional_string(value: Any) -> str:
    """Return a string value or an empty string for absent OpenAPI metadata."""
    return value if isinstance(value, str) else ""


def collect_http_operations(
    contract_path: Path,
    error_cls: type[ErrorT] = ManifestError,
) -> list[dict[str, Any]]:
    """Collect deterministic HTTP operation rows from one OpenAPI contract."""
    document = load_yaml(contract_path, error_cls)
    paths = require_mapping(
        document.get("paths"),
        f"{contract_path}.paths",
        error_cls,
    )
    operations: list[dict[str, Any]] = []
    for raw_path in sorted(paths):
        path_item = require_mapping(
            paths[raw_path],
            f"{contract_path}.paths.{raw_path}",
            error_cls,
        )
        for method in HTTP_METHOD_ORDER:
            operation_value = path_item.get(method)
            if operation_value is None:
                continue
            operation = require_mapping(
                operation_value,
                f"{contract_path}.paths.{raw_path}.{method}",
                error_cls,
            )
            operation_id = require_string(
                operation.get("operationId"),
                f"{contract_path}.paths.{raw_path}.{method}.operationId",
                error_cls,
            )
            scopes = require_string_list(
                operation.get("x-kiva-required-scopes", []),
                f"{contract_path}.paths.{raw_path}.{method}.x-kiva-required-scopes",
                error_cls,
                allow_empty=True,
            )
            requirements = require_string_list(
                operation.get("x-kiva-requirements", []),
                f"{contract_path}.paths.{raw_path}.{method}.x-kiva-requirements",
                error_cls,
                allow_empty=True,
            )
            tags = require_string_list(
                operation.get("tags", []),
                f"{contract_path}.paths.{raw_path}.{method}.tags",
                error_cls,
                allow_empty=True,
            )
            operations.append(
                {
                    "openapi_contract": contract_path.as_posix(),
                    "path": raw_path,
                    "method": method,
                    "operation_id": operation_id,
                    "summary": optional_string(operation.get("summary")),
                    "required_scopes": scopes,
                    "requirements": requirements,
                    "tags": tags,
                }
            )

        unknown_methods = sorted(
            str(method).lower()
            for method in path_item
            if str(method).lower() in HTTP_METHODS
            and str(method).lower() not in HTTP_METHOD_ORDER
        )
        if unknown_methods:
            raise validation_error(
                error_cls,
                f"{contract_path}.paths.{raw_path} contains unsupported method order entries: "
                + ", ".join(unknown_methods),
            )
    return operations


def collect_operations_by_id(
    document: dict[str, Any],
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, dict[str, Any]]:
    """Return OpenAPI operations keyed by operationId."""
    operations: dict[str, dict[str, Any]] = {}
    paths = require_mapping(document.get("paths"), f"{label}.paths", error_cls)
    for raw_path, path_item in paths.items():
        path_mapping = require_mapping(path_item, f"{label}.paths.{raw_path}", error_cls)
        for method, operation_value in path_mapping.items():
            if str(method).lower() not in HTTP_METHODS:
                continue
            operation = require_mapping(
                operation_value,
                f"{label}.paths.{raw_path}.{method}",
                error_cls,
            )
            operation_id = require_string(
                operation.get("operationId"),
                f"{label}.paths.{raw_path}.{method}.operationId",
                error_cls,
            )
            previous = operations.setdefault(operation_id, operation)
            if previous is not operation:
                raise validation_error(error_cls, f"{label} operationId is duplicated: {operation_id}")
    return operations


def component_ref_name(
    value: Any,
    prefix: str,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> str | None:
    """Return a component reference name for local OpenAPI refs."""
    if not isinstance(value, str):
        return None
    if not value.startswith(prefix):
        raise validation_error(error_cls, f"{label} must use local component refs, got {value}")
    return value.removeprefix(prefix)


def collect_request_body_schema_names(
    openapi: dict[str, Any],
    openapi_path: str,
    error_cls: type[ErrorT] = ManifestError,
) -> set[str]:
    """Collect component schema names used by OpenAPI operation request bodies."""
    components = require_mapping(
        openapi.get("components"),
        f"{openapi_path}.components",
        error_cls,
    )
    request_bodies = require_mapping(
        components.get("requestBodies", {}),
        f"{openapi_path}.components.requestBodies",
        error_cls,
    )
    paths = require_mapping(openapi.get("paths"), f"{openapi_path}.paths", error_cls)
    schemas: set[str] = set()
    for path_name, path_item_value in paths.items():
        path_item = require_mapping(
            path_item_value,
            f"{openapi_path}.paths.{path_name}",
            error_cls,
        )
        for method, operation_value in path_item.items():
            if str(method).lower() not in HTTP_METHODS:
                continue
            operation = require_mapping(
                operation_value,
                f"{openapi_path}.paths.{path_name}.{method}",
                error_cls,
            )
            request_body_value = operation.get("requestBody")
            if request_body_value is None:
                continue
            request_body = require_mapping(
                request_body_value,
                f"{openapi_path}.paths.{path_name}.{method}.requestBody",
                error_cls,
            )
            body_ref = component_ref_name(
                request_body.get("$ref"),
                "#/components/requestBodies/",
                f"{openapi_path}.paths.{path_name}.{method}.requestBody.$ref",
                error_cls,
            )
            if body_ref is not None:
                request_body = require_mapping(
                    request_bodies.get(body_ref),
                    f"{openapi_path}.components.requestBodies.{body_ref}",
                    error_cls,
                )
            content = require_mapping(
                request_body.get("content"),
                f"{openapi_path}.paths.{path_name}.{method}.requestBody.content",
                error_cls,
            )
            for media_type, media in content.items():
                media_document = require_mapping(
                    media,
                    f"{openapi_path}.paths.{path_name}.{method}.requestBody.content.{media_type}",
                    error_cls,
                )
                schema = require_mapping(
                    media_document.get("schema"),
                    f"{openapi_path}.paths.{path_name}.{method}.requestBody.content.{media_type}.schema",
                    error_cls,
                )
                schema_ref = component_ref_name(
                    schema.get("$ref"),
                    "#/components/schemas/",
                    f"{openapi_path}.paths.{path_name}.{method}.requestBody.content.{media_type}.schema.$ref",
                    error_cls,
                )
                if schema_ref is not None:
                    schemas.add(schema_ref)
    return schemas


def collect_schema_ref_names(
    value: Any,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> set[str]:
    """Collect local component schema references recursively from an OpenAPI value."""
    refs: set[str] = set()
    if isinstance(value, dict):
        raw_ref = value.get("$ref")
        if raw_ref is not None:
            ref = component_ref_name(
                raw_ref,
                "#/components/schemas/",
                f"{label}.$ref",
                error_cls,
            )
            if ref is not None:
                refs.add(ref)
        for key, nested in value.items():
            if key != "$ref":
                refs.update(collect_schema_ref_names(nested, f"{label}.{key}", error_cls))
    elif isinstance(value, list):
        for index, nested in enumerate(value):
            refs.update(collect_schema_ref_names(nested, f"{label}[{index}]", error_cls))
    return refs


def schema_requires_dto_boundary(schema: dict[str, Any]) -> bool:
    """Return whether a component schema should have a Rust DTO boundary type."""
    if isinstance(schema.get("enum"), list):
        return True
    if schema.get("type") == "object":
        return True
    return any(key in schema for key in ("allOf", "anyOf", "oneOf"))


def collect_dto_boundary_schema_closure(
    openapi: dict[str, Any],
    openapi_path: str,
    seed_schemas: set[str],
    error_cls: type[ErrorT] = ManifestError,
) -> set[str]:
    """Collect DTO-bearing component schemas reachable from seed schemas."""
    components = require_mapping(
        openapi.get("components"),
        f"{openapi_path}.components",
        error_cls,
    )
    schemas = require_mapping(
        components.get("schemas"),
        f"{openapi_path}.components.schemas",
        error_cls,
    )
    pending = list(seed_schemas)
    seen: set[str] = set()
    dto_schemas: set[str] = set()
    while pending:
        schema_name = pending.pop()
        if schema_name in seen:
            continue
        seen.add(schema_name)
        schema = require_mapping(
            schemas.get(schema_name),
            f"{openapi_path}.components.schemas.{schema_name}",
            error_cls,
        )
        if schema_requires_dto_boundary(schema):
            dto_schemas.add(schema_name)
        pending.extend(
            sorted(
                collect_schema_ref_names(
                    schema,
                    f"{openapi_path}.components.schemas.{schema_name}",
                    error_cls,
                ).difference(seen)
            )
        )
    return dto_schemas


def collect_request_body_dto_schema_closure_names(
    openapi: dict[str, Any],
    openapi_path: str,
    error_cls: type[ErrorT] = ManifestError,
) -> set[str]:
    """Collect DTO-bearing request-body schema closure."""
    return collect_dto_boundary_schema_closure(
        openapi,
        openapi_path,
        collect_request_body_schema_names(openapi, openapi_path, error_cls),
        error_cls,
    )


def _response_schema_names_from_response(
    response: dict[str, Any],
    label: str,
    error_cls: type[ErrorT],
) -> set[str]:
    """Collect component schema names from one OpenAPI response object."""
    schemas: set[str] = set()
    content = response.get("content")
    if content is None:
        return schemas
    content_mapping = require_mapping(content, f"{label}.content", error_cls)
    for media_type, media in content_mapping.items():
        media_document = require_mapping(
            media,
            f"{label}.content.{media_type}",
            error_cls,
        )
        schema = require_mapping(
            media_document.get("schema"),
            f"{label}.content.{media_type}.schema",
            error_cls,
        )
        schema_ref = component_ref_name(
            schema.get("$ref"),
            "#/components/schemas/",
            f"{label}.content.{media_type}.schema.$ref",
            error_cls,
        )
        if schema_ref is not None:
            schemas.add(schema_ref)
    return schemas


def collect_success_response_schema_names(
    openapi: dict[str, Any],
    openapi_path: str,
    error_cls: type[ErrorT] = ManifestError,
) -> set[str]:
    """Collect component schema names used by 2xx OpenAPI operation responses."""
    components = require_mapping(
        openapi.get("components"),
        f"{openapi_path}.components",
        error_cls,
    )
    response_components = require_mapping(
        components.get("responses", {}),
        f"{openapi_path}.components.responses",
        error_cls,
    )
    paths = require_mapping(openapi.get("paths"), f"{openapi_path}.paths", error_cls)
    schemas: set[str] = set()
    for path_name, path_item_value in paths.items():
        path_item = require_mapping(
            path_item_value,
            f"{openapi_path}.paths.{path_name}",
            error_cls,
        )
        for method, operation_value in path_item.items():
            if str(method).lower() not in HTTP_METHODS:
                continue
            operation = require_mapping(
                operation_value,
                f"{openapi_path}.paths.{path_name}.{method}",
                error_cls,
            )
            responses = require_mapping(
                operation.get("responses"),
                f"{openapi_path}.paths.{path_name}.{method}.responses",
                error_cls,
            )
            for status_code, response_value in responses.items():
                if not str(status_code).startswith("2"):
                    continue
                response = require_mapping(
                    response_value,
                    f"{openapi_path}.paths.{path_name}.{method}.responses.{status_code}",
                    error_cls,
                )
                response_ref = component_ref_name(
                    response.get("$ref"),
                    "#/components/responses/",
                    f"{openapi_path}.paths.{path_name}.{method}.responses.{status_code}.$ref",
                    error_cls,
                )
                if response_ref is not None:
                    response = require_mapping(
                        response_components.get(response_ref),
                        f"{openapi_path}.components.responses.{response_ref}",
                        error_cls,
                    )
                schemas.update(
                    _response_schema_names_from_response(
                        response,
                        f"{openapi_path}.paths.{path_name}.{method}.responses.{status_code}",
                        error_cls,
                    )
                )
    return schemas


def collect_success_response_dto_schema_closure_names(
    openapi: dict[str, Any],
    openapi_path: str,
    error_cls: type[ErrorT] = ManifestError,
) -> set[str]:
    """Collect DTO-bearing 2xx response schema closure."""
    return collect_dto_boundary_schema_closure(
        openapi,
        openapi_path,
        collect_success_response_schema_names(openapi, openapi_path, error_cls),
        error_cls,
    )
