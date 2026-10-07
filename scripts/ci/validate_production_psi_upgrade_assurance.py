#!/usr/bin/env python3
"""Validate Kiva's CC11 production PSI upgrade assurance profile."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

import yaml

from source_scan import (
    missing_confidential_rust_terms,
    missing_kani_harnesses,
    missing_tamarin_lemmas,
    registry_string_list,
    server_rust_source,
)


DEFAULT_PROFILE = Path("spec/confidential/production-psi-upgrade-assurance.yaml")
PROFILE = "kiva-psi-oprf-dh-voprf-review-gated-v1"
ELEMENT_DOMAIN = "kiva-confidential:psi-oprf-dh-voprf-element:v1"
TRANSCRIPT_DOMAIN = "kiva-confidential:psi-oprf-dh-voprf-transcript:v1"
SUITE = "ristretto255-sha512-voprf"
FUTURE_PDS_SOURCE_KIND = "private-set-intersection-result"


class ProductionPsiUpgradeAssuranceError(RuntimeError):
    """Raised when the CC11 assurance package is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise ProductionPsiUpgradeAssuranceError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise ProductionPsiUpgradeAssuranceError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a list or raise a validation error."""
    if not isinstance(value, list):
        raise ProductionPsiUpgradeAssuranceError(f"{label} must be a list")
    return value


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality with a stable error message."""
    if actual != expected:
        raise ProductionPsiUpgradeAssuranceError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def require_path(value: Any, label: str) -> Path:
    """Require an existing repository path."""
    if not isinstance(value, str) or not value:
        raise ProductionPsiUpgradeAssuranceError(f"{label} must be a non-empty path")
    path = Path(value)
    if not path.exists():
        raise ProductionPsiUpgradeAssuranceError(f"{label} does not exist: {path}")
    return path


def require_string_items(value: Any, label: str) -> list[str]:
    """Require a non-empty list of strings."""
    items = require_list(value, label)
    if not items or not all(isinstance(item, str) and item for item in items):
        raise ProductionPsiUpgradeAssuranceError(
            f"{label} must be a non-empty list of strings",
        )
    return items


def validate_pds_boundary(pds: dict[str, Any], label: str) -> None:
    """Validate the current non-dedicated PDS boundary for DH-OPRF PSI."""
    require_equal(
        pds.get("dedicated_data_product_source_kind_implemented"),
        False,
        f"{label}.dedicated_data_product_source_kind_implemented",
    )
    require_equal(pds.get("future_source_kind"), FUTURE_PDS_SOURCE_KIND, f"{label}.future_source_kind")
    require_equal(pds.get("resource_class"), "aggregate-insights", f"{label}.resource_class")
    require_equal(pds.get("raw_record_read"), False, f"{label}.raw_record_read")


def validate_no_public_pds_source_kind() -> None:
    """Require the future PSI source kind to remain absent from PDS public surfaces."""
    checks = {
        Path("spec/openapi/kiva-pds.yaml"): FUTURE_PDS_SOURCE_KIND,
        Path("db/schema.sql"): FUTURE_PDS_SOURCE_KIND,
        Path("crates/kiva-policy/src/lib.rs"): "PrivateSetIntersectionResult",
        Path("crates/kiva-policy/src/policy/data_products.rs"): "PrivateSetIntersectionResult",
        Path("crates/kiva-policy/src/policy/decision.rs"): "PrivateSetIntersectionResult",
    }
    for path, token in checks.items():
        if token in path.read_text(encoding="utf-8"):
            raise ProductionPsiUpgradeAssuranceError(
                f"dedicated PSI PDS source kind is unexpectedly implemented in {path}",
            )
    if FUTURE_PDS_SOURCE_KIND in server_rust_source():
        raise ProductionPsiUpgradeAssuranceError(
            "dedicated PSI PDS source kind is unexpectedly implemented in kiva-server",
        )


def len_prefixed(value: bytes) -> bytes:
    """Return the Rust-compatible u128 length-prefixed encoding."""
    return len(value).to_bytes(16, "big") + value


def canonicalize_identifier(identifier: str) -> str:
    """Apply the deterministic fixture canonicalization rule."""
    canonical = identifier.strip().lower()
    if not canonical:
        raise ProductionPsiUpgradeAssuranceError("identifier became empty")
    if any(ord(character) < 0x20 or ord(character) == 0x7F for character in canonical):
        raise ProductionPsiUpgradeAssuranceError("identifier contains an ASCII control character")
    return canonical


