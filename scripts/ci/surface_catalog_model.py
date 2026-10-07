"""Typed model helpers for the Kiva surface operation catalog."""

from __future__ import annotations

from dataclasses import dataclass
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


@dataclass(frozen=True)
class HttpOperation:
    """OpenAPI-derived public HTTP operation identity."""

    openapi_contract: str
    method: str
    path: str
    operation_id: str
    required_scopes: tuple[str, ...]

    @property
    def route_key(self) -> tuple[str, str, str]:
        """Return the OpenAPI route key."""
        return (self.openapi_contract, self.method, self.path)

    @property
    def operation_key(self) -> tuple[str, str]:
        """Return the OpenAPI operation key."""
        return (self.openapi_contract, self.operation_id)

    def to_mapping(self) -> dict[str, Any]:
        """Render this HTTP operation as a YAML-compatible mapping."""
        return {
            "openapi_contract": self.openapi_contract,
            "method": self.method,
            "path": self.path,
            "operation_id": self.operation_id,
            "required_scopes": list(self.required_scopes),
        }


@dataclass(frozen=True)
class SemanticOperationOverlay:
    """Human-owned semantic operation overlay projected onto an HTTP operation."""

    application_operation_id: str | None = None
    application_surface: str | None = None
    workflow_label: str | None = None
    rust_workflow_kind: str | None = None
    workflow_family: str | None = None
    pds_resource_family: str | None = None
    pds_route_context: str | None = None

    @property
    def is_empty(self) -> bool:
        """Return whether this overlay carries no semantic ownership."""
        return not any(self.to_mapping())

    def to_mapping(self) -> dict[str, Any]:
        """Render only semantic fields that are present."""
        entry: dict[str, Any] = {}
        optional_fields = {
            "application_operation_id": self.application_operation_id,
            "application_surface": self.application_surface,
            "workflow_label": self.workflow_label,
            "rust_workflow_kind": self.rust_workflow_kind,
            "workflow_family": self.workflow_family,
            "pds_resource_family": self.pds_resource_family,
            "pds_route_context": self.pds_route_context,
        }
        for key, value in optional_fields.items():
            if value is not None:
                entry[key] = value
        return entry


@dataclass(frozen=True)
class SurfaceEntry:
    """Joined public surface operation: OpenAPI HTTP operation plus semantic overlay."""

    http: HttpOperation
    overlay: SemanticOperationOverlay

    @property
    def openapi_contract(self) -> str:
        """Return the OpenAPI contract path."""
        return self.http.openapi_contract

    @property
    def method(self) -> str:
        """Return the HTTP method."""
        return self.http.method

    @property
    def path(self) -> str:
        """Return the HTTP path."""
        return self.http.path

    @property
    def operation_id(self) -> str:
        """Return the OpenAPI operationId."""
        return self.http.operation_id

    @property
    def required_scopes(self) -> tuple[str, ...]:
        """Return the OpenAPI-derived OAuth/OIDC scopes."""
        return self.http.required_scopes

    @property
    def application_operation_id(self) -> str | None:
        """Return the application operation id when this operation enters a workflow."""
        return self.overlay.application_operation_id

    @property
    def application_surface(self) -> str | None:
        """Return the application surface when present."""
        return self.overlay.application_surface

    @property
    def workflow_label(self) -> str | None:
        """Return the workflow label when present."""
        return self.overlay.workflow_label

    @property
    def rust_workflow_kind(self) -> str | None:
        """Return the Rust workflow kind when present."""
        return self.overlay.rust_workflow_kind

    @property
    def workflow_family(self) -> str | None:
        """Return the workflow family when present."""
        return self.overlay.workflow_family

    @property
    def pds_resource_family(self) -> str | None:
        """Return the PDS resource family when present."""
        return self.overlay.pds_resource_family

    @property
    def pds_route_context(self) -> str | None:
        """Return the PDS route context when present."""
        return self.overlay.pds_route_context

    @property
    def route_key(self) -> tuple[str, str, str]:
        """Return the OpenAPI route key."""
        return self.http.route_key

    @property
    def operation_key(self) -> tuple[str, str]:
        """Return the OpenAPI operation key."""
        return self.http.operation_key

    def to_mapping(self) -> dict[str, Any]:
        """Render this entry as the YAML-compatible joined mapping."""
        return self.http.to_mapping() | self.overlay.to_mapping()

    def surface_index_row(self) -> dict[str, Any]:
        """Render this entry as a generated surface-index row."""
        return {
            "openapi_contract": self.openapi_contract,
            "method": self.method,
            "path": self.path,
            "operation_id": self.operation_id,
            "required_scopes": sorted(self.required_scopes),
            "application_operation_id": self.application_operation_id,
            "application_surface": self.application_surface,
            "workflow_label": self.workflow_label,
            "rust_workflow_kind": self.rust_workflow_kind,
            "pds_resource_family": self.pds_resource_family,
            "pds_route_context": self.pds_route_context,
        }


def _optional_string(
    entry: dict[str, Any],
    field: str,
    label: str,
    error_cls: type[ErrorT],
) -> str | None:
    """Return an optional string field."""
    value = entry.get(field)
    if value is None:
        return None
    return require_string(value, f"{label}.{field}", error_cls)


