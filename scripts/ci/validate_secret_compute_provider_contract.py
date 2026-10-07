#!/usr/bin/env python3
"""Validate Kiva's secret/confidential-compute provider contract."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

import yaml

from confidential_mechanism_catalog_model import validated_mechanism_rows
from source_scan import missing_confidential_rust_terms


DEFAULT_CONTRACT = Path("spec/confidential/secret-compute-provider-contract.yaml")
DEFAULT_MATURITY = Path("spec/confidential/mechanism-maturity.yaml")
PROFILE = "kiva-secret-compute-provider-contract-v1"
REFERENCE_PROFILE = "kiva-reference-primitives-v1"
DOMAIN = "kiva-confidential:secret-compute-provider-binding:v1"
EXPECTED_PROVIDER_KINDS = {
    "secret-custody",
    "trusted-execution-environment",
    "fully-homomorphic-encryption",
    "private-set-intersection",
    "threshold-cryptography",
}
EXPECTED_SECRET_CUSTODY_PROVIDER_KINDS = {"secret-custody"}
EXPECTED_COMPUTE_PROVIDER_KINDS = EXPECTED_PROVIDER_KINDS.difference(
    EXPECTED_SECRET_CUSTODY_PROVIDER_KINDS,
)
EXPECTED_PROVIDER_KIND_RUST_VARIANTS = {
    "reference-primitive": "ReferencePrimitive",
    "secret-custody": "SecretCustody",
    "trusted-execution-environment": "TrustedExecutionEnvironment",
    "fully-homomorphic-encryption": "FullyHomomorphicEncryption",
    "private-set-intersection": "PrivateSetIntersection",
    "threshold-cryptography": "ThresholdCryptography",
}
EXPECTED_PROVIDER_ALGEBRA_ORDER = [
    "required-capability",
    "evidence-gate",
    "runtime-boundary",
    "release-gate",
]


class SecretComputeProviderContractError(RuntimeError):
    """Raised when the provider contract is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise SecretComputeProviderContractError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise SecretComputeProviderContractError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a non-empty list or raise a validation error."""
    if not isinstance(value, list) or not value:
        raise SecretComputeProviderContractError(f"{label} must be a non-empty list")
    return value


def require_string_list(value: Any, label: str) -> list[str]:
    """Require a non-empty list of unique strings."""
    values = require_list(value, label)
    if not all(isinstance(item, str) and item for item in values):
        raise SecretComputeProviderContractError(f"{label} must contain only strings")
    if len(values) != len(set(values)):
        raise SecretComputeProviderContractError(f"{label} must not contain duplicates")
    return values


def require_string(value: Any, label: str) -> str:
    """Require one non-empty string."""
    if not isinstance(value, str) or not value:
        raise SecretComputeProviderContractError(f"{label} must be a non-empty string")
    return value


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality."""
    if actual != expected:
        raise SecretComputeProviderContractError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def require_path(path_value: Any, label: str) -> Path:
    """Require an existing repository path."""
    if not isinstance(path_value, str) or not path_value:
        raise SecretComputeProviderContractError(f"{label} must be a non-empty path")
    path = Path(path_value)
    if not path.exists():
        raise SecretComputeProviderContractError(f"{label} does not exist: {path}")
    return path


def seed_bytes(seed: Any, label: str) -> bytes | None:
    """Return deterministic 32-byte fixture material or None."""
    if seed is None:
        return None
    if not isinstance(seed, int) or not 1 <= seed <= 255:
        raise SecretComputeProviderContractError(f"{label} must be an integer seed in 1..255")
    return bytes([seed]) * 32


def update_len_prefixed(digest: Any, value: str) -> None:
    """Add Rust-compatible 128-bit-length-prefixed UTF-8 bytes."""
    encoded = value.encode("utf-8")
    digest.update(len(encoded).to_bytes(16, "big"))
    digest.update(encoded)


