#!/usr/bin/env python3
"""Generate secret-compute provider-kind registry artifacts from the contract."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.manifest import load_yaml, require_mapping, require_string, require_string_list
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_CONTRACT = Path("spec/confidential/secret-compute-provider-contract.yaml")
DEFAULT_LEDGER = Path("spec/confidential/secret-compute-provider-kind-registry.yaml")
DEFAULT_RUST = Path("crates/kiva-secret-provider-contract/src/kind_registry.rs")


class ProviderKindRegistryGenerationError(RuntimeError):
    """Raised when provider-kind registry artifacts cannot be generated."""


def provider_kind_entries(contract: dict[str, Any]) -> list[dict[str, Any]]:
    """Return normalized provider-kind registry entries from the contract."""
    rust_variants = require_mapping(
        contract.get("provider_kind_rust_variants"),
        "provider_kind_rust_variants",
        ProviderKindRegistryGenerationError,
    )
    reference = require_mapping(contract.get("reference_provider"), "reference_provider", ProviderKindRegistryGenerationError)
    external = require_mapping(
        contract.get("external_provider_contract"),
        "external_provider_contract",
        ProviderKindRegistryGenerationError,
    )
    boundary_types = require_mapping(
        external.get("provider_boundary_types"),
        "external_provider_contract.provider_boundary_types",
        ProviderKindRegistryGenerationError,
    )
    entries = [
        {
            "kind": require_string(reference.get("kind"), "reference_provider.kind", ProviderKindRegistryGenerationError),
            "boundary_type": "reference_provider",
            "rust_marker_type": require_string(
                reference.get("rust_type"),
                "reference_provider.rust_type",
                ProviderKindRegistryGenerationError,
            ),
            "rust_kind_type": "ReferenceProviderKind",
            "mechanism_catalog_participation": require_string(
                reference.get("mechanism_catalog_participation"),
                "reference_provider.mechanism_catalog_participation",
                ProviderKindRegistryGenerationError,
            ),
            "production_claim_allowed": bool(reference.get("production_claim_allowed")),
        }
    ]
    for boundary_id in ("secret_custody_provider", "compute_mechanism_provider"):
        boundary = require_mapping(
            boundary_types.get(boundary_id),
            f"external_provider_contract.provider_boundary_types.{boundary_id}",
            ProviderKindRegistryGenerationError,
        )
        for kind in require_string_list(
            boundary.get("provider_kinds"),
            f"{boundary_id}.provider_kinds",
            ProviderKindRegistryGenerationError,
        ):
            entries.append(
                {
                    "kind": kind,
                    "boundary_type": boundary_id,
                    "rust_marker_type": require_string(
                        boundary.get("rust_type"),
                        f"{boundary_id}.rust_type",
                        ProviderKindRegistryGenerationError,
                    ),
                    "rust_kind_type": require_string(
                        boundary.get("rust_kind_type"),
                        f"{boundary_id}.rust_kind_type",
                        ProviderKindRegistryGenerationError,
                    ),
                    "mechanism_catalog_participation": require_string(
                        boundary.get("mechanism_catalog_participation"),
                        f"{boundary_id}.mechanism_catalog_participation",
                        ProviderKindRegistryGenerationError,
                    ),
                    "production_claim_allowed": boundary_id == "compute_mechanism_provider",
                }
            )
    seen: set[str] = set()
    for entry in entries:
        kind = entry["kind"]
        variant = require_string(
            rust_variants.get(kind),
            f"provider_kind_rust_variants.{kind}",
            ProviderKindRegistryGenerationError,
        )
        if kind in seen:
            raise ProviderKindRegistryGenerationError(f"duplicate provider kind: {kind}")
        seen.add(kind)
        entry["rust_variant"] = variant
    missing_variants = sorted(set(rust_variants).difference(seen))
    if missing_variants:
        raise ProviderKindRegistryGenerationError(
            f"provider_kind_rust_variants contains entries that are not provider kinds: {missing_variants}"
        )
    return entries


def registry_ledger_document(contract: dict[str, Any], contract_path: Path) -> dict[str, Any]:
    """Return generated provider-kind registry ledger."""
    if contract.get("schema_version") != 1:
        raise ProviderKindRegistryGenerationError(f"{contract_path}.schema_version must be 1")
    if contract.get("kind") != "secret-compute-provider-contract":
        raise ProviderKindRegistryGenerationError(f"{contract_path}.kind must be secret-compute-provider-contract")
    return {
        "schema_version": 1,
        "kind": "secret-compute-provider-kind-registry",
        "status": "generated",
        "generated_from": [contract_path.as_posix()],
        "entries": provider_kind_entries(contract),
    }


def render_rust(ledger: dict[str, Any]) -> str:
    """Render generated Rust provider-kind registry."""
    rows = []
    for entry in ledger["entries"]:
        rows.append(
            "    SecretComputeProviderKindRegistryEntry {\n"
            f"        label: \"{entry['kind']}\",\n"
            f"        kind: SecretComputeProviderKind::{entry['rust_variant']},\n"
            f"        boundary_type: \"{entry['boundary_type']}\",\n"
            f"        rust_marker_type: \"{entry['rust_marker_type']}\",\n"
            f"        rust_kind_type: \"{entry['rust_kind_type']}\",\n"
            "        mechanism_catalog_participation: "
            f"\"{entry['mechanism_catalog_participation']}\",\n"
            f"        production_claim_allowed: {str(entry['production_claim_allowed']).lower()},\n"
            "    },"
        )
    return (
        "//! Generated secret-compute provider-kind registry.\n"
        "//!\n"
        "//! Generated by `scripts/ci/generate_secret_compute_provider_kind_registry.py`.\n"
        "\n"
        "use crate::SecretComputeProviderKind;\n"
        "\n"
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n"
        "/// One generated provider-kind registry entry.\n"
        "pub struct SecretComputeProviderKindRegistryEntry {\n"
        "    /// Stable provider-kind label from the provider contract.\n"
        "    pub label: &'static str,\n"
        "    /// Rust provider-kind enum variant.\n"
        "    pub kind: SecretComputeProviderKind,\n"
        "    /// Contract boundary family that owns this provider kind.\n"
        "    pub boundary_type: &'static str,\n"
        "    /// Rust marker type used for this boundary family.\n"
        "    pub rust_marker_type: &'static str,\n"
        "    /// Rust enum family that classifies this provider kind.\n"
        "    pub rust_kind_type: &'static str,\n"
        "    /// Mechanism-catalog participation policy for this provider kind.\n"
        "    pub mechanism_catalog_participation: &'static str,\n"
        "    /// Whether this provider kind may back a production security claim.\n"
        "    pub production_claim_allowed: bool,\n"
        "}\n"
        "\n"
        "/// Generated registry of all provider kinds declared by the contract.\n"
        "pub const SECRET_COMPUTE_PROVIDER_KIND_REGISTRY: &[SecretComputeProviderKindRegistryEntry] = &[\n"
        + "\n".join(rows)
        + "\n];\n"
        "\n"
        "#[must_use]\n"
        "/// Returns the generated registry entry for a provider kind.\n"
        "pub fn registry_entry_for_provider_kind(\n"
        "    kind: SecretComputeProviderKind,\n"
        ") -> Option<&'static SecretComputeProviderKindRegistryEntry> {\n"
        "    SECRET_COMPUTE_PROVIDER_KIND_REGISTRY\n"
        "        .iter()\n"
        "        .find(|entry| entry.kind == kind)\n"
        "}\n"
        "\n"
        "#[must_use]\n"
        "/// Returns the generated provider-kind label.\n"
        "pub fn provider_kind_label(kind: SecretComputeProviderKind) -> &'static str {\n"
        "    registry_entry_for_provider_kind(kind).map_or(\"\", |entry| entry.label)\n"
        "}\n"
        "\n"
        "#[must_use]\n"
        "/// Parses a provider-kind label through the generated registry.\n"
        "pub fn provider_kind_from_label(value: &str) -> Option<SecretComputeProviderKind> {\n"
        "    SECRET_COMPUTE_PROVIDER_KIND_REGISTRY\n"
        "        .iter()\n"
        "        .find(|entry| entry.label == value)\n"
        "        .map(|entry| entry.kind)\n"
        "}\n"
    )


def render_outputs(root: Path, contract_path: Path) -> tuple[str, str]:
    """Render YAML and Rust registry outputs."""
    contract = load_yaml(root / contract_path, ProviderKindRegistryGenerationError)
    ledger = registry_ledger_document(contract, contract_path)
    return dump_yaml(ledger), render_rust(ledger)


def main() -> int:
    """Generate or validate provider-kind registry artifacts."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    parser.add_argument("--ledger-out", type=Path, default=DEFAULT_LEDGER)
    parser.add_argument("--rust-out", type=Path, default=DEFAULT_RUST)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    ledger_text, rust_text = render_outputs(root, args.contract)
    artifacts = [
        GeneratedArtifact(root / args.ledger_out, ledger_text),
        GeneratedArtifact(root / args.rust_out, rust_text),
    ]
    command_hint = "scripts/ci/generate_secret_compute_provider_kind_registry.py"
    try:
        write_or_check_generated_artifacts(
            artifacts,
            check=args.check,
            command_hint=command_hint,
            error_cls=ProviderKindRegistryGenerationError,
        )
    except ProviderKindRegistryGenerationError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated provider-kind registry: {args.ledger_out}, {args.rust_out}")
        return 0
    print(f"generated provider-kind registry: {args.ledger_out}, {args.rust_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
