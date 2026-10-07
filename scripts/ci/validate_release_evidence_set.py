#!/usr/bin/env python3
"""Validate that conformance and formal evidence bundles form one release evidence set."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from validate_conformance_evidence_bundle import (
    DEFAULT_MANIFEST_SCHEMA as DEFAULT_CONFORMANCE_MANIFEST_SCHEMA,
)
from validate_conformance_evidence_bundle import (
    validate_bundle as validate_conformance_bundle,
)
from validate_formal_verification_evidence_bundle import (
    DEFAULT_MANIFEST_SCHEMA as DEFAULT_FORMAL_MANIFEST_SCHEMA,
)
from validate_formal_verification_evidence_bundle import (
    DEFAULT_REPORT_SCHEMA as DEFAULT_FORMAL_REPORT_SCHEMA,
)
from validate_formal_verification_evidence_bundle import (
    validate_bundle as validate_formal_bundle,
)


class ReleaseEvidenceSetError(RuntimeError):
    """Raised when release evidence bundles cannot support one release claim."""


def load_json(path: Path) -> Any:
    """Load JSON from a file."""
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def load_manifest(bundle: Path) -> dict[str, Any]:
    """Load one bundle manifest as a JSON object."""
    manifest_path = bundle / "metadata/manifest.json"
    if not manifest_path.is_file():
        raise ReleaseEvidenceSetError(f"missing manifest: {manifest_path}")
    manifest = load_json(manifest_path)
    if not isinstance(manifest, dict):
        raise ReleaseEvidenceSetError(f"manifest root must be an object: {manifest_path}")
    return manifest


def require(condition: bool, message: str) -> None:
    """Raise a release evidence error unless condition holds."""
    if not condition:
        raise ReleaseEvidenceSetError(message)


def require_clean_release_manifest(label: str, manifest: dict[str, Any]) -> None:
    """Require a manifest to come from a clean release-candidate evidence run."""
    git = manifest["git"]
    release_requirements = manifest["release_requirements"]
    require(not git["dirty"], f"{label} evidence was generated from a dirty tree")
    require(
        release_requirements["require_clean_git"],
        f"{label} evidence was not generated with --require-clean-git",
    )
    require(
        release_requirements["met_clean_git"],
        f"{label} evidence did not satisfy the clean-git release requirement",
    )


def validate_commit_pair(
    conformance: dict[str, Any],
    formal: dict[str, Any],
    *,
    allow_mismatched_commit: bool,
    allow_unknown_commit: bool,
) -> None:
    """Validate commit identity across both evidence bundles."""
    conformance_commit = conformance["git"]["commit"]
    formal_commit = formal["git"]["commit"]
    if not allow_unknown_commit:
        require(conformance_commit != "unknown", "conformance evidence has unknown git commit")
        require(formal_commit != "unknown", "formal evidence has unknown git commit")
    if not allow_mismatched_commit:
        require(
            conformance_commit == formal_commit,
            "evidence bundles refer to different commits: "
            f"conformance={conformance_commit}, formal={formal_commit}",
        )


def validate_conformance_release_scope(
    conformance: dict[str, Any],
    conformance_root: Path,
    *,
    allow_static_conformance: bool,
    require_adl_run: bool,
) -> None:
    """Require full live xAPI evidence unless static conformance is explicitly allowed."""
    if not allow_static_conformance:
        require(
            conformance["live_http"]["memory"],
            "release conformance evidence must include memory HTTP execution",
        )
        require(
            conformance["live_http"]["postgres"],
            "release conformance evidence must include PostgreSQL HTTP execution",
        )
        require(
            conformance["verification"]["postgres"],
            "release conformance evidence must include PostgreSQL execution verification",
        )
    if require_adl_run:
        report_path = conformance_root / "reports/adl-run.json"
        require(report_path.is_file(), "release conformance evidence must include an ADL run")
        report = load_json(report_path)
        require(isinstance(report, dict), "ADL run report root must be an object")
        require(report["status"] == "passed", "ADL run report is not passed")
        totals = report["totals"]
        require(totals["mismatched"] == 0, "ADL run has mismatched expectations")
        require(totals["unknown"] == 0, "ADL run has unknown test ids")
        require(totals["missing"] == 0, "ADL run has missing test ids")


def validate_formal_release_scope(
    formal: dict[str, Any],
    *,
    allow_formal_smoke: bool,
) -> None:
    """Require complete formal evidence for a release claim."""
    require(formal["status"] == "passed", "formal evidence manifest is not passed")
    require(formal["totals"]["failed"] == 0, "formal evidence contains failed checks")
    require(formal["totals"]["timeout"] == 0, "formal evidence contains timeout checks")
    for tool_name in ["fstar", "kani", "tamarin"]:
        tool = formal["verification"][tool_name]
        require(tool["enabled"], f"release formal evidence must enable {tool_name}")
        require(tool["status"] == "passed", f"{tool_name} evidence is not passed")
        require(tool["checks"] > 0, f"{tool_name} evidence has no checks")
    kani_suite = formal["verification"]["kani"]["suite"]
    if allow_formal_smoke:
        require(kani_suite in {"smoke", "regression"}, f"unexpected Kani suite: {kani_suite}")
    else:
        require(
            kani_suite == "regression",
            f"release formal evidence must use Kani regression suite, got {kani_suite}",
        )


def validate_release_evidence_set(args: argparse.Namespace) -> None:
    """Validate both bundles and their combined release evidence boundary."""
    conformance_root = args.conformance.resolve()
    formal_root = args.formal.resolve()
    require(conformance_root.is_dir(), f"conformance bundle is not a directory: {conformance_root}")
    require(formal_root.is_dir(), f"formal bundle is not a directory: {formal_root}")

    validate_conformance_bundle(conformance_root, args.conformance_manifest_schema)
    validate_formal_bundle(
        formal_root,
        args.formal_manifest_schema,
        args.formal_report_schema,
    )

    conformance = load_manifest(conformance_root)
    formal = load_manifest(formal_root)
    validate_commit_pair(
        conformance,
        formal,
        allow_mismatched_commit=args.allow_mismatched_commit,
        allow_unknown_commit=args.allow_unknown_commit,
    )
    if not args.allow_dirty:
        require_clean_release_manifest("conformance", conformance)
        require_clean_release_manifest("formal", formal)
    validate_conformance_release_scope(
        conformance,
        conformance_root,
        allow_static_conformance=args.allow_static_conformance,
        require_adl_run=args.require_adl_run,
    )
    validate_formal_release_scope(
        formal,
        allow_formal_smoke=args.allow_formal_smoke,
    )


def main() -> int:
    """Run release evidence-set validation."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--conformance", required=True, type=Path)
    parser.add_argument("--formal", required=True, type=Path)
    parser.add_argument(
        "--conformance-manifest-schema",
        type=Path,
        default=DEFAULT_CONFORMANCE_MANIFEST_SCHEMA,
    )
    parser.add_argument(
        "--formal-manifest-schema",
        type=Path,
        default=DEFAULT_FORMAL_MANIFEST_SCHEMA,
    )
    parser.add_argument("--formal-report-schema", type=Path, default=DEFAULT_FORMAL_REPORT_SCHEMA)
    parser.add_argument("--allow-dirty", action="store_true")
    parser.add_argument("--allow-mismatched-commit", action="store_true")
    parser.add_argument("--allow-unknown-commit", action="store_true")
    parser.add_argument(
        "--allow-static-conformance",
        action="store_true",
        help="allow bundles that skip live HTTP and PostgreSQL execution evidence",
    )
    parser.add_argument(
        "--allow-formal-smoke",
        action="store_true",
        help="allow Kani smoke evidence instead of the regression suite",
    )
    parser.add_argument(
        "--require-adl-run",
        action="store_true",
        help="require a successful optional full ADL execution report",
    )
    args = parser.parse_args()

    validate_release_evidence_set(args)
    print(
        "validated release evidence set: "
        f"conformance={args.conformance.resolve()} formal={args.formal.resolve()}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
