#!/usr/bin/env python3
"""Validate Kiva's CC13 review-gated FROST threshold-signing assurance profile."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

import yaml

from source_scan import missing_confidential_rust_terms


DEFAULT_PROFILE = Path("spec/confidential/threshold-frost-assurance.yaml")
DEFAULT_CONTRACT = Path("spec/confidential/threshold-frost-job-contract.yaml")
PROFILE = "kiva-threshold-frost-ed25519-review-gated-v1"
SUITE = "ed25519-sha512-frost"
KEY_CEREMONY_DOMAIN = "kiva-confidential:threshold-frost-key-ceremony:v1"
SIGNATURE_TRANSCRIPT_DOMAIN = "kiva-confidential:threshold-frost-signature-transcript:v1"


class ThresholdFrostAssuranceError(RuntimeError):
    """Raised when the FROST threshold-signing assurance profile is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise ThresholdFrostAssuranceError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise ThresholdFrostAssuranceError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a non-empty list or raise a validation error."""
    if not isinstance(value, list) or not value:
        raise ThresholdFrostAssuranceError(f"{label} must be a non-empty list")
    return value


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality with a stable error message."""
    if actual != expected:
        raise ThresholdFrostAssuranceError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def require_path(path_value: Any, label: str) -> Path:
    """Require a repository path to exist."""
    if not isinstance(path_value, str) or not path_value:
        raise ThresholdFrostAssuranceError(f"{label} must be a non-empty path")
    path = Path(path_value)
    if not path.exists():
        raise ThresholdFrostAssuranceError(f"{label} does not exist: {path}")
    return path


def require_true_fields(section: dict[str, Any], label: str, fields: tuple[str, ...]) -> None:
    """Require a set of boolean fields to be true."""
    for field in fields:
        require_equal(section.get(field), True, f"{label}.{field}")


def update_len_prefixed(digest: Any, value: str) -> None:
    """Add Rust-compatible 128-bit-length-prefixed UTF-8 bytes."""
    encoded = value.encode("utf-8")
    digest.update(len(encoded).to_bytes(16, "big"))
    digest.update(encoded)


def seed_bytes(entry: dict[str, Any], key: str) -> bytes:
    """Return the 32-byte deterministic fixture value for a seed field."""
    return bytes([int(entry[key])]) * 32


def derive_key_ceremony_digest_hex(vector: dict[str, Any]) -> str:
    """Derive the expected FROST key-ceremony digest."""
    digest = hashlib.sha256()
    digest.update(KEY_CEREMONY_DOMAIN.encode("utf-8"))
    update_len_prefixed(digest, PROFILE)
    update_len_prefixed(digest, SUITE)
    digest.update(seed_bytes(vector, "job_seed"))
    digest.update(int(vector["participant_count"]).to_bytes(16, "big"))
    digest.update(int(vector["threshold"]).to_bytes(16, "big"))
    digest.update(int(vector["key_epoch"]).to_bytes(8, "big"))
    digest.update(seed_bytes(vector, "runtime_artifact_seed"))
    digest.update(seed_bytes(vector, "review_evidence_seed"))
    participants = sorted(
        require_list(vector.get("participants"), "vector.participants"),
        key=lambda item: int(require_mapping(item, "vector.participants[]")["participant_index"]),
    )
    for participant_value in participants:
        participant = require_mapping(participant_value, "vector.participants[]")
        digest.update(int(participant["participant_index"]).to_bytes(16, "big"))
        digest.update(seed_bytes(participant, "public_commitment_seed"))
        digest.update(seed_bytes(participant, "share_custody_seed"))
    return digest.hexdigest()


def derive_signature_transcript_digest_hex(
    vector: dict[str, Any],
    key_ceremony_digest_hex: str,
) -> str:
    """Derive the expected FROST signing transcript digest."""
    digest = hashlib.sha256()
    digest.update(SIGNATURE_TRANSCRIPT_DOMAIN.encode("utf-8"))
    update_len_prefixed(digest, PROFILE)
    update_len_prefixed(digest, SUITE)
    digest.update(bytes.fromhex(key_ceremony_digest_hex))
    digest.update(seed_bytes(vector, "message_digest_seed"))
    shares = sorted(
        require_list(vector.get("signature_shares"), "vector.signature_shares"),
        key=lambda item: int(require_mapping(item, "vector.signature_shares[]")["participant_index"]),
    )
    digest.update(len(shares).to_bytes(16, "big"))
    for share_value in shares:
        share = require_mapping(share_value, "vector.signature_shares[]")
        digest.update(int(share["participant_index"]).to_bytes(16, "big"))
        digest.update(seed_bytes(share, "signature_share_seed"))
    digest.update(seed_bytes(vector, "signature_digest_seed"))
    return digest.hexdigest()


