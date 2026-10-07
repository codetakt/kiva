"""Shared compiler helpers for generated architecture projections."""

from __future__ import annotations

from pathlib import Path
from typing import Any, TypeVar

import yaml

from .architecture import artifact_catalog_from_sources, classified_source_map
from .manifest import (
    generated_target_values,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
    resolve_projection_sources,
    validate_projection_check_refs,
    validation_error,
)


ErrorT = TypeVar("ErrorT", bound=RuntimeError)

ARCHITECTURE_GRAPH_KIND = "kiva-architecture-graph-source"
ARCHITECTURE_POLICY_KIND = "kiva-architecture-policy-source"
PROJECTION_SPEC_KIND = "kiva-architecture-projection-spec-source"
COMPILED_ARCHITECTURE_KIND = "kiva-compiled-architecture"
PROJECTION_SPEC_FIELDS = {
    "bundle",
    "generated_artifacts",
    "index",
    "projection_checks",
    "projection_exports",
    "projection_generator",
    "projection_id",
    "projection_kind",
    "projection_source_refs",
    "projection_sources",
    "projection_targets",
    "source",
    "strategy",
    "validator",
}


class NoAliasDumper(yaml.SafeDumper):
    """YAML dumper that keeps generated files free of anchors and aliases."""

    def ignore_aliases(self, data: object) -> bool:
        return True


def dump_yaml(document: dict[str, Any]) -> str:
    """Return stable generated YAML text without PyYAML anchors."""
    return yaml.dump(
        document,
        Dumper=NoAliasDumper,
        sort_keys=False,
        allow_unicode=False,
        width=1000,
    )


def architecture_policy_source_path(
    graph: dict[str, Any],
    graph_path: Path,
    error_cls: type[ErrorT],
) -> Path:
    """Return the architecture policy source declared by an architecture graph."""
    policy = require_mapping(
        graph.get("projection_policy"),
        f"{graph_path}.projection_policy",
        error_cls,
    )
    return Path(
        require_string(
            policy.get("architecture_policy_source"),
            f"{graph_path}.projection_policy.architecture_policy_source",
            error_cls,
        )
    )


def projection_spec_source_path(
    graph: dict[str, Any],
    graph_path: Path,
    error_cls: type[ErrorT],
) -> Path:
    """Return the projection-spec source declared by an architecture graph."""
    policy = require_mapping(
        graph.get("projection_policy"),
        f"{graph_path}.projection_policy",
        error_cls,
    )
    return Path(
        require_string(
            policy.get("projection_spec_source"),
            f"{graph_path}.projection_policy.projection_spec_source",
            error_cls,
        )
    )


def validate_architecture_graph_envelope(
    graph: dict[str, Any],
    graph_path: Path,
    error_cls: type[ErrorT],
) -> None:
    """Validate the common architecture graph envelope."""
    if graph.get("schema_version") != 1:
        raise validation_error(error_cls, f"{graph_path}.schema_version must be 1")
    if graph.get("kind") != ARCHITECTURE_GRAPH_KIND:
        raise validation_error(error_cls, f"{graph_path}.kind must be {ARCHITECTURE_GRAPH_KIND}")
    if graph.get("status") != "active":
        raise validation_error(error_cls, f"{graph_path}.status must be active")


def validate_architecture_policy_envelope(
    policy: dict[str, Any],
    policy_path: Path,
    error_cls: type[ErrorT],
) -> None:
    """Validate the common architecture policy envelope."""
    if policy.get("schema_version") != 1:
        raise validation_error(error_cls, f"{policy_path}.schema_version must be 1")
    if policy.get("kind") != ARCHITECTURE_POLICY_KIND:
        raise validation_error(error_cls, f"{policy_path}.kind must be {ARCHITECTURE_POLICY_KIND}")
    if policy.get("status") != "active":
        raise validation_error(error_cls, f"{policy_path}.status must be active")


def validate_projection_spec_envelope(
    projection_spec: dict[str, Any],
    projection_spec_path: Path,
    graph_path: Path,
    error_cls: type[ErrorT],
) -> None:
    """Validate the common architecture ProjectionSpec envelope."""
    if projection_spec.get("schema_version") != 1:
        raise validation_error(error_cls, f"{projection_spec_path}.schema_version must be 1")
    if projection_spec.get("kind") != PROJECTION_SPEC_KIND:
        raise validation_error(error_cls, f"{projection_spec_path}.kind must be {PROJECTION_SPEC_KIND}")
    if projection_spec.get("status") != "active":
        raise validation_error(error_cls, f"{projection_spec_path}.status must be active")
    source_graph = require_string(
        projection_spec.get("source_graph"),
        f"{projection_spec_path}.source_graph",
        error_cls,
    )
    if source_graph != graph_path.as_posix():
        raise validation_error(
            error_cls,
            f"{projection_spec_path}.source_graph must be {graph_path.as_posix()}",
        )


