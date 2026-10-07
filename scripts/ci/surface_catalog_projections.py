"""Projection helpers for the generated Kiva surface catalog."""

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
from surface_catalog_model import SurfaceEntry, surface_entries, surface_operation_projection_basis


ErrorT = TypeVar("ErrorT", bound=RuntimeError)

def project_application_operation_catalog(
    document: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Project the application operation catalog from the surface catalog."""
    projection = require_mapping(
        document.get("application_operation_projection"),
        "application_operation_projection",
        error_cls,
    )
    entries: list[dict[str, Any]] = []
    for entry in surface_entries(document, error_cls):
        application_id = entry.application_operation_id
        if application_id is None:
            continue
        projected = {
            "id": application_id,
            "surface": entry.application_surface,
            "workflow_label": entry.workflow_label,
            "rust_workflow_kind": entry.rust_workflow_kind,
            "openapi_contract": entry.openapi_contract,
            "operation_id": entry.operation_id,
            "required_scopes": list(entry.required_scopes),
        }
        if entry.workflow_family is not None:
            projected["workflow_family"] = entry.workflow_family
        entries.append(projected)
    return {
        "schema_version": 1,
        "kind": "kiva-application-operation-catalog",
        "status": "active",
        "generated_from": "generated/architecture/surface-catalog.yaml",
        "projection_basis": surface_operation_projection_basis(
            document,
            "workflow-and-scope-binding-projection",
            error_cls,
        ),
        "description": require_string(
            projection.get("description"),
            "application_operation_projection.description",
            error_cls,
        ),
        "rust_registry_path": require_string(
            projection.get("rust_registry_path"),
            "application_operation_projection.rust_registry_path",
            error_cls,
        ),
        "rust_operation_ids_path": require_string(
            projection.get("rust_operation_ids_path"),
            "application_operation_projection.rust_operation_ids_path",
            error_cls,
        ),
        "rust_workflow_metadata_path": require_string(
            projection.get("rust_workflow_metadata_path"),
            "application_operation_projection.rust_workflow_metadata_path",
            error_cls,
        ),
        "source_paths": require_string_list(
            projection.get("source_paths"),
            "application_operation_projection.source_paths",
            error_cls,
        ),
        "entries": entries,
    }


def project_pds_resource_family_ledger(
    document: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Project the PDS resource-family ledger from the surface catalog."""
    projection = require_mapping(
        document.get("pds_resource_family_projection"),
        "pds_resource_family_projection",
        error_cls,
    )
    family_entries = require_list(
        document.get("pds_resource_families"),
        "pds_resource_families",
        error_cls,
    )
    families: list[dict[str, Any]] = []
    operations_by_family: dict[str, list[dict[str, str]]] = {}
    for entry in surface_entries(document, error_cls):
        family_id = entry.pds_resource_family
        if family_id is None:
            continue
        operations_by_family.setdefault(str(family_id), []).append(
            {
                "path": entry.path,
                "method": entry.method,
                "route_context": str(entry.pds_route_context),
            }
        )
    seen_families: set[str] = set()
    for index, value in enumerate(family_entries):
        family = require_mapping(value, f"pds_resource_families[{index}]", error_cls)
        family_id = require_string(family.get("id"), f"pds_resource_families[{index}].id", error_cls)
        if family_id in seen_families:
            raise validation_error(error_cls, f"duplicate PDS resource family: {family_id}")
        seen_families.add(family_id)
        families.append(
            {
                "id": family_id,
                "description": require_string(
                    family.get("description"),
                    f"pds_resource_families[{index}].description",
                    error_cls,
                ),
                "implementation_files": require_string_list(
                    family.get("implementation_files"),
                    f"pds_resource_families[{index}].implementation_files",
                    error_cls,
                ),
                "operations": operations_by_family.get(family_id, []),
            }
        )
    boilerplate_policy = require_mapping(
        projection.get("server_resource_family_boilerplate_policy"),
        "pds_resource_family_projection.server_resource_family_boilerplate_policy",
        error_cls,
    )
    return {
        "schema_version": 1,
        "kind": "kiva-pds-resource-family-ledger",
        "status": "active",
        "generated_from": "generated/architecture/surface-catalog.yaml",
        "projection_basis": surface_operation_projection_basis(
            document,
            "pds-route-context-and-handler-ownership-projection",
            error_cls,
        ),
        "description": require_string(
            projection.get("description"),
            "pds_resource_family_projection.description",
            error_cls,
        ),
        "source_contract": require_string(
            projection.get("source_contract"),
            "pds_resource_family_projection.source_contract",
            error_cls,
        ),
        "http_operation_catalog": require_string(
            projection.get("http_operation_catalog"),
            "pds_resource_family_projection.http_operation_catalog",
            error_cls,
        ),
        "operation_catalog": require_string(
            projection.get("operation_catalog"),
            "pds_resource_family_projection.operation_catalog",
            error_cls,
        ),
        "route_file": require_string(
            projection.get("route_file"),
            "pds_resource_family_projection.route_file",
            error_cls,
        ),
        "path_constants_file": require_string(
            projection.get("path_constants_file"),
            "pds_resource_family_projection.path_constants_file",
            error_cls,
        ),
        "derived_fields": require_string_list(
            projection.get("derived_fields"),
            "pds_resource_family_projection.derived_fields",
            error_cls,
        ),
        "resource_family_profile_model": require_mapping(
            projection.get("resource_family_profile_model"),
            "pds_resource_family_projection.resource_family_profile_model",
            error_cls,
        ),
        "application_operation_catalog_policy": require_mapping(
            projection.get("application_operation_catalog_policy"),
            "pds_resource_family_projection.application_operation_catalog_policy",
            error_cls,
        ),
        "server_resource_family_boilerplate_policy": boilerplate_policy,
        "server_resource_family_boilerplate_projection": pds_boilerplate_projection(
            boilerplate_policy,
            families,
            error_cls,
        ),
        "resource_families": families,
    }


def pds_boilerplate_projection(
    policy: dict[str, Any],
    families: list[dict[str, Any]],
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Project required server implementation roles for each PDS family."""
    role_matrix = require_mapping(
        policy.get("family_role_matrix"),
        "server_resource_family_boilerplate_policy.family_role_matrix",
        error_cls,
    )
    role_prefixes = require_mapping(
        policy.get("role_prefixes"),
        "server_resource_family_boilerplate_policy.role_prefixes",
        error_cls,
    )
    projected_families: list[dict[str, Any]] = []
    for family in families:
        family_id = require_string(family.get("id"), "pds_resource_family.id", error_cls)
        implementation_files = require_string_list(
            family.get("implementation_files"),
            f"{family_id}.implementation_files",
            error_cls,
        )
        roles: list[dict[str, Any]] = []
        for role in require_string_list(
            role_matrix.get(family_id),
            f"server_resource_family_boilerplate_policy.family_role_matrix.{family_id}",
            error_cls,
        ):
            prefix = require_string(
                role_prefixes.get(role),
                f"server_resource_family_boilerplate_policy.role_prefixes.{role}",
                error_cls,
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
        projected_families.append({"family": family_id, "roles": roles})
    return {
        "source_policy": "server_resource_family_boilerplate_policy",
        "families": projected_families,
    }


def _pds_family_sources_by_id(
    document: dict[str, Any],
    error_cls: type[ErrorT],
) -> dict[str, dict[str, Any]]:
    """Return PDS resource-family source rows keyed by family id."""
    result: dict[str, dict[str, Any]] = {}
    for index, value in enumerate(
        require_list(document.get("pds_resource_families"), "pds_resource_families", error_cls)
    ):
        family = require_mapping(value, f"pds_resource_families[{index}]", error_cls)
        family_id = require_string(family.get("id"), f"pds_resource_families[{index}].id", error_cls)
        if family_id in result:
            raise validation_error(error_cls, f"duplicate PDS resource family: {family_id}")
        result[family_id] = family
    return result


def _surface_family_operation_row(entry: SurfaceEntry) -> dict[str, Any]:
    """Render one operation row owned by a generated surface family."""
    row: dict[str, Any] = {
        "openapi_contract": entry.openapi_contract,
        "method": entry.method,
        "path": entry.path,
        "operation_id": entry.operation_id,
        "required_scopes": list(entry.required_scopes),
    }
    if entry.application_operation_id is not None:
        row["application_operation_id"] = entry.application_operation_id
    if entry.application_surface is not None:
        row["application_surface"] = entry.application_surface
    if entry.workflow_family is not None:
        row["workflow_family"] = entry.workflow_family
    if entry.pds_resource_family is not None:
        row["pds_resource_family"] = entry.pds_resource_family
    if entry.pds_route_context is not None:
        row["pds_route_context"] = entry.pds_route_context
    return row


def _surface_family_prefix_match(path: str, prefixes: list[str]) -> int | None:
    """Return the most-specific matching prefix length for a route path."""
    matches = [len(prefix) for prefix in prefixes if path.startswith(prefix)]
    if not matches:
        return None
    return max(matches)


def project_surface_family_ledger(
    document: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Project the cross-surface family ledger from the surface catalog."""
    projection = require_mapping(
        document.get("surface_family_projection"),
        "surface_family_projection",
        error_cls,
    )
    pds_sources = _pds_family_sources_by_id(document, error_cls)
    raw_families = require_list(document.get("surface_families"), "surface_families", error_cls)
    family_rows: list[dict[str, Any]] = []
    seen_family_ids: set[str] = set()
    for index, value in enumerate(raw_families):
        label = f"surface_families[{index}]"
        family = require_mapping(value, label, error_cls)
        family_id = require_string(family.get("id"), f"{label}.id", error_cls)
        if family_id in seen_family_ids:
            raise validation_error(error_cls, f"duplicate surface family id: {family_id}")
        seen_family_ids.add(family_id)
        source_pds_family = family.get("source_pds_resource_family")
        path_prefixes = family.get("path_prefixes")
        if source_pds_family is not None and path_prefixes is not None:
            raise validation_error(
                error_cls,
                f"{label} must use source_pds_resource_family or path_prefixes, not both",
            )
        if source_pds_family is None and path_prefixes is None:
            raise validation_error(
                error_cls,
                f"{label} must declare source_pds_resource_family or path_prefixes",
            )
        implementation_files = family.get("implementation_files")
        if implementation_files is None and source_pds_family is not None:
            pds_family_id = require_string(
                source_pds_family,
                f"{label}.source_pds_resource_family",
                error_cls,
            )
            pds_source = pds_sources.get(pds_family_id)
            if pds_source is None:
                raise validation_error(
                    error_cls,
                    f"{label}.source_pds_resource_family references missing PDS family: {pds_family_id}",
                )
            implementation_files = pds_source.get("implementation_files")
        family_rows.append(
            {
                "id": family_id,
                "surface": require_string(family.get("surface"), f"{label}.surface", error_cls),
                "openapi_contract": require_string(
                    family.get("openapi_contract"),
                    f"{label}.openapi_contract",
                    error_cls,
                ),
                "description": require_string(family.get("description"), f"{label}.description", error_cls),
                "source_pds_resource_family": (
                    None
                    if source_pds_family is None
                    else require_string(source_pds_family, f"{label}.source_pds_resource_family", error_cls)
                ),
                "path_prefixes": (
                    []
                    if path_prefixes is None
                    else require_string_list(path_prefixes, f"{label}.path_prefixes", error_cls)
                ),
                "implementation_files": require_string_list(
                    implementation_files,
                    f"{label}.implementation_files",
                    error_cls,
                ),
                "operations": [],
            }
        )

    assigned_counts = {family["id"]: 0 for family in family_rows}
    for entry in surface_entries(document, error_cls):
        candidates: list[tuple[int, dict[str, Any]]] = []
        for family in family_rows:
            if entry.openapi_contract != family["openapi_contract"]:
                continue
            source_pds_family = family["source_pds_resource_family"]
            if source_pds_family is not None and entry.pds_resource_family == source_pds_family:
                candidates.append((10_000 + len(source_pds_family), family))
                continue
            prefix_length = _surface_family_prefix_match(entry.path, family["path_prefixes"])
            if prefix_length is not None:
                candidates.append((prefix_length, family))
        if not candidates:
            raise validation_error(
                error_cls,
                f"surface operation has no surface-family owner: {entry.openapi_contract} {entry.method} {entry.path}",
            )
        best_specificity = max(specificity for specificity, _family in candidates)
        best = [family for specificity, family in candidates if specificity == best_specificity]
        if len(best) != 1:
            family_ids = sorted(family["id"] for family in best)
            raise validation_error(
                error_cls,
                f"surface operation has ambiguous surface-family owners: "
                f"{entry.openapi_contract} {entry.method} {entry.path}: {family_ids}",
            )
        family = best[0]
        family["operations"].append(_surface_family_operation_row(entry))
        assigned_counts[family["id"]] += 1

    empty_families = sorted(family_id for family_id, count in assigned_counts.items() if count == 0)
    if empty_families:
        raise validation_error(error_cls, f"surface families own no operations: {empty_families}")

    families: list[dict[str, Any]] = []
    for family in family_rows:
        projected = {
            "id": family["id"],
            "surface": family["surface"],
            "openapi_contract": family["openapi_contract"],
            "description": family["description"],
            "implementation_files": family["implementation_files"],
            "operation_count": len(family["operations"]),
            "operations": family["operations"],
        }
        if family["source_pds_resource_family"] is not None:
            projected["source_pds_resource_family"] = family["source_pds_resource_family"]
        else:
            projected["path_prefixes"] = family["path_prefixes"]
        families.append(projected)

    return {
        "schema_version": 1,
        "kind": "kiva-surface-family-ledger",
        "status": "active",
        "generated_from": "generated/architecture/surface-catalog.yaml",
        "projection_basis": surface_operation_projection_basis(
            document,
            "cross-surface-family-ownership-projection",
            error_cls,
        ),
        "description": require_string(
            projection.get("description"),
            "surface_family_projection.description",
            error_cls,
        ),
        "assignment_policy": require_mapping(
            projection.get("assignment_policy"),
            "surface_family_projection.assignment_policy",
            error_cls,
        ),
        "implementation_anchor_policy": require_mapping(
            document.get("surface_family_implementation_anchor_policy"),
            "surface_family_implementation_anchor_policy",
            error_cls,
        ),
        "families": families,
    }


SURFACE_PROFILE_DEFINITIONS: dict[tuple[str | None, str | None], dict[str, str]] = {
    ("analytics", None): {
        "prefix": "ANALYTICS",
        "struct": "AnalyticsSurfaceProfile",
        "profile_id": "analytics-application",
        "runtime_role": "application-workflow",
        "description": "governed analytics application surface",
        "failure_label": "analytics",
    },
    ("pds", "pds"): {
        "prefix": "PDS_REPOSITORY",
        "struct": "PdsRepositorySurfaceProfile",
        "profile_id": "pds-repository",
        "runtime_role": "repository-query",
        "description": "PDS repository/query surface",
        "failure_label": "PDS",
    },
    ("pds", "pds-application"): {
        "prefix": "PDS_APPLICATION",
        "struct": "PdsApplicationSurfaceProfile",
        "profile_id": "pds-application",
        "runtime_role": "application-workflow",
        "description": "PDS application workflow surface",
        "failure_label": "PDS application",
    },
    ("confidential", None): {
        "prefix": "CONFIDENTIAL",
        "struct": "ConfidentialSurfaceProfile",
        "profile_id": "confidential-application",
        "runtime_role": "application-workflow",
        "description": "Confidential-computation application surface",
        "failure_label": "confidential",
    },
}


def _rust_option(value: str | None) -> str:
    """Render an optional static string as Rust."""
    if value is None:
        return "None"
    return f'Some("{value}")'


def render_surface_profile_registry(
    document: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> str:
    """Render the generated Rust server surface-profile registry."""
    profile_counts: dict[tuple[str | None, str | None], int] = {}
    for entry in surface_entries(document, error_cls):
        key = (entry.application_surface, entry.pds_route_context)
        if key == (None, None):
            continue
        profile_counts[key] = profile_counts.get(key, 0) + 1

    expected_keys = set(SURFACE_PROFILE_DEFINITIONS)
    actual_keys = set(profile_counts)
    if actual_keys != expected_keys:
        missing = sorted(str(key) for key in expected_keys.difference(actual_keys))
        unexpected = sorted(str(key) for key in actual_keys.difference(expected_keys))
        raise validation_error(
            error_cls,
            f"surface profile registry drift; missing={missing}, unexpected={unexpected}",
        )

    lines = [
        "//! Generated from `generated/architecture/surface-catalog.yaml`.",
        "//! Do not edit by hand.",
        "",
        "use super::surface_operation::{SurfaceOperationProfile, SurfaceProfileSpec};",
        "",
        "/// Number of generated server surface profiles.",
        f"pub(crate) const SURFACE_PROFILE_COUNT: usize = {len(SURFACE_PROFILE_DEFINITIONS)};",
        "",
    ]
    structs: list[str] = []
    for key, definition in SURFACE_PROFILE_DEFINITIONS.items():
        application_surface, pds_route_context = key
        operation_count = profile_counts[key]
        prefix = definition["prefix"]
        description = definition["description"]
        struct_name = definition["struct"]
        profile_id = definition["profile_id"]
        runtime_role = definition["runtime_role"]
        failure_label = definition["failure_label"]
        lines.extend(
            [
                f"/// Stable generated profile id for the {description}.",
                f'pub(crate) const {prefix}_PROFILE_ID: &str = "{profile_id}";',
                "",
                f"/// Runtime role for the generated {description}.",
                f'pub(crate) const {prefix}_RUNTIME_ROLE: &str = "{runtime_role}";',
                "",
                f"/// Application-surface label for the generated {description}.",
                f"pub(crate) const {prefix}_APPLICATION_SURFACE: Option<&'static str> = "
                f"{_rust_option(application_surface)};",
                "",
                f"/// PDS route-context label for the generated {description}.",
                f"pub(crate) const {prefix}_PDS_ROUTE_CONTEXT: Option<&'static str> = "
                f"{_rust_option(pds_route_context)};",
                "",
                f"/// Surface operation count covered by the generated {description}.",
                f"pub(crate) const {prefix}_OPERATION_COUNT: usize = {operation_count};",
                "",
                f"/// Generated marker for the {description}.",
                "#[derive(Debug, Clone, Copy)]",
                f"pub(crate) struct {struct_name};",
                "",
                f"impl SurfaceOperationProfile for {struct_name} {{",
                f"    const APPLICATION_SURFACE: Option<&'static str> = {prefix}_APPLICATION_SURFACE;",
                f'    const FAILURE_LABEL: &\'static str = "{failure_label}";',
                f"    const OPERATION_COUNT: usize = {prefix}_OPERATION_COUNT;",
                f"    const PDS_ROUTE_CONTEXT: Option<&'static str> = {prefix}_PDS_ROUTE_CONTEXT;",
                f"    const PROFILE_ID: &'static str = {prefix}_PROFILE_ID;",
                f"    const RUNTIME_ROLE: &'static str = {prefix}_RUNTIME_ROLE;",
                "}",
                "",
            ]
        )
        structs.append(struct_name)
    lines.extend(
        [
            "/// Generated static profile metadata aligned with the surface catalog labels.",
            "pub(crate) const SURFACE_PROFILE_SPECS: [SurfaceProfileSpec; SURFACE_PROFILE_COUNT] = [",
        ]
    )
    lines.extend(f"    SurfaceProfileSpec::for_profile::<{struct_name}>()," for struct_name in structs)
    lines.extend(["];", ""])
    return "\n".join(lines)