def derive_binding_digest_hex(vector: dict[str, Any]) -> str:
    """Derive the expected provider-binding digest."""
    digest = hashlib.sha256()
    digest.update(DOMAIN.encode("utf-8"))
    update_len_prefixed(digest, str(vector["profile"]))
    update_len_prefixed(digest, str(vector["kind"]))
    update_len_prefixed(digest, str(vector["capability"]))
    digest.update(seed_bytes(vector.get("provider_id_seed"), "provider_id_seed") or b"")
    digest.update(seed_bytes(vector.get("artifact_seed"), "artifact_seed") or b"")
    parameter_set = seed_bytes(vector.get("parameter_set_seed"), "parameter_set_seed")
    if parameter_set is None:
        digest.update(b"\x00")
    else:
        digest.update(b"\x01")
        digest.update(parameter_set)
    review_evidence = seed_bytes(vector.get("review_evidence_seed"), "review_evidence_seed")
    if review_evidence is None:
        digest.update(b"\x00")
    else:
        digest.update(b"\x01")
        digest.update(review_evidence)
    digest.update(b"\x01" if vector.get("external_review_completed") else b"\x00")
    digest.update(b"\x01" if vector.get("production_claim_requested") else b"\x00")
    return digest.hexdigest()


def validate_reference_provider(contract: dict[str, Any]) -> None:
    """Validate fixed reference-provider non-claim language."""
    reference = require_mapping(contract.get("reference_provider"), "reference_provider")
    require_equal(reference.get("profile"), REFERENCE_PROFILE, "reference_provider.profile")
    require_equal(reference.get("kind"), "reference-primitive", "reference_provider.kind")
    require_equal(reference.get("rust_type"), "ReferencePrimitiveProvider", "reference_provider.rust_type")
    require_equal(reference.get("capability"), "reference-fixtures", "reference_provider.capability")
    require_equal(
        reference.get("mechanism_catalog_participation"),
        "fixture-coverage-map-only",
        "reference_provider.mechanism_catalog_participation",
    )
    require_equal(
        reference.get("production_claim_allowed"),
        False,
        "reference_provider.production_claim_allowed",
    )
    require_equal(
        reference.get("production_secret_custody_allowed"),
        False,
        "reference_provider.production_secret_custody_allowed",
    )
    primitives = require_mapping(reference.get("fixed_primitives"), "reference_provider.fixed_primitives")
    for primitive_name, primitive_value in primitives.items():
        primitive = require_mapping(primitive_value, f"fixed_primitives.{primitive_name}")
        for key, value in primitive.items():
            if key.endswith("_claim_allowed"):
                require_equal(value, False, f"fixed_primitives.{primitive_name}.{key}")


def validate_provider_kind_rust_variants(contract: dict[str, Any]) -> None:
    """Validate provider-kind labels own their Rust variant mapping in the contract."""
    variants = require_mapping(contract.get("provider_kind_rust_variants"), "provider_kind_rust_variants")
    require_equal(
        variants,
        EXPECTED_PROVIDER_KIND_RUST_VARIANTS,
        "provider_kind_rust_variants",
    )


