#!/usr/bin/env python3
"""Validate confidential capability taxonomy/provider vocabulary alignment."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string
from kiva_ci.projection import require_envelope


DEFAULT_TAXONOMY = Path("spec/confidential/capability-taxonomy.source.yaml")
DEFAULT_BINDINGS = Path("spec/confidential/capability-bindings.source.yaml")
DEFAULT_MECHANISMS = Path("spec/confidential/mechanism-catalog.source.yaml")
DEFAULT_PROVIDER = Path("spec/confidential/secret-compute-provider-contract.yaml")


class CapabilityTaxonomyError(RuntimeError):
    """Raised when confidential capability taxonomy validation fails."""


def unique_ids(rows: Any, label: str) -> set[str]:
    """Return unique row ids."""
    ids: set[str] = set()
    for index, raw_row in enumerate(require_list(rows, label, CapabilityTaxonomyError)):
        row = require_mapping(raw_row, f"{label}[{index}]", CapabilityTaxonomyError)
        row_id = require_string(row.get("id"), f"{label}[{index}].id", CapabilityTaxonomyError)
        if row_id in ids:
            raise CapabilityTaxonomyError(f"duplicate {label} id: {row_id}")
        ids.add(row_id)
    return ids


def taxonomy_capabilities(taxonomy: dict[str, Any], taxonomy_path: Path) -> set[str]:
    """Return declared taxonomy capability ids."""
    require_envelope(
        taxonomy,
        path=taxonomy_path,
        kind="kiva-confidential-capability-taxonomy-source",
        error_cls=CapabilityTaxonomyError,
    )
    port_model = require_mapping(
        taxonomy.get("capability_port_model"),
        f"{taxonomy_path}.capability_port_model",
        CapabilityTaxonomyError,
    )
    fields = {
        require_string(value, f"{taxonomy_path}.capability_port_model.port_fields[]", CapabilityTaxonomyError)
        for value in require_list(
            port_model.get("port_fields"),
            f"{taxonomy_path}.capability_port_model.port_fields",
            CapabilityTaxonomyError,
        )
    }
    if "sealed_provider_boundary" not in fields:
        raise CapabilityTaxonomyError("CapabilityPort must include sealed_provider_boundary")
    return unique_ids(taxonomy.get("capabilities"), "taxonomy.capabilities")


def binding_capabilities(bindings: dict[str, Any], bindings_path: Path) -> set[str]:
    """Return capabilities referenced by CapabilityBinding rows."""
    require_envelope(
        bindings,
        path=bindings_path,
        kind="kiva-confidential-capability-bindings-source",
        error_cls=CapabilityTaxonomyError,
    )
    capabilities: set[str] = set()
    seen: set[str] = set()
    for index, raw_binding in enumerate(
        require_list(bindings.get("bindings"), "bindings.bindings", CapabilityTaxonomyError)
    ):
        binding = require_mapping(raw_binding, f"bindings.bindings[{index}]", CapabilityTaxonomyError)
        binding_id = require_string(binding.get("id"), f"bindings.bindings[{index}].id", CapabilityTaxonomyError)
        if binding_id in seen:
            raise CapabilityTaxonomyError(f"duplicate binding id: {binding_id}")
        seen.add(binding_id)
        capabilities.add(
            require_string(
                binding.get("capability"),
                f"bindings.bindings[{index}].capability",
                CapabilityTaxonomyError,
            )
        )
    return capabilities


def mechanism_provider_capabilities(mechanisms: dict[str, Any]) -> tuple[set[str], set[str]]:
    """Return reference and external provider capabilities from mechanism catalog."""
    if mechanisms.get("schema_version") != 1:
        raise CapabilityTaxonomyError("mechanism catalog schema_version must be 1")
    if mechanisms.get("kind") != "kiva-confidential-mechanism-catalog-source":
        raise CapabilityTaxonomyError("mechanism catalog kind must be kiva-confidential-mechanism-catalog-source")
    reference: set[str] = set()
    external: set[str] = set()
    for index, raw_mechanism in enumerate(
        require_list(mechanisms.get("mechanisms"), "mechanisms.mechanisms", CapabilityTaxonomyError)
    ):
        mechanism = require_mapping(raw_mechanism, f"mechanisms.mechanisms[{index}]", CapabilityTaxonomyError)
        provider_kind = require_string(
            mechanism.get("provider_kind"),
            f"mechanisms.mechanisms[{index}].provider_kind",
            CapabilityTaxonomyError,
        )
        capability = require_string(
            mechanism.get("required_provider_capability"),
            f"mechanisms.mechanisms[{index}].required_provider_capability",
            CapabilityTaxonomyError,
        )
        if provider_kind == "reference-primitive":
            reference.add(capability)
        else:
            external.add(capability)
    return reference, external


def validate_provider_projection(provider: dict[str, Any]) -> None:
    """Validate that the provider contract exposes the expected capability projection fields."""
    if provider.get("kind") != "secret-compute-provider-contract":
        raise CapabilityTaxonomyError("provider contract kind must be secret-compute-provider-contract")
    projection = require_mapping(
        provider.get("capability_map_projection"),
        "provider.capability_map_projection",
        CapabilityTaxonomyError,
    )
    if projection.get("capability_field") != "required_provider_capability":
        raise CapabilityTaxonomyError("provider capability projection must use required_provider_capability")
    reference = require_mapping(
        provider.get("reference_provider"),
        "provider.reference_provider",
        CapabilityTaxonomyError,
    )
    if reference.get("capability") != "reference-fixtures":
        raise CapabilityTaxonomyError("reference provider capability must be reference-fixtures")
    sealed = require_mapping(
        provider.get("sealed_capability_boundary"),
        "provider.sealed_capability_boundary",
        CapabilityTaxonomyError,
    )
    if sealed.get("root_type") != "SealedCapabilityBoundary":
        raise CapabilityTaxonomyError("provider sealed boundary must be SealedCapabilityBoundary")


def validate_taxonomy_alignment(
    taxonomy_path: Path,
    bindings_path: Path,
    mechanisms_path: Path,
    provider_path: Path,
) -> None:
    """Validate taxonomy, bindings, mechanism catalog, and provider contract alignment."""
    taxonomy = load_yaml(taxonomy_path, CapabilityTaxonomyError)
    bindings = load_yaml(bindings_path, CapabilityTaxonomyError)
    mechanisms = load_yaml(mechanisms_path, CapabilityTaxonomyError)
    provider = load_yaml(provider_path, CapabilityTaxonomyError)

    taxonomy_ids = taxonomy_capabilities(taxonomy, taxonomy_path)
    binding_ids = binding_capabilities(bindings, bindings_path)
    reference_capabilities, external_capabilities = mechanism_provider_capabilities(mechanisms)
    validate_provider_projection(provider)

    unknown_binding_ids = sorted(binding_ids.difference(taxonomy_ids))
    if unknown_binding_ids:
        raise CapabilityTaxonomyError(f"CapabilityBinding references unknown taxonomy capabilities: {unknown_binding_ids}")
    unknown_external = sorted(external_capabilities.difference(taxonomy_ids))
    if unknown_external:
        raise CapabilityTaxonomyError(f"mechanism catalog external capabilities are missing from taxonomy: {unknown_external}")
    if reference_capabilities != {"reference-fixtures"}:
        raise CapabilityTaxonomyError(f"reference provider capabilities must be exactly reference-fixtures: {sorted(reference_capabilities)}")


def main() -> int:
    """Validate confidential capability taxonomy alignment."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--taxonomy", type=Path, default=DEFAULT_TAXONOMY)
    parser.add_argument("--bindings", type=Path, default=DEFAULT_BINDINGS)
    parser.add_argument("--mechanisms", type=Path, default=DEFAULT_MECHANISMS)
    parser.add_argument("--provider", type=Path, default=DEFAULT_PROVIDER)
    args = parser.parse_args()

    validate_taxonomy_alignment(args.taxonomy, args.bindings, args.mechanisms, args.provider)
    print("validated confidential capability taxonomy/provider vocabulary alignment")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