def validate_vectors(profile: dict[str, Any]) -> dict[str, Any]:
    """Validate deterministic FROST transcript vectors."""
    vectors_config = require_mapping(profile.get("vectors"), "vectors")
    vectors_path = require_path(vectors_config.get("path"), "vectors.path")
    vectors = load_yaml(vectors_path)
    require_equal(vectors.get("schema_version"), 1, "vectors.schema_version")
    require_equal(vectors.get("kind"), "threshold-frost-vectors", "vectors.kind")
    entries = require_list(vectors.get("vectors"), "vectors.vectors")
    for entry_value in entries:
        entry = require_mapping(entry_value, "vectors[]")
        require_equal(entry.get("profile"), PROFILE, "vector.profile")
        require_equal(entry.get("key_ceremony_domain"), KEY_CEREMONY_DOMAIN, "vector.key_ceremony_domain")
        require_equal(
            entry.get("signature_transcript_domain"),
            SIGNATURE_TRANSCRIPT_DOMAIN,
            "vector.signature_transcript_domain",
        )
        require_equal(entry.get("suite"), SUITE, "vector.suite")
        require_equal(
            entry.get("external_review_completed"),
            True,
            "vector.external_review_completed",
        )
        if int(entry["threshold"]) < 2:
            raise ThresholdFrostAssuranceError("vector threshold must be at least two")
        if int(entry["threshold"]) > int(entry["participant_count"]):
            raise ThresholdFrostAssuranceError("vector threshold exceeds participant_count")
        require_equal(
            len(require_list(entry.get("participants"), "vector.participants")),
            int(entry["participant_count"]),
            "vector participants count",
        )
        if len(require_list(entry.get("signature_shares"), "vector.signature_shares")) < int(
            entry["threshold"],
        ):
            raise ThresholdFrostAssuranceError("vector signature shares below threshold")
        ceremony_digest = derive_key_ceremony_digest_hex(entry)
        require_equal(
            entry.get("expected_key_ceremony_digest_hex"),
            ceremony_digest,
            "vector.expected_key_ceremony_digest_hex",
        )
        require_equal(
            entry.get("expected_signature_transcript_digest_hex"),
            derive_signature_transcript_digest_hex(entry, ceremony_digest),
            "vector.expected_signature_transcript_digest_hex",
        )
    return {"vectors_path": str(vectors_path), "vector_count": len(entries)}


def validate_job_contract(path: Path) -> dict[str, Any]:
    """Validate the FROST threshold-signing job contract."""
    contract = load_yaml(path)
    require_equal(contract.get("schema_version"), 1, "job_contract.schema_version")
    require_equal(contract.get("kind"), "threshold-frost-job-contract", "job_contract.kind")
    require_equal(contract.get("status"), "review-gated", "job_contract.status")
    require_equal(contract.get("profile"), PROFILE, "job_contract.profile")
    require_equal(contract.get("milestone"), "CC13", "job_contract.milestone")

    runtime = require_mapping(contract.get("runtime_adapter"), "job_contract.runtime_adapter")
    require_equal(runtime.get("suite"), SUITE, "job_contract.runtime_adapter.suite")
    require_true_fields(
        runtime,
        "job_contract.runtime_adapter",
        (
            "artifact_digest_required",
            "external_review_evidence_digest_required",
            "external_review_required_before_production_claim",
            "performs_distributed_key_generation",
            "performs_proactive_refresh",
            "performs_threshold_signing",
            "reports_complaint_resolution",
            "reports_equivocation_evidence",
        ),
    )
    for section_name in (
        "key_ceremony",
        "signing",
        "release",
        "operational_assumptions",
    ):
        section = require_mapping(contract.get(section_name), f"job_contract.{section_name}")
        for key, value in section.items():
            require_equal(value, True, f"job_contract.{section_name}.{key}")

    for path_value in require_mapping(contract.get("evidence"), "job_contract.evidence").values():
        if isinstance(path_value, str):
            require_path(path_value, f"job_contract.evidence[{path_value}]")
    for key, value in require_mapping(contract.get("non_goals"), "job_contract.non_goals").items():
        require_equal(value, False, f"job_contract.non_goals.{key}")
    return {"path": str(path), "profile": contract["profile"]}


