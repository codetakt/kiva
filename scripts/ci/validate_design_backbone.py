#!/usr/bin/env python3
"""Validate the Kiva architecture design-backbone ledger."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from kiva_ci.architecture import (
    SOURCE_KIND_BY_LAYER_FIELD,
    SOURCE_LAYER_FIELDS,
    validate_artifact_catalog_entries,
)
from kiva_ci.manifest import (
    generated_artifact_values,
    generated_target_values,
    load_json,
    load_yaml,
    relative_path,
    require_existing_path,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
    resolve_projection_sources,
    validate_projection_check_refs,
)
from kiva_ci.architecture_compiler import projection_spec_source_path
from kiva_ci.ledger import require_declared_count, rows_by_unique_string_field
from generate_design_backbone import DEFAULT_COMPILED_ARCHITECTURE, render_design_backbone


DEFAULT_MANIFEST = Path("spec/architecture/design-backbone.yaml")
DEFAULT_GRAPH_SOURCE = Path("spec/architecture/architecture-graph.source.yaml")
DEFAULT_PROJECTION_ARTIFACTS_SOURCE = Path("spec/architecture/projection-artifacts.source.yaml")
DEFAULT_PROJECTION_RUNTIME_SOURCE = Path("spec/architecture/projection-runtime.source.yaml")
DEFAULT_PROJECTION_CHECK_BINDINGS_SOURCE = Path("spec/architecture/projection-check-bindings.source.yaml")
DEFAULT_PROJECTION_SPEC_SOURCE = Path("spec/architecture/projection-specs.source.yaml")
DEFAULT_ARCHITECTURE_POLICY_SOURCE = Path("spec/architecture/architecture-policy.source.yaml")
DEFAULT_COMPILED_ARCHITECTURE_SOURCE = DEFAULT_COMPILED_ARCHITECTURE
PATH_LIST_FIELDS = (
    "artifacts",
    "generated_artifacts",
    "projection_sources",
    "projection_exports",
    "projection_targets",
)
POLICY_SECTION_OWNED_FIELDS = {
    "source_layers",
    "artifact_class_policy",
    "architecture_ir_policy",
    "operation_kernel_policy",
    "confidential_assurance_source_model",
    "pds_resource_family_pattern",
    "control_plane_data_plane_boundary",
    "workflow_entrypoint_policy",
    "http_adapter_boundary_policy",
    "architecture_domain_model",
}


class DesignBackboneError(RuntimeError):
    """Raised when design-backbone metadata drifts from repository state."""


def validate_architecture_graph_source(
    root: Path,
    document: dict[str, Any],
    manifest_path: Path,
) -> list[str]:
    """Validate that design-backbone remains a projection of compiled architecture IR."""
    generated_from = require_string_list(
        document.get("generated_from"),
        "generated_from",
        DesignBackboneError,
    )
    if generated_from != [DEFAULT_COMPILED_ARCHITECTURE_SOURCE.as_posix()]:
        raise DesignBackboneError(
            f"generated_from must be [{DEFAULT_COMPILED_ARCHITECTURE_SOURCE.as_posix()!r}]"
        )
    compiled = load_yaml(root / DEFAULT_COMPILED_ARCHITECTURE_SOURCE, DesignBackboneError)
    if compiled.get("schema_version") != 1:
        raise DesignBackboneError(f"{DEFAULT_COMPILED_ARCHITECTURE_SOURCE}.schema_version must be 1")
    if compiled.get("kind") != "kiva-compiled-architecture":
        raise DesignBackboneError(
            f"{DEFAULT_COMPILED_ARCHITECTURE_SOURCE}.kind must be kiva-compiled-architecture"
        )
    if compiled.get("status") != "active":
        raise DesignBackboneError(f"{DEFAULT_COMPILED_ARCHITECTURE_SOURCE}.status must be active")
    compiled_sources = require_string_list(
        compiled.get("generated_from"),
        f"{DEFAULT_COMPILED_ARCHITECTURE_SOURCE}.generated_from",
        DesignBackboneError,
    )
    if compiled_sources != [
        DEFAULT_GRAPH_SOURCE.as_posix(),
        DEFAULT_PROJECTION_ARTIFACTS_SOURCE.as_posix(),
        DEFAULT_PROJECTION_RUNTIME_SOURCE.as_posix(),
        DEFAULT_PROJECTION_CHECK_BINDINGS_SOURCE.as_posix(),
        DEFAULT_ARCHITECTURE_POLICY_SOURCE.as_posix(),
    ]:
        raise DesignBackboneError(
            f"{DEFAULT_COMPILED_ARCHITECTURE_SOURCE}.generated_from must identify graph, projection normal-form ledgers, and policy sources"
        )
    sidecars = require_mapping(
        compiled.get("compatibility_sidecars"),
        f"{DEFAULT_COMPILED_ARCHITECTURE_SOURCE}.compatibility_sidecars",
        DesignBackboneError,
    )
    if sidecars.get("projection_spec") != f"{DEFAULT_PROJECTION_SPEC_SOURCE.as_posix()}#/projections":
        raise DesignBackboneError(
            f"{DEFAULT_COMPILED_ARCHITECTURE_SOURCE}.compatibility_sidecars.projection_spec must reference ProjectionSpec as a sidecar"
        )
    graph = load_yaml(root / DEFAULT_GRAPH_SOURCE, DesignBackboneError)
    if graph.get("schema_version") != 1:
        raise DesignBackboneError(f"{DEFAULT_GRAPH_SOURCE}.schema_version must be 1")
    if graph.get("kind") != "kiva-architecture-graph-source":
        raise DesignBackboneError(
            f"{DEFAULT_GRAPH_SOURCE}.kind must be kiva-architecture-graph-source"
        )
    if graph.get("status") != "active":
        raise DesignBackboneError(f"{DEFAULT_GRAPH_SOURCE}.status must be active")
    generated_artifacts = [
        require_string(
            require_mapping(
                artifact,
                f"{DEFAULT_COMPILED_ARCHITECTURE_SOURCE}.artifact_catalog[]",
                DesignBackboneError,
            ).get("path"),
            f"{DEFAULT_COMPILED_ARCHITECTURE_SOURCE}.artifact_catalog[].path",
            DesignBackboneError,
        )
        for artifact in require_list(
            compiled.get("artifact_catalog"),
            f"{DEFAULT_COMPILED_ARCHITECTURE_SOURCE}.artifact_catalog",
            DesignBackboneError,
        )
    ]
    if manifest_path.as_posix() not in generated_artifacts:
        raise DesignBackboneError(
            f"{DEFAULT_GRAPH_SOURCE}.generated_artifacts must include {manifest_path}"
        )
    graph_sources = require_mapping(
        graph.get("sources"),
        f"{DEFAULT_GRAPH_SOURCE}.sources",
        DesignBackboneError,
    )
    for source_key, raw_source in graph_sources.items():
        source = require_mapping(
            raw_source,
            f"{DEFAULT_GRAPH_SOURCE}.sources.{source_key}",
            DesignBackboneError,
        )
        if "source_layer" in source or "source_kind" in source:
            raise DesignBackboneError(
                f"{DEFAULT_GRAPH_SOURCE}.sources.{source_key} must not declare "
                "source_layer/source_kind; classification is owned by architecture policy"
            )
    policy = require_mapping(
        graph.get("projection_policy"),
        f"{DEFAULT_GRAPH_SOURCE}.projection_policy",
        DesignBackboneError,
    )
    if policy.get("design_backbone_is_generated_review_projection") is not True:
        raise DesignBackboneError(
            f"{DEFAULT_GRAPH_SOURCE}.projection_policy.design_backbone_is_generated_review_projection must be true"
        )
    policy_source = require_string(
        policy.get("architecture_policy_source"),
        f"{DEFAULT_GRAPH_SOURCE}.projection_policy.architecture_policy_source",
        DesignBackboneError,
    )
    if policy_source != DEFAULT_ARCHITECTURE_POLICY_SOURCE.as_posix():
        raise DesignBackboneError(
            f"{DEFAULT_GRAPH_SOURCE}.projection_policy.architecture_policy_source "
            f"must be {DEFAULT_ARCHITECTURE_POLICY_SOURCE}"
        )
    projection_spec_source = projection_spec_source_path(
        graph,
        DEFAULT_GRAPH_SOURCE,
        DesignBackboneError,
    )
    if projection_spec_source != DEFAULT_PROJECTION_SPEC_SOURCE:
        raise DesignBackboneError(
            f"{DEFAULT_GRAPH_SOURCE}.projection_policy.projection_spec_source "
            f"must be {DEFAULT_PROJECTION_SPEC_SOURCE}"
        )
    projection_spec = load_yaml(root / DEFAULT_PROJECTION_SPEC_SOURCE, DesignBackboneError)
    if projection_spec.get("schema_version") != 1:
        raise DesignBackboneError(f"{DEFAULT_PROJECTION_SPEC_SOURCE}.schema_version must be 1")
    if projection_spec.get("kind") != "kiva-architecture-projection-spec-source":
        raise DesignBackboneError(
            f"{DEFAULT_PROJECTION_SPEC_SOURCE}.kind must be kiva-architecture-projection-spec-source"
        )
    if projection_spec.get("status") != "active":
        raise DesignBackboneError(f"{DEFAULT_PROJECTION_SPEC_SOURCE}.status must be active")
    if projection_spec.get("source_graph") != DEFAULT_GRAPH_SOURCE.as_posix():
        raise DesignBackboneError(
            f"{DEFAULT_PROJECTION_SPEC_SOURCE}.source_graph must be {DEFAULT_GRAPH_SOURCE}"
        )
    architecture_policy = load_yaml(root / DEFAULT_ARCHITECTURE_POLICY_SOURCE, DesignBackboneError)
    if architecture_policy.get("schema_version") != 1:
        raise DesignBackboneError(f"{DEFAULT_ARCHITECTURE_POLICY_SOURCE}.schema_version must be 1")
    if architecture_policy.get("kind") != "kiva-architecture-policy-source":
        raise DesignBackboneError(
            f"{DEFAULT_ARCHITECTURE_POLICY_SOURCE}.kind must be kiva-architecture-policy-source"
        )
    if architecture_policy.get("status") != "active":
        raise DesignBackboneError(f"{DEFAULT_ARCHITECTURE_POLICY_SOURCE}.status must be active")
    policy_includes = set(
        require_string_list(
            architecture_policy.get("includes", []),
            f"{DEFAULT_ARCHITECTURE_POLICY_SOURCE}.includes",
            DesignBackboneError,
            allow_empty=True,
        )
    )
    architecture_policy_source = require_mapping(
        graph_sources.get("architecture_policy"),
        f"{DEFAULT_GRAPH_SOURCE}.sources.architecture_policy",
        DesignBackboneError,
    )
    architecture_policy_artifacts = set(
        require_string_list(
            architecture_policy_source.get("artifacts"),
            f"{DEFAULT_GRAPH_SOURCE}.sources.architecture_policy.artifacts",
            DesignBackboneError,
        )
    )
    required_policy_artifacts = {
        DEFAULT_ARCHITECTURE_POLICY_SOURCE.as_posix(),
        *policy_includes,
    }
    missing_policy_artifacts = sorted(
        required_policy_artifacts.difference(architecture_policy_artifacts)
    )
    if missing_policy_artifacts:
        raise DesignBackboneError(
            "architecture_policy source artifacts must register every policy include: "
            + ", ".join(missing_policy_artifacts)
        )
    expected = render_design_backbone(root, DEFAULT_COMPILED_ARCHITECTURE_SOURCE)
    actual = (root / manifest_path).read_text(encoding="utf-8")
    if actual != expected:
        raise DesignBackboneError(
            f"{manifest_path} is not current; run scripts/ci/generate_design_backbone.py"
        )
    return [
        DEFAULT_GRAPH_SOURCE.as_posix(),
        DEFAULT_ARCHITECTURE_POLICY_SOURCE.as_posix(),
        *generated_artifacts,
    ]


def validate_policy_sections(document: dict[str, Any]) -> None:
    """Validate typed policy-section ownership before per-section checks run."""
    sections = rows_by_unique_string_field(
        document.get("policy_sections"),
        label="policy_sections",
        error_cls=DesignBackboneError,
        duplicate_label="policy section id",
    )
    owned_fields: set[str] = set()
    for section_id, section in sections.items():
        require_string(
            section.get("policy_kind"),
            f"policy_sections.{section_id}.policy_kind",
            DesignBackboneError,
        )
        owns = require_string(
            section.get("owns"),
            f"policy_sections.{section_id}.owns",
            DesignBackboneError,
        )
        if owns not in POLICY_SECTION_OWNED_FIELDS:
            raise DesignBackboneError(f"{section_id}.owns is invalid: {owns}")
        if owns in owned_fields:
            raise DesignBackboneError(f"policy field is owned by multiple sections: {owns}")
        owned_fields.add(owns)
        split_candidate = require_string(
            section.get("split_candidate"),
            f"policy_sections.{section_id}.split_candidate",
            DesignBackboneError,
        )
        if not split_candidate.startswith("spec/architecture/") or not split_candidate.endswith(".yaml"):
            raise DesignBackboneError(
                f"{section_id}.split_candidate must be a spec/architecture YAML path"
            )
    missing = sorted(POLICY_SECTION_OWNED_FIELDS.difference(owned_fields))
    if missing:
        raise DesignBackboneError(
            "policy_sections missing ownership for: " + ", ".join(missing)
        )


def validate_architecture_domain_model(document: dict[str, Any]) -> None:
    """Validate the source-domain reduction model."""
    model = require_mapping(
        document.get("architecture_domain_model"),
        "architecture_domain_model",
        DesignBackboneError,
    )
    root_type = require_string(
        model.get("root_type"),
        "architecture_domain_model.root_type",
        DesignBackboneError,
    )
    if root_type != "ArchitectureDomainModel":
        raise DesignBackboneError("architecture_domain_model.root_type must be ArchitectureDomainModel")
    domains = require_mapping(
        model.get("domains"),
        "architecture_domain_model.domains",
        DesignBackboneError,
    )
    require_declared_count(
        len(domains),
        model.get("domain_count"),
        label="architecture_domain_model.domain_count",
        error_cls=DesignBackboneError,
    )
    expected_domains = {"contracts", "operations", "assurance", "governance"}
    if set(domains) != expected_domains:
        raise DesignBackboneError(
            "architecture_domain_model.domains drift: "
            f"expected={sorted(expected_domains)}, actual={sorted(domains)}"
        )
    covered_layers: set[str] = set()
    for domain_id, raw_domain in domains.items():
        domain = require_mapping(
            raw_domain,
            f"architecture_domain_model.domains.{domain_id}",
            DesignBackboneError,
        )
        require_string(
            domain.get("description"),
            f"architecture_domain_model.domains.{domain_id}.description",
            DesignBackboneError,
        )
        layers = set(
            require_string_list(
                domain.get("source_layers"),
                f"architecture_domain_model.domains.{domain_id}.source_layers",
                DesignBackboneError,
            )
        )
        overlap = sorted(covered_layers.intersection(layers))
        if overlap:
            raise DesignBackboneError(
                f"architecture_domain_model source layers assigned more than once: {overlap}"
            )
        covered_layers.update(layers)
        require_string_list(
            domain.get("owns"),
            f"architecture_domain_model.domains.{domain_id}.owns",
            DesignBackboneError,
        )
    source_layers = set(
        require_mapping(
            document.get("source_layers"),
            "source_layers",
            DesignBackboneError,
        )
    )
    if covered_layers != source_layers:
        raise DesignBackboneError(
            "architecture_domain_model source layer coverage drift: "
            f"expected={sorted(source_layers)}, actual={sorted(covered_layers)}"
        )


def validate_architecture_ir_policy(root: Path, document: dict[str, Any]) -> None:
    """Validate that review and CI projections consume the compiled architecture IR."""
    policy = require_mapping(
        document.get("architecture_ir_policy"),
        "architecture_ir_policy",
        DesignBackboneError,
    )
    canonical_ir = require_string(
        policy.get("canonical_ir"),
        "architecture_ir_policy.canonical_ir",
        DesignBackboneError,
    )
    if canonical_ir != DEFAULT_COMPILED_ARCHITECTURE_SOURCE.as_posix():
        raise DesignBackboneError(
            f"architecture_ir_policy.canonical_ir must be {DEFAULT_COMPILED_ARCHITECTURE_SOURCE}"
        )
    compiler = require_string(
        policy.get("compiler"),
        "architecture_ir_policy.compiler",
        DesignBackboneError,
    )
    if compiler != "scripts/ci/generate_compiled_architecture.py":
        raise DesignBackboneError(
            "architecture_ir_policy.compiler must be scripts/ci/generate_compiled_architecture.py"
        )
    source_graph = require_string(
        policy.get("source_graph"),
        "architecture_ir_policy.source_graph",
        DesignBackboneError,
    )
    if source_graph != DEFAULT_GRAPH_SOURCE.as_posix():
        raise DesignBackboneError(
            f"architecture_ir_policy.source_graph must be {DEFAULT_GRAPH_SOURCE}"
        )
    projection_model_sources = require_string_list(
        policy.get("projection_model_sources"),
        "architecture_ir_policy.projection_model_sources",
        DesignBackboneError,
    )
    expected_projection_model_sources = [
        DEFAULT_PROJECTION_ARTIFACTS_SOURCE.as_posix(),
        DEFAULT_PROJECTION_RUNTIME_SOURCE.as_posix(),
        DEFAULT_PROJECTION_CHECK_BINDINGS_SOURCE.as_posix(),
    ]
    if projection_model_sources != expected_projection_model_sources:
        raise DesignBackboneError(
            "architecture_ir_policy.projection_model_sources must identify the "
            "ProjectionRuntime normal-form ledgers"
        )
    projection_compatibility_view = require_string(
        policy.get("projection_compatibility_view"),
        "architecture_ir_policy.projection_compatibility_view",
        DesignBackboneError,
    )
    if projection_compatibility_view != f"{DEFAULT_PROJECTION_SPEC_SOURCE.as_posix()}#/projections":
        raise DesignBackboneError(
            "architecture_ir_policy.projection_compatibility_view must reference "
            "ProjectionSpec as a sidecar"
        )
    policy_source = require_string(
        policy.get("policy_source"),
        "architecture_ir_policy.policy_source",
        DesignBackboneError,
    )
    if policy_source != DEFAULT_ARCHITECTURE_POLICY_SOURCE.as_posix():
        raise DesignBackboneError(
            f"architecture_ir_policy.policy_source must be {DEFAULT_ARCHITECTURE_POLICY_SOURCE}"
        )
    for path in (canonical_ir, compiler, source_graph, *projection_model_sources, policy_source):
        require_existing_path(root, path, "architecture_ir_policy", DesignBackboneError)
    require_existing_path(
        root,
        projection_compatibility_view.split("#", maxsplit=1)[0],
        "architecture_ir_policy.projection_compatibility_view",
        DesignBackboneError,
    )

    downstream = set(
        require_string_list(
            policy.get("required_downstream_projections"),
            "architecture_ir_policy.required_downstream_projections",
            DesignBackboneError,
        )
    )
    expected_downstream = {
        DEFAULT_MANIFEST.as_posix(),
        "spec/ci/projection-graph.yaml",
        "generated/claim-readiness-matrix.yaml",
        "docs/architecture/generated/claim-readiness-matrix.md",
    }
    if downstream != expected_downstream:
        raise DesignBackboneError(
            "architecture_ir_policy.required_downstream_projections drift: "
            f"expected={sorted(expected_downstream)}, actual={sorted(downstream)}"
        )
    for path in downstream:
        require_existing_path(
            root,
            path,
            "architecture_ir_policy.required_downstream_projections",
            DesignBackboneError,
        )
    for field in require_string_list(
        policy.get("generated_model_fields"),
        "architecture_ir_policy.generated_model_fields",
        DesignBackboneError,
    ):
        if field not in document:
            raise DesignBackboneError(
                f"architecture_ir_policy.generated_model_fields references missing field: {field}"
            )
    require_string(policy.get("rule"), "architecture_ir_policy.rule", DesignBackboneError)


def validate_operation_kernel_policy(root: Path, document: dict[str, Any]) -> None:
    """Validate OperationKernel as Kiva's top-level public operation model."""
    policy = require_mapping(
        document.get("operation_kernel_policy"),
        "operation_kernel_policy",
        DesignBackboneError,
    )
    root_model = require_string(
        policy.get("root_model"),
        "operation_kernel_policy.root_model",
        DesignBackboneError,
    )
    if root_model != "OperationKernel":
        raise DesignBackboneError("operation_kernel_policy.root_model must be OperationKernel")
    source_root_model = require_string(
        policy.get("source_root_model"),
        "operation_kernel_policy.source_root_model",
        DesignBackboneError,
    )
    if source_root_model != "SurfaceOperation":
        raise DesignBackboneError(
            "operation_kernel_policy.source_root_model must be SurfaceOperation"
        )
    source_model = require_string(
        policy.get("source_model"),
        "operation_kernel_policy.source_model",
        DesignBackboneError,
    )
    if source_model != "spec/architecture/surface-model.source.yaml":
        raise DesignBackboneError(
            "operation_kernel_policy.source_model must be spec/architecture/surface-model.source.yaml"
        )
    surface_model = load_yaml(root / source_model, DesignBackboneError)
    algebra = require_mapping(
        surface_model.get("surface_operation_algebra"),
        f"{source_model}.surface_operation_algebra",
        DesignBackboneError,
    )
    if algebra.get("sole_operation_algebra") is not True:
        raise DesignBackboneError(
            f"{source_model}.surface_operation_algebra.sole_operation_algebra must be true"
        )
    if algebra.get("root_type") != source_root_model:
        raise DesignBackboneError(
            f"{source_model}.surface_operation_algebra.root_type must match operation kernel source root"
        )

    generated_catalog = require_string(
        policy.get("generated_catalog"),
        "operation_kernel_policy.generated_catalog",
        DesignBackboneError,
    )
    generated_kernel = require_string(
        policy.get("generated_kernel"),
        "operation_kernel_policy.generated_kernel",
        DesignBackboneError,
    )
    expected_targets = {
        "generated/architecture/operation-kernel.yaml",
        "docs/architecture/generated/operation-kernel.md",
        "generated/architecture/operation-catalog.yaml",
        "generated/architecture/pds-resource-families.yaml",
        "generated/architecture/surface-families.yaml",
        "docs/architecture/generated/surface-operation-index.md",
        "generated/surface-operation-index.evidence.json",
        "docs/architecture/generated/route-coverage.md",
        "generated/route-coverage.evidence.json",
        "crates/kiva-server/src/core/state/generated_surface_profiles.rs",
    }
    projection_targets = set(
        require_string_list(
            policy.get("projection_targets"),
            "operation_kernel_policy.projection_targets",
            DesignBackboneError,
        )
    )
    if projection_targets != expected_targets:
        raise DesignBackboneError(
            "operation_kernel_policy.projection_targets drift: "
            f"expected={sorted(expected_targets)}, actual={sorted(projection_targets)}"
        )
    for path in (source_model, generated_catalog, generated_kernel, *projection_targets):
        require_existing_path(root, path, "operation_kernel_policy", DesignBackboneError)

    runtime_bindings = require_mapping(
        policy.get("runtime_bindings"),
        "operation_kernel_policy.runtime_bindings",
        DesignBackboneError,
    )
    for field in ("application_registry", "server_surface_profile_registry"):
        require_existing_path(
            root,
            require_string(
                runtime_bindings.get(field),
                f"operation_kernel_policy.runtime_bindings.{field}",
                DesignBackboneError,
            ),
            f"operation_kernel_policy.runtime_bindings.{field}",
            DesignBackboneError,
        )
    require_string(policy.get("rule"), "operation_kernel_policy.rule", DesignBackboneError)