def projection_specs_by_source(
    projection_spec: dict[str, Any],
    projection_spec_path: Path,
    error_cls: type[ErrorT],
) -> dict[str, dict[str, Any]]:
    """Return ProjectionSpec rows keyed by ArchitectureGraph source id."""
    rows = require_mapping(
        projection_spec.get("projections"),
        f"{projection_spec_path}.projections",
        error_cls,
    )
    specs: dict[str, dict[str, Any]] = {}
    for source_key, raw_spec in rows.items():
        if not isinstance(source_key, str) or not source_key:
            raise validation_error(error_cls, f"{projection_spec_path}.projections keys must be non-empty strings")
        spec = require_mapping(
            raw_spec,
            f"{projection_spec_path}.projections.{source_key}",
            error_cls,
        )
        unexpected = sorted(set(spec).difference(PROJECTION_SPEC_FIELDS))
        if unexpected:
            raise validation_error(
                error_cls,
                f"{projection_spec_path}.projections.{source_key} has non-projection fields: {unexpected}",
            )
        specs[source_key] = spec
    return specs


def merge_projection_specs(
    raw_sources: dict[str, Any],
    projection_spec: dict[str, Any],
    projection_spec_path: Path,
    error_cls: type[ErrorT],
) -> dict[str, dict[str, Any]]:
    """Attach per-source ProjectionSpec rows to ArchitectureGraph source nodes."""
    specs = projection_specs_by_source(projection_spec, projection_spec_path, error_cls)
    unknown_specs = sorted(set(specs).difference(raw_sources))
    if unknown_specs:
        raise validation_error(
            error_cls,
            f"{projection_spec_path}.projections references unknown ArchitectureGraph sources: {unknown_specs}",
        )
    merged: dict[str, dict[str, Any]] = {}
    for source_key, raw_source in raw_sources.items():
        source = require_mapping(raw_source, f"sources.{source_key}", error_cls)
        duplicate_projection_fields = sorted(set(source).intersection(PROJECTION_SPEC_FIELDS))
        spec = specs.get(source_key, {})
        for field in duplicate_projection_fields:
            if field not in spec or source[field] != spec[field]:
                raise validation_error(
                    error_cls,
                    f"sources.{source_key}.{field} must move to {projection_spec_path} without drift",
                )
        stripped_source = {
            key: value for key, value in source.items() if key not in PROJECTION_SPEC_FIELDS
        }
        merged[source_key] = {**stripped_source, **spec}
    return merged


def merge_projection_rows(
    raw_sources: dict[str, Any],
    specs: dict[str, dict[str, Any]],
    specs_label: str,
    error_cls: type[ErrorT],
) -> dict[str, dict[str, Any]]:
    """Attach normal-form projection rows to ArchitectureGraph source nodes."""
    unknown_specs = sorted(set(specs).difference(raw_sources))
    if unknown_specs:
        raise validation_error(
            error_cls,
            f"{specs_label} references unknown ArchitectureGraph sources: {unknown_specs}",
        )
    merged: dict[str, dict[str, Any]] = {}
    for source_key, raw_source in raw_sources.items():
        source = require_mapping(raw_source, f"sources.{source_key}", error_cls)
        duplicate_projection_fields = sorted(set(source).intersection(PROJECTION_SPEC_FIELDS))
        spec = specs.get(source_key, {})
        for field in duplicate_projection_fields:
            if field not in spec or source[field] != spec[field]:
                raise validation_error(
                    error_cls,
                    f"sources.{source_key}.{field} must move to {specs_label} without drift",
                )
        stripped_source = {
            key: value for key, value in source.items() if key not in PROJECTION_SPEC_FIELDS
        }
        merged[source_key] = {**stripped_source, **spec}
    return merged