def validate_rust_core() -> None:
    """Require the Rust core to expose the expected CC13 boundary terms."""
    missing = missing_confidential_rust_terms("threshold-frost")
    if missing:
        raise ThresholdFrostAssuranceError(
            "kiva-confidential is missing FROST threshold terms: " + ", ".join(missing),
        )


def validate_profile(profile: dict[str, Any], contract_path: Path) -> dict[str, Any]:
    """Validate profile invariants and return a report payload."""
    require_equal(profile.get("schema_version"), 1, "schema_version")
    require_equal(profile.get("kind"), "threshold-frost-assurance-profile", "kind")
    require_equal(profile.get("status"), "review-gated", "status")
    require_equal(profile.get("profile"), PROFILE, "profile")
    require_equal(profile.get("milestone"), "CC13", "milestone")

    target = require_mapping(profile.get("target_runtime"), "target_runtime")
    require_equal(target.get("suite"), SUITE, "target_runtime.suite")
    require_equal(target.get("digest"), "sha256", "target_runtime.digest")
    require_equal(
        target.get("runtime_adapter"),
        "review-gated-external-frost-runtime",
        "target_runtime.runtime_adapter",
    )
    require_equal(
        target.get("production_threshold_cryptography_claim_allowed"),
        False,
        "target_runtime.production_threshold_cryptography_claim_allowed",
    )
    require_equal(
        target.get("malicious_participant_robustness_claim_allowed"),
        False,
        "target_runtime.malicious_participant_robustness_claim_allowed",
    )

    ceremony = require_mapping(profile.get("key_ceremony"), "key_ceremony")
    require_equal(ceremony.get("profile"), PROFILE, "key_ceremony.profile")
    require_equal(ceremony.get("domain"), KEY_CEREMONY_DOMAIN, "key_ceremony.domain")
    require_equal(ceremony.get("digest"), "sha256", "key_ceremony.digest")
    require_true_fields(
        ceremony,
        "key_ceremony",
        (
            "records_runtime_artifact_digest",
            "records_review_evidence_digest",
            "records_key_epoch",
            "records_public_commitment_digest",
            "records_share_custody_digest",
            "records_ceremony_digest",
        ),
    )
    signing = require_mapping(profile.get("signing"), "signing")
    require_equal(
        signing.get("signature_transcript_domain"),
        SIGNATURE_TRANSCRIPT_DOMAIN,
        "signing.signature_transcript_domain",
    )
    for section_name in ("admission", "signing", "release"):
        section = require_mapping(profile.get(section_name), section_name)
        for key, value in section.items():
            if key.endswith("_domain"):
                continue
            require_equal(value, True, f"{section_name}.{key}")

    review = require_mapping(profile.get("external_cryptographic_review"), "external_cryptographic_review")
    require_equal(
        review.get("required_before_production_claim"),
        True,
        "external_cryptographic_review.required_before_production_claim",
    )
    require_equal(
        review.get("production_threshold_cryptography_claim_allowed"),
        False,
        "external_cryptographic_review.production_threshold_cryptography_claim_allowed",
    )
    require_equal(
        review.get("malicious_participant_robustness_claim_allowed"),
        False,
        "external_cryptographic_review.malicious_participant_robustness_claim_allowed",
    )
    require_equal(
        review.get("review_evidence_digest_required"),
        True,
        "external_cryptographic_review.review_evidence_digest_required",
    )

    for path_value in require_mapping(profile.get("evidence"), "evidence").values():
        if isinstance(path_value, str):
            require_path(path_value, f"evidence[{path_value}]")
    for key, value in require_mapping(profile.get("non_goals"), "non_goals").items():
        require_equal(value, False, f"non_goals.{key}")

    validate_rust_core()
    return {
        "kind": "threshold-frost-assurance-validation",
        "status": "passed",
        "profile": profile["profile"],
        "job_contract": validate_job_contract(contract_path),
        "vectors": validate_vectors(profile),
    }


def main() -> int:
    """Run the FROST threshold-signing assurance validator."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", type=Path, default=DEFAULT_PROFILE)
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    report = validate_profile(load_yaml(args.profile), args.contract)
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
