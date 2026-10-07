#!/usr/bin/env python3
"""Generate the confidential MechanismProfile review index."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.manifest import (
    load_json,
    load_yaml,
    require_list,
    require_mapping,
    require_string,
)
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_MECHANISM_CATALOG = Path("spec/confidential/mechanism-catalog.yaml")
DEFAULT_MECHANISM_PACKAGES = Path("spec/confidential/mechanism-packages.source.yaml")
DEFAULT_MATURITY = Path("spec/confidential/mechanism-maturity.yaml")
DEFAULT_ASSURANCE_REGISTRY = Path("spec/confidential/assurance-evidence-registry.yaml")
DEFAULT_VERIFIED_KERNELS = Path("spec/verification/confidential-verified-kernels.yaml")
DEFAULT_PROVIDER_CONTRACT = Path("spec/confidential/secret-compute-provider-contract.yaml")
DEFAULT_OPERATION_KERNEL = Path("generated/architecture/operation-kernel.yaml")
DEFAULT_CAPABILITY_BINDINGS = Path("generated/confidential/capability-binding-index.yaml")
DEFAULT_OUTPUT = Path("generated/confidential/mechanism-profile-index.yaml")
DEFAULT_MARKDOWN = Path("docs/architecture/generated/confidential-mechanism-profile-index.md")
DEFAULT_SCHEMA = Path("spec/confidential/schemas/mechanism-profile-index.schema.json")


class MechanismProfileIndexError(RuntimeError):
    """Raised when the mechanism profile index cannot be generated."""


def rows_by_id(document: dict[str, Any], field: str, label: str) -> dict[str, dict[str, Any]]:
    """Return rows keyed by id."""
    rows: dict[str, dict[str, Any]] = {}
    for index, value in enumerate(require_list(document.get(field), f"{label}.{field}", MechanismProfileIndexError)):
        row = require_mapping(value, f"{label}.{field}[{index}]", MechanismProfileIndexError)
        row_id = require_string(row.get("id"), f"{label}.{field}[{index}].id", MechanismProfileIndexError)
        if row_id in rows:
            raise MechanismProfileIndexError(f"duplicate {label}.{field} id: {row_id}")
        rows[row_id] = row
    return rows


def assurance_entries_by_milestone(registry: dict[str, Any]) -> dict[str, list[dict[str, Any]]]:
    """Return assurance registry entries grouped by milestone."""
    entries = require_mapping(registry.get("entries"), "assurance_registry.entries", MechanismProfileIndexError)
    grouped: dict[str, list[dict[str, Any]]] = {}
    for entry_id, raw_entry in entries.items():
        entry = require_mapping(raw_entry, f"assurance_registry.entries.{entry_id}", MechanismProfileIndexError)
        milestone = require_string(
            entry.get("milestone"),
            f"assurance_registry.entries.{entry_id}.milestone",
            MechanismProfileIndexError,
        )
        grouped.setdefault(milestone, []).append(
            {
                "id": str(entry_id),
                "profile": require_string(
                    entry.get("profile"),
                    f"assurance_registry.entries.{entry_id}.profile",
                    MechanismProfileIndexError,
                ),
                "evidence_count": len(
                    require_list(
                        entry.get("evidence"),
                        f"assurance_registry.entries.{entry_id}.evidence",
                        MechanismProfileIndexError,
                    )
                ),
            }
        )
    return grouped


def kernel_matches_maturity(kernel_mechanism: str, maturity_mechanism: str) -> bool:
    """Return whether a verified kernel row belongs to one maturity mechanism."""
    if kernel_mechanism == maturity_mechanism:
        return True
    if maturity_mechanism.startswith("fhe-") and kernel_mechanism == "fhe":
        return True
    if maturity_mechanism.startswith("psi-") and kernel_mechanism == "private-set-intersection":
        return True
    if maturity_mechanism.startswith("tee-") and kernel_mechanism == "tee-attestation":
        return True
    return maturity_mechanism == "threshold-frost-signing" and kernel_mechanism == "threshold-frost-signing"


def verified_kernel_refs(kernels: dict[str, Any], maturity_mechanism: str) -> list[dict[str, Any]]:
    """Return verified-kernel rows attached to one maturity mechanism."""
    refs: list[dict[str, Any]] = []
    for index, value in enumerate(
        require_list(kernels.get("kernels"), "verified_kernels.kernels", MechanismProfileIndexError)
    ):
        kernel = require_mapping(value, f"verified_kernels.kernels[{index}]", MechanismProfileIndexError)
        mechanism = require_string(
            kernel.get("mechanism"),
            f"verified_kernels.kernels[{index}].mechanism",
            MechanismProfileIndexError,
        )
        if not kernel_matches_maturity(mechanism, maturity_mechanism):
            continue
        refs.append(
            {
                "id": require_string(kernel.get("id"), f"verified_kernels.kernels[{index}].id", MechanismProfileIndexError),
                "mechanism": mechanism,
                "required_evidence_kinds": list(
                    require_list(
                        kernel.get("required_evidence_kinds"),
                        f"verified_kernels.kernels[{index}].required_evidence_kinds",
                        MechanismProfileIndexError,
                    )
                ),
            }
        )
    return refs


def capability_bindings_by_maturity(bindings: dict[str, Any]) -> dict[str, dict[str, Any]]:
    """Return generated CapabilityBinding rows keyed by maturity mechanism."""
    rows: dict[str, dict[str, Any]] = {}
    for index, value in enumerate(
        require_list(bindings.get("bindings"), "capability_bindings.bindings", MechanismProfileIndexError)
    ):
        row = require_mapping(value, f"capability_bindings.bindings[{index}]", MechanismProfileIndexError)
        maturity = require_string(
            row.get("maturity_mechanism"),
            f"capability_bindings.bindings[{index}].maturity_mechanism",
            MechanismProfileIndexError,
        )
        if maturity in rows:
            raise MechanismProfileIndexError(f"duplicate CapabilityBinding maturity_mechanism: {maturity}")
        rows[maturity] = row
    return rows


def capability_binding_ref(binding: dict[str, Any]) -> dict[str, Any]:
    """Return the compact CapabilityBinding reference embedded in a profile."""
    return {
        "id": require_string(binding.get("id"), "capability_binding.id", MechanismProfileIndexError),
        "capability": require_string(
            binding.get("capability"),
            "capability_binding.capability",
            MechanismProfileIndexError,
        ),
        "binding_kind": require_string(
            binding.get("binding_kind"),
            "capability_binding.binding_kind",
            MechanismProfileIndexError,
        ),
        "operation_count": len(
            require_list(
                binding.get("operation_refs"),
                "capability_binding.operation_refs",
                MechanismProfileIndexError,
            )
        ),
    }


def operation_refs_from_binding(binding: dict[str, Any]) -> list[dict[str, str]]:
    """Return OperationKernel HTTP operations attached by CapabilityBinding."""
    return [
        dict(require_mapping(value, "capability_binding.operation_refs[]", MechanismProfileIndexError))
        for value in require_list(
            binding.get("operation_refs"),
            "capability_binding.operation_refs",
            MechanismProfileIndexError,
        )
    ]


def profile_index_document(
    mechanism_packages_path: Path,
    mechanism_catalog_path: Path,
    maturity_path: Path,
    assurance_registry_path: Path,
    verified_kernels_path: Path,
    provider_contract_path: Path,
    operation_kernel_path: Path,
    capability_bindings_path: Path,
    output_path: Path,
    markdown_path: Path,
) -> dict[str, Any]:
    """Return the generated MechanismProfile index."""
    mechanism_packages = load_yaml(mechanism_packages_path, MechanismProfileIndexError)
    mechanism_catalog = load_yaml(mechanism_catalog_path, MechanismProfileIndexError)
    maturity = load_yaml(maturity_path, MechanismProfileIndexError)
    assurance_registry = load_yaml(assurance_registry_path, MechanismProfileIndexError)
    verified_kernels = load_yaml(verified_kernels_path, MechanismProfileIndexError)
    provider_contract = load_yaml(provider_contract_path, MechanismProfileIndexError)
    load_yaml(operation_kernel_path, MechanismProfileIndexError)
    capability_bindings = load_yaml(capability_bindings_path, MechanismProfileIndexError)
    maturity_by_id = rows_by_id(maturity, "mechanisms", "maturity")
    assurance_by_milestone = assurance_entries_by_milestone(assurance_registry)
    bindings_by_maturity = capability_bindings_by_maturity(capability_bindings)
    sealed_boundary = require_mapping(
        provider_contract.get("sealed_capability_boundary"),
        "provider_contract.sealed_capability_boundary",
        MechanismProfileIndexError,
    )
    gate_sequence = [
        str(value)
        for value in require_list(
            sealed_boundary.get("uniform_gate_sequence"),
            "provider_contract.sealed_capability_boundary.uniform_gate_sequence",
            MechanismProfileIndexError,
        )
    ]

    profiles: list[dict[str, Any]] = []
    for index, value in enumerate(
        require_list(mechanism_catalog.get("mechanisms"), "mechanism_catalog.mechanisms", MechanismProfileIndexError)
    ):
        mechanism = require_mapping(value, f"mechanism_catalog.mechanisms[{index}]", MechanismProfileIndexError)
        mechanism_id = require_string(mechanism.get("id"), f"mechanism_catalog.mechanisms[{index}].id", MechanismProfileIndexError)
        maturity_id = require_string(
            mechanism.get("maturity_mechanism"),
            f"mechanism_catalog.mechanisms[{index}].maturity_mechanism",
            MechanismProfileIndexError,
        )
        maturity_row = require_mapping(maturity_by_id.get(maturity_id), f"maturity.mechanisms.{maturity_id}", MechanismProfileIndexError)
        milestone = require_string(maturity_row.get("milestone"), f"maturity.mechanisms.{maturity_id}.milestone", MechanismProfileIndexError)
        capability_binding = require_mapping(bindings_by_maturity.get(maturity_id), f"capability_bindings.{maturity_id}", MechanismProfileIndexError)
        profiles.append(
            {
                "id": mechanism_id,
                "profile_type": "MechanismProfile",
                "maturity_mechanism": maturity_id,
                "technique": require_string(mechanism.get("technique"), f"{mechanism_id}.technique", MechanismProfileIndexError),
                "provider": {
                    "kind": require_string(mechanism.get("provider_kind"), f"{mechanism_id}.provider_kind", MechanismProfileIndexError),
                    "boundary_type": require_string(mechanism.get("provider_boundary_type"), f"{mechanism_id}.provider_boundary_type", MechanismProfileIndexError),
                    "sealed_boundary": True,
                    "gate_sequence": gate_sequence,
                    "required_capability": require_string(mechanism.get("required_provider_capability"), f"{mechanism_id}.required_provider_capability", MechanismProfileIndexError),
                    "production_review_required": bool(mechanism.get("production_provider_review_required")),
                },
                "participant_model": {"requires_all_participants": bool(mechanism.get("requires_all_participants"))},
                "maturity": {
                    "milestone": milestone,
                    "level": require_string(maturity_row.get("current_level"), f"maturity.mechanisms.{maturity_id}.current_level", MechanismProfileIndexError),
                    "production_claim": bool(maturity_row.get("production_claim")),
                    "claim_boundary": require_string(maturity_row.get("claim_boundary"), f"maturity.mechanisms.{maturity_id}.claim_boundary", MechanismProfileIndexError),
                },
                "capability_binding": capability_binding_ref(capability_binding),
                "operation_refs": operation_refs_from_binding(capability_binding),
                "assurance_registry_entries": assurance_by_milestone.get(milestone, []),
                "verification_kernel_refs": verified_kernel_refs(verified_kernels, maturity_id),
            }
        )

    package_profiles: list[dict[str, Any]] = []
    for package_id, package in rows_by_id(mechanism_packages, "packages", "mechanism_packages").items():
        maturity_id = require_string(package.get("maturity_mechanism"), f"package.{package_id}.maturity_mechanism", MechanismProfileIndexError)
        maturity_row = require_mapping(maturity_by_id.get(maturity_id), f"maturity.mechanisms.{maturity_id}", MechanismProfileIndexError)
        binding = require_mapping(bindings_by_maturity.get(maturity_id), f"capability_bindings.{maturity_id}", MechanismProfileIndexError)
        binding_id = require_string(binding.get("id"), f"capability_bindings.{maturity_id}.id", MechanismProfileIndexError)
        if binding_id != package.get("capability_binding"):
            raise MechanismProfileIndexError(f"package {package_id} capability binding drifts from generated binding index")
        package_profiles.append(
            {
                "id": package_id,
                "profile_type": "PackageProfile",
                "maturity_mechanism": maturity_id,
                "capability": require_string(package.get("capability"), f"package.{package_id}.capability", MechanismProfileIndexError),
                "provider_kind": require_string(package.get("provider_kind"), f"package.{package_id}.provider_kind", MechanismProfileIndexError),
                "mechanism_catalog_id": package.get("mechanism_catalog_id"),
                "capability_binding": capability_binding_ref(binding),
                "maturity": {
                    "milestone": require_string(maturity_row.get("milestone"), f"maturity.{maturity_id}.milestone", MechanismProfileIndexError),
                    "level": require_string(maturity_row.get("current_level"), f"maturity.{maturity_id}.current_level", MechanismProfileIndexError),
                    "production_claim": bool(maturity_row.get("production_claim")),
                    "claim_boundary": require_string(maturity_row.get("claim_boundary"), f"maturity.{maturity_id}.claim_boundary", MechanismProfileIndexError),
                },
                "assurance_registry_entry": package.get("assurance_registry_entry"),
                "verification_family": require_string(package.get("verification_family"), f"package.{package_id}.verification_family", MechanismProfileIndexError),
                "verification_kernel_refs": verified_kernel_refs(verified_kernels, maturity_id),
            }
        )

    return {
        "schema_version": 1,
        "kind": "kiva-confidential-mechanism-profile-index",
        "status": "active",
        "description": (
            "Generated MechanismProfile and PackageProfile index joining the authoritative "
            "MechanismPackage rows with mechanism, provider, maturity, OperationKernel, "
            "assurance, and verification facets."
        ),
        "generated_from": [
            mechanism_packages_path.as_posix(),
            mechanism_catalog_path.as_posix(),
            maturity_path.as_posix(),
            assurance_registry_path.as_posix(),
            verified_kernels_path.as_posix(),
            provider_contract_path.as_posix(),
            operation_kernel_path.as_posix(),
            capability_bindings_path.as_posix(),
        ],
        "generated_artifacts": {
            "profile_index": output_path.as_posix(),
            "markdown": markdown_path.as_posix(),
        },
        "profile_count": len(profiles),
        "profiles": profiles,
        "package_count": len(package_profiles),
        "package_profiles": package_profiles,
    }


def validate_schema(document: dict[str, Any], schema_path: Path) -> None:
    """Validate the generated MechanismProfile index."""
    schema = load_json(schema_path, MechanismProfileIndexError)
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document),
        key=lambda error: list(error.path),
    )
    if errors:
        details = "\n".join(
            f"- {'/'.join(str(part) for part in error.path) or '<root>'}: {error.message}"
            for error in errors
        )
        raise MechanismProfileIndexError(f"MechanismProfile index violates {schema_path}:\n{details}")


def markdown_cell(value: object) -> str:
    """Render a Markdown table cell."""
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_markdown(document: dict[str, Any]) -> str:
    """Render a compact mechanism profile table."""
    rows = [
        "| Profile | Maturity | Provider | Operations | Assurance Entries | Verified Kernels |",
        "| --- | --- | --- | ---: | ---: | ---: |",
    ]
    for value in require_list(document.get("profiles"), "profiles", MechanismProfileIndexError):
        profile = require_mapping(value, "profiles[]", MechanismProfileIndexError)
        provider = require_mapping(profile.get("provider"), "profile.provider", MechanismProfileIndexError)
        rows.append(
            "| "
            + " | ".join(
                [
                    f"`{markdown_cell(profile['id'])}`",
                    f"`{markdown_cell(profile['maturity_mechanism'])}`",
                    f"`{markdown_cell(provider['kind'])}`",
                    str(len(profile.get("operation_refs", []))),
                    str(len(profile.get("assurance_registry_entries", []))),
                    str(len(profile.get("verification_kernel_refs", []))),
                ]
            )
            + " |"
        )
    package_rows = [
        "| Package | Maturity | Capability | Provider | Catalog |",
        "| --- | --- | --- | --- | --- |",
    ]
    for value in require_list(document.get("package_profiles"), "package_profiles", MechanismProfileIndexError):
        package = require_mapping(value, "package_profiles[]", MechanismProfileIndexError)
        package_rows.append(
            "| "
            + " | ".join(
                [
                    f"`{markdown_cell(package['id'])}`",
                    f"`{markdown_cell(package['maturity_mechanism'])}`",
                    f"`{markdown_cell(package['capability'])}`",
                    f"`{markdown_cell(package['provider_kind'])}`",
                    f"`{markdown_cell(package.get('mechanism_catalog_id') or 'none')}`",
                ]
            )
            + " |"
        )
    return f"""# Generated Confidential MechanismProfile Index

