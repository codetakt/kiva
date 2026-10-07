#!/usr/bin/env python3
"""Generate the public-claim readiness matrix from claim and evidence ledgers."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.evidence import (
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
from kiva_ci.projection_graph import load_projection_graph
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_CATALOG = Path("spec/architecture/claim-catalog.yaml")
DEFAULT_MATURITY = Path("spec/architecture/claim-maturity.yaml")
DEFAULT_PROJECTION_GRAPH = Path("spec/ci/projection-graph.yaml")
DEFAULT_EVIDENCE_CHECKS = Path("spec/ci/evidence-checks.yaml")
DEFAULT_MATRIX = Path("generated/claim-readiness-matrix.yaml")
DEFAULT_MARKDOWN = Path("docs/architecture/generated/claim-readiness-matrix.md")
DEFAULT_EVIDENCE = Path("generated/claim-readiness-matrix.evidence.json")
READINESS_EVIDENCE_MODES = {"projection-backed", "path-only"}


class ClaimReadinessError(RuntimeError):
    """Raised when the claim readiness matrix cannot be generated."""


def maturity_level_ids(maturity: dict[str, Any]) -> set[str]:
    """Return canonical claim maturity ids."""
    if maturity.get("schema_version") != 1:
        raise ClaimReadinessError("claim maturity schema_version must be 1")
    if maturity.get("kind") != "kiva-claim-maturity-catalog":
        raise ClaimReadinessError("claim maturity kind is invalid")
    level_ids: set[str] = set()
    for index, raw_level in enumerate(
        require_list(maturity.get("levels"), "claim-maturity.levels", ClaimReadinessError)
    ):
        level = require_mapping(raw_level, f"claim-maturity.levels[{index}]", ClaimReadinessError)
        level_id = require_string(level.get("id"), f"claim-maturity.levels[{index}].id", ClaimReadinessError)
        if level_id in level_ids:
            raise ClaimReadinessError(f"duplicate claim maturity level: {level_id}")
        level_ids.add(level_id)
    return level_ids


def evidence_check_matrix(root: Path, manifest_path: Path) -> dict[tuple[str, str], dict[str, Any]]:
    """Return evidence check metadata keyed by group and check id."""
    manifest = load_evidence_check_manifest(root / manifest_path, ClaimReadinessError)
    groups = validate_evidence_check_manifest_metadata(
        manifest,
        manifest_path.as_posix(),
        ClaimReadinessError,
    )
    checks: dict[tuple[str, str], dict[str, Any]] = {}
    for group_id, group in groups.items():
        for index, raw_check in enumerate(
            require_list(group.get("checks"), f"{group_id}.checks", ClaimReadinessError)
        ):
            check = require_mapping(raw_check, f"{group_id}.checks[{index}]", ClaimReadinessError)
            check_id = require_string(
                check.get("id"),
                f"{group_id}.checks[{index}].id",
                ClaimReadinessError,
            )
            checks[(group_id, check_id)] = {
                "group": group_id,
                "check_id": check_id,
                "check_kind": require_string(
                    check.get("check_kind"),
                    f"{group_id}.{check_id}.check_kind",
                    ClaimReadinessError,
                ),
                "artifact_class": require_string(
                    check.get("artifact_class"),
                    f"{group_id}.{check_id}.artifact_class",
                    ClaimReadinessError,
                ),
            }
    return checks


def require_nonnegative_int(value: Any, label: str) -> int:
    """Return a non-negative integer readiness threshold."""
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise ClaimReadinessError(f"{label} must be a non-negative integer")
    return value


def projection_rows(graph: dict[str, Any]) -> list[dict[str, Any]]:
    """Return normalized projection rows from the projection graph."""
    rows: list[dict[str, Any]] = []
    for index, raw_projection in enumerate(
        require_list(graph.get("projections"), "projection-graph.projections", ClaimReadinessError)
    ):
        projection = require_mapping(
            raw_projection,
            f"projection-graph.projections[{index}]",
            ClaimReadinessError,
        )
        validators = []
        for validator_index, raw_validator in enumerate(
            require_list(
                projection.get("validators"),
                f"projection-graph.projections[{index}].validators",
                ClaimReadinessError,
            )
        ):
            validator = require_mapping(
                raw_validator,
                f"projection-graph.projections[{index}].validators[{validator_index}]",
                ClaimReadinessError,
            )
            validators.append(
                {
                    "group": require_string(
                        validator.get("group"),
                        f"projection-graph.projections[{index}].validators[{validator_index}].group",
                        ClaimReadinessError,
                    ),
                    "check_id": require_string(
                        validator.get("check_id"),
                        f"projection-graph.projections[{index}].validators[{validator_index}].check_id",
                        ClaimReadinessError,
                    ),
                }
            )
        rows.append(
            {
                "id": require_string(
                    projection.get("id"),
                    f"projection-graph.projections[{index}].id",
                    ClaimReadinessError,
                ),
                "source_layer": require_string(
                    projection.get("source_layer"),
                    f"projection-graph.projections[{index}].source_layer",
                    ClaimReadinessError,
                ),
                "sources": set(
                    require_string_list(
                        projection.get("sources"),
                        f"projection-graph.projections[{index}].sources",
                        ClaimReadinessError,
                    )
                ),
                "targets": set(
                    require_string_list(
                        projection.get("targets"),
                        f"projection-graph.projections[{index}].targets",
                        ClaimReadinessError,
                    )
                ),
                "validators": validators,
            }
        )
    return rows


def claim_entries(catalog: dict[str, Any], maturity_ids: set[str]) -> list[dict[str, Any]]:
    """Return normalized public claim rows."""
    if catalog.get("schema_version") != 1:
        raise ClaimReadinessError("claim catalog schema_version must be 1")
    if catalog.get("kind") != "kiva-architecture-claim-catalog":
        raise ClaimReadinessError("claim catalog kind is invalid")
    entries: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, raw_entry in enumerate(
        require_list(catalog.get("entries"), "claim-catalog.entries", ClaimReadinessError)
    ):
        entry = require_mapping(raw_entry, f"claim-catalog.entries[{index}]", ClaimReadinessError)
        entry_id = require_string(entry.get("id"), f"claim-catalog.entries[{index}].id", ClaimReadinessError)
        if entry_id in seen:
            raise ClaimReadinessError(f"duplicate claim id: {entry_id}")
        seen.add(entry_id)
        maturity = require_string(entry.get("claim_maturity"), f"{entry_id}.claim_maturity", ClaimReadinessError)
        if maturity not in maturity_ids:
            raise ClaimReadinessError(f"{entry_id}.claim_maturity is invalid: {maturity}")
        requirement = require_mapping(
            entry.get("readiness_requirement"),
            f"{entry_id}.readiness_requirement",
            ClaimReadinessError,
        )
        evidence_mode = require_string(
            requirement.get("evidence_mode"),
            f"{entry_id}.readiness_requirement.evidence_mode",
            ClaimReadinessError,
        )
        if evidence_mode not in READINESS_EVIDENCE_MODES:
            raise ClaimReadinessError(
                f"{entry_id}.readiness_requirement.evidence_mode is invalid: {evidence_mode}"
            )
        minimum_projection_matches = require_nonnegative_int(
            requirement.get("minimum_projection_matches"),
            f"{entry_id}.readiness_requirement.minimum_projection_matches",
        )
        minimum_evidence_check_matches = require_nonnegative_int(
            requirement.get("minimum_evidence_check_matches"),
            f"{entry_id}.readiness_requirement.minimum_evidence_check_matches",
        )
        if evidence_mode == "projection-backed" and (
            minimum_projection_matches < 1 or minimum_evidence_check_matches < 1
        ):
            raise ClaimReadinessError(
                f"{entry_id}.readiness_requirement for projection-backed claims must require "
                "at least one projection and one evidence check"
            )
        if evidence_mode == "path-only" and (
            minimum_projection_matches != 0 or minimum_evidence_check_matches != 0
        ):
            raise ClaimReadinessError(
                f"{entry_id}.readiness_requirement for path-only claims must use zero thresholds"
            )
        normalized_requirement = {
            "evidence_mode": evidence_mode,
            "minimum_projection_matches": minimum_projection_matches,
            "minimum_evidence_check_matches": minimum_evidence_check_matches,
            "rationale": require_string(
                requirement.get("rationale"),
                f"{entry_id}.readiness_requirement.rationale",
                ClaimReadinessError,
            ),
        }
        entries.append(
            {
                "id": entry_id,
                "status": require_string(entry.get("status"), f"{entry_id}.status", ClaimReadinessError),
                "claim_maturity": maturity,
                "boundary": require_string(entry.get("boundary"), f"{entry_id}.boundary", ClaimReadinessError),
                "claim": require_string(entry.get("claim"), f"{entry_id}.claim", ClaimReadinessError),
                "readiness_requirement": normalized_requirement,
                "evidence_paths": require_string_list(
                    entry.get("evidence_paths"),
                    f"{entry_id}.evidence_paths",
                    ClaimReadinessError,
                ),
            }
        )
    return entries


def readiness_matrix(
    root: Path,
    catalog: dict[str, Any],
    maturity: dict[str, Any],
    graph: dict[str, Any],
    check_matrix: dict[tuple[str, str], dict[str, Any]],
) -> dict[str, Any]:
    """Return a claim readiness matrix."""
    maturity_ids = maturity_level_ids(maturity)
    projections = projection_rows(graph)
    claims = []
    not_ready: list[str] = []
    for entry in claim_entries(catalog, maturity_ids):
        for path in entry["evidence_paths"]:
            require_existing_path(root, path, f"{entry['id']}.evidence_paths", ClaimReadinessError)
        evidence_path_set = set(entry["evidence_paths"])
        matched_projections = []
        matched_checks: dict[tuple[str, str], dict[str, Any]] = {}
        for projection in projections:
            projection_paths = projection["sources"].union(projection["targets"])
            if evidence_path_set.isdisjoint(projection_paths):
                continue
            matched_projections.append(projection["id"])
            for validator in projection["validators"]:
                key = (validator["group"], validator["check_id"])
                check = check_matrix.get(key)
                if check is None:
                    raise ClaimReadinessError(
                        f"{projection['id']} references missing check {key[0]}.{key[1]}"
                )
                matched_checks[key] = check
        matched_projection_ids = sorted(set(matched_projections))
        matched_evidence_checks = [matched_checks[key] for key in sorted(matched_checks)]
        requirement = entry["readiness_requirement"]
        readiness_reasons = []
        if entry["status"] != "internally-verified":
            readiness_reasons.append(f"status is {entry['status']}")
        if len(matched_projection_ids) < requirement["minimum_projection_matches"]:
            readiness_reasons.append(
                "projection matches "
                f"{len(matched_projection_ids)} < {requirement['minimum_projection_matches']}"
            )
        if len(matched_evidence_checks) < requirement["minimum_evidence_check_matches"]:
            readiness_reasons.append(
                "evidence check matches "
                f"{len(matched_evidence_checks)} < {requirement['minimum_evidence_check_matches']}"
            )
        readiness = "ready" if not readiness_reasons else "not-ready"
        if readiness != "ready":
            not_ready.append(f"{entry['id']} ({'; '.join(readiness_reasons)})")
        claims.append(
            {
                **entry,
                "readiness": readiness,
                "readiness_reason": (
                    "requirement satisfied"
                    if readiness == "ready"
                    else "; ".join(readiness_reasons)
                ),
                "evidence_path_count": len(entry["evidence_paths"]),
                "matched_projection_ids": matched_projection_ids,
                "matched_evidence_checks": matched_evidence_checks,
            }
        )
    if not_ready:
        raise ClaimReadinessError(
            "claim readiness requirements are not satisfied: " + ", ".join(not_ready)
        )
    return {
        "schema_version": 1,
        "kind": "kiva-claim-readiness-matrix",
        "status": "ready",
        "generated_from": [
            DEFAULT_CATALOG.as_posix(),
            DEFAULT_MATURITY.as_posix(),
            DEFAULT_PROJECTION_GRAPH.as_posix(),
            DEFAULT_EVIDENCE_CHECKS.as_posix(),
        ],
        "claim_count": len(claims),
        "claims": claims,
    }


def markdown_cell(value: object) -> str:
    """Return a Markdown table cell."""
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_markdown(matrix: dict[str, Any]) -> str:
    """Render the claim readiness matrix as review Markdown."""
    rows = [
        "| Contract ID | Boundary | Evidence Mode | Artifact Paths | Projections | Check Definitions |",
        "| --- | --- | --- | ---: | ---: | ---: |",
    ]
    for claim in matrix["claims"]:
        rows.append(
            "| "
            + " | ".join(
                [
                    f"`{markdown_cell(claim['id'])}`",
                    f"`{markdown_cell(claim['boundary'])}`",
                    f"`{markdown_cell(claim['readiness_requirement']['evidence_mode'])}`",
                    str(claim["evidence_path_count"]),
                    str(len(claim["matched_projection_ids"])),
                    str(len(claim["matched_evidence_checks"])),
                ]
            )
            + " |"
        )
    return f"""# Generated Claim Readiness Matrix