def validate_external_provider_contract(contract: dict[str, Any]) -> None:
    """Validate external provider production-claim gates."""
    external = require_mapping(contract.get("external_provider_contract"), "external_provider_contract")
    boundary_types = require_mapping(
        external.get("provider_boundary_types"),
        "external_provider_contract.provider_boundary_types",
    )
    secret_custody = require_mapping(
        boundary_types.get("secret_custody_provider"),
        "provider_boundary_types.secret_custody_provider",
    )
    compute = require_mapping(
        boundary_types.get("compute_mechanism_provider"),
        "provider_boundary_types.compute_mechanism_provider",
    )
    require_equal(
        require_string(secret_custody.get("rust_type"), "secret_custody_provider.rust_type"),
        "SecretCustodyProvider",
        "secret_custody_provider.rust_type",
    )
    require_equal(
        require_string(
            secret_custody.get("rust_kind_type"),
            "secret_custody_provider.rust_kind_type",
        ),
        "SecretCustodyProviderKind",
        "secret_custody_provider.rust_kind_type",
    )
    require_equal(
        set(require_string_list(secret_custody.get("provider_kinds"), "secret_custody_provider.provider_kinds")),
        EXPECTED_SECRET_CUSTODY_PROVIDER_KINDS,
        "secret_custody_provider.provider_kinds",
    )
    require_equal(
        require_string(
            secret_custody.get("mechanism_catalog_participation"),
            "secret_custody_provider.mechanism_catalog_participation",
        ),
        "excluded-from-compute-mechanism-catalog",
        "secret_custody_provider.mechanism_catalog_participation",
    )
    require_equal(
        require_string(compute.get("rust_type"), "compute_mechanism_provider.rust_type"),
        "ComputeMechanismProvider",
        "compute_mechanism_provider.rust_type",
    )
    require_equal(
        require_string(
            compute.get("rust_kind_type"),
            "compute_mechanism_provider.rust_kind_type",
        ),
        "ComputeMechanismProviderKind",
        "compute_mechanism_provider.rust_kind_type",
    )
    compute_kinds = set(
        require_string_list(
            compute.get("provider_kinds"),
            "compute_mechanism_provider.provider_kinds",
        )
    )
    require_equal(
        compute_kinds,
        EXPECTED_COMPUTE_PROVIDER_KINDS,
        "compute_mechanism_provider.provider_kinds",
    )
    require_equal(
        require_string(
            compute.get("mechanism_catalog_participation"),
            "compute_mechanism_provider.mechanism_catalog_participation",
        ),
        "required-for-confidential-mechanism-capability-map",
        "compute_mechanism_provider.mechanism_catalog_participation",
    )
    provider_kinds = set(require_string_list(external.get("provider_kinds"), "provider_kinds"))
    require_equal(provider_kinds, EXPECTED_PROVIDER_KINDS, "external_provider_contract.provider_kinds")
    require_equal(
        provider_kinds,
        EXPECTED_SECRET_CUSTODY_PROVIDER_KINDS.union(compute_kinds),
        "external_provider_contract.provider_boundary_types union",
    )
    required_fields = set(require_string_list(external.get("required_binding_fields"), "required_binding_fields"))
    for field in ("provider_id_digest", "artifact_digest", "capability", "profile"):
        if field not in required_fields:
            raise SecretComputeProviderContractError(f"required_binding_fields missing {field}")
    gate = require_mapping(external.get("production_claim_gate"), "production_claim_gate")
    for key, value in gate.items():
        if key == "reference_provider_allowed":
            require_equal(value, False, f"production_claim_gate.{key}")
        else:
            require_equal(value, True, f"production_claim_gate.{key}")