def compiled_architecture_sources(
    graph: dict[str, Any],
    graph_path: Path,
    projection_rows: dict[str, dict[str, Any]],
    projection_rows_label: str,
    policy: dict[str, Any],
    policy_path: Path,
    error_cls: type[ErrorT],
) -> dict[str, dict[str, Any]]:
    """Return source entries with policy-owned classification injected."""
    validate_architecture_graph_envelope(graph, graph_path, error_cls)
    validate_architecture_policy_envelope(policy, policy_path, error_cls)
    raw_sources = require_mapping(
        graph.get("sources"),
        f"{graph_path}.sources",
        error_cls,
    )
    merged_sources = merge_projection_rows(
        raw_sources,
        projection_rows,
        projection_rows_label,
        error_cls,
    )
    return classified_source_map(
        merged_sources,
        policy,
        policy_path.as_posix(),
        error_cls,
    )


def compiled_artifact_catalog(
    graph: dict[str, Any],
    graph_path: Path,
    sources: dict[str, Any],
    policy: dict[str, Any],
    error_cls: type[ErrorT],
) -> list[dict[str, Any]]:
    """Return generated-artifact metadata from the graph and artifact policy."""
    generated_artifacts = require_string_list(
        graph.get("generated_artifacts"),
        f"{graph_path}.generated_artifacts",
        error_cls,
    )
    return artifact_catalog_from_sources(
        sources,
        generated_artifacts,
        policy,
        error_cls,
    )


def architecture_graph_normal_form(
    sources: dict[str, dict[str, Any]],
    error_cls: type[ErrorT],
) -> dict[str, list[dict[str, Any]]]:
    """Return normalized node, projection-edge, and check tables."""
    nodes: list[dict[str, Any]] = []
    projection_edges: list[dict[str, Any]] = []
    projection_checks: list[dict[str, Any]] = []
    for source_key in sorted(sources):
        source = require_mapping(sources[source_key], f"sources.{source_key}", error_cls)
        nodes.append(
            {
                "id": source_key,
                "source_layer": require_string(
                    source.get("source_layer"),
                    f"sources.{source_key}.source_layer",
                    error_cls,
                ),
                "source_kind": require_string(
                    source.get("source_kind"),
                    f"sources.{source_key}.source_kind",
                    error_cls,
                ),
                "owner": require_string(
                    source.get("owner"),
                    f"sources.{source_key}.owner",
                    error_cls,
                ),
                "purpose": require_string(
                    source.get("purpose"),
                    f"sources.{source_key}.purpose",
                    error_cls,
                ),
            }
        )
        projection_id_value = source.get("projection_id")
        if projection_id_value is None:
            continue
        projection_id = require_string(
            projection_id_value,
            f"sources.{source_key}.projection_id",
            error_cls,
        )
        projection_edges.append(
            {
                "id": projection_id,
                "source": source_key,
                "projection_kind": require_string(
                    source.get("projection_kind"),
                    f"sources.{source_key}.projection_kind",
                    error_cls,
                ),
                "sources": resolve_projection_sources(source_key, source, sources, error_cls),
                "targets": generated_target_values(source_key, source, error_cls),
            }
        )
        if "projection_checks" not in source:
            continue
        for check in validate_projection_check_refs(
            source.get("projection_checks"),
            f"sources.{source_key}.projection_checks",
            error_cls,
        ):
            projection_checks.append(
                {
                    "projection_id": projection_id,
                    "source": source_key,
                    "group": require_string(
                        check.get("group"),
                        f"sources.{source_key}.projection_checks.group",
                        error_cls,
                    ),
                    "check_id": require_string(
                        check.get("check_id"),
                        f"sources.{source_key}.projection_checks.check_id",
                        error_cls,
                    ),
                }
            )
    return {
        "nodes": nodes,
        "projection_edges": projection_edges,
        "projection_checks": projection_checks,
    }


