#!/usr/bin/env python3
"""Generate the design-backbone review projection from compiled architecture IR."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import (
    dump_yaml,
    validate_compiled_architecture_envelope,
)
from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_COMPILED_ARCHITECTURE = Path("generated/architecture/compiled-architecture.yaml")
DEFAULT_OUTPUT = Path("spec/architecture/design-backbone.yaml")
DESIGN_BACKBONE_KIND = "kiva-architecture-design-backbone"


class DesignBackboneGenerationError(RuntimeError):
    """Raised when the design-backbone projection cannot be generated."""


def design_backbone_document(
    compiled: dict[str, Any],
    compiled_path: Path,
) -> dict[str, Any]:
    """Project the review-oriented design-backbone document from compiled IR."""
    validate_compiled_architecture_envelope(
        compiled,
        compiled_path,
        DesignBackboneGenerationError,
    )
    required_policy_sections = [
        "policy_sections",
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
    ]
    projected: dict[str, Any] = {
        "schema_version": 1,
        "kind": DESIGN_BACKBONE_KIND,
        "status": "active",
        "description": (
            "Source-of-truth ownership map for Kiva design claims, public HTTP "
            "contracts, application operation bindings, milestone status, "
            "architecture drift guards, and generated publication artifacts."
        ),
        "generated_from": [compiled_path.as_posix()],
        "projection_role": "architecture-graph-review-projection",
        "source_model": require_string(
            compiled.get("source_model"),
            f"{compiled_path}.source_model",
            DesignBackboneGenerationError,
        ),
        "projection_model": require_string(
            compiled.get("projection_model"),
            f"{compiled_path}.projection_model",
            DesignBackboneGenerationError,
        ),
        "policy_source": require_string(
            compiled.get("policy_source"),
            f"{compiled_path}.policy_source",
            DesignBackboneGenerationError,
        ),
        "sources": require_mapping(
            compiled.get("sources"),
            f"{compiled_path}.sources",
            DesignBackboneGenerationError,
        ),
        "architecture_normal_form": require_mapping(
            compiled.get("architecture_normal_form"),
            f"{compiled_path}.architecture_normal_form",
            DesignBackboneGenerationError,
        ),
        "artifact_catalog": require_list(
            compiled.get("artifact_catalog"),
            f"{compiled_path}.artifact_catalog",
            DesignBackboneGenerationError,
        ),
    }
    for section in required_policy_sections:
        if section == "policy_sections":
            projected[section] = require_list(
                compiled.get(section),
                f"{compiled_path}.{section}",
                DesignBackboneGenerationError,
            )
            continue
        projected[section] = require_mapping(
            compiled.get(section),
            f"{compiled_path}.{section}",
            DesignBackboneGenerationError,
        )
    return projected


def render_design_backbone(
    root: Path,
    compiled_path: Path = DEFAULT_COMPILED_ARCHITECTURE,
) -> str:
    """Render the expected design-backbone YAML text."""
    compiled = load_yaml(root / compiled_path, DesignBackboneGenerationError)
    document = design_backbone_document(compiled, compiled_path)
    return dump_yaml(document)


def compiled_artifact_paths(compiled: dict[str, Any], compiled_path: Path) -> list[str]:
    """Return artifact paths registered in compiled architecture IR."""
    paths: list[str] = []
    for index, raw_artifact in enumerate(
        require_list(
            compiled.get("artifact_catalog"),
            f"{compiled_path}.artifact_catalog",
            DesignBackboneGenerationError,
        )
    ):
        artifact = require_mapping(
            raw_artifact,
            f"{compiled_path}.artifact_catalog[{index}]",
            DesignBackboneGenerationError,
        )
        paths.append(
            require_string(
                artifact.get("path"),
                f"{compiled_path}.artifact_catalog[{index}].path",
                DesignBackboneGenerationError,
            )
        )
    return paths


def main() -> int:
    """Generate or validate the design-backbone projection."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--compiled", type=Path, default=DEFAULT_COMPILED_ARCHITECTURE)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    compiled = load_yaml(root / args.compiled, DesignBackboneGenerationError)
    rendered = render_design_backbone(root, args.compiled)
    output_path = root / args.out
    if args.out.as_posix() not in compiled_artifact_paths(compiled, args.compiled):
        raise DesignBackboneGenerationError(
            f"{args.compiled}.artifact_catalog must include {args.out}"
        )
    command_hint = "scripts/ci/generate_design_backbone.py"
    try:
        write_or_check_generated_artifacts(
            [GeneratedArtifact(output_path, rendered)],
            check=args.check,
            command_hint=command_hint,
            error_cls=DesignBackboneGenerationError,
        )
    except DesignBackboneGenerationError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated design backbone: {args.out}")
        return 0
    print(f"generated design backbone: {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
