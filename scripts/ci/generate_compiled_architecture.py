#!/usr/bin/env python3
"""Generate the compiled architecture IR from graph and policy sources."""

from __future__ import annotations

import argparse
from pathlib import Path

from kiva_ci.architecture_compiler import (
    architecture_policy_source_path,
    compiled_architecture_document,
    dump_yaml,
    merge_projection_rows,
    projection_spec_source_path,
)
from kiva_ci.architecture import load_architecture_manifest
from kiva_ci.manifest import load_yaml, require_mapping, require_string_list
from kiva_ci.projection import (
    projection_specs_from_normal_form_paths,
    projection_specs_from_normal_form,
    validate_projection_spec_compatibility_view,
)
from kiva_ci.runtime import GeneratedArtifact, ProjectionRuntime


DEFAULT_GRAPH_SOURCE = Path("spec/architecture/architecture-graph.source.yaml")
DEFAULT_PROJECTION_ARTIFACTS_SOURCE = Path("spec/architecture/projection-artifacts.source.yaml")
DEFAULT_PROJECTION_RUNTIME_SOURCE = Path("spec/architecture/projection-runtime.source.yaml")
DEFAULT_PROJECTION_CHECK_BINDINGS_SOURCE = Path("spec/architecture/projection-check-bindings.source.yaml")
DEFAULT_OUTPUT = Path("generated/architecture/compiled-architecture.yaml")


class CompiledArchitectureGenerationError(RuntimeError):
    """Raised when the compiled architecture IR cannot be generated."""


def projection_spec_document(root: Path, projection_spec_path: Path) -> dict[str, object]:
    """Return a ProjectionSpec compatibility document backed by normal-form ledgers."""
    projection_spec = load_yaml(root / projection_spec_path, CompiledArchitectureGenerationError)
    normal_form_rows = projection_specs_from_normal_form(
        root,
        projection_spec,
        projection_spec_path,
        CompiledArchitectureGenerationError,
    )
    validate_projection_spec_compatibility_view(
        projection_spec,
        projection_spec_path,
        normal_form_rows,
        CompiledArchitectureGenerationError,
    )
    return {**projection_spec, "projections": normal_form_rows}


def render_compiled_architecture(root: Path, graph_path: Path = DEFAULT_GRAPH_SOURCE) -> str:
    """Render the expected compiled architecture IR YAML."""
    graph = load_yaml(root / graph_path, CompiledArchitectureGenerationError)
    projection_spec_path = projection_spec_source_path(
        graph,
        graph_path,
        CompiledArchitectureGenerationError,
    )
    projection_spec_document(root, projection_spec_path)
    projection_rows = projection_specs_from_normal_form_paths(
        root,
        projection_spec_path,
        DEFAULT_PROJECTION_ARTIFACTS_SOURCE,
        DEFAULT_PROJECTION_RUNTIME_SOURCE,
        DEFAULT_PROJECTION_CHECK_BINDINGS_SOURCE,
        CompiledArchitectureGenerationError,
    )
    policy_path = architecture_policy_source_path(
        graph,
        graph_path,
        CompiledArchitectureGenerationError,
    )
    policy, _ = load_architecture_manifest(
        root,
        policy_path,
        CompiledArchitectureGenerationError,
    )
    document = compiled_architecture_document(
        graph,
        graph_path,
        projection_rows,
        [
            DEFAULT_PROJECTION_ARTIFACTS_SOURCE.as_posix(),
            DEFAULT_PROJECTION_RUNTIME_SOURCE.as_posix(),
            DEFAULT_PROJECTION_CHECK_BINDINGS_SOURCE.as_posix(),
        ],
        f"{projection_spec_path.as_posix()}#/projections",
        policy,
        policy_path,
        CompiledArchitectureGenerationError,
    )
    return dump_yaml(document)


def declared_compiled_architecture_targets(
    root: Path,
    graph: dict[str, object],
    graph_path: Path,
) -> list[str]:
    """Return graph-owned targets that may declare the compiled IR output."""
    generated_artifacts = require_string_list(
        graph.get("generated_artifacts"),
        f"{graph_path}.generated_artifacts",
        CompiledArchitectureGenerationError,
        allow_empty=True,
    )
    sources = require_mapping(
        graph.get("sources"),
        f"{graph_path}.sources",
        CompiledArchitectureGenerationError,
    )
    projection_spec_path = projection_spec_source_path(
        graph,
        graph_path,
        CompiledArchitectureGenerationError,
    )
    projection_spec_document(root, projection_spec_path)
    projection_rows = projection_specs_from_normal_form_paths(
        root,
        projection_spec_path,
        DEFAULT_PROJECTION_ARTIFACTS_SOURCE,
        DEFAULT_PROJECTION_RUNTIME_SOURCE,
        DEFAULT_PROJECTION_CHECK_BINDINGS_SOURCE,
        CompiledArchitectureGenerationError,
    )
    sources = merge_projection_rows(
        sources,
        projection_rows,
        "normal-form ProjectionRuntime ledgers",
        CompiledArchitectureGenerationError,
    )
    compiled_source = require_mapping(
        sources.get("compiled_architecture"),
        f"{graph_path}.sources.compiled_architecture",
        CompiledArchitectureGenerationError,
    )
    return sorted(
        set(generated_artifacts).union(
            require_string_list(
                compiled_source.get("generated_artifacts"),
                f"{graph_path}.sources.compiled_architecture.generated_artifacts",
                CompiledArchitectureGenerationError,
            )
        )
    )


def main() -> int:
    """Generate or validate the compiled architecture IR."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--graph-source", type=Path, default=DEFAULT_GRAPH_SOURCE)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    rendered = render_compiled_architecture(root, args.graph_source)
    output_path = root / args.out
    graph = load_yaml(root / args.graph_source, CompiledArchitectureGenerationError)
    if args.out.as_posix() not in declared_compiled_architecture_targets(root, graph, args.graph_source):
        raise CompiledArchitectureGenerationError(
            f"{args.graph_source}.sources.compiled_architecture.generated_artifacts must include {args.out}"
        )
    projection = ProjectionRuntime(
        command_hint="scripts/ci/generate_compiled_architecture.py",
        artifacts=(GeneratedArtifact(output_path, rendered),),
    )
    try:
        projection.write_or_check(check=args.check, error_cls=CompiledArchitectureGenerationError)
    except CompiledArchitectureGenerationError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated compiled architecture IR: {args.out}")
        return 0
    print(f"generated compiled architecture IR: {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