def validate_source_paths(root: Path, source_key: str, source: dict[str, Any]) -> list[str]:
    """Validate source path fields and return checked path strings."""
    checked: list[str] = []
    for field in PATH_LIST_FIELDS:
        if field not in source:
            continue
        for raw_path in require_string_list(
            source.get(field),
            f"{source_key}.{field}",
            DesignBackboneError,
        ):
            require_existing_path(root, raw_path, f"{source_key}.{field}", DesignBackboneError)
            checked.append(raw_path)

    for field in ("validator", "projection_generator", "bundle", "index"):
        if field not in source:
            continue
        raw_path = require_string(source.get(field), f"{source_key}.{field}", DesignBackboneError)
        require_existing_path(root, raw_path, f"{source_key}.{field}", DesignBackboneError)
        checked.append(raw_path)
    return checked


def validate_projection_metadata(
    source_key: str,
    source: dict[str, Any],
    sources: dict[str, Any],
) -> list[str]:
    """Validate projection graph metadata declared on one source entry."""
    has_projection_checks = "projection_checks" in source
    has_generated_targets = any(
        field in source for field in ("generated_artifacts", "projection_targets", "bundle", "index")
    )
    if has_projection_checks:
        for field in ("projection_id", "projection_kind"):
            if field not in source:
                raise DesignBackboneError(
                    f"sources.{source_key}.{field} is required when projection_checks is set"
                )
        require_string(
            source.get("projection_id"),
            f"sources.{source_key}.projection_id",
            DesignBackboneError,
        )
        require_string(
            source.get("projection_kind"),
            f"sources.{source_key}.projection_kind",
            DesignBackboneError,
        )
        resolved_sources = resolve_projection_sources(
            source_key,
            source,
            sources,
            DesignBackboneError,
        )
        if not has_generated_targets:
            raise DesignBackboneError(
                f"sources.{source_key} declares projection_checks but no generated target field"
            )
        validate_projection_check_refs(
            source.get("projection_checks"),
            f"sources.{source_key}.projection_checks",
            DesignBackboneError,
        )
        return resolved_sources
    return []


