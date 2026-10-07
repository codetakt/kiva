#!/usr/bin/env python3
"""Generate the CI projection graph from compiled architecture IR."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.architecture import (
    artifact_class_for_source,
    artifact_class_policy_from_architecture_policy,
)
from kiva_ci.architecture_compiler import (
    dump_yaml,
    validate_compiled_architecture_envelope,
)
from kiva_ci.manifest import (
    generated_target_values,
    load_yaml,
    require_list,
    require_mapping,
    require_string,
    resolve_projection_sources,
    validate_projection_check_refs,
)
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_COMPILED_ARCHITECTURE = Path("generated/architecture/compiled-architecture.yaml")
DEFAULT_OUTPUT = Path("spec/ci/projection-graph.yaml")
PROJECTION_GRAPH_KIND = "kiva-ci-projection-graph"
SOURCE_MODEL_REF = "#/sources"
DESCRIPTION = (
    "Machine-readable projection graph for CI-checked Kiva contract, operation, "
    "DTO, claim, and architecture evidence artifacts. This graph is derived "
    "from the compiled architecture IR, which owns source classification, "
    "generated artifact metadata, and source-to-projection relationships."
)


class ProjectionGraphGenerationError(RuntimeError):
    """Raised when the projection graph cannot be generated."""


def projection_graph_document(
    compiled: dict[str, Any],
    compiled_path: Path,
) -> dict[str, Any]:
    """Project CI-checked architecture source entries into a graph."""
    validate_compiled_architecture_envelope(
        compiled,
        compiled_path,
        ProjectionGraphGenerationError,
    )
    sources = require_mapping(
        compiled.get("sources"),
        f"{compiled_path}.sources",
        ProjectionGraphGenerationError,
    )
    artifact_policy = artifact_class_policy_from_architecture_policy(
        compiled,
        compiled_path.as_posix(),
        ProjectionGraphGenerationError,
    )
    projections = projection_edges_from_sources(sources, artifact_policy)
    return {
        "schema_version": 1,
        "kind": PROJECTION_GRAPH_KIND,
        "source_model": f"{compiled_path.as_posix()}{SOURCE_MODEL_REF}",
        "classification_source": f"{compiled_path.as_posix()}#/source_layers",
        "description": DESCRIPTION,
        "artifact_catalog": require_list(
            compiled.get("artifact_catalog"),
            f"{compiled_path}.artifact_catalog",
            ProjectionGraphGenerationError,
        ),
        "projections": projections,
    }


def projection_edges_from_sources(
    sources: dict[str, Any],
    artifact_policy: dict[str, Any],
) -> list[dict[str, Any]]:
    """Return deterministic projection graph edges declared by source entries."""
    projections: list[dict[str, Any]] = []
    for source_key, raw_source in sources.items():
        source = require_mapping(raw_source, f"sources.{source_key}", ProjectionGraphGenerationError)
        if "projection_checks" not in source:
            continue
        projections.append(projection_edge(source_key, source, sources, artifact_policy))
    if not projections:
        raise ProjectionGraphGenerationError("sources must declare at least one projection_checks entry")
    return projections


def projection_edge(
    source_key: str,
    source: dict[str, Any],
    sources: dict[str, Any],
    artifact_policy: dict[str, Any],
) -> dict[str, Any]:
    """Return one projection graph edge from one compiled source entry."""
    projection_id = require_string(
        source.get("projection_id"),
        f"sources.{source_key}.projection_id",
        ProjectionGraphGenerationError,
    )
    source_layer = require_string(
        source.get("source_layer"),
        f"sources.{source_key}.source_layer",
        ProjectionGraphGenerationError,
    )
    projection_kind = require_string(
        source.get("projection_kind"),
        f"sources.{source_key}.projection_kind",
        ProjectionGraphGenerationError,
    )
    projection_sources = resolve_projection_sources(
        source_key,
        source,
        sources,
        ProjectionGraphGenerationError,
    )
    projection_targets = generated_targets(source_key, source)
    artifact_class = artifact_class_for_source(source_key, source, artifact_policy)
    target_artifacts = [
        {
            "path": target,
            "artifact_class": artifact_class,
            "retention_class": retention_class_for_target(target),
        }
        for target in projection_targets
    ]
    validators = validate_projection_check_refs(
        source.get("projection_checks"),
        f"sources.{source_key}.projection_checks",
        ProjectionGraphGenerationError,
    )

    return {
        "id": projection_id,
        "source_layer": source_layer,
        "projection_kind": projection_kind,
        "sources": sorted(set(projection_sources)),
        "targets": projection_targets,
        "target_artifacts": target_artifacts,
        "validators": validators,
    }


def retention_class_for_target(target: str) -> str:
    """Return the retention class for one projection target."""
    if target.endswith(".rs"):
        return "committed-generated-code"
    if target.startswith("docs/"):
        return "committed-review-doc"
    if "schemas" in target:
        return "committed-schema-artifact"
    if target.endswith(".evidence.json"):
        return "committed-generated-evidence"
    return "committed-generated-ledger"


def generated_targets(source_key: str, source: dict[str, Any]) -> list[str]:
    """Return generated graph targets declared by a source entry."""
    targets = generated_target_values(source_key, source, ProjectionGraphGenerationError)
    if not targets:
        raise ProjectionGraphGenerationError(
            f"sources.{source_key} declares projection_checks but no generated targets"
        )
    return targets


def render_projection_graph(
    root: Path,
    compiled_path: Path = DEFAULT_COMPILED_ARCHITECTURE,
) -> str:
    """Render the expected projection graph YAML."""
    compiled = load_yaml(root / compiled_path, ProjectionGraphGenerationError)
    document = projection_graph_document(compiled, compiled_path)
    return dump_yaml(document)


def compiled_artifact_paths(compiled: dict[str, Any], compiled_path: Path) -> list[str]:
    """Return artifact paths registered in compiled architecture IR."""
    paths: list[str] = []
    for index, raw_artifact in enumerate(
        require_list(
            compiled.get("artifact_catalog"),
            f"{compiled_path}.artifact_catalog",
            ProjectionGraphGenerationError,
        )
    ):
        artifact = require_mapping(
            raw_artifact,
            f"{compiled_path}.artifact_catalog[{index}]",
            ProjectionGraphGenerationError,
        )
        paths.append(
            require_string(
                artifact.get("path"),
                f"{compiled_path}.artifact_catalog[{index}].path",
                ProjectionGraphGenerationError,
            )
        )
    return paths


def main() -> int:
    """Generate or validate the projection graph artifact."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--compiled", type=Path, default=DEFAULT_COMPILED_ARCHITECTURE)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    compiled = load_yaml(root / args.compiled, ProjectionGraphGenerationError)
    rendered = render_projection_graph(root, args.compiled)
    output_path = root / args.out
    if args.out.as_posix() not in compiled_artifact_paths(compiled, args.compiled):
        raise ProjectionGraphGenerationError(
            f"{args.compiled}.artifact_catalog must include {args.out}"
        )
    command_hint = "scripts/ci/generate_projection_graph.py"
    try:
        write_or_check_generated_artifacts(
            [GeneratedArtifact(output_path, rendered)],
            check=args.check,
            command_hint=command_hint,
            error_cls=ProjectionGraphGenerationError,
        )
    except ProjectionGraphGenerationError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated projection graph: {args.out}")
        return 0
    print(f"generated projection graph: {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
