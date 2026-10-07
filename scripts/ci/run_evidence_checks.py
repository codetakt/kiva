#!/usr/bin/env python3
"""Run CI evidence validators declared in a manifest."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

from kiva_ci.evidence import (
    evidence_check_effective_inputs,
    load_evidence_check_manifest,
    select_evidence_check_group,
)
from kiva_ci.manifest import (
    require_list,
    require_mapping,
    require_string,
    require_string_list,
)
from kiva_ci.projection_graph import derived_inputs_by_check, load_projection_graph
from kiva_ci.runtime import RegistryCommand


DEFAULT_MANIFEST = Path("spec/ci/evidence-checks.yaml")
DEFAULT_PROJECTION_GRAPH = Path("spec/ci/projection-graph.yaml")


class EvidenceCheckError(RuntimeError):
    """Raised when evidence-check metadata is invalid."""


def select_group(manifest: dict[str, Any], group_id: str) -> dict[str, Any]:
    """Return one named evidence-check group."""
    return select_evidence_check_group(
        manifest,
        group_id,
        "spec/ci/evidence-checks.yaml",
        EvidenceCheckError,
    )


def run_group(
    root: Path,
    manifest_path: Path,
    projection_graph_path: Path,
    group_id: str,
    reports_dir: Path,
) -> dict[str, Any]:
    """Run all validators in one manifest group."""
    manifest = load_evidence_check_manifest(manifest_path, EvidenceCheckError)
    group = select_group(manifest, group_id)
    projection_graph = load_projection_graph(projection_graph_path, EvidenceCheckError)
    derived_inputs = derived_inputs_by_check(projection_graph, EvidenceCheckError)
    reports_dir.mkdir(parents=True, exist_ok=True)
    completed: list[dict[str, Any]] = []
    seen_ids: set[str] = set()

    for index, check in enumerate(
        require_list(group.get("checks"), f"{group_id}.checks", EvidenceCheckError)
    ):
        check = require_mapping(
            check,
            f"{group_id}.checks[{index}]",
            EvidenceCheckError,
        )
        check_id = require_string(
            check.get("id"),
            f"{group_id}.checks[{index}].id",
            EvidenceCheckError,
        )
        if check_id in seen_ids:
            raise EvidenceCheckError(f"duplicate evidence check id: {check_id}")
        seen_ids.add(check_id)
        check_kind = require_string(
            check.get("check_kind"),
            f"{check_id}.check_kind",
            EvidenceCheckError,
        )
        artifact_class = require_string(
            check.get("artifact_class"),
            f"{check_id}.artifact_class",
            EvidenceCheckError,
        )
        projection_inputs = sorted(derived_inputs.get((group_id, check_id), set()))
        input_model = evidence_check_effective_inputs(
            check,
            projection_inputs,
            check_id,
            EvidenceCheckError,
            require_projection_graph_inputs=True,
        )
        for projection_input in projection_inputs:
            if not (root / projection_input).exists():
                raise EvidenceCheckError(
                    f"{check_id}.derived_projection_inputs path does not exist: "
                    f"{projection_input}"
                )
        outputs = require_string_list(
            check.get("outputs", []),
            f"{check_id}.outputs",
            EvidenceCheckError,
            allow_empty=True,
        )
        mutates_repository = check.get("mutates_repository")
        if not isinstance(mutates_repository, bool):
            raise EvidenceCheckError(f"{check_id}.mutates_repository must be a boolean")
        requires_network = check.get("requires_network")
        if not isinstance(requires_network, bool):
            raise EvidenceCheckError(f"{check_id}.requires_network must be a boolean")
        script = root / require_string(
            check.get("script"),
            f"{check_id}.script",
            EvidenceCheckError,
        )
        if not script.exists():
            raise EvidenceCheckError(f"{check_id}.script does not exist: {script}")

        check_args = require_string_list(
            check.get("args", []),
            f"{check_id}.args",
            EvidenceCheckError,
            allow_empty=True,
        )
        report_name = check.get("json_report")
        report_path = None
        forwarded_runtime_args: list[str] = []
        if report_name is not None:
            report_path = reports_dir / require_string(
                report_name,
                f"{check_id}.json_report",
                EvidenceCheckError,
            )
            forwarded_runtime_args.extend(["--json-report", report_path.as_posix()])

        ci_command = RegistryCommand.from_check(root, check, check_id, EvidenceCheckError)
        subprocess.run(ci_command.argv(forwarded_runtime_args), cwd=root, check=True)
        completed.append(
            {
                "id": check_id,
                "check_kind": check_kind,
                "artifact_class": artifact_class,
                "inputs": input_model["inputs"],
                "derived_input_policy": input_model["derived_input_policy"],
                "derived_projection_inputs": input_model["derived_projection_inputs"],
                "effective_inputs": input_model["effective_inputs"],
                "outputs": outputs,
                "mutates_repository": mutates_repository,
                "requires_network": requires_network,
                "script": script.relative_to(root).as_posix(),
                "args": check_args,
                "ci_command": ci_command.report(root),
                "json_report": None
                if report_path is None
                else display_path(report_path, root),
            }
        )

    return {
        "kind": "kiva-ci-evidence-check-run",
        "status": "passed",
        "manifest": manifest_path.relative_to(root).as_posix(),
        "group": group_id,
        "checks": completed,
        "check_count": len(completed),
    }


def display_path(path: Path, root: Path) -> str:
    """Return a repository-relative path when possible."""
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def main() -> int:
    """Run manifest-declared evidence checks."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--projection-graph", type=Path, default=DEFAULT_PROJECTION_GRAPH)
    parser.add_argument("--group", required=True)
    parser.add_argument("--reports-dir", type=Path, default=Path("reports"))
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    root = args.root.resolve()
    manifest_path = (root / args.manifest).resolve()
    projection_graph_path = (root / args.projection_graph).resolve()
    reports_dir = (root / args.reports_dir).resolve()
    report = run_group(root, manifest_path, projection_graph_path, args.group, reports_dir)
    if args.json_report is not None:
        report_path = (root / args.json_report).resolve()
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    print(
        "validated evidence-check group "
        f"{args.group}: {report['check_count']} checks"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