def validate_sources(root: Path, document: dict[str, Any]) -> list[str]:
    """Validate the source ownership map."""
    sources = require_mapping(document.get("sources"), "sources", DesignBackboneError)
    if not sources:
        raise DesignBackboneError("sources must not be empty")

    checked: list[str] = []
    for source_key, source_value in sources.items():
        source = require_mapping(source_value, f"sources.{source_key}", DesignBackboneError)
        require_string(
            source.get("source_layer"),
            f"sources.{source_key}.source_layer",
            DesignBackboneError,
        )
        source_kind = require_string(
            source.get("source_kind"),
            f"sources.{source_key}.source_kind",
            DesignBackboneError,
        )
        if source_kind not in set(SOURCE_KIND_BY_LAYER_FIELD.values()):
            raise DesignBackboneError(
                f"sources.{source_key}.source_kind is invalid: {source_kind}"
            )
        require_string(source.get("owner"), f"sources.{source_key}.owner", DesignBackboneError)
        require_string(
            source.get("purpose"),
            f"sources.{source_key}.purpose",
            DesignBackboneError,
        )
        resolved_sources = validate_projection_metadata(source_key, source, sources)
        checked.extend(validate_source_paths(root, source_key, source))
        for raw_path in resolved_sources:
            require_existing_path(
                root,
                raw_path,
                f"sources.{source_key}.resolved_projection_sources",
                DesignBackboneError,
            )
            checked.append(raw_path)

    generated_targets: set[str] = set()
    canonical_artifacts: set[str] = set()
    for source_key, source_value in sources.items():
        source = require_mapping(source_value, f"sources.{source_key}", DesignBackboneError)
        source_kind = require_string(
            source.get("source_kind"),
            f"sources.{source_key}.source_kind",
            DesignBackboneError,
        )
        if source_kind == "canonical" and "artifacts" in source:
            canonical_artifacts.update(
                require_string_list(
                    source.get("artifacts"),
                    f"sources.{source_key}.artifacts",
                    DesignBackboneError,
                )
            )
        generated_targets.update(
            generated_artifact_values(source_key, source, DesignBackboneError)
        )
    generated_canonical_overlap = sorted(canonical_artifacts.intersection(generated_targets))
    if generated_canonical_overlap:
        raise DesignBackboneError(
            "generated targets must not be listed as canonical artifacts: "
            + ", ".join(generated_canonical_overlap)
        )

    generated = require_mapping(
        sources.get("generated_schema_artifacts"),
        "sources.generated_schema_artifacts",
        DesignBackboneError,
    )
    strategy = require_string(
        generated.get("strategy"),
        "sources.generated_schema_artifacts.strategy",
        DesignBackboneError,
    )

    bundle = require_string(
        generated.get("bundle"),
        "sources.generated_schema_artifacts.bundle",
        DesignBackboneError,
    )
    index = require_string(
        generated.get("index"),
        "sources.generated_schema_artifacts.index",
        DesignBackboneError,
    )
    index_document = load_json(
        root / relative_path(index, "generated_schema_artifacts.index", DesignBackboneError),
        DesignBackboneError,
    )
    if index_document.get("artifact_strategy") != strategy:
        raise DesignBackboneError(
            f"{index} artifact_strategy must match design-backbone strategy: {strategy}"
        )
    if index_document.get("bundle") != Path(bundle).name:
        raise DesignBackboneError(
            f"{index} bundle must match design-backbone bundle basename: {bundle}"
        )
    return checked


