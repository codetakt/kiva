#!/usr/bin/env python3
"""Generate Capability/Policy/Evidence rows for confidential mechanisms."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_BINDINGS = Path("generated/confidential/capability-binding-index.yaml")
DEFAULT_PROFILES = Path("generated/confidential/mechanism-profile-index.yaml")
DEFAULT_OUTPUT = Path("generated/confidential/capability-policy-evidence.yaml")
DEFAULT_MARKDOWN = Path("docs/architecture/generated/confidential-capability-policy-evidence.md")


class CapabilityPolicyEvidenceError(RuntimeError):
    """Raised when capability policy evidence generation fails."""


def bindings_by_mechanism(bindings: dict[str, Any]) -> dict[str, dict[str, Any]]:
    """Return CapabilityBinding rows keyed by maturity mechanism."""
    rows: dict[str, dict[str, Any]] = {}
    for index, raw_binding in enumerate(require_list(bindings.get("bindings"), "bindings.bindings", CapabilityPolicyEvidenceError)):
        binding = require_mapping(raw_binding, f"bindings.bindings[{index}]", CapabilityPolicyEvidenceError)
        mechanism = require_string(binding.get("maturity_mechanism"), f"bindings.bindings[{index}].maturity_mechanism", CapabilityPolicyEvidenceError)
        if mechanism in rows:
            raise CapabilityPolicyEvidenceError(f"duplicate capability binding for mechanism: {mechanism}")
        rows[mechanism] = binding
    return rows


def graph_document(root: Path, bindings_path: Path, profiles_path: Path, output_path: Path, markdown_path: Path) -> dict[str, Any]:
    """Return generated CapabilityPolicyEvidence rows."""
    bindings = load_yaml(root / bindings_path, CapabilityPolicyEvidenceError)
    profiles = load_yaml(root / profiles_path, CapabilityPolicyEvidenceError)
    binding_by_mechanism = bindings_by_mechanism(bindings)
    rows: list[dict[str, Any]] = []
    for index, raw_profile in enumerate(require_list(profiles.get("profiles"), "profiles.profiles", CapabilityPolicyEvidenceError)):
        profile = require_mapping(raw_profile, f"profiles.profiles[{index}]", CapabilityPolicyEvidenceError)
        mechanism = require_string(profile.get("maturity_mechanism"), f"profiles.profiles[{index}].maturity_mechanism", CapabilityPolicyEvidenceError)
        binding = binding_by_mechanism.get(mechanism)
        if binding is None:
            raise CapabilityPolicyEvidenceError(f"profile has no CapabilityBinding: {mechanism}")
        provider = require_mapping(profile.get("provider"), f"profiles.profiles[{index}].provider", CapabilityPolicyEvidenceError)
        maturity = require_mapping(profile.get("maturity"), f"profiles.profiles[{index}].maturity", CapabilityPolicyEvidenceError)
        rows.append(
            {
                "mechanism": mechanism,
                "profile": require_string(profile.get("id"), f"profiles.profiles[{index}].id", CapabilityPolicyEvidenceError),
                "capability": require_string(binding.get("capability"), f"binding.{mechanism}.capability", CapabilityPolicyEvidenceError),
                "binding_kind": require_string(binding.get("binding_kind"), f"binding.{mechanism}.binding_kind", CapabilityPolicyEvidenceError),
                "provider_kind": require_string(provider.get("kind"), f"profile.{mechanism}.provider.kind", CapabilityPolicyEvidenceError),
                "provider_boundary_type": require_string(provider.get("boundary_type"), f"profile.{mechanism}.provider.boundary_type", CapabilityPolicyEvidenceError),
                "sealed_boundary": bool(provider.get("sealed_boundary")),
                "gate_sequence": [
                    str(value)
                    for value in require_list(
                        provider.get("gate_sequence"),
                        f"profile.{mechanism}.provider.gate_sequence",
                        CapabilityPolicyEvidenceError,
                    )
                ],
                "production_review_required": bool(provider.get("production_review_required")),
                "maturity_level": require_string(maturity.get("level"), f"profile.{mechanism}.maturity.level", CapabilityPolicyEvidenceError),
                "operation_count": int(binding.get("operation_count", 0)),
                "assurance_registry_entry_count": len(require_list(profile.get("assurance_registry_entries", []), f"profile.{mechanism}.assurance_registry_entries", CapabilityPolicyEvidenceError)),
                "verification_kernel_count": len(require_list(profile.get("verification_kernel_refs", []), f"profile.{mechanism}.verification_kernel_refs", CapabilityPolicyEvidenceError)),
            }
        )
    return {
        "schema_version": 1,
        "kind": "kiva-confidential-capability-policy-evidence",
        "status": "active",
        "description": "Generated Capability/Policy/Evidence view joining confidential capabilities, provider policies, maturity, operations, assurance, and verification evidence.",
        "generated_from": [bindings_path.as_posix(), profiles_path.as_posix()],
        "generated_artifacts": {
            "capability_policy_evidence": output_path.as_posix(),
            "markdown": markdown_path.as_posix(),
        },
        "mechanism_count": len(rows),
        "mechanisms": sorted(rows, key=lambda row: row["mechanism"]),
    }


def render_markdown(document: dict[str, Any]) -> str:
    """Render a concise CapabilityPolicyEvidence table."""
    lines = [
        "# Generated Confidential Capability Policy Evidence",
        "",
        "This file is generated by `scripts/ci/generate_confidential_capability_policy_evidence.py`.",
        "",
        "| Mechanism | Capability | Provider | Boundary | Review | Operations | Verification |",
        "| --- | --- | --- | --- | --- | ---: | ---: |",
    ]
    for row in require_list(document.get("mechanisms"), "mechanisms", CapabilityPolicyEvidenceError):
        mechanism = require_mapping(row, "mechanisms[]", CapabilityPolicyEvidenceError)
        lines.append(
            "| "
            + " | ".join(
                [
                    f"`{mechanism['mechanism']}`",
                    f"`{mechanism['capability']}`",
                    f"`{mechanism['provider_kind']}`",
                    "`sealed`" if mechanism["sealed_boundary"] else "`open`",
                    "`required`" if mechanism["production_review_required"] else "`not-required`",
                    str(mechanism["operation_count"]),
                    str(mechanism["verification_kernel_count"]),
                ]
            )
            + " |"
        )
    return "\n".join(lines) + "\n"


def rendered_artifacts(root: Path, bindings_path: Path, profiles_path: Path, output_path: Path, markdown_path: Path) -> list[GeneratedArtifact]:
    """Return rendered CapabilityPolicyEvidence artifacts."""
    document = graph_document(root, bindings_path, profiles_path, output_path, markdown_path)
    return [
        GeneratedArtifact(root / output_path, dump_yaml(document)),
        GeneratedArtifact(root / markdown_path, render_markdown(document)),
    ]


def main() -> int:
    """Generate or validate CapabilityPolicyEvidence artifacts."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--bindings", type=Path, default=DEFAULT_BINDINGS)
    parser.add_argument("--profiles", type=Path, default=DEFAULT_PROFILES)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    artifacts = rendered_artifacts(root, args.bindings, args.profiles, args.out, args.markdown_out)
    write_or_check_generated_artifacts(
        artifacts,
        check=args.check,
        command_hint="scripts/ci/generate_confidential_capability_policy_evidence.py",
        error_cls=CapabilityPolicyEvidenceError,
    )
    verb = "validated" if args.check else "generated"
    print(f"{verb} confidential CapabilityPolicyEvidence: {args.out}, {args.markdown_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
