"""Shared helpers for the canonical Kiva surface operation catalog."""

from __future__ import annotations

from pathlib import Path
from typing import Any, TypeVar

import yaml

from kiva_ci.manifest import (
    ManifestError,
    load_json,
    load_yaml,
    relative_path,
    require_existing_path,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
    validation_error,
)
from surface_catalog_model import (
    SurfaceEntry,
    require_surface_entries,
    surface_entries,
    surface_index_rows,
    surface_operation_normal_form,
)
from surface_operation_ownership import (
    OWNERSHIP_SOURCE_KIND,
    project_surface_entries_from_ownership,
)
from surface_catalog_projections import (
    project_application_operation_catalog,
    project_pds_resource_family_ledger,
    project_surface_family_ledger,
    render_surface_profile_registry,
)


SURFACE_CATALOG_KIND = "kiva-surface-operation-catalog"
SURFACE_PROJECTION_POLICY_KIND = "kiva-surface-projection-policy"
SURFACE_OPERATION_MODEL_SOURCE_KIND = "kiva-surface-operation-model-source"
SURFACE_GENERATION_POLICY_KIND = "kiva-surface-generation-policy"
SURFACE_IMPLEMENTATION_POLICY_KIND = "kiva-surface-implementation-policy"
PDS_RESOURCE_FAMILY_SOURCE_KIND = "kiva-pds-resource-family-source"
SURFACE_FAMILY_SOURCE_KIND = "kiva-surface-family-source"

ErrorT = TypeVar("ErrorT", bound=RuntimeError)

SURFACE_PROJECTION_PATH_IDS: dict[str, str] = {
    "operation_kernel": "operation_kernel",
    "operation_kernel_markdown": "operation_kernel_markdown",
    "application_catalog": "application_operation_catalog",
    "pds_families": "pds_resource_family_ledger",
    "surface_families": "surface_family_ledger",
    "markdown": "surface_operation_index",
    "evidence": "surface_operation_index_evidence",
    "route_coverage_markdown": "route_coverage",
    "route_coverage_evidence": "route_coverage_evidence",
    "rust_surface_profile_registry": "server_surface_profile_registry",
}


def render_yaml(document: dict[str, Any]) -> str:
    """Render a deterministic YAML document."""
    return yaml.safe_dump(
        document,
        sort_keys=False,
        allow_unicode=False,
        width=1000,
    )