def validate_artifact_catalog(
    root: Path,
    document: dict[str, Any],
    expected: list[Any],
) -> list[str]:
    """Validate typed generated-artifact metadata copied from compiled IR."""
    actual = require_list(
        document.get("artifact_catalog"),
        "artifact_catalog",
        DesignBackboneError,
    )
    if actual != expected:
        raise DesignBackboneError(
            "artifact_catalog is not current with compiled architecture IR; "
            "run scripts/ci/generate_compiled_architecture.py and scripts/ci/generate_design_backbone.py"
        )
    return validate_artifact_catalog_entries(root, actual, "artifact_catalog", DesignBackboneError)


def validate_source_layers(root: Path, document: dict[str, Any]) -> None:
    """Validate the source-of-truth hierarchy across source layers."""
    layers = require_mapping(document.get("source_layers"), "source_layers", DesignBackboneError)
    sources = require_mapping(document.get("sources"), "sources", DesignBackboneError)
    if not layers:
        raise DesignBackboneError("source_layers must not be empty")
    classified_sources: dict[str, str] = {}
    for layer_id in layers:
        layer = require_mapping(layers.get(layer_id), f"source_layers.{layer_id}", DesignBackboneError)
        require_string(layer.get("description"), f"source_layers.{layer_id}.description", DesignBackboneError)
        for field in SOURCE_LAYER_FIELDS:
            actual_values = require_string_list(
                layer.get(field),
                f"source_layers.{layer_id}.{field}",
                DesignBackboneError,
                allow_empty=True,
            )
            for value in actual_values:
                if value in sources:
                    previous_layer = classified_sources.setdefault(value, layer_id)
                    if previous_layer != layer_id:
                        raise DesignBackboneError(
                            f"{value} is classified by multiple source layers: "
                            f"{previous_layer}, {layer_id}"
                        )
                    source_layer = require_string(
                        sources[value].get("source_layer"),
                        f"sources.{value}.source_layer",
                        DesignBackboneError,
                    )
                    if source_layer != layer_id:
                        raise DesignBackboneError(
                            f"source_layers.{layer_id}.{field} references {value}, "
                            f"but sources.{value}.source_layer is {source_layer}"
                        )
                    source_kind = require_string(
                        sources[value].get("source_kind"),
                        f"sources.{value}.source_kind",
                        DesignBackboneError,
                    )
                    expected_kind = SOURCE_KIND_BY_LAYER_FIELD[field]
                    if source_kind != expected_kind:
                        raise DesignBackboneError(
                            f"source_layers.{layer_id}.{field} references {value}, "
                            f"but sources.{value}.source_kind is {source_kind}; "
                            f"expected {expected_kind}"
                        )
                else:
                    require_existing_path(
                        root,
                        value,
                        f"source_layers.{layer_id}.{field}",
                        DesignBackboneError,
                    )
    missing = sorted(set(sources).difference(classified_sources))
    if missing:
        raise DesignBackboneError(
            "sources are not classified by source_layers: " + ", ".join(missing)
        )


