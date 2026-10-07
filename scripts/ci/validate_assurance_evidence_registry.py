#!/usr/bin/env python3
"""Validate the confidential-computation assurance evidence registry."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from source_scan import (
    DEFAULT_ASSURANCE_EVIDENCE_REGISTRY,
    EVIDENCE_KIND_FIELDS,
    confidential_rust_source,
    load_assurance_evidence_registry,
    missing_confidential_rust_terms,
    missing_kani_harnesses,
    missing_rust_tests,
    missing_tamarin_lemmas,
    registry_evidence_entries,
    registry_string_list,
)


class AssuranceEvidenceRegistryError(RuntimeError):
    """Raised when the assurance evidence registry is inconsistent."""


def require_string(value: Any, label: str) -> str:
    """Require a non-empty string."""
    if not isinstance(value, str) or not value:
        raise AssuranceEvidenceRegistryError(f"{label} must be a non-empty string")
    return value


def require_source_contains(source_file: str, needle: str, label: str) -> None:
    """Require a source file to exist and contain the expected evidence marker."""
    path = Path(source_file)
    if not path.exists():
        raise AssuranceEvidenceRegistryError(f"{label} source file does not exist: {source_file}")
    source = path.read_text(encoding="utf-8")
    if needle not in source:
        raise AssuranceEvidenceRegistryError(f"{label} source file does not contain: {needle}")


def evidence_needle(entry: dict[str, str]) -> str:
    """Return the text marker expected for one typed evidence entry."""
    kind = entry["kind"]
    value = entry[EVIDENCE_KIND_FIELDS[kind]]
    if kind == "rust-test" or kind == "kani-harness":
        return value.rsplit("::", maxsplit=1)[-1]
    if kind == "tamarin-lemma":
        return f"lemma {value}"
    return value


def validate_typed_evidence(entry_id: str) -> dict[str, int]:
    """Validate typed registry evidence entries and return counts by kind."""
    counts = {kind: 0 for kind in EVIDENCE_KIND_FIELDS}
    evidence = registry_evidence_entries(entry_id)
    for item in evidence:
        kind = item["kind"]
        counts[kind] += 1
        require_source_contains(
            item["source_file"],
            evidence_needle(item),
            f"{entry_id}.{item['id']}",
        )
    return counts


def validate_entry(entry_id: str, entry: dict[str, Any], confidential_source: str) -> dict[str, Any]:
    """Validate one registry entry and return report metadata."""
    require_string(entry.get("milestone"), f"{entry_id}.milestone")
    require_string(entry.get("profile"), f"{entry_id}.profile")
    typed_counts = validate_typed_evidence(entry_id)

    term_count = len(registry_string_list(entry_id, "confidential_rust_terms"))
    missing_terms = missing_confidential_rust_terms(entry_id, confidential_source)
    if missing_terms:
        raise AssuranceEvidenceRegistryError(
            f"{entry_id} missing kiva-confidential terms: " + ", ".join(missing_terms),
        )

    rust_tests = registry_string_list(entry_id, "rust_tests")
    missing_tests = missing_rust_tests(entry_id)
    if missing_tests:
        raise AssuranceEvidenceRegistryError(
            f"{entry_id} missing Rust tests: " + ", ".join(missing_tests),
        )

    kani_harnesses = registry_string_list(entry_id, "kani_harnesses")
    missing_harnesses = missing_kani_harnesses(entry_id)
    if missing_harnesses:
        raise AssuranceEvidenceRegistryError(
            f"{entry_id} missing Kani harnesses: " + ", ".join(missing_harnesses),
        )

    tamarin_lemmas = registry_string_list(entry_id, "tamarin_lemmas")
    if tamarin_lemmas:
        model_path = Path(f"proofs/tamarin/kiva_{entry_id.replace('-', '_')}_boundary.spthy")
        if entry_id == "production-psi-upgrade":
            model_path = Path("proofs/tamarin/kiva_psi_oprf_boundary.spthy")
        if not model_path.exists():
            raise AssuranceEvidenceRegistryError(
                f"{entry_id} Tamarin model does not exist: {model_path}",
            )
        missing_lemmas = missing_tamarin_lemmas(
            entry_id,
            model_path.read_text(encoding="utf-8"),
        )
        if missing_lemmas:
            raise AssuranceEvidenceRegistryError(
                f"{entry_id} missing Tamarin lemmas: " + ", ".join(missing_lemmas),
            )

    return {
        "entry_id": entry_id,
        "milestone": entry["milestone"],
        "profile": entry["profile"],
        "confidential_rust_terms": term_count,
        "rust_tests": len(rust_tests),
        "kani_harnesses": len(kani_harnesses),
        "tamarin_lemmas": len(tamarin_lemmas),
        "typed_evidence": typed_counts,
    }


def validate_registry(path: Path) -> dict[str, Any]:
    """Validate the complete registry."""
    registry = load_assurance_evidence_registry(path)
    entries = registry["entries"]
    confidential_source = confidential_rust_source()
    reports = [
        validate_entry(entry_id, entry, confidential_source)
        for entry_id, entry in sorted(entries.items())
    ]
    return {
        "schema_version": 1,
        "kind": "confidential-assurance-evidence-registry-validation",
        "status": "passed",
        "registry": str(path),
        "entry_count": len(reports),
        "entries": reports,
    }


def main() -> int:
    """Run the assurance evidence registry validator."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("registry", nargs="?", type=Path, default=DEFAULT_ASSURANCE_EVIDENCE_REGISTRY)
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    report = validate_registry(args.registry)
    if args.json_report is not None:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