def validate_capability_map_projection(
    contract: dict[str, Any],
    contract_path: Path,
) -> tuple[dict[str, dict[str, Any]], dict[str, dict[str, Any]]]:
    """Validate and derive provider capability-to-mechanism mapping."""
    projection = require_mapping(
        contract.get("capability_map_projection"),
        "capability_map_projection",
    )
    require_equal(
        projection.get("ownership"),
        "generated-from-mechanism-source",
        "capability_map_projection.ownership",
    )
    require_equal(
        projection.get("provider_boundary_type"),
        "ComputeMechanismProvider",
        "capability_map_projection.provider_boundary_type",
    )
    require_equal(
        projection.get("reference_provider_participation"),
        "excluded-from-compute-capability-map",
        "capability_map_projection.reference_provider_participation",
    )
    require_equal(
        projection.get("provider_kind_field"),
        "provider_kind",
        "capability_map_projection.provider_kind_field",
    )
    require_equal(
        projection.get("capability_field"),
        "required_provider_capability",
        "capability_map_projection.capability_field",
    )
    require_equal(
        projection.get("mechanism_field"),
        "maturity_mechanism",
        "capability_map_projection.mechanism_field",
    )
    source = require_path(projection.get("source"), "capability_map_projection.source")
    reference_projection = require_mapping(
        contract.get("reference_capability_map_projection"),
        "reference_capability_map_projection",
    )
    require_equal(
        reference_projection.get("source"),
        projection.get("source"),
        "reference_capability_map_projection.source",
    )
    require_equal(
        reference_projection.get("ownership"),
        "generated-from-mechanism-source",
        "reference_capability_map_projection.ownership",
    )
    require_equal(
        reference_projection.get("provider_boundary_type"),
        "ReferencePrimitiveProvider",
        "reference_capability_map_projection.provider_boundary_type",
    )
    for key in ("provider_kind_field", "capability_field", "mechanism_field"):
        require_equal(
            reference_projection.get(key),
            projection.get(key),
            f"reference_capability_map_projection.{key}",
        )
    rows = validated_mechanism_rows(source, DEFAULT_MATURITY, contract_path)
    capability_map: dict[str, dict[str, Any]] = {}
    reference_capability_map: dict[str, dict[str, Any]] = {}
    reference_kind = require_string(
        require_mapping(contract["reference_provider"], "reference_provider").get("kind"),
        "reference_provider.kind",
    )
    for row in rows:
        provider_kind = str(row["provider_kind"])
        capability = str(row["required_provider_capability"])
        target_map = reference_capability_map if provider_kind == reference_kind else capability_map
        entry = target_map.setdefault(
            provider_kind,
            {
                "required_capability": capability,
                "mechanisms": [],
            },
        )
        require_equal(
            entry["required_capability"],
            capability,
            f"derived capability map for {provider_kind}",
        )
        entry["mechanisms"].append(row["maturity_mechanism"])

    external_contract = require_mapping(
        contract["external_provider_contract"],
        "external_provider_contract",
    )
    external_provider_kinds = set(
        require_string_list(
            require_mapping(
                require_mapping(
                    external_contract.get("provider_boundary_types"),
                    "external_provider_contract.provider_boundary_types",
                ).get("compute_mechanism_provider"),
                "provider_boundary_types.compute_mechanism_provider",
            ).get("provider_kinds"),
            "compute_mechanism_provider.provider_kinds",
        )
    )
    missing_external = sorted(external_provider_kinds.difference(capability_map))
    if missing_external:
        raise SecretComputeProviderContractError(
            "capability_map_projection has no mechanisms for external provider kinds: "
            + ", ".join(missing_external)
        )
    if reference_kind in capability_map:
        raise SecretComputeProviderContractError(
            f"capability_map_projection must exclude reference provider: {reference_kind}"
        )
    if reference_kind not in reference_capability_map:
        raise SecretComputeProviderContractError(
            f"reference_capability_map_projection has no mechanisms for reference provider: {reference_kind}"
        )
    reference_entry = reference_capability_map[reference_kind]
    require_equal(
        reference_entry["required_capability"],
        "reference-fixtures",
        f"reference_capability_map_projection.{reference_kind}.required_capability",
    )
    for entry in capability_map.values():
        entry["mechanisms"] = sorted(entry["mechanisms"])
    for entry in reference_capability_map.values():
        entry["mechanisms"] = sorted(entry["mechanisms"])
    return dict(sorted(capability_map.items())), dict(sorted(reference_capability_map.items()))


def validate_provider_algebra(contract: dict[str, Any]) -> None:
    """Validate the provider capability/evidence/runtime/release algebra."""
    algebra = require_mapping(contract.get("provider_algebra"), "provider_algebra")
    require_equal(
        require_string(algebra.get("expression"), "provider_algebra.expression"),
        "ProviderCapability -> EvidenceGate -> RuntimeBoundary -> ReleaseGate",
        "provider_algebra.expression",
    )
    expected_types = {
        "kind_contract_trait": "SecretComputeProviderKindContract",
        "reference_kind_type": "ReferenceProviderKind",
        "custody_kind_type": "SecretCustodyProviderKind",
        "compute_kind_type": "ComputeMechanismProviderKind",
        "capability_type": "SecretComputeProviderCapability",
        "evidence_gate_type": "SecretComputeProviderEvidenceGate",
        "runtime_boundary_type": "SecretComputeProviderRuntimeBoundary",
        "release_gate_type": "SecretComputeProviderReleaseGate",
    }
    for key, expected_type in expected_types.items():
        require_equal(
            require_string(algebra.get(key), f"provider_algebra.{key}"),
            expected_type,
            f"provider_algebra.{key}",
        )
    require_equal(
        require_string_list(algebra.get("invariant_order"), "provider_algebra.invariant_order"),
        EXPECTED_PROVIDER_ALGEBRA_ORDER,
        "provider_algebra.invariant_order",
    )
    if len(require_string_list(algebra.get("laws"), "provider_algebra.laws")) < 3:
        raise SecretComputeProviderContractError("provider_algebra.laws must contain at least three laws")