def validate_confidential_assurance_source_model(
    root: Path,
    manifest_path: Path,
    document: dict[str, Any],
) -> None:
    """Validate the confidential assurance one-way source model."""
    model = require_mapping(
        document.get("confidential_assurance_source_model"),
        "confidential_assurance_source_model",
        DesignBackboneError,
    )
    package = require_string(
        model.get("package"),
        "confidential_assurance_source_model.package",
        DesignBackboneError,
    )
    package_path = require_existing_path(
        root,
        package,
        "confidential_assurance_source_model.package",
        DesignBackboneError,
    )
    package_document = load_yaml(root / package_path, DesignBackboneError)
    expected_ref = f"{manifest_path}#/confidential_assurance_source_model"
    actual_ref = require_string(
        package_document.get("assurance_source_model_ref"),
        f"{package}.assurance_source_model_ref",
        DesignBackboneError,
    )
    if actual_ref != expected_ref:
        raise DesignBackboneError(
            f"{package}.assurance_source_model_ref must point to {expected_ref}"
        )
    for key in ("package_role", "evidence_registry_role", "verified_kernel_ledger_role"):
        require_string(
            model.get(key),
            f"confidential_assurance_source_model.{key}",
            DesignBackboneError,
        )
    for key in ("evidence_registry", "verified_kernel_ledger"):
        raw_path = require_string(
            model.get(key),
            f"confidential_assurance_source_model.{key}",
            DesignBackboneError,
        )
        require_existing_path(
            root,
            raw_path,
            f"confidential_assurance_source_model.{key}",
            DesignBackboneError,
        )
    require_string_list(
        model.get("reference_direction"),
        "confidential_assurance_source_model.reference_direction",
        DesignBackboneError,
    )
    require_string(
        model.get("typed_evidence_owner"),
        "confidential_assurance_source_model.typed_evidence_owner",
        DesignBackboneError,
    )
    require_string(
        model.get("verification_boundary_owner"),
        "confidential_assurance_source_model.verification_boundary_owner",
        DesignBackboneError,
    )