This file is generated by `scripts/ci/generate_confidential_mechanism_profile_index.py`.

`MechanismProfile` is the confidential-computation review row tying mechanism
semantics, provider runtime requirements, HTTP operations, assurance evidence,
and verification boundaries together.

`PackageProfile` is the complete package-owned review row. It also covers
capabilities that intentionally have no standalone mechanism catalog entry.

{chr(10).join(rows)}

{chr(10).join(package_rows)}
"""


def main() -> int:
    """Generate or validate the confidential MechanismProfile index."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mechanism-packages", type=Path, default=DEFAULT_MECHANISM_PACKAGES)
    parser.add_argument("--mechanism-catalog", type=Path, default=DEFAULT_MECHANISM_CATALOG)
    parser.add_argument("--maturity", type=Path, default=DEFAULT_MATURITY)
    parser.add_argument("--assurance-registry", type=Path, default=DEFAULT_ASSURANCE_REGISTRY)
    parser.add_argument("--verified-kernels", type=Path, default=DEFAULT_VERIFIED_KERNELS)
    parser.add_argument("--provider-contract", type=Path, default=DEFAULT_PROVIDER_CONTRACT)
    parser.add_argument("--operation-kernel", type=Path, default=DEFAULT_OPERATION_KERNEL)
    parser.add_argument("--capability-bindings", type=Path, default=DEFAULT_CAPABILITY_BINDINGS)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    document = profile_index_document(
        args.mechanism_packages,
        args.mechanism_catalog,
        args.maturity,
        args.assurance_registry,
        args.verified_kernels,
        args.provider_contract,
        args.operation_kernel,
        args.capability_bindings,
        args.out,
        args.markdown_out,
    )
    validate_schema(document, args.schema)
    rendered_yaml = dump_yaml(document)
    rendered_markdown = render_markdown(document)
    command_hint = "scripts/ci/generate_confidential_mechanism_profile_index.py"
    try:
        write_or_check_generated_artifacts(
            [
                GeneratedArtifact(args.out, rendered_yaml),
                GeneratedArtifact(args.markdown_out, rendered_markdown),
            ],
            check=args.check,
            command_hint=command_hint,
            error_cls=MechanismProfileIndexError,
        )
    except MechanismProfileIndexError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated MechanismProfile index: {args.out}, {args.markdown_out}")
        return 0
    print(f"generated MechanismProfile index: {args.out}, {args.markdown_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