def validate_sealed_capability_boundary(contract: dict[str, Any]) -> None:
    """Validate the uniform sealed boundary for reference and external providers."""
    sealed = require_mapping(contract.get("sealed_capability_boundary"), "sealed_capability_boundary")
    require_equal(
        require_string(sealed.get("root_type"), "sealed_capability_boundary.root_type"),
        "SealedCapabilityBoundary",
        "sealed_capability_boundary.root_type",
    )
    require_equal(
        require_string(sealed.get("boundary_trait"), "sealed_capability_boundary.boundary_trait"),
        "SecretComputeProviderKindContract",
        "sealed_capability_boundary.boundary_trait",
    )
    expected_profiles = {"reference-primitive", *EXPECTED_PROVIDER_KINDS}
    require_equal(
        set(require_string_list(sealed.get("provider_profiles"), "sealed_capability_boundary.provider_profiles")),
        expected_profiles,
        "sealed_capability_boundary.provider_profiles",
    )
    require_equal(
        require_string_list(sealed.get("uniform_gate_sequence"), "sealed_capability_boundary.uniform_gate_sequence"),
        require_string_list(require_mapping(contract.get("admission_model"), "admission_model").get("flow"), "admission_model.flow"),
        "sealed_capability_boundary.uniform_gate_sequence",
    )
    reference_rule = require_string(sealed.get("reference_provider_rule"), "sealed_capability_boundary.reference_provider_rule")
    external_rule = require_string(sealed.get("external_provider_rule"), "sealed_capability_boundary.external_provider_rule")
    for token in ("same admission", "fixture-only", "cannot request production"):
        if token not in reference_rule:
            raise SecretComputeProviderContractError(f"sealed_capability_boundary.reference_provider_rule must mention {token}")
    for token in ("same admission", "production claims", "runtime-boundary review"):
        if token not in external_rule:
            raise SecretComputeProviderContractError(f"sealed_capability_boundary.external_provider_rule must mention {token}")
    partition = require_mapping(sealed.get("release_gate_partition"), "sealed_capability_boundary.release_gate_partition")
    require_equal(set(partition), expected_profiles, "sealed_capability_boundary.release_gate_partition keys")
    require_equal(partition.get("reference-primitive"), "fixture-only", "release_gate_partition.reference-primitive")
    for provider_kind in EXPECTED_PROVIDER_KINDS:
        require_equal(
            partition.get(provider_kind),
            "non-production-or-production",
            f"release_gate_partition.{provider_kind}",
        )


def validate_confidential_compute_layer_model(contract: dict[str, Any]) -> None:
    """Validate mechanism-kernel and provider-runtime boundary separation."""
    model = require_mapping(
        contract.get("confidential_compute_layer_model"),
        "confidential_compute_layer_model",
    )
    require_equal(
        require_string(model.get("root_type"), "confidential_compute_layer_model.root_type"),
        "ConfidentialComputeBoundaryModel",
        "confidential_compute_layer_model.root_type",
    )
    require_equal(model.get("layer_count"), 2, "confidential_compute_layer_model.layer_count")
    separation_rule = require_string(
        model.get("separation_rule"),
        "confidential_compute_layer_model.separation_rule",
    )
    for token in ("mechanism kernels", "provider runtime contracts"):
        if token not in separation_rule:
            raise SecretComputeProviderContractError(
                f"confidential_compute_layer_model.separation_rule must mention {token}"
            )
    layers = require_mapping(model.get("layers"), "confidential_compute_layer_model.layers")
    require_equal(
        set(layers),
        {"mechanism_kernel", "provider_runtime_contract"},
        "confidential_compute_layer_model.layers",
    )
    mechanism = require_mapping(layers.get("mechanism_kernel"), "layers.mechanism_kernel")
    runtime = require_mapping(layers.get("provider_runtime_contract"), "layers.provider_runtime_contract")
    mechanism_owns = set(require_string_list(mechanism.get("owns"), "layers.mechanism_kernel.owns"))
    runtime_owns = set(require_string_list(runtime.get("owns"), "layers.provider_runtime_contract.owns"))
    if mechanism_owns.intersection(runtime_owns):
        raise SecretComputeProviderContractError(
            "confidential_compute_layer_model layers must not share owned responsibilities"
        )
    required_mechanism_terms = {"mechanism profile", "admissible operation shape"}
    required_runtime_terms = {"runtime boundary claim", "release gate", "review evidence digest"}
    if not required_mechanism_terms.issubset(mechanism_owns):
        raise SecretComputeProviderContractError("mechanism_kernel.owns missing required mechanism terms")
    if not required_runtime_terms.issubset(runtime_owns):
        raise SecretComputeProviderContractError("provider_runtime_contract.owns missing required runtime terms")
    mechanism_must_not = set(
        require_string_list(
            mechanism.get("must_not_own"),
            "layers.mechanism_kernel.must_not_own",
        )
    )
    runtime_must_not = set(
        require_string_list(
            runtime.get("must_not_own"),
            "layers.provider_runtime_contract.must_not_own",
        )
    )
    if "production secret custody" not in mechanism_must_not:
        raise SecretComputeProviderContractError(
            "mechanism_kernel.must_not_own must exclude production secret custody"
        )
    if "mechanism semantic definition" not in runtime_must_not:
        raise SecretComputeProviderContractError(
            "provider_runtime_contract.must_not_own must exclude mechanism semantic definition"
        )


