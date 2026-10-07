"""Semantic model for Kiva confidential mechanism catalog generation."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import yaml

from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string


SOURCE_KIND = "kiva-confidential-mechanism-catalog-source"
CATALOG_KIND = "kiva-confidential-mechanism-catalog"
RUST_KIND_PREFIX = "ConfidentialMechanismKind::"
RUST_TECHNIQUE_PREFIX = "Technique::"
RUST_CAPABILITY_PREFIX = "SecretComputeProviderCapability::"


class MechanismCatalogError(RuntimeError):
    """Raised when mechanism catalog artifacts cannot be generated."""


def kebab_to_pascal(value: str) -> str:
    """Convert a stable kebab-case label into a Rust-style PascalCase variant."""
    parts = value.split("-")
    if not parts or any(not part for part in parts):
        raise MechanismCatalogError(f"invalid kebab-case label: {value}")
    return "".join(part.capitalize() for part in parts)


def rust_kind(mechanism_id: str) -> str:
    """Return the Rust mechanism kind projected from a semantic mechanism id."""
    return f"{RUST_KIND_PREFIX}{kebab_to_pascal(mechanism_id)}"


def rust_technique(technique: str) -> str:
    """Return the Rust technique variant projected from a semantic technique id."""
    return f"{RUST_TECHNIQUE_PREFIX}{kebab_to_pascal(technique)}"


def rust_capability(capability: str) -> str:
    """Return the Rust capability variant projected from a semantic capability id."""
    return f"{RUST_CAPABILITY_PREFIX}{kebab_to_pascal(capability)}"


def maturity_ids(path: Path) -> set[str]:
    """Return mechanism ids declared by the maturity ledger."""
    document = load_yaml(path, MechanismCatalogError)
    mechanisms = require_list(document.get("mechanisms"), f"{path}.mechanisms", MechanismCatalogError)
    result: set[str] = set()
    for index, value in enumerate(mechanisms):
        mechanism = require_mapping(value, f"{path}.mechanisms[{index}]", MechanismCatalogError)
        result.add(require_string(mechanism.get("id"), f"{path}.mechanisms[{index}].id", MechanismCatalogError))
    return result


def require_bool(value: Any, label: str) -> bool:
    """Return a boolean or raise a catalog validation error."""
    if not isinstance(value, bool):
        raise MechanismCatalogError(f"{label} must be a boolean")
    return value


def provider_contract_kinds(path: Path) -> set[str]:
    """Return provider kinds admitted by the provider contract."""
    contract = load_yaml(path, MechanismCatalogError)
    external = require_mapping(
        contract.get("external_provider_contract"),
        f"{path}.external_provider_contract",
        MechanismCatalogError,
    )
    provider_kinds = set(
        require_list(
            external.get("provider_kinds"),
            f"{path}.external_provider_contract.provider_kinds",
            MechanismCatalogError,
        )
    )
    if not all(isinstance(item, str) and item for item in provider_kinds):
        raise MechanismCatalogError(
            f"{path}.external_provider_contract.provider_kinds must be non-empty strings"
        )
    reference = require_mapping(
        contract.get("reference_provider"),
        f"{path}.reference_provider",
        MechanismCatalogError,
    )
    provider_kinds.add(
        require_string(reference.get("kind"), f"{path}.reference_provider.kind", MechanismCatalogError)
    )
    return provider_kinds


def reference_provider_kind(path: Path) -> str:
    """Return the reference-provider kind admitted by the provider contract."""
    contract = load_yaml(path, MechanismCatalogError)
    reference = require_mapping(
        contract.get("reference_provider"),
        f"{path}.reference_provider",
        MechanismCatalogError,
    )
    return require_string(reference.get("kind"), f"{path}.reference_provider.kind", MechanismCatalogError)


def validate_source_header(source: dict[str, Any], source_path: Path, maturity_path: Path, provider_path: Path) -> None:
    """Validate source ledger envelope and referenced ledgers."""
    if source.get("schema_version") != 1:
        raise MechanismCatalogError(f"{source_path}.schema_version must be 1")
    if source.get("kind") != SOURCE_KIND:
        raise MechanismCatalogError(f"{source_path}.kind must be {SOURCE_KIND}")
    if source.get("status") != "active":
        raise MechanismCatalogError(f"{source_path}.status must be active")
    expected_maturity = require_string(
        source.get("maturity_ledger"),
        f"{source_path}.maturity_ledger",
        MechanismCatalogError,
    )
    if expected_maturity != maturity_path.as_posix():
        raise MechanismCatalogError(
            f"{source_path}.maturity_ledger must match --maturity: {maturity_path.as_posix()}"
        )
    expected_provider = require_string(
        source.get("provider_contract"),
        f"{source_path}.provider_contract",
        MechanismCatalogError,
    )
    if expected_provider != provider_path.as_posix():
        raise MechanismCatalogError(
            f"{source_path}.provider_contract must match --provider-contract: {provider_path.as_posix()}"
        )
    algebra = require_mapping(
        source.get("provider_algebra"),
        f"{source_path}.provider_algebra",
        MechanismCatalogError,
    )
    expected_algebra = {
        "reference_provider_boundary_type": "ReferencePrimitiveProvider",
        "compute_provider_boundary_type": "ComputeMechanismProvider",
        "reference_provider_capability_map": "fixture-coverage-map",
        "compute_provider_capability_map": "external-compute-capability-map",
        "mechanism_field": "maturity_mechanism",
        "provider_kind_field": "provider_kind",
        "capability_field": "required_provider_capability",
    }
    for key, expected in expected_algebra.items():
        actual = require_string(
            algebra.get(key),
            f"{source_path}.provider_algebra.{key}",
            MechanismCatalogError,
        )
        if actual != expected:
            raise MechanismCatalogError(
                f"{source_path}.provider_algebra.{key} must be {expected}, got {actual}"
            )


def validated_mechanism_rows(
    source_path: Path,
    maturity_path: Path,
    provider_path: Path,
) -> list[dict[str, Any]]:
    """Load and validate human-owned semantic mechanism source rows."""
    document = load_yaml(source_path, MechanismCatalogError)
    validate_source_header(document, source_path, maturity_path, provider_path)
    maturity_id_set = maturity_ids(maturity_path)
    provider_kinds = provider_contract_kinds(provider_path)
    reference_kind = reference_provider_kind(provider_path)

    rows: list[dict[str, Any]] = []
    seen_ids: set[str] = set()
    capabilities_by_provider_kind: dict[str, str] = {}
    for index, value in enumerate(
        require_list(document.get("mechanisms"), f"{source_path}.mechanisms", MechanismCatalogError)
    ):
        label = f"{source_path}.mechanisms[{index}]"
        mechanism = require_mapping(value, label, MechanismCatalogError)
        mechanism_id = require_string(mechanism.get("id"), f"{label}.id", MechanismCatalogError)
        if mechanism_id in seen_ids:
            raise MechanismCatalogError(f"duplicate confidential mechanism id: {mechanism_id}")
        seen_ids.add(mechanism_id)
        technique = require_string(mechanism.get("technique"), f"{label}.technique", MechanismCatalogError)
        provider_kind = require_string(
            mechanism.get("provider_kind"),
            f"{label}.provider_kind",
            MechanismCatalogError,
        )
        if provider_kind not in provider_kinds:
            raise MechanismCatalogError(
                f"{mechanism_id} provider kind is not admitted by provider contract: {provider_kind}"
            )
        capability = require_string(
            mechanism.get("required_provider_capability"),
            f"{label}.required_provider_capability",
            MechanismCatalogError,
        )
        maturity_id = require_string(
            mechanism.get("maturity_mechanism"),
            f"{label}.maturity_mechanism",
            MechanismCatalogError,
        )
        if maturity_id not in maturity_id_set:
            raise MechanismCatalogError(f"{mechanism_id} maturity mapping is missing: {maturity_id}")
        existing_capability = capabilities_by_provider_kind.setdefault(provider_kind, capability)
        if existing_capability != capability:
            raise MechanismCatalogError(
                f"{provider_kind} maps to multiple provider capabilities: "
                f"{existing_capability}, {capability}"
            )

        rows.append(
            {
                "id": mechanism_id,
                "rust_kind": rust_kind(mechanism_id),
                "technique": technique,
                "rust_technique": rust_technique(technique),
                "provider_kind": provider_kind,
                "provider_boundary_type": "ReferencePrimitiveProvider"
                if provider_kind == reference_kind
                else "ComputeMechanismProvider",
                "required_provider_capability": capability,
                "rust_required_provider_capability": rust_capability(capability),
                "production_provider_review_required": require_bool(
                    mechanism.get("production_provider_review_required"),
                    f"{label}.production_provider_review_required",
                ),
                "requires_all_participants": require_bool(
                    mechanism.get("requires_all_participants"),
                    f"{label}.requires_all_participants",
                ),
                "maturity_mechanism": maturity_id,
            }
        )
    if not rows:
        raise MechanismCatalogError(f"{source_path}.mechanisms must not be empty")
    return rows


def mechanism_catalog_document(
    *,
    source_path: Path,
    maturity_path: Path,
    provider_path: Path,
) -> dict[str, Any]:
    """Return the generated mechanism catalog document."""
    return {
        "schema_version": 1,
        "kind": CATALOG_KIND,
        "status": "active",
        "generated_from": [
            source_path.as_posix(),
            maturity_path.as_posix(),
            provider_path.as_posix(),
        ],
        "description": "Generated confidential mechanism descriptor catalog projected from the semantic mechanism/provider algebra source.",
        "mechanisms": validated_mechanism_rows(source_path, maturity_path, provider_path),
    }


def render_yaml(document: dict[str, Any]) -> str:
    """Render a deterministic YAML document."""
    return yaml.safe_dump(document, sort_keys=False, allow_unicode=False, width=1000)