def _read_required_text(root: Path, raw_path: str, label: str) -> str:
    """Return required repository text for a design-backbone path."""
    path = require_existing_path(root, raw_path, label, DesignBackboneError)
    return path.read_text(encoding="utf-8")


def validate_pds_domain_event_flow(root: Path, pattern: dict[str, Any]) -> None:
    """Validate the PDS command-event-ledger projection model."""
    flow = require_mapping(
        pattern.get("domain_event_flow"),
        "pds_resource_family_pattern.domain_event_flow",
        DesignBackboneError,
    )
    root_type = require_string(
        flow.get("root_type"),
        "pds_resource_family_pattern.domain_event_flow.root_type",
        DesignBackboneError,
    )
    if root_type != "PdsDomainEventFlow":
        raise DesignBackboneError("pds_resource_family_pattern.domain_event_flow.root_type drift")
    expression = require_string(
        flow.get("flow_expression"),
        "pds_resource_family_pattern.domain_event_flow.flow_expression",
        DesignBackboneError,
    )
    for token in ("Command", "PdsDomainEvent", "ledger", "projection"):
        if token not in expression:
            raise DesignBackboneError(
                "pds_resource_family_pattern.domain_event_flow.flow_expression "
                f"must mention {token}"
            )

    texts = {
        key: _read_required_text(
            root,
            require_string(
                flow.get(key),
                f"pds_resource_family_pattern.domain_event_flow.{key}",
                DesignBackboneError,
            ),
            f"pds_resource_family_pattern.domain_event_flow.{key}",
        )
        for key in (
            "event_enum",
            "admission_owner",
            "write_port_owner",
            "projector_owner",
            "projection_owner",
            "ledger_owner",
        )
    }
    ledger_anchor = require_string(
        flow.get("ledger_projection_anchor"),
        "pds_resource_family_pattern.domain_event_flow.ledger_projection_anchor",
        DesignBackboneError,
    )
    if ledger_anchor != "PdsLedgerEventState":
        raise DesignBackboneError("PDS domain event flow must anchor on PdsLedgerEventState")
    for owner in ("projection_owner", "ledger_owner"):
        if ledger_anchor not in texts[owner]:
            raise DesignBackboneError(
                f"pds_resource_family_pattern.domain_event_flow.{owner} "
                f"must contain {ledger_anchor}"
            )
    if "PdsDomainEventAdmissionPolicy" not in texts["admission_owner"]:
        raise DesignBackboneError("PDS domain event flow admission owner must own admission policy")
    if "fn append_domain_event" not in texts["write_port_owner"]:
        raise DesignBackboneError("PDS domain event flow write port must expose append_domain_event")
    if "match event.into_inner()" not in texts["projector_owner"]:
        raise DesignBackboneError("PDS domain event projector must dispatch admitted events")

    events = require_list(
        flow.get("events"),
        "pds_resource_family_pattern.domain_event_flow.events",
        DesignBackboneError,
    )
    variant_count = 0
    seen_families: set[str] = set()
    seen_variants: set[str] = set()
    for index, raw_family in enumerate(events):
        family = require_mapping(
            raw_family,
            f"pds_resource_family_pattern.domain_event_flow.events[{index}]",
            DesignBackboneError,
        )
        family_id = require_string(
            family.get("family"),
            f"pds_resource_family_pattern.domain_event_flow.events[{index}].family",
            DesignBackboneError,
        )
        if family_id in seen_families:
            raise DesignBackboneError(f"PDS domain event flow duplicates family {family_id}")
        seen_families.add(family_id)
        variants = require_string_list(
            family.get("variants"),
            f"pds_resource_family_pattern.domain_event_flow.events[{index}].variants",
            DesignBackboneError,
        )
        projector_methods = require_string_list(
            family.get("projector_methods"),
            f"pds_resource_family_pattern.domain_event_flow.events[{index}].projector_methods",
            DesignBackboneError,
        )
        ledger_kinds = require_string_list(
            family.get("ledger_kinds"),
            f"pds_resource_family_pattern.domain_event_flow.events[{index}].ledger_kinds",
            DesignBackboneError,
        )
        if not projector_methods or not ledger_kinds:
            raise DesignBackboneError(f"PDS domain event flow family {family_id} has empty projection")
        for variant in variants:
            if variant in seen_variants:
                raise DesignBackboneError(f"PDS domain event flow duplicates variant {variant}")
            seen_variants.add(variant)
            variant_count += 1
            if f"{variant}(" not in texts["event_enum"]:
                raise DesignBackboneError(f"PdsDomainEvent enum is missing variant {variant}")
            if f"PdsDomainEvent::{variant}" not in texts["projector_owner"]:
                raise DesignBackboneError(f"PDS domain event projector is missing variant {variant}")
        for method in projector_methods:
            if method not in texts["projector_owner"]:
                raise DesignBackboneError(f"PDS domain event projector is missing method {method}")
        for ledger_kind in ledger_kinds:
            if ledger_kind not in texts["ledger_owner"]:
                raise DesignBackboneError(f"PDS ledger state is missing kind {ledger_kind}")

    declared_count = flow.get("event_count")
    if declared_count != variant_count:
        raise DesignBackboneError(
            "pds_resource_family_pattern.domain_event_flow.event_count "
            f"drift: declared={declared_count} actual={variant_count}"
        )
    excluded = require_list(
        flow.get("excluded_ledger_only_events"),
        "pds_resource_family_pattern.domain_event_flow.excluded_ledger_only_events",
        DesignBackboneError,
    )
    if not any(
        isinstance(entry, dict) and entry.get("kind") == "AccessDecisionRecorded"
        for entry in excluded
    ):
        raise DesignBackboneError(
            "PDS domain event flow must explicitly classify AccessDecisionRecorded as ledger-only"
        )