def validate_binding_digest(contract: dict[str, Any]) -> list[str]:
    """Validate digest profile and deterministic vectors."""
    digest = require_mapping(contract.get("binding_digest"), "binding_digest")
    require_equal(digest.get("digest"), "sha256", "binding_digest.digest")
    require_equal(digest.get("domain"), DOMAIN, "binding_digest.domain")
    require_equal(digest.get("length_prefix"), "uint128-be", "binding_digest.length_prefix")
    vector_ids: list[str] = []
    for vector_value in require_list(contract.get("vectors"), "vectors"):
        vector = require_mapping(vector_value, "vectors[]")
        vector_id = str(vector.get("id"))
        vector_ids.append(vector_id)
        if vector.get("profile") == REFERENCE_PROFILE:
            require_equal(vector.get("kind"), "reference-primitive", f"{vector_id}.kind")
            require_equal(vector.get("capability"), "reference-fixtures", f"{vector_id}.capability")
            require_equal(
                vector.get("production_claim_requested"),
                False,
                f"{vector_id}.production_claim_requested",
            )
        if vector.get("production_claim_requested"):
            require_equal(
                vector.get("external_review_completed"),
                True,
                f"{vector_id}.external_review_completed",
            )
            if vector.get("review_evidence_seed") is None:
                raise SecretComputeProviderContractError(f"{vector_id}.review_evidence_seed is required")
        require_equal(
            vector.get("expected_binding_digest_hex"),
            derive_binding_digest_hex(vector),
            f"{vector_id}.expected_binding_digest_hex",
        )
    return vector_ids