def derive_element_hex(job_seed: int, namespace: str, identifier: str) -> str:
    """Derive the deterministic Rust fixture element."""
    namespace = namespace.strip()
    if not namespace:
        raise ProductionPsiUpgradeAssuranceError("namespace became empty")
    if any(ord(character) < 0x20 or ord(character) == 0x7F for character in namespace):
        raise ProductionPsiUpgradeAssuranceError("namespace contains an ASCII control character")
    digest = hashlib.sha256()
    digest.update(ELEMENT_DOMAIN.encode("utf-8"))
    digest.update(len_prefixed(PROFILE.encode("utf-8")))
    digest.update(len_prefixed(SUITE.encode("utf-8")))
    digest.update(bytes([job_seed]) * 32)
    digest.update(len_prefixed(namespace.encode("utf-8")))
    digest.update(len_prefixed(canonicalize_identifier(identifier).encode("utf-8")))
    return digest.hexdigest()


def derive_transcript_digest_hex(vector: dict[str, Any], participant_elements: list[tuple[int, list[str]]]) -> str:
    """Derive the Rust-compatible accepted transcript digest."""
    digest = hashlib.sha256()
    digest.update(TRANSCRIPT_DOMAIN.encode("utf-8"))
    digest.update(len_prefixed(PROFILE.encode("utf-8")))
    digest.update(len_prefixed(SUITE.encode("utf-8")))
    digest.update(bytes([int(vector["job_seed"])]) * 32)
    digest.update(int(vector["participant_count"]).to_bytes(16, "big"))
    digest.update(bytes([int(vector["runtime_artifact_seed"])]) * 32)
    digest.update(bytes([int(vector["review_evidence_seed"])]) * 32)
    for participant_index, elements in sorted(participant_elements):
        digest.update(int(participant_index).to_bytes(16, "big"))
        ordered = sorted(elements)
        digest.update(len(ordered).to_bytes(16, "big"))
        for element_hex in ordered:
            digest.update(bytes.fromhex(element_hex))
    return digest.hexdigest()


def validate_job_contract(path: Path) -> None:
    """Validate the CC11 job contract."""
    contract = load_yaml(path)
    require_equal(contract.get("schema_version"), 1, "job_contract.schema_version")
    require_equal(contract.get("kind"), "psi-oprf-dh-voprf-job-contract", "job_contract.kind")
    require_equal(contract.get("profile"), PROFILE, "job_contract.profile")
    require_equal(contract.get("status"), "review-gated-runtime-boundary", "job_contract.status")
    protocol = require_mapping(contract.get("protocol"), "job_contract.protocol")
    require_equal(protocol.get("family"), "dh-oprf-psi", "protocol.family")
    require_equal(protocol.get("suite"), SUITE, "protocol.suite")
    require_equal(protocol.get("requires_all_participants"), True, "protocol.requires_all_participants")
    abort = require_mapping(contract.get("abort_semantics"), "job_contract.abort_semantics")
    require_equal(
        abort.get("any_aborted_participant_blocks_intersection"),
        True,
        "abort_semantics.any_aborted_participant_blocks_intersection",
    )
    release_gate = require_mapping(contract.get("release_gate"), "job_contract.release_gate")
    required = set(require_string_items(release_gate.get("requires"), "release_gate.requires"))
    for item in {
        "intersection_computed",
        "participants_active",
        "external_crypto_review_completed",
        "policy_approved",
        "governed_pds_data_product",
    }:
        if item not in required:
            raise ProductionPsiUpgradeAssuranceError(f"release gate is missing {item}")
    record = require_mapping(contract.get("transcript_record_boundary"), "job_contract.transcript_record_boundary")
    require_equal(record.get("record_shape"), "PsiOprfTranscriptRecord", "transcript_record_boundary.record_shape")
    require_equal(record.get("dedicated_http_api"), False, "transcript_record_boundary.dedicated_http_api")
    require_equal(
        record.get("dedicated_postgresql_store"),
        False,
        "transcript_record_boundary.dedicated_postgresql_store",
    )
    require_equal(
        record.get("generic_confidential_job_evidence_boundary"),
        True,
        "transcript_record_boundary.generic_confidential_job_evidence_boundary",
    )
    pds = require_mapping(contract.get("pds"), "job_contract.pds")
    validate_pds_boundary(pds, "job_contract.pds")
    require_equal(
        pds.get("data_product_admission_requires"),
        [],
        "job_contract.pds.data_product_admission_requires",
    )


