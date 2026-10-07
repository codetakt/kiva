#!/usr/bin/env python3
"""Validate the confidential verified-kernel ledger."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from kiva_ci.manifest import (
    load_yaml,
    relative_path,
    require_existing_path,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
)


DEFAULT_LEDGER = Path("spec/verification/confidential-verified-kernels.yaml")
EXPECTED_KERNEL_IDS = {
    "additive-share-admission-and-release",
    "differential-privacy-budget-release",
    "shamir-vss-reconstruction",
    "psi-cohort-and-oprf-release",
    "tee-attestation-gates",
    "threshold-frost-runtime",
    "fhe-feasibility-and-runtime",
    "confidential-job-participant-lifecycle",
    "provider-runtime-boundary",
}
EVIDENCE_SECTION_BY_KIND = {
    "implementation": "implementation_anchors",
    "rust-test": "rust_tests",
    "kani": "kani_harnesses",
    "tamarin": "tamarin_lemmas",
    "fstar": "fstar_modules",
    "assurance-registry": "assurance_registry_entries",
}


class ConfidentialVerifiedKernelError(RuntimeError):
    """Raised when the confidential verified-kernel ledger drifts."""


def read_source(root: Path, raw_path: str, label: str) -> str:
    """Read an existing repository-relative source file."""
    path = require_existing_path(root, raw_path, label, ConfidentialVerifiedKernelError)
    return (root / path).read_text(encoding="utf-8")


def require_anchor(text: str, anchor: str, label: str) -> None:
    """Require a source text anchor to exist."""
    if anchor not in text:
        raise ConfidentialVerifiedKernelError(f"{label} missing source anchor: {anchor}")


def validate_anchor_entries(
    root: Path,
    kernel_id: str,
    entries: list[Any],
    section: str,
    marker_field: str,
    marker_prefix: str = "",
) -> int:
    """Validate entries shaped as file plus list of source markers."""
    count = 0
    for index, value in enumerate(entries):
        entry = require_mapping(
            value,
            f"{kernel_id}.{section}[{index}]",
            ConfidentialVerifiedKernelError,
        )
        raw_file = require_string(
            entry.get("file"),
            f"{kernel_id}.{section}[{index}].file",
            ConfidentialVerifiedKernelError,
        )
        text = read_source(root, raw_file, f"{kernel_id}.{section}[{index}].file")
        markers = require_string_list(
            entry.get(marker_field),
            f"{kernel_id}.{section}[{index}].{marker_field}",
            ConfidentialVerifiedKernelError,
        )
        for marker in markers:
            require_anchor(text, f"{marker_prefix}{marker}", f"{kernel_id}.{section}[{index}]")
            count += 1
    return count


def validate_kani_harnesses(
    root: Path,
    runner_text: str,
    kernel_id: str,
    kernel: dict[str, Any],
) -> int:
    """Validate Kani source anchors and runner coverage."""
    count = 0
    entries = require_list(
        kernel.get("kani_harnesses", []),
        f"{kernel_id}.kani_harnesses",
        ConfidentialVerifiedKernelError,
    )
    for index, value in enumerate(entries):
        entry = require_mapping(
            value,
            f"{kernel_id}.kani_harnesses[{index}]",
            ConfidentialVerifiedKernelError,
        )
        target = require_string(
            entry.get("target"),
            f"{kernel_id}.kani_harnesses[{index}].target",
            ConfidentialVerifiedKernelError,
        )
        text = read_source(
            root,
            require_string(
                entry.get("file"),
                f"{kernel_id}.kani_harnesses[{index}].file",
                ConfidentialVerifiedKernelError,
            ),
            f"{kernel_id}.kani_harnesses[{index}].file",
        )
        harnesses = require_string_list(
            entry.get("harnesses"),
            f"{kernel_id}.kani_harnesses[{index}].harnesses",
            ConfidentialVerifiedKernelError,
        )
        for harness in harnesses:
            require_anchor(text, f"fn {harness}", f"{kernel_id}.kani_harnesses[{index}]")
            require_anchor(
                runner_text,
                f"{target}|kani_harnesses::{harness}",
                f"{kernel_id}.kani_harnesses[{index}]",
            )
            count += 1
    return count


def validate_fstar_modules(root: Path, kernel_id: str, kernel: dict[str, Any]) -> int:
    """Validate F* module and lemma anchors."""
    count = 0
    entries = require_list(
        kernel.get("fstar_modules", []),
        f"{kernel_id}.fstar_modules",
        ConfidentialVerifiedKernelError,
    )
    for index, value in enumerate(entries):
        entry = require_mapping(
            value,
            f"{kernel_id}.fstar_modules[{index}]",
            ConfidentialVerifiedKernelError,
        )
        raw_file = require_string(
            entry.get("file"),
            f"{kernel_id}.fstar_modules[{index}].file",
            ConfidentialVerifiedKernelError,
        )
        text = read_source(root, raw_file, f"{kernel_id}.fstar_modules[{index}].file")
        module = require_string(
            entry.get("module"),
            f"{kernel_id}.fstar_modules[{index}].module",
            ConfidentialVerifiedKernelError,
        )
        require_anchor(text, f"module {module}", f"{kernel_id}.fstar_modules[{index}]")
        for lemma in require_string_list(
            entry.get("lemmas"),
            f"{kernel_id}.fstar_modules[{index}].lemmas",
            ConfidentialVerifiedKernelError,
        ):
            require_anchor(text, f"let {lemma}", f"{kernel_id}.fstar_modules[{index}]")
            count += 1
    return count


def validate_assurance_registry_refs(
    registry_entries: dict[str, Any],
    kernel_id: str,
    kernel: dict[str, Any],
) -> int:
    """Validate assurance registry references."""
    entry_ids = require_string_list(
        kernel.get("assurance_registry_entries", []),
        f"{kernel_id}.assurance_registry_entries",
        ConfidentialVerifiedKernelError,
        allow_empty=True,
    )
    for entry_id in entry_ids:
        if entry_id not in registry_entries:
            raise ConfidentialVerifiedKernelError(
                f"{kernel_id}.assurance_registry_entries references missing entry: {entry_id}"
            )
    return len(entry_ids)


def validate_required_evidence(kernel_id: str, required: list[str], counts: dict[str, int]) -> None:
    """Validate one kernel satisfies its declared required evidence kinds."""
    for kind in required:
        if kind not in EVIDENCE_SECTION_BY_KIND:
            raise ConfidentialVerifiedKernelError(
                f"{kernel_id}.required_evidence_kinds is invalid: {kind}"
            )
        if counts.get(kind, 0) == 0:
            raise ConfidentialVerifiedKernelError(
                f"{kernel_id} missing required evidence kind: {kind}"
            )


def validate_kernel(
    root: Path,
    runner_text: str,
    registry_entries: dict[str, Any],
    index: int,
    value: Any,
) -> dict[str, Any]:
    """Validate one kernel ledger entry."""
    kernel = require_mapping(value, f"kernels[{index}]", ConfidentialVerifiedKernelError)
    kernel_id = require_string(kernel.get("id"), f"kernels[{index}].id", ConfidentialVerifiedKernelError)
    if kernel_id not in EXPECTED_KERNEL_IDS:
        raise ConfidentialVerifiedKernelError(f"unexpected confidential verified kernel id: {kernel_id}")
    if require_string(kernel.get("status"), f"{kernel_id}.status", ConfidentialVerifiedKernelError) != "active":
        raise ConfidentialVerifiedKernelError(f"{kernel_id}.status must be active")
    require_string(kernel.get("mechanism"), f"{kernel_id}.mechanism", ConfidentialVerifiedKernelError)
    require_string(kernel.get("claim_boundary"), f"{kernel_id}.claim_boundary", ConfidentialVerifiedKernelError)
    required = require_string_list(
        kernel.get("required_evidence_kinds"),
        f"{kernel_id}.required_evidence_kinds",
        ConfidentialVerifiedKernelError,
    )
    counts = {
        "implementation": validate_anchor_entries(
            root,
            kernel_id,
            require_list(
                kernel.get("implementation_anchors"),
                f"{kernel_id}.implementation_anchors",
                ConfidentialVerifiedKernelError,
            ),
            "implementation_anchors",
            "anchors",
        ),
        "rust-test": validate_anchor_entries(
            root,
            kernel_id,
            require_list(kernel.get("rust_tests", []), f"{kernel_id}.rust_tests", ConfidentialVerifiedKernelError),
            "rust_tests",
            "tests",
            "fn ",
        ),
        "kani": validate_kani_harnesses(root, runner_text, kernel_id, kernel),
        "tamarin": validate_anchor_entries(
            root,
            kernel_id,
            require_list(
                kernel.get("tamarin_lemmas", []),
                f"{kernel_id}.tamarin_lemmas",
                ConfidentialVerifiedKernelError,
            ),
            "tamarin_lemmas",
            "lemmas",
            "lemma ",
        ),
        "fstar": validate_fstar_modules(root, kernel_id, kernel),
        "assurance-registry": validate_assurance_registry_refs(registry_entries, kernel_id, kernel),
    }
    validate_required_evidence(kernel_id, required, counts)
    return {
        "id": kernel_id,
        "mechanism": kernel["mechanism"],
        "required_evidence_kinds": required,
        "evidence_counts": counts,
    }


def validate_ledger(root: Path, ledger_path: Path) -> dict[str, Any]:
    """Validate the complete confidential verified-kernel ledger."""
    ledger = load_yaml(root / ledger_path, ConfidentialVerifiedKernelError)
    if ledger.get("schema_version") != 1:
        raise ConfidentialVerifiedKernelError("schema_version must be 1")
    if ledger.get("kind") != "kiva-confidential-verified-kernel-ledger":
        raise ConfidentialVerifiedKernelError("kind must be kiva-confidential-verified-kernel-ledger")
    if ledger.get("status") != "active":
        raise ConfidentialVerifiedKernelError("status must be active")

    registries = require_mapping(
        ledger.get("source_registries"),
        "source_registries",
        ConfidentialVerifiedKernelError,
    )
    assurance_registry_path = require_existing_path(
        root,
        require_string(
            registries.get("assurance_evidence_registry"),
            "source_registries.assurance_evidence_registry",
            ConfidentialVerifiedKernelError,
        ),
        "source_registries.assurance_evidence_registry",
        ConfidentialVerifiedKernelError,
    )
    assurance_registry = load_yaml(root / assurance_registry_path, ConfidentialVerifiedKernelError)
    registry_entries = require_mapping(
        assurance_registry.get("entries"),
        "assurance_evidence_registry.entries",
        ConfidentialVerifiedKernelError,
    )
    runner_text = read_source(
        root,
        require_string(registries.get("kani_runner"), "source_registries.kani_runner", ConfidentialVerifiedKernelError),
        "source_registries.kani_runner",
    )
    read_source(
        root,
        require_string(
            registries.get("tamarin_runner"),
            "source_registries.tamarin_runner",
            ConfidentialVerifiedKernelError,
        ),
        "source_registries.tamarin_runner",
    )
    read_source(
        root,
        require_string(
            registries.get("fstar_runner"),
            "source_registries.fstar_runner",
            ConfidentialVerifiedKernelError,
        ),
        "source_registries.fstar_runner",
    )
    boundary_model = require_mapping(
        ledger.get("kernel_boundary_model"),
        "kernel_boundary_model",
        ConfidentialVerifiedKernelError,
    )
    if require_string(
        boundary_model.get("root_type"),
        "kernel_boundary_model.root_type",
        ConfidentialVerifiedKernelError,
    ) != "ConfidentialKernelBoundary":
        raise ConfidentialVerifiedKernelError("kernel_boundary_model.root_type must be ConfidentialKernelBoundary")
    boundary_text = read_source(
        root,
        require_string(
            boundary_model.get("implementation_anchor"),
            "kernel_boundary_model.implementation_anchor",
            ConfidentialVerifiedKernelError,
        ),
        "kernel_boundary_model.implementation_anchor",
    )
    require_anchor(boundary_text, "enum ConfidentialKernelBoundary", "kernel_boundary_model")
    if "provider adapters" not in require_string(
        boundary_model.get("rule"),
        "kernel_boundary_model.rule",
        ConfidentialVerifiedKernelError,
    ):
        raise ConfidentialVerifiedKernelError("kernel_boundary_model.rule must separate pure kernels from provider adapters")

    reports: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, value in enumerate(
        require_list(ledger.get("kernels"), "kernels", ConfidentialVerifiedKernelError)
    ):
        report = validate_kernel(root, runner_text, registry_entries, index, value)
        if report["id"] in seen:
            raise ConfidentialVerifiedKernelError(f"duplicate confidential verified kernel: {report['id']}")
        seen.add(report["id"])
        reports.append(report)
    if seen != EXPECTED_KERNEL_IDS:
        raise ConfidentialVerifiedKernelError(
            "confidential verified kernel set drift: "
            f"expected={sorted(EXPECTED_KERNEL_IDS)} actual={sorted(seen)}"
        )
    return {
        "ledger": str(ledger_path),
        "kernel_count": len(reports),
        "kernels": reports,
    }


def parse_args() -> argparse.Namespace:
    """Parse CLI arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--ledger", type=Path, default=DEFAULT_LEDGER)
    parser.add_argument("--json-report", type=Path)
    return parser.parse_args()


def main() -> int:
    """CLI entrypoint."""
    args = parse_args()
    root = args.root.resolve()
    ledger_path = relative_path(str(args.ledger), "--ledger", ConfidentialVerifiedKernelError)
    try:
        report = validate_ledger(root, ledger_path)
    except ConfidentialVerifiedKernelError as error:
        print(f"confidential verified-kernel validation failed: {error}", file=sys.stderr)
        return 1

    if args.json_report is not None:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    else:
        print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
