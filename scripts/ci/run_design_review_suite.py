#!/usr/bin/env python3
"""Run a small design-review CI suite over manifest-declared evidence groups."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from kiva_ci.evidence import load_evidence_check_manifest
from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string
from run_evidence_checks import DEFAULT_MANIFEST, DEFAULT_PROJECTION_GRAPH, run_group


class DesignReviewSuiteError(RuntimeError):
    """Raised when a design-review suite cannot be resolved."""


DEFAULT_READINESS_MATRIX = Path("generated/claim-readiness-matrix.yaml")

SUITES = {
    "all": "all required evidence-check groups",
    "contract": "OpenAPI contract and architecture projection checks",
    "confidential": "Confidential-computation assurance checks",
}

SUITE_CLAIM_BOUNDARIES = {
    "contract": "openapi-contract",
    "confidential": "confidential-computation-assurance",
}


def required_group_metadata(manifest: dict[str, Any]) -> dict[str, dict[str, str]]:
    """Return top-level required evidence-check group metadata keyed by id."""
    groups: dict[str, dict[str, str]] = {}
    for index, value in enumerate(
        require_list(
            manifest.get("required_groups"),
            "required_groups",
            DesignReviewSuiteError,
        )
    ):
        entry = require_mapping(
            value,
            f"required_groups[{index}]",
            DesignReviewSuiteError,
        )
        group_id = require_string(
            entry.get("id"),
            f"required_groups[{index}].id",
            DesignReviewSuiteError,
        )
        if group_id in groups:
            raise DesignReviewSuiteError(f"duplicate required evidence-check group: {group_id}")
        groups[group_id] = {
            "claim_boundary": require_string(
                entry.get("claim_boundary"),
                f"required_groups[{index}].claim_boundary",
                DesignReviewSuiteError,
            ),
            "nix_gate": require_string(
                entry.get("nix_gate"),
                f"required_groups[{index}].nix_gate",
                DesignReviewSuiteError,
            ),
        }
    if not groups:
        raise DesignReviewSuiteError("required_groups must not be empty")
    return groups


def required_group_ids(manifest: dict[str, Any]) -> list[str]:
    """Return top-level required evidence-check groups in manifest order."""
    return list(required_group_metadata(manifest))


def group_ids_for_claim_boundary(manifest: dict[str, Any], claim_boundary: str) -> list[str]:
    """Return required evidence groups with the requested claim boundary."""
    groups = [
        group_id
        for group_id, metadata in required_group_metadata(manifest).items()
        if metadata["claim_boundary"] == claim_boundary
    ]
    if not groups:
        raise DesignReviewSuiteError(
            f"no required evidence-check group declares claim_boundary={claim_boundary}"
        )
    return groups


def suite_group_ids(
    manifest: dict[str, Any],
    suite: str,
    explicit_groups: list[str],
) -> list[str]:
    """Resolve the groups for one named suite."""
    if explicit_groups:
        return explicit_groups
    if suite == "all":
        return required_group_ids(manifest)
    claim_boundary = SUITE_CLAIM_BOUNDARIES.get(suite)
    if claim_boundary is None:
        valid = ", ".join(sorted(SUITES))
        raise DesignReviewSuiteError(f"unknown suite: {suite}; expected one of {valid}")
    return group_ids_for_claim_boundary(manifest, claim_boundary)


def readiness_supported_groups(readiness: dict[str, Any]) -> set[str]:
    """Return evidence-check groups attached to ready public claims."""
    groups: set[str] = set()
    for index, raw_claim in enumerate(
        require_list(readiness.get("claims"), "claim-readiness.claims", DesignReviewSuiteError)
    ):
        claim = require_mapping(raw_claim, f"claim-readiness.claims[{index}]", DesignReviewSuiteError)
        if claim.get("readiness") != "ready":
            continue
        for check_index, raw_check in enumerate(
            require_list(
                claim.get("matched_evidence_checks", []),
                f"claim-readiness.claims[{index}].matched_evidence_checks",
                DesignReviewSuiteError,
            )
        ):
            check = require_mapping(
                raw_check,
                f"claim-readiness.claims[{index}].matched_evidence_checks[{check_index}]",
                DesignReviewSuiteError,
            )
            groups.add(
                require_string(
                    check.get("group"),
                    f"claim-readiness.claims[{index}].matched_evidence_checks[{check_index}].group",
                    DesignReviewSuiteError,
                )
            )
    return groups


def validate_readiness_support(readiness_path: Path, selected_groups: list[str]) -> None:
    """Validate that selected evidence groups appear in the readiness matrix."""
    if not readiness_path.exists():
        return
    readiness = load_yaml(readiness_path, DesignReviewSuiteError)
    if readiness.get("kind") != "kiva-claim-readiness-matrix":
        raise DesignReviewSuiteError(f"{readiness_path}.kind must be kiva-claim-readiness-matrix")
    supported = readiness_supported_groups(readiness)
    missing = sorted(set(selected_groups).difference(supported))
    if missing:
        raise DesignReviewSuiteError(
            f"{readiness_path} has no ready claim evidence for groups: {', '.join(missing)}"
        )


def run_suite(
    root: Path,
    manifest_path: Path,
    projection_graph_path: Path,
    readiness_path: Path,
    suite: str,
    groups: list[str],
    reports_dir: Path,
) -> dict[str, Any]:
    """Run the resolved design-review suite."""
    manifest = load_evidence_check_manifest(manifest_path, DesignReviewSuiteError)
    selected_groups = suite_group_ids(manifest, suite, groups)
    validate_readiness_support(readiness_path, selected_groups)
    reports_dir.mkdir(parents=True, exist_ok=True)
    group_reports = []
    for group_id in selected_groups:
        group_reports.append(
            run_group(
                root,
                manifest_path,
                projection_graph_path,
                group_id,
                reports_dir / group_id,
            )
        )
    return {
        "kind": "kiva-design-review-suite-run",
        "status": "passed",
        "suite": suite,
        "manifest": manifest_path.relative_to(root).as_posix(),
        "readiness_matrix": readiness_path.relative_to(root).as_posix(),
        "groups": group_reports,
        "group_count": len(group_reports),
        "check_count": sum(group["check_count"] for group in group_reports),
    }


def main() -> int:
    """Run a design-review CI suite."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--projection-graph", type=Path, default=DEFAULT_PROJECTION_GRAPH)
    parser.add_argument("--readiness-matrix", type=Path, default=DEFAULT_READINESS_MATRIX)
    parser.add_argument("--suite", choices=sorted(SUITES), default="all")
    parser.add_argument("--group", action="append", default=[])
    parser.add_argument("--reports-dir", type=Path, default=Path("reports/design-review"))
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    root = args.root.resolve()
    report = run_suite(
        root,
        (root / args.manifest).resolve(),
        (root / args.projection_graph).resolve(),
        (root / args.readiness_matrix).resolve(),
        args.suite,
        args.group,
        (root / args.reports_dir).resolve(),
    )
    if args.json_report is not None:
        report_path = (root / args.json_report).resolve()
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    print(
        "validated design-review suite "
        f"{args.suite}: {report['group_count']} groups, {report['check_count']} checks"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
