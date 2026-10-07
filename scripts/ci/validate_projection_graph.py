#!/usr/bin/env python3
"""Validate the Kiva CI projection graph manifest."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from kiva_ci.architecture import validate_artifact_catalog_entries
from kiva_ci.evidence import (
    evidence_check_effective_inputs,
    load_evidence_check_manifest,
    validate_evidence_check_manifest_metadata,
)
from kiva_ci.manifest import (
    load_yaml,
    require_existing_path,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
)
from kiva_ci.projection_graph import derived_inputs_by_check
from generate_projection_graph import (
    DEFAULT_COMPILED_ARCHITECTURE,
    SOURCE_MODEL_REF,
    render_projection_graph,
)


DEFAULT_GRAPH = Path("spec/ci/projection-graph.yaml")
DEFAULT_EVIDENCE_MANIFEST = Path("spec/ci/evidence-checks.yaml")


class ProjectionGraphError(RuntimeError):
    """Raised when projection graph metadata drifts."""


def checks_by_group(root: Path, evidence_manifest_path: Path) -> dict[str, dict[str, dict[str, Any]]]:
    """Return CI evidence check ids keyed by group id."""
    manifest = load_evidence_check_manifest(
        root / evidence_manifest_path,
        ProjectionGraphError,
    )
    groups = validate_evidence_check_manifest_metadata(
        manifest,
        evidence_manifest_path.as_posix(),
        ProjectionGraphError,
    )
    check_ids: dict[str, dict[str, dict[str, Any]]] = {}
    for group_id, group in groups.items():
        check_ids[group_id] = {}
        for index, check in enumerate(
            require_list(group.get("checks"), f"{group_id}.checks", ProjectionGraphError)
        ):
            check = require_mapping(check, f"{group_id}.checks[{index}]", ProjectionGraphError)
            check_id = require_string(
                check.get("id"),
                f"{group_id}.checks[{index}].id",
                ProjectionGraphError,
            )
            check_ids[group_id][check_id] = check
    return check_ids


def validate_projection(
    root: Path,
    projection: dict[str, Any],
    index: int,
    check_matrix: dict[str, dict[str, dict[str, Any]]],
) -> dict[str, Any]:
    """Validate one projection graph edge."""
    projection_id = require_string(
        projection.get("id"),
        f"projections[{index}].id",
        ProjectionGraphError,
    )
    source_layer = require_string(
        projection.get("source_layer"),
        f"{projection_id}.source_layer",
        ProjectionGraphError,
    )
    if source_layer not in {"contract", "operation", "assurance", "governance"}:
        raise ProjectionGraphError(f"{projection_id}.source_layer is invalid: {source_layer}")
    require_string(
        projection.get("projection_kind"),
        f"{projection_id}.projection_kind",
        ProjectionGraphError,
    )

    sources = require_string_list(
        projection.get("sources"),
        f"{projection_id}.sources",
        ProjectionGraphError,
    )
    targets = require_string_list(
        projection.get("targets"),
        f"{projection_id}.targets",
        ProjectionGraphError,
    )
    for source in sources:
        require_existing_path(root, source, f"{projection_id}.sources", ProjectionGraphError)
    for target in targets:
        require_existing_path(root, target, f"{projection_id}.targets", ProjectionGraphError)
    artifact_rows = require_list(
        projection.get("target_artifacts"),
        f"{projection_id}.target_artifacts",
        ProjectionGraphError,
    )
    artifact_paths = []
    for artifact_index, value in enumerate(artifact_rows):
        artifact = require_mapping(
            value,
            f"{projection_id}.target_artifacts[{artifact_index}]",
            ProjectionGraphError,
        )
        artifact_paths.append(
            require_string(
                artifact.get("path"),
                f"{projection_id}.target_artifacts[{artifact_index}].path",
                ProjectionGraphError,
            )
        )
        require_string(
            artifact.get("artifact_class"),
            f"{projection_id}.target_artifacts[{artifact_index}].artifact_class",
            ProjectionGraphError,
        )
        require_string(
            artifact.get("retention_class"),
            f"{projection_id}.target_artifacts[{artifact_index}].retention_class",
            ProjectionGraphError,
        )
    if sorted(artifact_paths) != sorted(targets):
        raise ProjectionGraphError(f"{projection_id}.target_artifacts must match targets")

    validators = require_list(
        projection.get("validators"),
        f"{projection_id}.validators",
        ProjectionGraphError,
    )
    if not validators:
        raise ProjectionGraphError(f"{projection_id}.validators must not be empty")
    validator_refs = []
    for validator_index, value in enumerate(validators):
        validator = require_mapping(
            value,
            f"{projection_id}.validators[{validator_index}]",
            ProjectionGraphError,
        )
        group = require_string(
            validator.get("group"),
            f"{projection_id}.validators[{validator_index}].group",
            ProjectionGraphError,
        )
        check_id = require_string(
            validator.get("check_id"),
            f"{projection_id}.validators[{validator_index}].check_id",
            ProjectionGraphError,
        )
        if group not in check_matrix:
            raise ProjectionGraphError(f"{projection_id} references missing CI group: {group}")
        if check_id not in check_matrix[group]:
            raise ProjectionGraphError(
                f"{projection_id} references missing CI check {group}.{check_id}"
            )
        validator_refs.append(f"{group}.{check_id}")

    return {
        "id": projection_id,
        "source_layer": source_layer,
        "source_count": len(sources),
        "target_count": len(targets),
        "validators": validator_refs,
    }


def validate(
    root: Path,
    graph_path: Path,
    evidence_manifest_path: Path,
    compiled_path: Path,
) -> dict[str, Any]:
    """Validate the projection graph manifest."""
    rendered = render_projection_graph(root, compiled_path)
    actual = (root / graph_path).read_text(encoding="utf-8")
    if actual != rendered:
        raise ProjectionGraphError(
            f"{graph_path} is not current with {compiled_path}{SOURCE_MODEL_REF}"
        )

    graph = load_yaml(root / graph_path, ProjectionGraphError)
    if graph.get("schema_version") != 1:
        raise ProjectionGraphError("schema_version must be 1")
    if graph.get("kind") != "kiva-ci-projection-graph":
        raise ProjectionGraphError("kind must be kiva-ci-projection-graph")
    require_string(graph.get("classification_source"), "classification_source", ProjectionGraphError)
    require_string(graph.get("description"), "description", ProjectionGraphError)
    validate_artifact_catalog_entries(
        root,
        graph.get("artifact_catalog"),
        "artifact_catalog",
        ProjectionGraphError,
    )

    check_matrix = checks_by_group(root, evidence_manifest_path)
    seen_ids: set[str] = set()
    projections = []
    for index, value in enumerate(
        require_list(graph.get("projections"), "projections", ProjectionGraphError)
    ):
        projection = validate_projection(
            root,
            require_mapping(value, f"projections[{index}]", ProjectionGraphError),
            index,
            check_matrix,
        )
        projection_id = projection["id"]
        if projection_id in seen_ids:
            raise ProjectionGraphError(f"duplicate projection id: {projection_id}")
        seen_ids.add(projection_id)
        projections.append(projection)
    if not projections:
        raise ProjectionGraphError("projections must not be empty")
    derived_inputs = derived_inputs_by_check(graph, ProjectionGraphError)
    input_reports = []
    for (group_id, check_id), paths in sorted(derived_inputs.items()):
        check = check_matrix.get(group_id, {}).get(check_id)
        if check is None:
            raise ProjectionGraphError(f"projection graph references missing check: {group_id}.{check_id}")
        script = require_string(
            check.get("script"),
            f"{group_id}.{check_id}.script",
            ProjectionGraphError,
        )
        input_model = evidence_check_effective_inputs(
            check,
            paths,
            f"{group_id}.{check_id}",
            ProjectionGraphError,
            require_projection_graph_inputs=True,
            explicit_input_must_include=script,
        )
        for path in paths:
            require_existing_path(root, path, f"{group_id}.{check_id}.derived_inputs", ProjectionGraphError)
        input_reports.append(
            {
                "group": group_id,
                "check_id": check_id,
                "derived_input_policy": input_model["derived_input_policy"],
                "declared_input_count": len(input_model["inputs"]),
                "derived_projection_input_count": len(input_model["derived_projection_inputs"]),
            }
        )

    return {
        "kind": "kiva-ci-projection-graph-validation",
        "status": "passed",
        "graph": graph_path.as_posix(),
        "source_model": f"{compiled_path.as_posix()}{SOURCE_MODEL_REF}",
        "evidence_manifest": evidence_manifest_path.as_posix(),
        "projection_count": len(projections),
        "projections": projections,
        "check_input_derivations": input_reports,
    }


def main() -> int:
    """Run projection graph validation."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--graph", type=Path, default=DEFAULT_GRAPH)
    parser.add_argument("--compiled", type=Path, default=DEFAULT_COMPILED_ARCHITECTURE)
    parser.add_argument("--evidence-manifest", type=Path, default=DEFAULT_EVIDENCE_MANIFEST)
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    root = args.root.resolve()
    report = validate(root, args.graph, args.evidence_manifest, args.compiled)
    if args.json_report is not None:
        report_path = (root / args.json_report).resolve()
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    print(
        "validated projection graph: "
        f"{report['projection_count']} projections"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