def compiled_architecture_document(
    graph: dict[str, Any],
    graph_path: Path,
    projection_rows: dict[str, dict[str, Any]],
    projection_model_sources: list[str],
    projection_compatibility_view: str,
    policy: dict[str, Any],
    policy_path: Path,
    error_cls: type[ErrorT],
) -> dict[str, Any]:
    """Return the persistent architecture IR used by all review projections."""
    sources = compiled_architecture_sources(
        graph,
        graph_path,
        projection_rows,
        "normal-form ProjectionRuntime ledgers",
        policy,
        policy_path,
        error_cls,
    )
    return {
        "schema_version": 1,
        "kind": COMPILED_ARCHITECTURE_KIND,
        "status": "active",
        "description": (
            "Compiled architecture IR derived from the graph source and "
            "architecture policy. Human review projections and CI projection "
            "graphs must derive from this file instead of recompiling the "
            "graph/policy pair independently."
        ),
        "generated_from": [
            graph_path.as_posix(),
            *projection_model_sources,
            policy_path.as_posix(),
        ],
        "source_model": f"{graph_path.as_posix()}#/sources",
        "projection_model": " + ".join(f"{source}#/" for source in projection_model_sources),
        "projection_compatibility_view": projection_compatibility_view,
        "compatibility_sidecars": {
            "projection_spec": projection_compatibility_view,
        },
        "policy_source": policy_path.as_posix(),
        "projection_role": "compiled-architecture-ir",
        "sources": sources,
        "architecture_normal_form": architecture_graph_normal_form(
            sources,
            error_cls,
        ),
        "artifact_catalog": compiled_artifact_catalog(
            graph,
            graph_path,
            sources,
            policy,
            error_cls,
        ),
        "policy_sections": require_list(
            policy.get("policy_sections"),
            f"{policy_path}.policy_sections",
            error_cls,
        ),
        "source_layers": require_mapping(
            policy.get("source_layers"),
            f"{policy_path}.source_layers",
            error_cls,
        ),
        "artifact_class_policy": require_mapping(
            policy.get("artifact_class_policy"),
            f"{policy_path}.artifact_class_policy",
            error_cls,
        ),
        "architecture_ir_policy": require_mapping(
            policy.get("architecture_ir_policy"),
            f"{policy_path}.architecture_ir_policy",
            error_cls,
        ),
        "operation_kernel_policy": require_mapping(
            policy.get("operation_kernel_policy"),
            f"{policy_path}.operation_kernel_policy",
            error_cls,
        ),
        "confidential_assurance_source_model": require_mapping(
            policy.get("confidential_assurance_source_model"),
            f"{policy_path}.confidential_assurance_source_model",
            error_cls,
        ),
        "pds_resource_family_pattern": require_mapping(
            policy.get("pds_resource_family_pattern"),
            f"{policy_path}.pds_resource_family_pattern",
            error_cls,
        ),
        "control_plane_data_plane_boundary": require_mapping(
            policy.get("control_plane_data_plane_boundary"),
            f"{policy_path}.control_plane_data_plane_boundary",
            error_cls,
        ),
        "workflow_entrypoint_policy": require_mapping(
            policy.get("workflow_entrypoint_policy"),
            f"{policy_path}.workflow_entrypoint_policy",
            error_cls,
        ),
        "http_adapter_boundary_policy": require_mapping(
            policy.get("http_adapter_boundary_policy"),
            f"{policy_path}.http_adapter_boundary_policy",
            error_cls,
        ),
        "architecture_domain_model": require_mapping(
            policy.get("architecture_domain_model"),
            f"{policy_path}.architecture_domain_model",
            error_cls,
        ),
    }


def validate_compiled_architecture_envelope(
    compiled: dict[str, Any],
    compiled_path: Path,
    error_cls: type[ErrorT],
) -> None:
    """Validate the common compiled architecture envelope."""
    if compiled.get("schema_version") != 1:
        raise validation_error(error_cls, f"{compiled_path}.schema_version must be 1")
    if compiled.get("kind") != COMPILED_ARCHITECTURE_KIND:
        raise validation_error(error_cls, f"{compiled_path}.kind must be {COMPILED_ARCHITECTURE_KIND}")
    if compiled.get("status") != "active":
        raise validation_error(error_cls, f"{compiled_path}.status must be active")


def compiled_architecture_sources_from_ir(
    compiled: dict[str, Any],
    compiled_path: Path,
    error_cls: type[ErrorT],
) -> dict[str, Any]:
    """Return compiled source entries from a persistent architecture IR."""
    validate_compiled_architecture_envelope(compiled, compiled_path, error_cls)
    return require_mapping(compiled.get("sources"), f"{compiled_path}.sources", error_cls)


def compiled_artifact_catalog_from_ir(
    compiled: dict[str, Any],
    compiled_path: Path,
    error_cls: type[ErrorT],
) -> list[Any]:
    """Return compiled artifact catalog entries from a persistent architecture IR."""
    validate_compiled_architecture_envelope(compiled, compiled_path, error_cls)
    artifact_catalog = compiled.get("artifact_catalog")
    if not isinstance(artifact_catalog, list):
        raise validation_error(error_cls, f"{compiled_path}.artifact_catalog must be a list")
    return artifact_catalog