def validate_reexport_retirement(contract: dict[str, Any]) -> None:
    """Validate the provider-vocabulary re-export retirement plan."""
    plan = require_mapping(contract.get("reexport_retirement"), "reexport_retirement")
    status = require_string(plan.get("status"), "reexport_retirement.status")
    if status not in {"active", "removed"}:
        raise SecretComputeProviderContractError(
            "reexport_retirement.status must be active or removed"
        )
    require_equal(
        require_string(plan.get("target_state"), "reexport_retirement.target_state"),
        "provider-vocabulary-imports-use-contract-crate-directly",
        "reexport_retirement.target_state",
    )
    vocabulary_owner = require_path(
        plan.get("vocabulary_owner"),
        "reexport_retirement.vocabulary_owner",
    )
    registry_owner = require_path(
        plan.get("generated_registry_owner"),
        "reexport_retirement.generated_registry_owner",
    )
    compatibility_owner = require_path(
        plan.get("compatibility_reexport_owner"),
        "reexport_retirement.compatibility_reexport_owner",
    )
    require_equal(
        require_string(
            plan.get("compatibility_ledger_entry"),
            "reexport_retirement.compatibility_ledger_entry",
        ),
        "secret-provider-compat-reexports",
        "reexport_retirement.compatibility_ledger_entry",
    )
    require_equal(
        require_string(plan.get("compatibility_surface"), "reexport_retirement.compatibility_surface"),
        "kiva-confidential::mechanism::provider",
        "reexport_retirement.compatibility_surface",
    )
    require_equal(
        require_string(plan.get("direct_import_surface"), "reexport_retirement.direct_import_surface"),
        "kiva-secret-provider-contract",
        "reexport_retirement.direct_import_surface",
    )
    rule = require_string(plan.get("retirement_rule"), "reexport_retirement.retirement_rule")
    for token in ("kiva-secret-provider-contract", "compatibility-only"):
        if token not in rule:
            raise SecretComputeProviderContractError(
                f"reexport_retirement.retirement_rule must mention {token}"
            )
    required_terms = require_mapping(
        plan.get("required_terms"),
        "reexport_retirement.required_terms",
    )
    text_by_owner = {
        "vocabulary_owner": vocabulary_owner.read_text(encoding="utf-8"),
        "generated_registry_owner": registry_owner.read_text(encoding="utf-8"),
        "compatibility_reexport_owner": compatibility_owner.read_text(encoding="utf-8"),
    }
    owner_keys = ["vocabulary_owner"]
    if status == "active":
        owner_keys.append("compatibility_reexport_owner")
    for owner_key in owner_keys:
        owner_terms = require_string_list(
            required_terms.get(owner_key),
            f"reexport_retirement.required_terms.{owner_key}",
        )
        owner_text = text_by_owner[owner_key]
        for term in owner_terms:
            if term not in owner_text:
                raise SecretComputeProviderContractError(
                    f"reexport_retirement.{owner_key} is missing term: {term}"
                )
    if status == "removed":
        removed_terms = require_mapping(
            plan.get("removed_terms"),
            "reexport_retirement.removed_terms",
        )
        compatibility_removed_terms = require_string_list(
            removed_terms.get("compatibility_reexport_owner"),
            "reexport_retirement.removed_terms.compatibility_reexport_owner",
        )
        owner_text = text_by_owner["compatibility_reexport_owner"]
        for term in compatibility_removed_terms:
            if term in owner_text:
                raise SecretComputeProviderContractError(
                    f"reexport_retirement.compatibility_reexport_owner still contains removed term: {term}"
                )
    if "SECRET_COMPUTE_PROVIDER_KIND_REGISTRY" not in text_by_owner["generated_registry_owner"]:
        raise SecretComputeProviderContractError(
            "reexport_retirement.generated_registry_owner must own SECRET_COMPUTE_PROVIDER_KIND_REGISTRY"
        )


def validate_evidence(contract: dict[str, Any]) -> None:
    """Validate evidence references and Rust core terms."""
    evidence = require_mapping(contract.get("evidence"), "evidence")
    for key, path_value in evidence.items():
        require_path(path_value, f"evidence.{key}")
    missing = missing_confidential_rust_terms("secret-compute-provider-contract")
    if missing:
        raise SecretComputeProviderContractError("Rust core is missing terms: " + ", ".join(missing))


def validate_non_goals(contract: dict[str, Any]) -> None:
    """Validate explicit non-goals."""
    non_goals = require_mapping(contract.get("non_goals"), "non_goals")
    for key, value in non_goals.items():
        require_equal(value, False, f"non_goals.{key}")


def validate_contract(contract: dict[str, Any], contract_path: Path = DEFAULT_CONTRACT) -> dict[str, Any]:
    """Validate provider contract and return a report."""
    require_equal(contract.get("schema_version"), 1, "schema_version")
    require_equal(contract.get("kind"), "secret-compute-provider-contract", "kind")
    require_equal(contract.get("status"), "published", "status")
    require_equal(contract.get("milestone"), "CC16", "milestone")
    require_equal(contract.get("profile"), PROFILE, "profile")
    validate_provider_kind_rust_variants(contract)
    validate_reference_provider(contract)
    validate_external_provider_contract(contract)
    capability_map, reference_capability_map = validate_capability_map_projection(
        contract,
        contract_path,
    )
    validate_provider_algebra(contract)
    validate_sealed_capability_boundary(contract)
    validate_confidential_compute_layer_model(contract)
    vectors = validate_binding_digest(contract)
    validate_reexport_retirement(contract)
    validate_evidence(contract)
    validate_non_goals(contract)
    return {
        "schema_version": 1,
        "kind": "secret-compute-provider-contract-validation",
        "status": "passed",
        "profile": PROFILE,
        "reference_profile": REFERENCE_PROFILE,
        "sealed_capability_boundary": contract["sealed_capability_boundary"],
        "capability_map": capability_map,
        "reference_capability_map": reference_capability_map,
        "vectors": vectors,
    }


def main() -> int:
    """Run the provider contract validator."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    report = validate_contract(load_yaml(args.contract), args.contract)
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