def validate_vectors(path: Path) -> dict[str, int]:
    """Validate deterministic CC11 fixture vectors."""
    vectors = load_yaml(path)
    require_equal(vectors.get("schema_version"), 1, "vectors.schema_version")
    entries = require_list(vectors.get("vectors"), "vectors.vectors")
    checked_elements = 0
    for entry_value in entries:
        entry = require_mapping(entry_value, "vectors[]")
        require_equal(entry.get("profile"), PROFILE, "vector.profile")
        require_equal(entry.get("element_domain"), ELEMENT_DOMAIN, "vector.element_domain")
        require_equal(entry.get("transcript_domain"), TRANSCRIPT_DOMAIN, "vector.transcript_domain")
        require_equal(entry.get("suite"), SUITE, "vector.suite")
        require_equal(entry.get("external_review_completed"), True, "vector.external_review_completed")
        participant_count = int(entry.get("participant_count"))
        participants = require_list(entry.get("participants"), "vector.participants")
        if len(participants) != participant_count:
            raise ProductionPsiUpgradeAssuranceError("vector participant count mismatch")
        participant_sets: list[set[str]] = []
        participant_elements: list[tuple[int, list[str]]] = []
        seen_participants: set[int] = set()
        for participant_value in participants:
            participant = require_mapping(participant_value, "vector.participants[]")
            participant_index = int(participant.get("participant_index"))
            if participant_index in seen_participants:
                raise ProductionPsiUpgradeAssuranceError("duplicate participant index")
            seen_participants.add(participant_index)
            if not 1 <= participant_index <= participant_count:
                raise ProductionPsiUpgradeAssuranceError("participant index out of range")
            identifiers = require_list(participant.get("identifiers"), "participant.identifiers")
            elements = require_list(participant.get("element_hex"), "participant.element_hex")
            if len(identifiers) != len(elements):
                raise ProductionPsiUpgradeAssuranceError("identifier/element length mismatch")
            seen_elements: set[str] = set()
            for identifier, element_hex in zip(identifiers, elements, strict=True):
                require_equal(
                    element_hex,
                    derive_element_hex(int(entry["job_seed"]), str(entry["namespace"]), str(identifier)),
                    "participant.element_hex[]",
                )
                if element_hex in seen_elements:
                    raise ProductionPsiUpgradeAssuranceError("duplicate element in participant")
                seen_elements.add(str(element_hex))
                checked_elements += 1
            participant_sets.append(seen_elements)
            participant_elements.append((participant_index, list(seen_elements)))
        require_equal(
            entry.get("expected_intersection_element_hex"),
            sorted(set.intersection(*participant_sets)),
            "vector.expected_intersection_element_hex",
        )
        require_equal(
            entry.get("expected_transcript_digest_hex"),
            derive_transcript_digest_hex(entry, participant_elements),
            "vector.expected_transcript_digest_hex",
        )
    return {"vector_count": len(entries), "checked_elements": checked_elements}


def validate_benchmarks(path: Path) -> None:
    """Validate benchmark-plan structure."""
    benchmarks = load_yaml(path)
    require_equal(benchmarks.get("kind"), "psi-oprf-dh-voprf-benchmark-plan", "benchmarks.kind")
    require_equal(benchmarks.get("profile"), PROFILE, "benchmarks.profile")
    workloads = require_list(benchmarks.get("workloads"), "benchmarks.workloads")
    if len(workloads) < 2:
        raise ProductionPsiUpgradeAssuranceError("at least two benchmark workloads are required")
    thresholds = require_mapping(benchmarks.get("acceptance_thresholds"), "benchmarks.acceptance_thresholds")
    for key in ("small_cohort_wall_clock_ms", "district_baseline_wall_clock_ms", "peak_memory_mib"):
        if int(thresholds.get(key, 0)) <= 0:
            raise ProductionPsiUpgradeAssuranceError(f"benchmark threshold must be positive: {key}")


def validate_interop(path: Path) -> None:
    """Validate interop-plan structure."""
    interop = load_yaml(path)
    require_equal(interop.get("kind"), "psi-oprf-dh-voprf-interop-plan", "interop.kind")
    require_equal(interop.get("profile"), PROFILE, "interop.profile")
    cases = require_list(interop.get("interop_cases"), "interop.interop_cases")
    if len(cases) < 4:
        raise ProductionPsiUpgradeAssuranceError("positive, abort, replay, and review-pending interop cases are required")
    compatibility = require_mapping(interop.get("wire_compatibility"), "interop.wire_compatibility")
    require_equal(
        compatibility.get("raw_identifiers_cross_kiva_boundary"),
        False,
        "wire_compatibility.raw_identifiers_cross_kiva_boundary",
    )