This file is generated from the claim catalog, claim maturity catalog, CI
projection graph, and evidence-check manifest by
`scripts/ci/generate_claim_readiness_matrix.py`.

It maps contract boundaries to artifact paths, projection edges and check
definitions. Run-specific readiness assessments are kept locally under `target/`.

{chr(10).join(rows)}
"""


def render_evidence(matrix: dict[str, Any]) -> str:
    """Render machine-readable evidence JSON."""
    evidence = {
        "schema_version": 1,
        "kind": "kiva-claim-readiness-matrix-evidence",
        "status": matrix["status"],
        "claim_count": matrix["claim_count"],
        "generated_from": matrix["generated_from"],
        "claims": matrix["claims"],
    }
    return json.dumps(evidence, indent=2, sort_keys=True) + "\n"


def main() -> int:
    """Generate or validate claim readiness artifacts."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=DEFAULT_CATALOG)
    parser.add_argument("--maturity", type=Path, default=DEFAULT_MATURITY)
    parser.add_argument("--projection-graph", type=Path, default=DEFAULT_PROJECTION_GRAPH)
    parser.add_argument("--evidence-checks", type=Path, default=DEFAULT_EVIDENCE_CHECKS)
    parser.add_argument("--matrix-out", type=Path, default=DEFAULT_MATRIX)
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--evidence-out", type=Path, default=DEFAULT_EVIDENCE)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = Path(".").resolve()
    matrix = readiness_matrix(
        root,
        load_yaml(root / args.catalog, ClaimReadinessError),
        load_yaml(root / args.maturity, ClaimReadinessError),
        load_projection_graph(root / args.projection_graph, ClaimReadinessError),
        evidence_check_matrix(root, args.evidence_checks),
    )
    matrix_yaml = dump_yaml(matrix)
    markdown = render_markdown(matrix)
    evidence = render_evidence(matrix)
    command_hint = "scripts/ci/generate_claim_readiness_matrix.py"
    try:
        write_or_check_generated_artifacts(
            [
                GeneratedArtifact(args.matrix_out, matrix_yaml),
                GeneratedArtifact(args.markdown_out, markdown),
                GeneratedArtifact(args.evidence_out, evidence),
            ],
            check=args.check,
            command_hint=command_hint,
            error_cls=ClaimReadinessError,
        )
    except ClaimReadinessError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated claim readiness matrix: {args.matrix_out}")
        return 0

    print(f"generated claim readiness matrix: {args.matrix_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