def validate_resource_pattern(root: Path, document: dict[str, Any]) -> None:
    """Validate the canonical PDS resource-family dependency path."""
    pattern = require_mapping(
        document.get("pds_resource_family_pattern"),
        "pds_resource_family_pattern",
        DesignBackboneError,
    )
    path = require_string_list(
        pattern.get("path"),
        "pds_resource_family_pattern.path",
        DesignBackboneError,
    )
    if path[0] != "OpenAPI DTO" or path[-1] != "evidence and boundary anchors":
        raise DesignBackboneError(
            "pds_resource_family_pattern.path must start at OpenAPI DTO "
            "and end at evidence and boundary anchors"
        )
    validate_pds_domain_event_flow(root, pattern)


def validate_control_data_plane_boundary(document: dict[str, Any]) -> None:
    """Validate control/data-plane ownership statements."""
    boundary = require_mapping(
        document.get("control_plane_data_plane_boundary"),
        "control_plane_data_plane_boundary",
        DesignBackboneError,
    )
    control_plane = require_string_list(
        boundary.get("control_plane_owned_by_kiva"),
        "control_plane_data_plane_boundary.control_plane_owned_by_kiva",
        DesignBackboneError,
    )
    data_plane = require_string_list(
        boundary.get("data_plane_not_owned_by_kiva_by_default"),
        "control_plane_data_plane_boundary.data_plane_not_owned_by_kiva_by_default",
        DesignBackboneError,
    )
    overlap = sorted({item.lower() for item in control_plane}.intersection(
        item.lower() for item in data_plane
    ))
    if overlap:
        raise DesignBackboneError(
            "control/data-plane ownership entries must be disjoint: "
            + ", ".join(overlap)
        )


def validate_workflow_entrypoint(document: dict[str, Any]) -> None:
    """Validate the preferred application workflow entrypoint."""
    policy = require_mapping(
        document.get("workflow_entrypoint_policy"),
        "workflow_entrypoint_policy",
        DesignBackboneError,
    )
    entrypoint = require_string(
        policy.get("preferred_entrypoint"),
        "workflow_entrypoint_policy.preferred_entrypoint",
        DesignBackboneError,
    )
    direction = require_string(
        policy.get("direction"),
        "workflow_entrypoint_policy.direction",
        DesignBackboneError,
    )
    if entrypoint not in direction:
        raise DesignBackboneError(
            "workflow_entrypoint_policy.direction must mention the preferred entrypoint"
        )