def validate_source_evidence() -> None:
    """Require Rust, Kani, and Tamarin evidence to name CC11 boundaries."""
    missing_terms = missing_confidential_rust_terms("production-psi-upgrade")
    if missing_terms:
        raise ProductionPsiUpgradeAssuranceError(
            "missing Rust evidence token: " + ", ".join(missing_terms),
        )
    missing_harnesses = missing_kani_harnesses("production-psi-upgrade")
    if missing_harnesses:
        raise ProductionPsiUpgradeAssuranceError(
            "Kani runner is missing: " + ", ".join(missing_harnesses),
        )
    tamarin = Path("proofs/tamarin/kiva_psi_oprf_boundary.spthy").read_text(encoding="utf-8")
    missing_lemmas = missing_tamarin_lemmas("production-psi-upgrade", tamarin)
    if missing_lemmas:
        raise ProductionPsiUpgradeAssuranceError(
            "Tamarin model is missing: " + ", ".join(missing_lemmas),
        )


def validate_profile(profile: dict[str, Any]) -> dict[str, Any]:
    """Validate the CC11 assurance profile."""
    require_equal(profile.get("schema_version"), 1, "schema_version")
    require_equal(profile.get("kind"), "production-psi-upgrade-assurance", "kind")
    require_equal(profile.get("status"), "review-gated", "status")
    require_equal(profile.get("milestone"), "CC11", "milestone")
    protocol = require_mapping(profile.get("selected_protocol"), "selected_protocol")
    require_equal(protocol.get("family"), "dh-oprf-psi", "selected_protocol.family")
    require_equal(protocol.get("suite"), SUITE, "selected_protocol.suite")
    require_equal(protocol.get("profile"), PROFILE, "selected_protocol.profile")
    assumptions = require_mapping(profile.get("assumptions"), "assumptions")
    require_equal(assumptions.get("raw_identifiers_admitted_to_kiva"), False, "assumptions.raw_identifiers_admitted_to_kiva")
    review = require_mapping(profile.get("external_crypto_review"), "external_crypto_review")
    require_equal(review.get("required_before_production_claim"), True, "external_crypto_review.required_before_production_claim")
    require_equal(review.get("status"), "pending", "external_crypto_review.status")
    require_equal(review.get("production_claim_allowed"), False, "external_crypto_review.production_claim_allowed")
    wire = require_mapping(profile.get("wire_and_record_boundary"), "wire_and_record_boundary")
    job_contract_path = require_path(wire.get("job_contract"), "wire_and_record_boundary.job_contract")
    vectors_path = require_path(wire.get("vectors"), "wire_and_record_boundary.vectors")
    benchmarks_path = require_path(wire.get("benchmarks"), "wire_and_record_boundary.benchmarks")
    interop_path = require_path(wire.get("interop"), "wire_and_record_boundary.interop")
    require_equal(wire.get("replay_record_shape"), "PsiOprfTranscriptRecord", "wire_and_record_boundary.replay_record_shape")
    require_equal(wire.get("dedicated_http_api"), False, "wire_and_record_boundary.dedicated_http_api")
    require_equal(
        wire.get("dedicated_postgresql_store"),
        False,
        "wire_and_record_boundary.dedicated_postgresql_store",
    )
    require_equal(
        wire.get("generic_confidential_job_evidence_boundary"),
        True,
        "wire_and_record_boundary.generic_confidential_job_evidence_boundary",
    )
    pds = require_mapping(profile.get("pds"), "pds")
    validate_pds_boundary(pds, "pds")
    require_equal(pds.get("governed_release_fact_required"), True, "pds.governed_release_fact_required")
    require_equal(
        pds.get("generic_confidential_job_evidence_boundary"),
        True,
        "pds.generic_confidential_job_evidence_boundary",
    )
    validate_job_contract(job_contract_path)
    vector_report = validate_vectors(vectors_path)
    validate_benchmarks(benchmarks_path)
    validate_interop(interop_path)
    verification = require_mapping(profile.get("verification"), "verification")
    for section in ("rust_tests", "kani_harnesses", "tamarin_models", "validators", "docs"):
        require_string_items(verification.get(section), f"verification.{section}")
    require_equal(
        set(verification["kani_harnesses"]),
        set(registry_string_list("production-psi-upgrade", "kani_harnesses")),
        "verification.kani_harnesses registry match",
    )
    for path_value in verification["tamarin_models"] + verification["validators"] + verification["docs"]:
        require_path(path_value, f"verification.{path_value}")
    validate_source_evidence()
    validate_no_public_pds_source_kind()
    return {
        "schema_version": 1,
        "kind": "production-psi-upgrade-assurance-validation",
        "status": "passed",
        "profile": PROFILE,
        "external_crypto_review_status": review["status"],
        "production_claim_allowed": review["production_claim_allowed"],
        "vectors": vector_report,
    }


def main() -> int:
    """Run the CC11 validator."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", type=Path, default=DEFAULT_PROFILE)
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    report = validate_profile(load_yaml(args.profile))
    if args.json_report is not None:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    else:
        print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