def load_surface_catalog(
    path: Path,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Load and validate the surface catalog envelope."""
    document = load_yaml(path, error_cls)
    if document.get("schema_version") != 1:
        raise validation_error(error_cls, f"{path}.schema_version must be 1")
    if document.get("kind") != SURFACE_CATALOG_KIND:
        raise validation_error(error_cls, f"{path}.kind must be {SURFACE_CATALOG_KIND}")
    if document.get("status") != "active":
        raise validation_error(error_cls, f"{path}.status must be active")
    return document


def _load_component(
    path: Path,
    kind: str,
    error_cls: type[ErrorT],
) -> dict[str, Any]:
    """Load one surface-catalog component source."""
    document = load_yaml(path, error_cls)
    if document.get("schema_version") != 1:
        raise validation_error(error_cls, f"{path}.schema_version must be 1")
    if document.get("kind") != kind:
        raise validation_error(error_cls, f"{path}.kind must be {kind}")
    if document.get("status") != "active":
        raise validation_error(error_cls, f"{path}.status must be active")
    return document


def load_operation_ownership_source(
    path: Path,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Load the canonical operation ownership overlay source ledger."""
    return _load_component(path, OWNERSHIP_SOURCE_KIND, error_cls)


def load_surface_operation_entries(
    path: Path,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Compatibility wrapper for the operation ownership overlay source."""
    return load_operation_ownership_source(path, error_cls)


def load_surface_projection_policy(
    path: Path,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Load the surface projection-policy source ledger."""
    document = _load_component(path, SURFACE_PROJECTION_POLICY_KIND, error_cls)
    if "split_policy_sources" not in document:
        return document
    split_sources = require_mapping(
        document.get("split_policy_sources"),
        f"{path}.split_policy_sources",
        error_cls,
    )
    source_specs = {
        "model": (
            SURFACE_OPERATION_MODEL_SOURCE_KIND,
            ("surface_operation_algebra", "join_policy"),
        ),
        "generation": (
            SURFACE_GENERATION_POLICY_KIND,
            (
                "component_sources",
                "generated_artifact_policy",
                "generated_bundle",
                "source_ledgers",
                "projection_policy",
                "validator",
            ),
        ),
        "implementation": (
            SURFACE_IMPLEMENTATION_POLICY_KIND,
            (
                "route_coverage_projection",
                "application_operation_projection",
                "pds_resource_family_projection",
                "surface_family_projection",
            ),
        ),
    }
    merged = dict(document)
    for source_id, (expected_kind, copied_sections) in source_specs.items():
        raw_path = require_string(
            split_sources.get(source_id),
            f"{path}.split_policy_sources.{source_id}",
            error_cls,
        )
        source_path = relative_path(
            raw_path,
            f"{path}.split_policy_sources.{source_id}",
            error_cls,
        )
        split_document = _load_component(source_path, expected_kind, error_cls)
        for section in copied_sections:
            if section in {"generated_bundle", "validator"}:
                value = require_string(
                    split_document.get(section),
                    f"{source_path}.{section}",
                    error_cls,
                )
            else:
                value = require_mapping(
                    split_document.get(section),
                    f"{source_path}.{section}",
                    error_cls,
                )
            if section in merged and merged[section] != value:
                raise validation_error(
                    error_cls,
                    f"{path}.{section} drifts from split source {source_path}",
                )
            merged[section] = value
    return merged


def load_pds_resource_family_source(
    path: Path,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Load the PDS resource-family source ledger."""
    return _load_component(path, PDS_RESOURCE_FAMILY_SOURCE_KIND, error_cls)


def load_surface_family_source(
    path: Path,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Load the cross-surface family source ledger."""
    return _load_component(path, SURFACE_FAMILY_SOURCE_KIND, error_cls)


def surface_component_paths(
    policy: dict[str, Any],
    policy_path: Path,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Path]:
    """Return component source paths declared by the projection policy."""
    components = require_mapping(
        policy.get("component_sources"),
        f"{policy_path}.component_sources",
        error_cls,
    )
    result: dict[str, Path] = {}
    for key in ("operation_ownership", "pds_resource_families", "surface_families"):
        raw_path = require_string(
            components.get(key),
            f"{policy_path}.component_sources.{key}",
            error_cls,
        )
        result[key] = relative_path(raw_path, f"{policy_path}.component_sources.{key}", error_cls)
    raw_bundle = require_string(
        policy.get("generated_bundle"),
        f"{policy_path}.generated_bundle",
        error_cls,
    )
    result["generated_bundle"] = relative_path(raw_bundle, f"{policy_path}.generated_bundle", error_cls)
    return result


def project_surface_catalog_bundle(
    operation_ownership_source: dict[str, Any],
    projection_policy: dict[str, Any],
    pds_family_source: dict[str, Any],
    surface_family_source: dict[str, Any],
    *,
    operation_ownership_path: Path,
    policy_path: Path,
    pds_family_path: Path,
    surface_family_path: Path,
    root: Path = Path("."),
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Project the generated surface catalog bundle."""
    source_ledgers = require_mapping(
        projection_policy.get("source_ledgers"),
        "surface_projection_policy.source_ledgers",
        error_cls,
    )
    http_operation_ledger = require_mapping(
        source_ledgers.get("http_operation_catalog"),
        "surface_projection_policy.source_ledgers.http_operation_catalog",
        error_cls,
    )
    http_catalog_path = relative_path(
        require_string(
            http_operation_ledger.get("path"),
            "surface_projection_policy.source_ledgers.http_operation_catalog.path",
            error_cls,
        ),
        "surface_projection_policy.source_ledgers.http_operation_catalog.path",
        error_cls,
    )
    http_operations = load_http_operation_catalog(root / http_catalog_path, error_cls)
    entries = project_surface_entries_from_ownership(
        operation_ownership_source,
        http_operations,
        error_cls,
    )
    families = require_list(
        pds_family_source.get("pds_resource_families"),
        "pds_resource_family_source.pds_resource_families",
        error_cls,
    )
    surface_families = require_list(
        surface_family_source.get("surface_families"),
        "surface_family_source.surface_families",
        error_cls,
    )
    return {
        "schema_version": 1,
        "kind": SURFACE_CATALOG_KIND,
        "status": "active",
        "description": require_string(
            projection_policy.get("description"),
            "surface_projection_policy.description",
            error_cls,
        ),
        "component_sources": {
            "operation_ownership": operation_ownership_path.as_posix(),
            "surface_projections": policy_path.as_posix(),
            "pds_resource_families": pds_family_path.as_posix(),
            "surface_families": surface_family_path.as_posix(),
        },
        "split_policy_sources": require_mapping(
            projection_policy.get("split_policy_sources"),
            "surface_projection_policy.split_policy_sources",
            error_cls,
        ),
        "source_ledgers": source_ledgers,
        "generated_artifact_policy": require_mapping(
            projection_policy.get("generated_artifact_policy"),
            "surface_projection_policy.generated_artifact_policy",
            error_cls,
        ),
        "surface_operation_algebra": require_mapping(
            projection_policy.get("surface_operation_algebra"),
            "surface_projection_policy.surface_operation_algebra",
            error_cls,
        ),
        "surface_operation_normal_form": surface_operation_normal_form(
            {"entries": entries},
            error_cls,
        ),
        "route_coverage_projection": require_mapping(
            projection_policy.get("route_coverage_projection"),
            "surface_projection_policy.route_coverage_projection",
            error_cls,
        ),
        "application_operation_projection": require_mapping(
            projection_policy.get("application_operation_projection"),
            "surface_projection_policy.application_operation_projection",
            error_cls,
        ),
        "pds_resource_family_projection": require_mapping(
            projection_policy.get("pds_resource_family_projection"),
            "surface_projection_policy.pds_resource_family_projection",
            error_cls,
        ),
        "surface_family_projection": require_mapping(
            projection_policy.get("surface_family_projection"),
            "surface_projection_policy.surface_family_projection",
            error_cls,
        ),
        "surface_family_implementation_anchor_policy": require_mapping(
            surface_family_source.get("implementation_anchor_policy"),
            "surface_family_source.implementation_anchor_policy",
            error_cls,
        ),
        "pds_resource_families": families,
        "surface_families": surface_families,
        "join_policy": require_mapping(
            projection_policy.get("join_policy"),
            "surface_projection_policy.join_policy",
            error_cls,
        ),
        "projection_policy": require_mapping(
            projection_policy.get("projection_policy"),
            "surface_projection_policy.projection_policy",
            error_cls,
        ),
        "entries": entries,
        "validator": require_string(
            projection_policy.get("validator"),
            "surface_projection_policy.validator",
            error_cls,
        ),
    }


def surface_operation_algebra(
    document: dict[str, Any],
    manifest_path: Path,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Return the sole public SurfaceOperation algebra declaration."""
    return require_mapping(
        document.get("surface_operation_algebra"),
        f"{manifest_path}.surface_operation_algebra",
        error_cls,
    )


def surface_projection_targets(
    document: dict[str, Any],
    manifest_path: Path,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, dict[str, str]]:
    """Return projection targets declared by the SurfaceOperation algebra."""
    algebra = surface_operation_algebra(document, manifest_path, error_cls)
    targets: dict[str, dict[str, str]] = {}
    for index, value in enumerate(
        require_list(
            algebra.get("projections"),
            f"{manifest_path}.surface_operation_algebra.projections",
            error_cls,
        )
    ):
        projection = require_mapping(
            value,
            f"{manifest_path}.surface_operation_algebra.projections[{index}]",
            error_cls,
        )
        projection_id = require_string(
            projection.get("id"),
            f"{manifest_path}.surface_operation_algebra.projections[{index}].id",
            error_cls,
        )
        if projection_id in targets:
            raise validation_error(error_cls, f"duplicate surface projection id: {projection_id}")
        targets[projection_id] = {
            "target": require_string(
                projection.get("target"),
                f"{manifest_path}.surface_operation_algebra.projections[{index}].target",
                error_cls,
            ),
            "role": require_string(
                projection.get("role"),
                f"{manifest_path}.surface_operation_algebra.projections[{index}].role",
                error_cls,
            ),
        }
    return targets


def surface_catalog_paths(
    document: dict[str, Any],
    manifest_path: Path,
    error_cls: type[ErrorT] = ManifestError,
    *,
    root: Path = Path("."),
    require_existing_inputs: bool = True,
) -> dict[str, Path]:
    """Return repository-relative paths declared by the surface catalog."""
    source_ledgers = require_mapping(
        document.get("source_ledgers"),
        f"{manifest_path}.source_ledgers",
        error_cls,
    )
    projections = surface_projection_targets(document, manifest_path, error_cls)

    def source_path(key: str) -> Path:
        ledger = require_mapping(
            source_ledgers.get(key),
            f"{manifest_path}.source_ledgers.{key}",
            error_cls,
        )
        raw_path = require_string(
            ledger.get("path"),
            f"{manifest_path}.source_ledgers.{key}.path",
            error_cls,
        )
        if require_existing_inputs:
            return require_existing_path(
                root,
                raw_path,
                f"{manifest_path}.source_ledgers.{key}.path",
                error_cls,
            )
        return relative_path(raw_path, f"{manifest_path}.source_ledgers.{key}.path", error_cls)

    def projection_path(key: str) -> Path:
        target = projections.get(key)
        if target is None:
            raise validation_error(
                error_cls,
                f"{manifest_path}.surface_operation_algebra.projections missing {key}",
            )
        raw_path = require_string(
            target.get("target"),
            f"{manifest_path}.surface_operation_algebra.projections.{key}.target",
            error_cls,
        )
        return relative_path(
            raw_path,
            f"{manifest_path}.surface_operation_algebra.projections.{key}.target",
            error_cls,
        )

    return {
        "http_catalog": source_path("http_operation_catalog"),
        **{
            path_key: projection_path(projection_id)
            for path_key, projection_id in SURFACE_PROJECTION_PATH_IDS.items()
        },
    }


def validate_entries_against_http_catalog(
    entries: list[dict[str, Any]],
    http_operations: list[dict[str, Any]],
    error_cls: type[ErrorT] = ManifestError,
) -> None:
    """Validate that surface entries exactly cover the generated HTTP catalog."""
    entry_by_route = {
        (entry["openapi_contract"], entry["method"], entry["path"]): entry
        for entry in entries
    }
    http_by_route = {
        (operation["openapi_contract"], operation["method"], operation["path"]): operation
        for operation in http_operations
    }
    entry_only = sorted(set(entry_by_route).difference(http_by_route))
    http_only = sorted(set(http_by_route).difference(entry_by_route))
    if entry_only:
        formatted = ", ".join(f"{contract} {method} {path}" for contract, method, path in entry_only)
        raise validation_error(error_cls, "surface entries missing from HTTP catalog: " + formatted)
    if http_only:
        formatted = ", ".join(f"{contract} {method} {path}" for contract, method, path in http_only)
        raise validation_error(error_cls, "HTTP catalog entries missing from surface catalog: " + formatted)
    for key, entry in entry_by_route.items():
        operation = http_by_route[key]
        if entry["operation_id"] != operation["operation_id"]:
            raise validation_error(
                error_cls,
                f"{key} operationId drift: surface={entry['operation_id']} http={operation['operation_id']}",
            )
        surface_scopes = sorted(entry["required_scopes"])
        http_scopes = sorted(operation["required_scopes"])
        if surface_scopes != http_scopes:
            raise validation_error(
                error_cls,
                f"{key} scope drift: surface={surface_scopes} http={http_scopes}",
            )


def load_http_operation_catalog(
    path: Path,
    error_cls: type[ErrorT] = ManifestError,
) -> list[dict[str, Any]]:
    """Load generated HTTP operation catalog evidence."""
    catalog = load_json(path, error_cls)
    if catalog.get("kind") != "kiva-http-operation-catalog-evidence":
        raise validation_error(error_cls, f"{path}.kind must be kiva-http-operation-catalog-evidence")
    operations = require_list(catalog.get("operations"), f"{path}.operations", error_cls)
    loaded: list[dict[str, Any]] = []
    seen_routes: set[tuple[str, str, str]] = set()
    seen_operations: set[tuple[str, str]] = set()
    for index, value in enumerate(operations):
        operation = require_mapping(value, f"{path}.operations[{index}]", error_cls)
        openapi_contract = require_string(
            operation.get("openapi_contract"),
            f"{path}.operations[{index}].openapi_contract",
            error_cls,
        )
        method = require_string(
            operation.get("method"),
            f"{path}.operations[{index}].method",
            error_cls,
        ).lower()
        route = require_string(
            operation.get("path"),
            f"{path}.operations[{index}].path",
            error_cls,
        )
        operation_id = require_string(
            operation.get("operation_id"),
            f"{path}.operations[{index}].operation_id",
            error_cls,
        )
        scopes = require_string_list(
            operation.get("required_scopes", []),
            f"{path}.operations[{index}].required_scopes",
            error_cls,
            allow_empty=True,
        )
        route_key = (openapi_contract, method, route)
        operation_key = (openapi_contract, operation_id)
        if route_key in seen_routes:
            raise validation_error(error_cls, f"{path} duplicates HTTP operation route: {route_key}")
        if operation_key in seen_operations:
            raise validation_error(error_cls, f"{path} duplicates HTTP operation id: {operation_key}")
        seen_routes.add(route_key)
        seen_operations.add(operation_key)
        loaded.append(
            {
                **operation,
                "openapi_contract": openapi_contract,
                "method": method,
                "path": route,
                "operation_id": operation_id,
                "required_scopes": scopes,
            }
        )
    return loaded