def validate_http_adapter_boundary(root: Path, document: dict[str, Any]) -> None:
    """Validate the HTTP adapter dependency and ownership policy."""
    policy = require_mapping(
        document.get("http_adapter_boundary_policy"),
        "http_adapter_boundary_policy",
        DesignBackboneError,
    )
    root_path = require_string(
        policy.get("root_path"),
        "http_adapter_boundary_policy.root_path",
        DesignBackboneError,
    )
    if root_path != "crates/kiva-server/src/surfaces":
        raise DesignBackboneError(
            "http_adapter_boundary_policy.root_path must be crates/kiva-server/src/surfaces"
        )
    require_existing_path(
        root,
        root_path,
        "http_adapter_boundary_policy.root_path",
        DesignBackboneError,
    )
    dependency_path = require_string(
        policy.get("dependency_path"),
        "http_adapter_boundary_policy.dependency_path",
        DesignBackboneError,
    )
    expected_dependency_path = (
        "HTTP surface -> OpenAPI DTO -> application command/query -> port -> repository -> storage"
    )
    if dependency_path != expected_dependency_path:
        raise DesignBackboneError(
            "http_adapter_boundary_policy.dependency_path must preserve the adapter dependency order"
        )
    handler_rule = require_string(
        policy.get("handler_rule"),
        "http_adapter_boundary_policy.handler_rule",
        DesignBackboneError,
    ).lower()
    if "handler" not in handler_rule or "persistence" not in handler_rule:
        raise DesignBackboneError(
            "http_adapter_boundary_policy.handler_rule must constrain handlers and persistence"
        )
    conversion_rule = require_string(
        policy.get("conversion_rule"),
        "http_adapter_boundary_policy.conversion_rule",
        DesignBackboneError,
    ).lower()
    if "request" not in conversion_rule or "response" not in conversion_rule:
        raise DesignBackboneError(
            "http_adapter_boundary_policy.conversion_rule must mention request and response conversion"
        )
    if "generated contract" not in conversion_rule and "openapi" not in conversion_rule:
        raise DesignBackboneError(
            "http_adapter_boundary_policy.conversion_rule must anchor OpenAPI/generated contract DTOs"
        )
    repository_rule = require_string(
        policy.get("repository_rule"),
        "http_adapter_boundary_policy.repository_rule",
        DesignBackboneError,
    ).lower()
    if "repository" not in repository_rule or "storage" not in repository_rule:
        raise DesignBackboneError(
            "http_adapter_boundary_policy.repository_rule must mention repository and storage"
        )
    require_string(
        policy.get("future_split_rule"),
        "http_adapter_boundary_policy.future_split_rule",
        DesignBackboneError,
    )
    markers = require_string_list(
        policy.get("thin_handler_markers"),
        "http_adapter_boundary_policy.thin_handler_markers",
        DesignBackboneError,
    )
    for marker in (
        "parse_pds_openapi_body",
        "statement_body_from_openapi_contract",
        "request_models_from_contract",
    ):
        if marker not in markers:
            raise DesignBackboneError(
                f"http_adapter_boundary_policy.thin_handler_markers must include {marker}"
            )
    adapter_roots = require_string_list(
        policy.get("adapter_roots"),
        "http_adapter_boundary_policy.adapter_roots",
        DesignBackboneError,
    )
    expected_roots = {
        "crates/kiva-server/src/surfaces/xapi",
        "crates/kiva-server/src/surfaces/pds",
        "crates/kiva-server/src/surfaces/confidential",
    }
    if set(adapter_roots) != expected_roots:
        raise DesignBackboneError(
            "http_adapter_boundary_policy.adapter_roots must enumerate xapi, pds, and confidential"
        )
    for adapter_root in adapter_roots:
        require_existing_path(
            root,
            adapter_root,
            "http_adapter_boundary_policy.adapter_roots",
            DesignBackboneError,
        )


def validate_source_group_registration(root: Path) -> None:
    """Validate that the backbone and validator are guarded by source groups."""
    source_group_path = root / "spec/architecture/boundaries/source-groups/ci-and-ledgers.yaml"
    source_groups = load_yaml(source_group_path, DesignBackboneError)
    text = source_group_path.read_text(encoding="utf-8")
    for needle in (
        "architecture-design-backbone",
        "spec/architecture/architecture-graph.source.yaml",
        "spec/architecture/architecture-policy.source.yaml",
        "spec/architecture/design-backbone.yaml",
        "scripts/ci/generate_design_backbone.py",
        "scripts/ci/validate_design_backbone.py",
        "generate_design_backbone.py",
        "validate_design_backbone.py",
    ):
        if needle not in text:
            raise DesignBackboneError(
                f"{source_group_path} must register design-backbone guard text: {needle}"
            )
    groups = source_groups.get("source_groups")
    if not isinstance(groups, list):
        raise DesignBackboneError(f"{source_group_path}.source_groups must be a list")
    if not any(
        isinstance(group, dict) and group.get("id") == "architecture-design-backbone"
        for group in groups
    ):
        raise DesignBackboneError(
            f"{source_group_path} missing architecture-design-backbone source group"
        )


def validate_design_backbone(root: Path, manifest_path: Path) -> dict[str, Any]:
    """Validate a design-backbone manifest and return a summary."""
    document = load_yaml(root / manifest_path, DesignBackboneError)
    if document.get("schema_version") != 1:
        raise DesignBackboneError("schema_version must be 1")
    if document.get("kind") != "kiva-architecture-design-backbone":
        raise DesignBackboneError("kind must be kiva-architecture-design-backbone")
    if document.get("status") != "active":
        raise DesignBackboneError("status must be active")
    require_string(document.get("description"), "description", DesignBackboneError)

    checked_paths = validate_architecture_graph_source(root, document, manifest_path)
    compiled = load_yaml(root / DEFAULT_COMPILED_ARCHITECTURE_SOURCE, DesignBackboneError)
    compiled_artifact_catalog = require_list(
        compiled.get("artifact_catalog"),
        f"{DEFAULT_COMPILED_ARCHITECTURE_SOURCE}.artifact_catalog",
        DesignBackboneError,
    )
    checked_paths.extend(validate_sources(root, document))
    checked_paths.extend(validate_artifact_catalog(root, document, compiled_artifact_catalog))
    validate_policy_sections(document)
    validate_architecture_ir_policy(root, document)
    validate_operation_kernel_policy(root, document)
    validate_architecture_domain_model(document)
    validate_source_layers(root, document)
    validate_confidential_assurance_source_model(root, manifest_path, document)
    validate_resource_pattern(root, document)
    validate_control_data_plane_boundary(document)
    validate_workflow_entrypoint(document)
    validate_http_adapter_boundary(root, document)
    validate_source_group_registration(root)

    return {
        "manifest": str(manifest_path),
        "kind": document["kind"],
        "source_count": len(document["sources"]),
        "checked_paths": sorted(set(checked_paths)),
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
        help="Design-backbone manifest path relative to --root.",
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
    manifest_path = relative_path(str(args.manifest), "--manifest", DesignBackboneError)
    try:
        report = validate_design_backbone(root, manifest_path)
    except DesignBackboneError as error:
        print(f"design-backbone validation failed: {error}", file=sys.stderr)
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