def _parse_surface_entry(
    value: Any,
    label: str,
    error_cls: type[ErrorT],
) -> SurfaceEntry:
    """Parse one canonical surface entry."""
    entry = require_mapping(value, label, error_cls)
    openapi_contract = require_string(entry.get("openapi_contract"), f"{label}.openapi_contract", error_cls)
    method = require_string(entry.get("method"), f"{label}.method", error_cls).lower()
    path = require_string(entry.get("path"), f"{label}.path", error_cls)
    operation_id = require_string(entry.get("operation_id"), f"{label}.operation_id", error_cls)
    required_scopes = tuple(
        require_string_list(
            entry.get("required_scopes", []),
            f"{label}.required_scopes",
            error_cls,
            allow_empty=True,
        )
    )
    application_operation_id = _optional_string(entry, "application_operation_id", label, error_cls)
    application_surface = _optional_string(entry, "application_surface", label, error_cls)
    workflow_label = _optional_string(entry, "workflow_label", label, error_cls)
    rust_workflow_kind = _optional_string(entry, "rust_workflow_kind", label, error_cls)
    workflow_family = _optional_string(entry, "workflow_family", label, error_cls)
    pds_resource_family = _optional_string(entry, "pds_resource_family", label, error_cls)
    pds_route_context = _optional_string(entry, "pds_route_context", label, error_cls)
    if application_operation_id is not None:
        for field, field_value in (
            ("application_surface", application_surface),
            ("workflow_label", workflow_label),
            ("rust_workflow_kind", rust_workflow_kind),
        ):
            if field_value is None:
                raise validation_error(error_cls, f"{label}.{field} is required")
    if pds_resource_family is not None and pds_route_context is None:
        raise validation_error(error_cls, f"{label}.pds_route_context is required")
    return SurfaceEntry(
        http=HttpOperation(
            openapi_contract=openapi_contract,
            method=method,
            path=path,
            operation_id=operation_id,
            required_scopes=required_scopes,
        ),
        overlay=SemanticOperationOverlay(
            application_operation_id=application_operation_id,
            application_surface=application_surface,
            workflow_label=workflow_label,
            rust_workflow_kind=rust_workflow_kind,
            workflow_family=workflow_family,
            pds_resource_family=pds_resource_family,
            pds_route_context=pds_route_context,
        ),
    )


def surface_entries(
    document: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> list[SurfaceEntry]:
    """Validate and return typed canonical surface operation entries."""
    entries = require_list(document.get("entries"), "surface_catalog.entries", error_cls)
    loaded: list[SurfaceEntry] = []
    seen_routes: set[tuple[str, str, str]] = set()
    seen_operations: set[tuple[str, str]] = set()
    seen_application_ids: set[str] = set()
    for index, value in enumerate(entries):
        parsed = _parse_surface_entry(value, f"surface_catalog.entries[{index}]", error_cls)
        if parsed.route_key in seen_routes:
            raise validation_error(error_cls, f"duplicate surface route entry: {parsed.route_key}")
        if parsed.operation_key in seen_operations:
            raise validation_error(error_cls, f"duplicate surface operation entry: {parsed.operation_key}")
        seen_routes.add(parsed.route_key)
        seen_operations.add(parsed.operation_key)
        application_id = parsed.application_operation_id
        if application_id is not None:
            if application_id in seen_application_ids:
                raise validation_error(error_cls, f"duplicate application operation id: {application_id}")
            seen_application_ids.add(application_id)
        loaded.append(parsed)
    return loaded


def surface_operation_projection_basis(
    document: dict[str, Any],
    projection_role: str,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Return common SurfaceOperation provenance for generated projections."""
    algebra = require_mapping(
        document.get("surface_operation_algebra"),
        "surface_catalog.surface_operation_algebra",
        error_cls,
    )
    normal_form = require_mapping(
        document.get("surface_operation_normal_form"),
        "surface_catalog.surface_operation_normal_form",
        error_cls,
    )
    joined_facets = require_string_list(
        algebra.get("joined_facets"),
        "surface_operation_algebra.joined_facets",
        error_cls,
    )
    return {
        "root_type": require_string(
            algebra.get("root_type"),
            "surface_operation_algebra.root_type",
            error_cls,
        ),
        "projection_role": projection_role,
        "normal_form": "surface_operation_normal_form",
        "operation_key_fields": ["openapi_contract", "operation_id"],
        "route_key_fields": ["openapi_contract", "method", "path"],
        "joined_facets": joined_facets,
        "operation_count": int(normal_form.get("entry_count", 0)),
    }


def require_surface_entries(
    document: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> list[dict[str, Any]]:
    """Validate and return canonical surface operation entries as mappings."""
    return [entry.to_mapping() for entry in surface_entries(document, error_cls)]


def surface_index_rows(
    document: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> list[dict[str, Any]]:
    """Return generated surface-index rows from canonical entries."""
    return [entry.surface_index_row() for entry in surface_entries(document, error_cls)]


def surface_operation_normal_form(
    document: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Return the normalized SurfaceOperation algebra projection."""
    rows = []
    for entry in surface_entries(document, error_cls):
        overlay = entry.overlay.to_mapping()
        rows.append(
            {
                "operation_key": {
                    "openapi_contract": entry.openapi_contract,
                    "operation_id": entry.operation_id,
                },
                "http_operation": entry.http.to_mapping(),
                "semantic_overlay": overlay,
                "overlay_present": bool(overlay),
                "surface_operation": entry.to_mapping(),
            }
        )
    return {
        "root_type": "SurfaceOperation",
        "join_type": "HttpOperation + Optional<SemanticOperationOverlay>",
        "entry_count": len(rows),
        "entries": rows,
    }
