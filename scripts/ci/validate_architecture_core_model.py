#!/usr/bin/env python3
"""Validate Kiva's minimal architecture core model."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.evidence import load_evidence_check_manifest
from kiva_ci.manifest import load_yaml, require_bool, require_list, require_mapping, require_string, require_string_list, validation_error
from validate_ci_command_registry import validate_ci_command_registry
from validate_ledger_catalog import validate_design_packages, validate_design_universe, validate_ledger_catalog
from validate_pds_core_planes import validate_pds_core_planes
from validate_pds_invariants import PdsInvariantError, validate_pds_invariants
from validate_pds_state_machines import PdsStateMachineError, validate_pds_state_machines
from validate_secret_compute_provider_tck import SecretComputeProviderTckError, validate_tck


DEFAULT_SOURCE = Path("spec/architecture/core-model.source.yaml")
DEFAULT_EVIDENCE_MANIFEST = Path("spec/ci/evidence-checks.yaml")
PRIMARY_MODEL_ROOT_TYPES = {
    "http-contract": "OpenAPI",
    "operation": "OperationKernel",
    "assurance": "AssurancePrimitive",
    "confidential-capability": "CapabilityPort",
}
SUPPORTING_MODEL_ROOT_TYPES = {
    "projection": "ProjectionRuntime",
    "ci-command": "RegistryDrivenCommand",
    "pds": "PdsCorePlane",
    "verification": "BoundaryProperty",
}
DESIGN_ENTRYPOINT_ORDER = [
    "spec/architecture/design-universe.source.yaml",
    "spec/architecture/core-model.source.yaml",
    "spec/architecture/design-source-nodes.source.yaml",
    "spec/architecture/design-package-rows.source.yaml",
    "spec/architecture/design-ledger-rows.source.yaml",
    "spec/openapi/kiva-analytics.yaml",
    "spec/openapi/kiva-xapi.yaml",
    "spec/openapi/kiva-pds.yaml",
    "spec/openapi/kiva-confidential.yaml",
    "spec/architecture/operation-kernel.source.yaml",
    "spec/evidence/assurance-primitive.source.yaml",
    "spec/confidential/mechanism-packages.source.yaml",
    "spec/evidence/review-bundle.source.yaml",
]
PRIMARY_REVIEW_SURFACES = {
    "source-registry": "generated/architecture/source-registry.yaml",
    "operation-kernel": "generated/architecture/operation-kernel.yaml",
    "assurance-graph": "generated/architecture/assurance-graph.yaml",
    "confidential-mechanism-profile-index": "generated/confidential/mechanism-profile-index.yaml",
}
COMPATIBILITY_RETIREMENT_PLANE = "generated/architecture/compatibility-retirement-plane.yaml"


class ArchitectureCoreModelError(RuntimeError):
    """Raised when the minimal architecture core model is inconsistent."""


def rows_by_id(rows: Any, label: str) -> dict[str, dict[str, Any]]:
    """Return mapping rows keyed by unique id."""
    result: dict[str, dict[str, Any]] = {}
    for index, raw_row in enumerate(require_list(rows, label, ArchitectureCoreModelError)):
        row = require_mapping(raw_row, f"{label}[{index}]", ArchitectureCoreModelError)
        row_id = require_string(row.get("id"), f"{label}[{index}].id", ArchitectureCoreModelError)
        if row_id in result:
            raise validation_error(ArchitectureCoreModelError, f"duplicate {label} id: {row_id}")
        result[row_id] = row
    return result


def manifest_check_ids(manifest: dict[str, Any]) -> set[str]:
    """Return check ids from the generated evidence-check manifest."""
    checks: set[str] = set()
    for group_index, raw_group in enumerate(require_list(manifest.get("groups"), "groups", ArchitectureCoreModelError)):
        group = require_mapping(raw_group, f"groups[{group_index}]", ArchitectureCoreModelError)
        group_id = require_string(group.get("id"), f"groups[{group_index}].id", ArchitectureCoreModelError)
        for check_index, raw_check in enumerate(
            require_list(group.get("checks"), f"groups.{group_id}.checks", ArchitectureCoreModelError)
        ):
            check = require_mapping(raw_check, f"groups.{group_id}.checks[{check_index}]", ArchitectureCoreModelError)
            checks.add(require_string(check.get("id"), f"groups.{group_id}.checks[{check_index}].id", ArchitectureCoreModelError))
    return checks


def require_path_exists(root: Path, path_value: str, label: str) -> None:
    """Validate one repository-relative path reference exists."""
    path = Path(path_value.split("#", maxsplit=1)[0])
    if not (root / path).exists():
        raise validation_error(ArchitectureCoreModelError, f"{label} references missing path: {path_value}")


def validate_source_paths(root: Path, model: dict[str, Any], label: str) -> None:
    """Validate source and projection references on one primary model row."""
    for field in ("source_refs", "compiled_refs", "review_refs", "projection_refs", "compatibility_refs"):
        for path_value in require_string_list(model.get(field, []), f"{label}.{field}", ArchitectureCoreModelError, allow_empty=True):
            require_path_exists(root, path_value, f"{label}.{field}")
    if "entrypoint_source" in model:
        require_path_exists(
            root,
            require_string(model.get("entrypoint_source"), f"{label}.entrypoint_source", ArchitectureCoreModelError),
            f"{label}.entrypoint_source",
        )


def validate_model_contract(source: dict[str, Any], source_path: Path) -> dict[str, Any]:
    """Return the source-owned model contract used by this validator."""
    contract = require_mapping(source.get("model_contract"), f"{source_path}.model_contract", ArchitectureCoreModelError)
    primary_ids = require_string_list(
        contract.get("primary_model_ids"),
        f"{source_path}.model_contract.primary_model_ids",
        ArchitectureCoreModelError,
    )
    supporting_ids = require_string_list(
        contract.get("supporting_model_ids"),
        f"{source_path}.model_contract.supporting_model_ids",
        ArchitectureCoreModelError,
    )
    if set(primary_ids) != set(PRIMARY_MODEL_ROOT_TYPES):
        raise validation_error(
            ArchitectureCoreModelError,
            f"{source_path}.model_contract.primary_model_ids must cover {sorted(PRIMARY_MODEL_ROOT_TYPES)}",
        )
    if set(supporting_ids) != set(SUPPORTING_MODEL_ROOT_TYPES):
        raise validation_error(
            ArchitectureCoreModelError,
            f"{source_path}.model_contract.supporting_model_ids must cover {sorted(SUPPORTING_MODEL_ROOT_TYPES)}",
        )
    flow = require_string_list(
        contract.get("assurance_primitive_flow"),
        f"{source_path}.model_contract.assurance_primitive_flow",
        ArchitectureCoreModelError,
    )
    if flow != ["Claim", "Obligation", "Discharge", "Receipt"]:
        raise validation_error(
            ArchitectureCoreModelError,
            f"{source_path}.model_contract.assurance_primitive_flow must be Claim -> Obligation -> Discharge -> Receipt",
        )
    entrypoints = require_string_list(
        contract.get("design_entrypoint_order"),
        f"{source_path}.model_contract.design_entrypoint_order",
        ArchitectureCoreModelError,
    )
    if entrypoints != DESIGN_ENTRYPOINT_ORDER:
        raise validation_error(
            ArchitectureCoreModelError,
            f"{source_path}.model_contract.design_entrypoint_order must be {DESIGN_ENTRYPOINT_ORDER}",
        )
    require_string(
        contract.get("compatibility_view_rule"),
        f"{source_path}.model_contract.compatibility_view_rule",
        ArchitectureCoreModelError,
    )
    for field, expected_path in (
        ("design_universe_source", "spec/architecture/design-universe.source.yaml"),
        ("design_package_source", "spec/architecture/design-packages.source.yaml"),
        ("ledger_catalog_source", "spec/architecture/ledger-catalog.source.yaml"),
    ):
        actual_path = require_string(
            contract.get(field),
            f"{source_path}.model_contract.{field}",
            ArchitectureCoreModelError,
        )
        if actual_path != expected_path:
            raise validation_error(
                ArchitectureCoreModelError,
                f"{source_path}.model_contract.{field} must be {expected_path}",
            )
    fragments = require_string_list(
        contract.get("design_universe_fragments"),
        f"{source_path}.model_contract.design_universe_fragments",
        ArchitectureCoreModelError,
    )
    expected_fragments = DESIGN_ENTRYPOINT_ORDER[2:5]
    if fragments != expected_fragments:
        raise validation_error(
            ArchitectureCoreModelError,
            f"{source_path}.model_contract.design_universe_fragments must be {expected_fragments}",
        )
    return contract


def validate_envelope(source: dict[str, Any], source_path: Path, contract: dict[str, Any]) -> tuple[dict[str, dict[str, Any]], dict[str, dict[str, Any]]]:
    """Validate the core model source envelope."""
    if source.get("schema_version") != 1:
        raise validation_error(ArchitectureCoreModelError, f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-architecture-core-model-source":
        raise validation_error(ArchitectureCoreModelError, f"{source_path}.kind must be kiva-architecture-core-model-source")
    if source.get("status") != "active":
        raise validation_error(ArchitectureCoreModelError, f"{source_path}.status must be active")
    models = rows_by_id(source.get("primary_models"), f"{source_path}.primary_models")
    primary_ids = require_string_list(
        contract.get("primary_model_ids"),
        f"{source_path}.model_contract.primary_model_ids",
        ArchitectureCoreModelError,
    )
    if list(models) != primary_ids:
        raise validation_error(ArchitectureCoreModelError, f"{source_path}.primary_models ids must be {primary_ids}")
    supporting = rows_by_id(source.get("supporting_models"), f"{source_path}.supporting_models")
    supporting_ids = require_string_list(
        contract.get("supporting_model_ids"),
        f"{source_path}.model_contract.supporting_model_ids",
        ArchitectureCoreModelError,
    )
    if list(supporting) != supporting_ids:
        raise validation_error(ArchitectureCoreModelError, f"{source_path}.supporting_models ids must be {supporting_ids}")
    for model_id, model in models.items():
        expected_root = PRIMARY_MODEL_ROOT_TYPES[model_id]
        if model.get("root_type") != expected_root:
            raise validation_error(ArchitectureCoreModelError, f"{source_path}.primary_models.{model_id}.root_type must be {expected_root}")
    for model_id, model in supporting.items():
        expected_root = SUPPORTING_MODEL_ROOT_TYPES[model_id]
        if model.get("root_type") != expected_root:
            raise validation_error(ArchitectureCoreModelError, f"{source_path}.supporting_models.{model_id}.root_type must be {expected_root}")
    return models, supporting


def validate_operation_model(root: Path, model: dict[str, Any]) -> None:
    """Validate OperationKernel is the internal operation IR."""
    if model.get("root_type") != "OperationKernel":
        raise validation_error(ArchitectureCoreModelError, "operation.root_type must be OperationKernel")
    entrypoint_path = Path(
        require_string(model.get("entrypoint_source"), "operation.entrypoint_source", ArchitectureCoreModelError)
    )
    if entrypoint_path.as_posix() != "spec/architecture/operation-kernel.source.yaml":
        raise validation_error(ArchitectureCoreModelError, "operation.entrypoint_source must be OperationKernelSource")
    entrypoint = load_yaml(root / entrypoint_path, ArchitectureCoreModelError)
    if entrypoint.get("kind") != "kiva-operation-kernel-source":
        raise validation_error(ArchitectureCoreModelError, "OperationKernelSource kind is invalid")
    operation_model = require_mapping(
        entrypoint.get("operation_kernel_model"),
        "operation-kernel-source.operation_kernel_model",
        ArchitectureCoreModelError,
    )
    if operation_model.get("root_type") != "OperationKernelSource":
        raise validation_error(ArchitectureCoreModelError, "operation entrypoint root_type must be OperationKernelSource")
    if operation_model.get("primary_review_surface") != "generated/architecture/operation-kernel.yaml":
        raise validation_error(ArchitectureCoreModelError, "OperationKernelSource primary review surface must be OperationKernel")
    included = require_string_list(
        entrypoint.get("included_ledgers"),
        "operation-kernel-source.included_ledgers",
        ArchitectureCoreModelError,
    )
    if len(included) != len(set(included)):
        raise validation_error(ArchitectureCoreModelError, "OperationKernelSource included_ledgers must be unique")
    if entrypoint_path.as_posix() in included:
        raise validation_error(ArchitectureCoreModelError, "OperationKernelSource must not include itself as a split ledger")
    for path_value in included:
        require_path_exists(root, path_value, "operation-kernel-source.included_ledgers")
    pds_plane = require_mapping(
        entrypoint.get("pds_supporting_plane"),
        "operation-kernel-source.pds_supporting_plane",
        ArchitectureCoreModelError,
    )
    if pds_plane.get("parent_model") != "OperationKernel":
        raise validation_error(ArchitectureCoreModelError, "PDS supporting plane must stay under OperationKernel")
    if pds_plane.get("state_machine_source") != "spec/pds/state-machines.source.yaml":
        raise validation_error(ArchitectureCoreModelError, "PDS supporting plane must reference PDS state machines")
    policy = load_yaml(root / "spec/architecture/policies/operation-kernel-policy.source.yaml", ArchitectureCoreModelError)
    kernel_policy = require_mapping(policy.get("operation_kernel_policy"), "operation_kernel_policy", ArchitectureCoreModelError)
    if kernel_policy.get("root_model") != "OperationKernel":
        raise validation_error(ArchitectureCoreModelError, "operation_kernel_policy.root_model must be OperationKernel")
    kernel = load_yaml(root / "generated/architecture/operation-kernel.yaml", ArchitectureCoreModelError)
    projection_basis = require_mapping(kernel.get("projection_basis"), "operation-kernel.projection_basis", ArchitectureCoreModelError)
    if projection_basis.get("root_type") != "OperationKernel":
        raise validation_error(ArchitectureCoreModelError, "generated OperationKernel root_type must be OperationKernel")
    operation_count = kernel.get("operation_count")
    operations = require_list(kernel.get("operations"), "operation-kernel.operations", ArchitectureCoreModelError)
    if operation_count != len(operations):
        raise validation_error(ArchitectureCoreModelError, "operation-kernel.operation_count must match operations length")
    for index, operation in enumerate(operations):
        row = require_mapping(operation, f"operation-kernel.operations[{index}]", ArchitectureCoreModelError)
        require_mapping(row.get("operation_key"), f"operation-kernel.operations[{index}].operation_key", ArchitectureCoreModelError)
        require_mapping(row.get("route_key"), f"operation-kernel.operations[{index}].route_key", ArchitectureCoreModelError)


def validate_assurance_model(root: Path, model: dict[str, Any], contract: dict[str, Any]) -> None:
    """Validate the reduced assurance primitive flow."""
    entrypoint_path = Path(
        require_string(model.get("entrypoint_source"), "assurance.entrypoint_source", ArchitectureCoreModelError)
    )
    if entrypoint_path.as_posix() != "spec/evidence/assurance-primitive.source.yaml":
        raise validation_error(ArchitectureCoreModelError, "assurance.entrypoint_source must be AssurancePrimitiveSource")
    entrypoint = load_yaml(root / entrypoint_path, ArchitectureCoreModelError)
    if entrypoint.get("kind") != "kiva-assurance-primitive-source":
        raise validation_error(ArchitectureCoreModelError, "AssurancePrimitiveSource kind is invalid")
    primitive = require_mapping(
        entrypoint.get("assurance_primitive_model"),
        "assurance-primitive.assurance_primitive_model",
        ArchitectureCoreModelError,
    )
    if primitive.get("root_type") != "AssurancePrimitiveSource":
        raise validation_error(ArchitectureCoreModelError, "assurance primitive root_type must be AssurancePrimitiveSource")
    expected_flow = require_string_list(
        contract.get("assurance_primitive_flow"),
        "model_contract.assurance_primitive_flow",
        ArchitectureCoreModelError,
    )
    if require_string_list(primitive.get("primitive_flow"), "assurance primitive flow", ArchitectureCoreModelError) != expected_flow:
        raise validation_error(ArchitectureCoreModelError, f"assurance primitive_flow must be {expected_flow}")
    if primitive.get("primary_review_surface") != "generated/architecture/assurance-graph.yaml":
        raise validation_error(ArchitectureCoreModelError, "AssurancePrimitiveSource primary surface must be AssuranceGraph")
    input_sources = require_string_list(
        entrypoint.get("materialized_input_sources"),
        "assurance-primitive.materialized_input_sources",
        ArchitectureCoreModelError,
    )
    if len(input_sources) != len(set(input_sources)):
        raise validation_error(ArchitectureCoreModelError, "assurance materialized inputs must be unique")
    for path_value in input_sources:
        require_path_exists(root, path_value, "assurance-primitive.materialized_input_sources")
    validate_review_bundle(root, Path(require_string(primitive.get("review_bundle"), "assurance review_bundle", ArchitectureCoreModelError)))
    if model.get("primary_review_surface") != "generated/architecture/assurance-graph.yaml":
        raise validation_error(ArchitectureCoreModelError, "assurance.primary_review_surface must be AssuranceGraph")
    evidence_source = load_yaml(root / "spec/evidence/evidence-set.source.yaml", ArchitectureCoreModelError)
    meta = require_mapping(evidence_source.get("assurance_meta_model"), "evidence-set.assurance_meta_model", ArchitectureCoreModelError)
    if meta.get("root_type") != "AssurancePrimitive":
        raise validation_error(ArchitectureCoreModelError, "assurance_meta_model.root_type must be AssurancePrimitive")
    if require_string_list(meta.get("primitive_flow"), "assurance_meta_model.primitive_flow", ArchitectureCoreModelError) != expected_flow:
        raise validation_error(ArchitectureCoreModelError, f"assurance primitive_flow must be {expected_flow}")
    receipts = load_yaml(root / "generated/evidence/evidence-receipts.yaml", ArchitectureCoreModelError)
    summary = require_mapping(receipts.get("summary"), "evidence-receipts.summary", ArchitectureCoreModelError)
    if int(summary.get("receipt_count", 0)) <= 0:
        raise validation_error(ArchitectureCoreModelError, "evidence receipt_count must be positive")
    claim_graph = load_yaml(root / "generated/architecture/claim-graph.yaml", ArchitectureCoreModelError)
    if claim_graph.get("kind") != "kiva-claim-graph":
        raise validation_error(ArchitectureCoreModelError, "claim graph must be generated")
    assurance_graph = load_yaml(root / "generated/architecture/assurance-graph.yaml", ArchitectureCoreModelError)
    if assurance_graph.get("kind") != "kiva-assurance-graph":
        raise validation_error(ArchitectureCoreModelError, "AssuranceGraph must be generated")
    if assurance_graph.get("primary_review_surface") is not True:
        raise validation_error(ArchitectureCoreModelError, "AssuranceGraph must be marked as the primary review surface")
    authority_model = require_mapping(
        entrypoint.get("authority_model"),
        "assurance-primitive.authority_model",
        ArchitectureCoreModelError,
    )
    authority_rows: dict[str, dict[str, Any]] = {}
    for index, raw_row in enumerate(
        require_list(authority_model.get("node_types"), "assurance-primitive.authority_model.node_types", ArchitectureCoreModelError)
    ):
        row = require_mapping(raw_row, f"assurance authority node_types[{index}]", ArchitectureCoreModelError)
        node_type = require_string(row.get("node_type"), f"assurance authority node_types[{index}].node_type", ArchitectureCoreModelError)
        if node_type in authority_rows:
            raise validation_error(ArchitectureCoreModelError, f"duplicate assurance authority node type: {node_type}")
        authority_rows[node_type] = row
    required_authority_types = {"claim", "evidence-receipt", "capability", "verification-obligation"}
    if set(authority_rows) != required_authority_types:
        raise validation_error(
            ArchitectureCoreModelError,
            f"assurance authority node types must be {sorted(required_authority_types)}",
        )
    authority_policy = require_mapping(
        assurance_graph.get("authority_level_policy"),
        "assurance-graph.authority_level_policy",
        ArchitectureCoreModelError,
    )
    for node_type, authority_row in authority_rows.items():
        authority_level = require_string(authority_row.get("authority_level"), f"authority.{node_type}.authority_level", ArchitectureCoreModelError)
        policy_key = require_string(authority_row.get("policy_key"), f"authority.{node_type}.policy_key", ArchitectureCoreModelError)
        policy = require_mapping(
            authority_policy.get(policy_key),
            f"assurance-graph.authority_level_policy.{policy_key}",
            ArchitectureCoreModelError,
        )
        if policy.get("authority_level") != authority_level:
            raise validation_error(
                ArchitectureCoreModelError,
                f"AssuranceGraph authority level for {node_type} must be {authority_level}",
            )
    for index, raw_node in enumerate(require_list(assurance_graph.get("nodes"), "assurance-graph.nodes", ArchitectureCoreModelError)):
        graph_node = require_mapping(raw_node, f"assurance-graph.nodes[{index}]", ArchitectureCoreModelError)
        node_type = require_string(
            graph_node.get("node_type"),
            f"assurance-graph.nodes[{index}].node_type",
            ArchitectureCoreModelError,
        )
        authority_row = authority_rows.get(node_type)
        if authority_row is None:
            raise validation_error(ArchitectureCoreModelError, f"AssuranceGraph node_type has no authority level: {node_type}")
        expected_authority = require_string(authority_row.get("authority_level"), f"authority.{node_type}.authority_level", ArchitectureCoreModelError)
        if graph_node.get("authority_level") != expected_authority:
            raise validation_error(
                ArchitectureCoreModelError,
                f"AssuranceGraph node {graph_node.get('id')} authority_level must be {expected_authority}",
            )


def validate_projection_model(root: Path, model: dict[str, Any]) -> None:
    """Validate the ProjectionSpec normal-form target is explicit."""
    if model.get("root_type") != "ProjectionRuntime":
        raise validation_error(ArchitectureCoreModelError, "projection.root_type must be ProjectionRuntime")
    if model.get("target_stage") != "normal-form-primary":
        raise validation_error(ArchitectureCoreModelError, "projection.target_stage must be normal-form-primary")
    policy = load_yaml(root / "spec/architecture/projection-normal-form-policy.source.yaml", ArchitectureCoreModelError)
    if policy.get("transition_stage") != "normal-form-primary":
        raise validation_error(ArchitectureCoreModelError, "projection normal form transition_stage must be normal-form-primary")
    if policy.get("target_stage") != "normal-form-primary":
        raise validation_error(ArchitectureCoreModelError, "projection normal form target_stage must be normal-form-primary")
    criteria = require_mapping(policy.get("promotion_criteria"), "projection-normal-form.promotion_criteria", ArchitectureCoreModelError)
    if criteria.get("compatibility_view_is_byte_for_byte_current") is not True:
        raise validation_error(ArchitectureCoreModelError, "projection compatibility view must be marked current")
    lifecycle = require_mapping(
        policy.get("compatibility_view_lifecycle"),
        "projection-normal-form.compatibility_view_lifecycle",
        ArchitectureCoreModelError,
    )
    if require_bool(lifecycle.get("human_editable"), "projection compatibility human_editable", ArchitectureCoreModelError):
        raise validation_error(ArchitectureCoreModelError, "ProjectionSpec compatibility view must not be human editable")
    if require_bool(lifecycle.get("generated_only"), "projection compatibility generated_only", ArchitectureCoreModelError) is not True:
        raise validation_error(ArchitectureCoreModelError, "ProjectionSpec compatibility view must be generated-only")
    if require_bool(lifecycle.get("canonical_source"), "projection compatibility canonical_source", ArchitectureCoreModelError):
        raise validation_error(ArchitectureCoreModelError, "ProjectionSpec compatibility view must not be canonical source")
    if lifecycle.get("review_role") != "generated-compatibility-view":
        raise validation_error(ArchitectureCoreModelError, "ProjectionSpec compatibility view review_role is invalid")


def validate_confidential_capability_model(root: Path, model: dict[str, Any]) -> None:
    """Validate CapabilityPort is the confidential-computation review boundary."""
    if model.get("root_type") != "CapabilityPort":
        raise validation_error(ArchitectureCoreModelError, "confidential-capability.root_type must be CapabilityPort")
    package_path = Path(
        require_string(model.get("entrypoint_source"), "confidential-capability.entrypoint_source", ArchitectureCoreModelError)
    )
    if package_path.as_posix() != "spec/confidential/mechanism-packages.source.yaml":
        raise validation_error(ArchitectureCoreModelError, "confidential capability entrypoint must be MechanismPackage source")
    packages = load_yaml(root / package_path, ArchitectureCoreModelError)
    validate_mechanism_packages(root, packages)
    if model.get("sealed_provider_boundary") != "spec/confidential/secret-compute-provider-contract.yaml#/sealed_capability_boundary":
        raise validation_error(ArchitectureCoreModelError, "confidential-capability.sealed_provider_boundary must reference the provider contract")
    provider = load_yaml(root / "spec/confidential/secret-compute-provider-contract.yaml", ArchitectureCoreModelError)
    sealed = require_mapping(
        provider.get("sealed_capability_boundary"),
        "secret-compute-provider.sealed_capability_boundary",
        ArchitectureCoreModelError,
    )
    if sealed.get("root_type") != "SealedCapabilityBoundary":
        raise validation_error(ArchitectureCoreModelError, "sealed capability boundary root_type must be SealedCapabilityBoundary")
    taxonomy = load_yaml(root / "spec/confidential/capability-taxonomy.source.yaml", ArchitectureCoreModelError)
    port_model = require_mapping(taxonomy.get("capability_port_model"), "capability_taxonomy.capability_port_model", ArchitectureCoreModelError)
    if port_model.get("root_type") != "CapabilityPort":
        raise validation_error(ArchitectureCoreModelError, "capability_port_model.root_type must be CapabilityPort")
    capabilities = rows_by_id(taxonomy.get("capabilities"), "capability_taxonomy.capabilities")
    maturity = load_yaml(root / "spec/confidential/mechanism-maturity.yaml", ArchitectureCoreModelError)
    mechanisms = rows_by_id(maturity.get("mechanisms"), "mechanism_maturity.mechanisms")
    bindings = load_yaml(root / "spec/confidential/capability-bindings.source.yaml", ArchitectureCoreModelError)
    for binding_id, binding in rows_by_id(bindings.get("bindings"), "capability_bindings.bindings").items():
        capability = require_string(binding.get("capability"), f"capability binding {binding_id}.capability", ArchitectureCoreModelError)
        mechanism = require_string(binding.get("maturity_mechanism"), f"capability binding {binding_id}.maturity_mechanism", ArchitectureCoreModelError)
        if capability not in capabilities:
            raise validation_error(ArchitectureCoreModelError, f"capability binding {binding_id} references missing capability: {capability}")
        if mechanism not in mechanisms:
            raise validation_error(ArchitectureCoreModelError, f"capability binding {binding_id} references missing mechanism: {mechanism}")
    index = load_yaml(root / "generated/confidential/capability-binding-index.yaml", ArchitectureCoreModelError)
    if index.get("kind") != "kiva-confidential-capability-binding-index":
        raise validation_error(ArchitectureCoreModelError, "generated capability binding index is missing")
    try:
        validate_tck(
            load_yaml(root / "spec/confidential/secret-compute-provider-tck.source.yaml", ArchitectureCoreModelError),
            Path("spec/confidential/secret-compute-provider-tck.source.yaml"),
            root,
        )
    except SecretComputeProviderTckError as error:
        raise validation_error(ArchitectureCoreModelError, f"provider TCK is invalid: {error}") from error


def validate_verification_model(root: Path, model: dict[str, Any]) -> None:
    """Validate boundary properties are primary verification roots."""
    if model.get("root_type") != "BoundaryProperty":
        raise validation_error(ArchitectureCoreModelError, "verification.root_type must be BoundaryProperty")
    source = load_yaml(root / "spec/verification/verification-obligations.source.yaml", ArchitectureCoreModelError)
    obligation_model = require_mapping(source.get("obligation_model"), "verification.obligation_model", ArchitectureCoreModelError)
    if obligation_model.get("primitive_root_type") != "BoundaryProperty":
        raise validation_error(ArchitectureCoreModelError, "verification primitive_root_type must be BoundaryProperty")
    generated = load_yaml(root / "generated/verification/verification-obligations.yaml", ArchitectureCoreModelError)
    summary = require_mapping(generated.get("summary"), "verification-obligations.summary", ArchitectureCoreModelError)
    if summary.get("obligation_count") != summary.get("complete_count"):
        raise validation_error(ArchitectureCoreModelError, "verification obligations must be complete")


def validate_pds_supporting_model(model: dict[str, Any]) -> None:
    """Validate PDS is modeled as an OperationKernel supporting plane."""
    if model.get("root_type") != "PdsCorePlane":
        raise validation_error(ArchitectureCoreModelError, "pds.root_type must be PdsCorePlane")
    if model.get("parent_model") != "operation":
        raise validation_error(ArchitectureCoreModelError, "pds.parent_model must be operation")


def validate_review_bundle(root: Path, bundle_path: Path) -> None:
    """Validate the compact human-facing evidence review bundle."""
    if bundle_path.as_posix() != "spec/evidence/review-bundle.source.yaml":
        raise validation_error(ArchitectureCoreModelError, "assurance review bundle must be spec/evidence/review-bundle.source.yaml")
    bundle = load_yaml(root / bundle_path, ArchitectureCoreModelError)
    if bundle.get("kind") != "kiva-evidence-review-bundle-source":
        raise validation_error(ArchitectureCoreModelError, "review bundle kind is invalid")
    model = require_mapping(bundle.get("review_bundle_model"), "review-bundle.review_bundle_model", ArchitectureCoreModelError)
    if model.get("root_type") != "EvidenceReviewBundle":
        raise validation_error(ArchitectureCoreModelError, "review bundle root_type must be EvidenceReviewBundle")
    surfaces = rows_by_id(
        bundle.get("primary_review_surfaces"),
        "review-bundle.primary_review_surfaces",
    )
    if list(surfaces) != list(PRIMARY_REVIEW_SURFACES):
        raise validation_error(
            ArchitectureCoreModelError,
            f"review bundle primary surfaces must be {list(PRIMARY_REVIEW_SURFACES)}",
        )
    for surface_id, expected_path in PRIMARY_REVIEW_SURFACES.items():
        actual_path = require_string(
            surfaces[surface_id].get("path"),
            f"review-bundle.primary_review_surfaces.{surface_id}.path",
            ArchitectureCoreModelError,
        )
        if actual_path != expected_path:
            raise validation_error(
                ArchitectureCoreModelError,
                f"review bundle surface {surface_id} path must be {expected_path}",
            )
        require_path_exists(root, actual_path, "review-bundle.primary_review_surfaces")
    supporting = require_string_list(
        bundle.get("supporting_generated_views"),
        "review-bundle.supporting_generated_views",
        ArchitectureCoreModelError,
    )
    if COMPATIBILITY_RETIREMENT_PLANE not in supporting:
        raise validation_error(
            ArchitectureCoreModelError,
            "compatibility-retirement plane must be a supporting generated view",
        )
    for path_value in supporting:
        require_path_exists(root, path_value, "review-bundle.supporting_generated_views")


def validate_mechanism_packages(root: Path, packages: dict[str, Any]) -> None:
    """Validate MechanismPackage rows cover confidential capability bindings."""
    if packages.get("kind") != "kiva-confidential-mechanism-packages-source":
        raise validation_error(ArchitectureCoreModelError, "MechanismPackage source kind is invalid")
    model = require_mapping(
        packages.get("mechanism_package_model"),
        "mechanism-packages.mechanism_package_model",
        ArchitectureCoreModelError,
    )
    if model.get("root_type") != "MechanismPackage":
        raise validation_error(ArchitectureCoreModelError, "mechanism package root_type must be MechanismPackage")
    if model.get("authoritative_edit_entrypoint") is not True:
        raise validation_error(ArchitectureCoreModelError, "MechanismPackage must be the authoritative edit entrypoint")
    if model.get("facet_sources_are_not_roots") is not True:
        raise validation_error(ArchitectureCoreModelError, "MechanismPackage facet sources must not be independent roots")
    facet_policy = require_mapping(
        model.get("facet_edit_policy"),
        "mechanism-packages.mechanism_package_model.facet_edit_policy",
        ArchitectureCoreModelError,
    )
    for field in (
        "package_is_only_confidential_edit_unit",
        "facet_sources_remain_supporting_rows",
        "review_must_enter_through_mechanism_profile_index",
        "every_facet_row_must_be_owned_by_exactly_one_package",
    ):
        if facet_policy.get(field) is not True:
            raise validation_error(ArchitectureCoreModelError, f"MechanismPackage facet_edit_policy.{field} must be true")
    if model.get("primary_review_surface") != "generated/confidential/mechanism-profile-index.yaml":
        raise validation_error(ArchitectureCoreModelError, "MechanismPackage primary surface must be MechanismProfile index")
    if model.get("package_profile_projection") != "generated/confidential/mechanism-profile-index.yaml#/package_profiles":
        raise validation_error(ArchitectureCoreModelError, "MechanismPackage PackageProfile projection is invalid")
    if model.get("sealed_provider_boundary") != "spec/confidential/secret-compute-provider-contract.yaml#/sealed_capability_boundary":
        raise validation_error(ArchitectureCoreModelError, "MechanismPackage must reference the sealed provider boundary")
    for path_value in require_string_list(
        packages.get("package_source_refs"),
        "mechanism-packages.package_source_refs",
        ArchitectureCoreModelError,
    ):
        require_path_exists(root, path_value, "mechanism-packages.package_source_refs")

    binding_source = load_yaml(root / "spec/confidential/capability-bindings.source.yaml", ArchitectureCoreModelError)
    binding_rows = rows_by_id(binding_source.get("bindings"), "capability_bindings.bindings")
    maturity_source = load_yaml(root / "spec/confidential/mechanism-maturity.yaml", ArchitectureCoreModelError)
    maturity_rows = rows_by_id(maturity_source.get("mechanisms"), "mechanism_maturity.mechanisms")
    catalog_source = load_yaml(root / "spec/confidential/mechanism-catalog.source.yaml", ArchitectureCoreModelError)
    catalog_rows = rows_by_id(catalog_source.get("mechanisms"), "mechanism_catalog.mechanisms")
    registry = load_yaml(root / "spec/confidential/assurance-evidence-registry.yaml", ArchitectureCoreModelError)
    registry_entries = require_mapping(registry.get("entries"), "assurance_registry.entries", ArchitectureCoreModelError)
    provider = load_yaml(root / "spec/confidential/secret-compute-provider-contract.yaml", ArchitectureCoreModelError)
    sealed = require_mapping(provider.get("sealed_capability_boundary"), "provider.sealed_capability_boundary", ArchitectureCoreModelError)
    provider_profiles = set(require_string_list(sealed.get("provider_profiles"), "sealed.provider_profiles", ArchitectureCoreModelError))

    package_rows = rows_by_id(packages.get("packages"), "mechanism_packages.packages")
    packages_by_binding: dict[str, dict[str, Any]] = {}
    for package_id, package in package_rows.items():
        maturity = require_string(package.get("maturity_mechanism"), f"{package_id}.maturity_mechanism", ArchitectureCoreModelError)
        binding_id = require_string(package.get("capability_binding"), f"{package_id}.capability_binding", ArchitectureCoreModelError)
        capability = require_string(package.get("capability"), f"{package_id}.capability", ArchitectureCoreModelError)
        provider_kind = require_string(package.get("provider_kind"), f"{package_id}.provider_kind", ArchitectureCoreModelError)
        if provider_kind not in provider_profiles:
            raise validation_error(ArchitectureCoreModelError, f"{package_id}.provider_kind is not in sealed provider profiles: {provider_kind}")
        if maturity not in maturity_rows:
            raise validation_error(ArchitectureCoreModelError, f"{package_id} references missing maturity mechanism: {maturity}")
        binding = binding_rows.get(binding_id)
        if binding is None:
            raise validation_error(ArchitectureCoreModelError, f"{package_id} references missing capability binding: {binding_id}")
        if binding.get("maturity_mechanism") != maturity:
            raise validation_error(ArchitectureCoreModelError, f"{package_id} maturity does not match CapabilityBinding")
        if binding.get("capability") != capability:
            raise validation_error(ArchitectureCoreModelError, f"{package_id} capability does not match CapabilityBinding")
        if binding_id in packages_by_binding:
            raise validation_error(ArchitectureCoreModelError, f"duplicate MechanismPackage for capability binding: {binding_id}")
        packages_by_binding[binding_id] = package

        catalog_id = package.get("mechanism_catalog_id")
        if catalog_id is not None:
            catalog = catalog_rows.get(require_string(catalog_id, f"{package_id}.mechanism_catalog_id", ArchitectureCoreModelError))
            if catalog is None:
                raise validation_error(ArchitectureCoreModelError, f"{package_id} references missing mechanism catalog row: {catalog_id}")
            if catalog.get("maturity_mechanism") != maturity:
                raise validation_error(ArchitectureCoreModelError, f"{package_id} catalog row maturity does not match package maturity")
            if catalog.get("provider_kind") != provider_kind:
                raise validation_error(ArchitectureCoreModelError, f"{package_id} provider kind does not match mechanism catalog")
        assurance_entry = package.get("assurance_registry_entry")
        if assurance_entry is not None and require_string(assurance_entry, f"{package_id}.assurance_registry_entry", ArchitectureCoreModelError) not in registry_entries:
            raise validation_error(ArchitectureCoreModelError, f"{package_id} references missing assurance registry entry: {assurance_entry}")

    missing_bindings = sorted(set(binding_rows).difference(packages_by_binding))
    if missing_bindings:
        raise validation_error(ArchitectureCoreModelError, f"MechanismPackage rows missing capability bindings: {missing_bindings}")

    profile_index = load_yaml(root / "generated/confidential/mechanism-profile-index.yaml", ArchitectureCoreModelError)
    package_profile_ids = {
        require_string(row.get("id"), f"package_profiles[{index}].id", ArchitectureCoreModelError)
        for index, raw_row in enumerate(
            require_list(profile_index.get("package_profiles"), "mechanism-profile-index.package_profiles", ArchitectureCoreModelError)
        )
        for row in [require_mapping(raw_row, f"package_profiles[{index}]", ArchitectureCoreModelError)]
    }
    if package_profile_ids != set(package_rows):
        raise validation_error(ArchitectureCoreModelError, "PackageProfile projection must cover every MechanismPackage exactly once")

    catalog_maturity_to_id = {
        require_string(row.get("maturity_mechanism"), f"mechanism_catalog.{row_id}.maturity_mechanism", ArchitectureCoreModelError): row_id
        for row_id, row in catalog_rows.items()
    }
    packaged_catalog_ids = {
        package.get("mechanism_catalog_id")
        for package in package_rows.values()
        if package.get("mechanism_catalog_id") is not None
    }
    missing_catalog_rows = sorted(set(catalog_rows).difference(packaged_catalog_ids))
    if missing_catalog_rows:
        raise validation_error(ArchitectureCoreModelError, f"MechanismPackage rows missing mechanism catalog ids: {missing_catalog_rows}")
    for maturity, catalog_id in catalog_maturity_to_id.items():
        package = next((row for row in package_rows.values() if row.get("maturity_mechanism") == maturity), None)
        if package is None or package.get("mechanism_catalog_id") != catalog_id:
            raise validation_error(ArchitectureCoreModelError, f"MechanismPackage missing catalog binding for maturity: {maturity}")


def validate_review_rules(root: Path, source: dict[str, Any], source_path: Path) -> None:
    """Validate typed global review rules and their executable enforcement."""
    rules = rows_by_id(source.get("review_rules"), f"{source_path}.review_rules")
    required = {
        "no_parallel_operation_authority",
        "operation_kernel_entrypoint_owns_split_sources",
        "evidence_views_are_not_new_claim_sources",
        "evidence_review_bundle_limits_primary_review_surfaces",
        "assurance_graph_is_primary_review_surface",
        "compatibility_surfaces_require_exit_gates",
        "pds_is_operation_kernel_supporting_plane",
        "confidential_runtime_claims_require_provider_policy_and_evidence",
        "confidential_providers_use_sealed_capability_boundary",
        "confidential_mechanism_packages_are_primary_edit_entrypoint",
        "pds_planes_do_not_expand_xapi_conformance_claim",
        "compatibility_retirement_is_history_not_primary_review_surface",
        "projection_spec_is_generated_only_compatibility_view",
        "ci_evidence_check_source_is_generated_only_compatibility_view",
        "pds_planes_package_invariants_with_planes",
        "pds_plane_packages_join_state_resources_and_checks",
        "design_packages_are_first_class_review_units",
        "ledger_catalog_joins_source_schema_lifecycle_and_review_surface",
        "design_universe_is_single_metamodel_root",
    }
    if set(rules) != required:
        missing = sorted(required.difference(rules))
        extra = sorted(set(rules).difference(required))
        raise validation_error(
            ArchitectureCoreModelError,
            f"{source_path}.review_rules mismatch; missing={missing}, extra={extra}",
        )
    for rule_id, rule in rules.items():
        require_string(rule.get("subject"), f"review_rules.{rule_id}.subject", ArchitectureCoreModelError)
        if rule.get("enforcement") != "required":
            raise validation_error(ArchitectureCoreModelError, f"review_rules.{rule_id}.enforcement must be required")
        validator = require_string(rule.get("validator"), f"review_rules.{rule_id}.validator", ArchitectureCoreModelError)
        require_path_exists(root, validator, f"review_rules.{rule_id}.validator")
        require_string(rule.get("rationale"), f"review_rules.{rule_id}.rationale", ArchitectureCoreModelError)


def validate_compatibility_views_are_not_canonical(root: Path, source: dict[str, Any]) -> None:
    """Validate retained compatibility refs are not classified as canonical sources."""
    registry_path = root / "generated/architecture/source-registry.yaml"
    if not registry_path.exists():
        return
    registry = load_yaml(registry_path, ArchitectureCoreModelError)
    source_rows = require_list(registry.get("sources"), "source-registry.sources", ArchitectureCoreModelError)
    artifact_to_kind: dict[str, str] = {}
    for index, raw_row in enumerate(source_rows):
        row = require_mapping(raw_row, f"source-registry.sources[{index}]", ArchitectureCoreModelError)
        source_kind = require_string(row.get("source_kind"), f"source-registry.sources[{index}].source_kind", ArchitectureCoreModelError)
        for artifact in require_string_list(
            row.get("artifacts"),
            f"source-registry.sources[{index}].artifacts",
            ArchitectureCoreModelError,
            allow_empty=True,
        ):
            artifact_to_kind[artifact] = source_kind
    for model_id, raw_model in enumerate(require_list(source.get("supporting_models"), "supporting_models", ArchitectureCoreModelError)):
        model = require_mapping(raw_model, f"supporting_models[{model_id}]", ArchitectureCoreModelError)
        for path_value in require_string_list(
            model.get("compatibility_refs", []),
            f"supporting_models[{model_id}].compatibility_refs",
            ArchitectureCoreModelError,
            allow_empty=True,
        ):
            source_kind = artifact_to_kind.get(path_value)
            if source_kind == "canonical":
                raise validation_error(
                    ArchitectureCoreModelError,
                    f"compatibility ref must not be classified canonical in SourceRegistry: {path_value}",
                )


def validate_architecture_core_model(root: Path, source_path: Path, manifest_path: Path) -> int:
    """Validate all primary model rows and their existing ledgers."""
    source = load_yaml(root / source_path, ArchitectureCoreModelError)
    manifest = load_evidence_check_manifest(root / manifest_path, ArchitectureCoreModelError)
    checks = manifest_check_ids(manifest)
    contract = validate_model_contract(source, source_path)
    for path_value in require_string_list(
        contract.get("design_entrypoint_order"),
        f"{source_path}.model_contract.design_entrypoint_order",
        ArchitectureCoreModelError,
    ):
        require_path_exists(root, path_value, "model_contract.design_entrypoint_order")
    require_path_exists(
        root,
        require_string(contract.get("design_universe_source"), "model_contract.design_universe_source", ArchitectureCoreModelError),
        "model_contract.design_universe_source",
    )
    require_path_exists(
        root,
        require_string(contract.get("design_package_source"), "model_contract.design_package_source", ArchitectureCoreModelError),
        "model_contract.design_package_source",
    )
    require_path_exists(
        root,
        require_string(contract.get("ledger_catalog_source"), "model_contract.ledger_catalog_source", ArchitectureCoreModelError),
        "model_contract.ledger_catalog_source",
    )
    models, supporting = validate_envelope(source, source_path, contract)
    validate_review_rules(root, source, source_path)
    validate_compatibility_views_are_not_canonical(root, source)
    for model_id, model in {**models, **supporting}.items():
        validate_source_paths(root, model, f"{source_path}.{model_id}")
        required_check = model.get("required_check")
        if required_check is not None and require_string(required_check, f"{model_id}.required_check", ArchitectureCoreModelError) not in checks:
            raise validation_error(ArchitectureCoreModelError, f"{model_id}.required_check is missing from evidence checks: {required_check}")

    validate_operation_model(root, models["operation"])
    validate_assurance_model(root, models["assurance"], contract)
    validate_projection_model(root, supporting["projection"])
    validate_ci_command_registry(root, manifest_path, Path("scripts/ci/kiva-ci"))
    validate_confidential_capability_model(root, models["confidential-capability"])
    validate_design_universe(
        root,
        Path("spec/architecture/design-universe.source.yaml"),
        Path("spec/architecture/architecture-graph.source.yaml"),
        Path("spec/architecture/design-packages.source.yaml"),
        Path("spec/architecture/ledger-catalog.source.yaml"),
    )
    validate_design_packages(
        root,
        Path("spec/architecture/design-packages.source.yaml"),
        source_path,
        manifest_path,
    )
    validate_ledger_catalog(
        root,
        Path("spec/architecture/ledger-catalog.source.yaml"),
        Path("spec/architecture/architecture-graph.source.yaml"),
        Path("spec/architecture/source-ledger-schemas.yaml"),
        Path("spec/architecture/design-packages.source.yaml"),
    )
    validate_pds_supporting_model(supporting["pds"])
    validate_pds_core_planes(root, Path("spec/pds/core-planes.source.yaml"), Path("spec/pds/publication-milestones.yaml"), manifest_path)
    try:
        validate_pds_invariants(root, Path("spec/pds/invariants.source.yaml"))
    except PdsInvariantError as error:
        raise validation_error(ArchitectureCoreModelError, f"PDS invariants are invalid: {error}") from error
    try:
        validate_pds_state_machines(root, Path("spec/pds/state-machines.source.yaml"))
    except PdsStateMachineError as error:
        raise validation_error(ArchitectureCoreModelError, f"PDS state machines are invalid: {error}") from error
    validate_verification_model(root, supporting["verification"])
    return len(models) + len(supporting)


def main() -> int:
    """Validate Kiva's minimal architecture core model."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--evidence-manifest", type=Path, default=DEFAULT_EVIDENCE_MANIFEST)
    args = parser.parse_args()

    count = validate_architecture_core_model(args.root.resolve(), args.source, args.evidence_manifest)
    print(f"validated architecture core model: {count} primary models")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
