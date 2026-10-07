#!/usr/bin/env python3
"""Validate Kiva portable TEE attestation assurance profile and vectors."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

import yaml

from source_scan import missing_confidential_rust_terms


DEFAULT_PROFILE = Path("spec/confidential/tee-attestation-assurance.yaml")
DEFAULT_CONTRACT = Path("spec/confidential/tee-attestation-job-contract.yaml")
PROFILE = "kiva-tee-portable-attestation-v1"
DOMAIN = "kiva-confidential:tee-attestation-evidence:v1"
FAMILY = "deployment-bound-portable"
FORMAT = "portable-evidence-v1"


class TeeAttestationAssuranceError(RuntimeError):
    """Raised when the TEE attestation assurance profile is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise TeeAttestationAssuranceError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise TeeAttestationAssuranceError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a list or raise a validation error."""
    if not isinstance(value, list):
        raise TeeAttestationAssuranceError(f"{label} must be a list")
    return value


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality with a stable error message."""
    if actual != expected:
        raise TeeAttestationAssuranceError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def derive_evidence_digest_hex(vector: dict[str, Any]) -> str:
    """Derive the expected portable TEE evidence digest."""
    digest = hashlib.sha256()
    digest.update(DOMAIN.encode("utf-8"))
    for value in (PROFILE, FAMILY, FORMAT):
        encoded = value.encode("utf-8")
        digest.update(len(encoded).to_bytes(16, "big"))
        digest.update(encoded)
    digest.update(bytes([int(vector["job_seed"])]) * 32)
    digest.update(bytes([int(vector["measurement_seed"])]) * 32)
    digest.update(bytes([int(vector["image_digest_seed"])]) * 32)
    digest.update(bytes([int(vector["policy_binding_seed"])]) * 32)
    digest.update(int(vector["not_before_epoch_seconds"]).to_bytes(8, "big"))
    digest.update(int(vector["not_after_epoch_seconds"]).to_bytes(8, "big"))
    digest.update(int(vector["evidence_sequence"]).to_bytes(8, "big"))
    return digest.hexdigest()


def validate_vectors(profile: dict[str, Any]) -> dict[str, Any]:
    """Validate deterministic TEE attestation vectors."""
    vectors_config = require_mapping(profile.get("vectors"), "vectors")
    vectors_path = Path(str(vectors_config.get("path")))
    vectors = load_yaml(vectors_path)
    entries = require_list(vectors.get("vectors"), "vectors.vectors")
    for entry in entries:
        vector = require_mapping(entry, "vectors[]")
        require_equal(vector.get("profile"), PROFILE, "vector.profile")
        require_equal(vector.get("domain"), DOMAIN, "vector.domain")
        require_equal(vector.get("family"), FAMILY, "vector.family")
        require_equal(vector.get("format"), FORMAT, "vector.format")
        if int(vector["not_after_epoch_seconds"]) < int(vector["not_before_epoch_seconds"]):
            raise TeeAttestationAssuranceError("vector validity window is invalid")
        if int(vector["evidence_sequence"]) < 1:
            raise TeeAttestationAssuranceError("vector evidence_sequence must be non-zero")
        require_equal(
            vector.get("expected_evidence_digest_hex"),
            derive_evidence_digest_hex(vector),
            "vector.expected_evidence_digest_hex",
        )
    return {
        "vectors_path": str(vectors_path),
        "vector_count": len(entries),
    }


def validate_job_contract(path: Path) -> dict[str, Any]:
    """Validate the portable TEE attestation job contract."""
    contract = load_yaml(path)
    require_equal(contract.get("schema_version"), 1, "job_contract.schema_version")
    require_equal(contract.get("kind"), "tee-attestation-job-contract", "job_contract.kind")
    require_equal(contract.get("status"), "published", "job_contract.status")
    require_equal(contract.get("profile"), PROFILE, "job_contract.profile")

    supported = require_mapping(contract.get("supported_tee"), "supported_tee")
    require_equal(supported.get("family"), FAMILY, "supported_tee.family")
    require_equal(
        supported.get("hardware_quote_verification"),
        False,
        "supported_tee.hardware_quote_verification",
    )

    attestation = require_mapping(contract.get("attestation"), "attestation")
    require_equal(attestation.get("format"), FORMAT, "attestation.format")
    require_equal(attestation.get("digest"), "sha256", "attestation.digest")
    require_equal(attestation.get("domain"), DOMAIN, "attestation.domain")
    require_equal(attestation.get("evidence_profile"), PROFILE, "attestation.evidence_profile")

    policy = require_mapping(contract.get("policy"), "policy")
    for key in (
        "requires_job_id",
        "requires_measurement",
        "requires_image_digest",
        "requires_policy_binding",
        "requires_policy_approval",
        "requires_validity_window",
        "requires_minimum_sequence",
        "requires_replay_record",
    ):
        require_equal(policy.get(key), True, f"policy.{key}")

    admission = require_mapping(contract.get("admission"), "admission")
    for key in (
        "rejects_unsupported_format",
        "rejects_job_mismatch",
        "rejects_measurement_mismatch",
        "rejects_image_digest_mismatch",
        "rejects_policy_binding_mismatch",
        "rejects_not_yet_valid_evidence",
        "rejects_expired_evidence",
        "rejects_rollback_sequence",
        "rejects_replayed_evidence",
        "rejects_digest_mismatch",
    ):
        require_equal(admission.get(key), True, f"admission.{key}")

    for section_name in ("secrets", "result_release", "audit_metadata"):
        section = require_mapping(contract.get(section_name), section_name)
        for key, value in section.items():
            require_equal(value, True, f"{section_name}.{key}")

    non_goals = require_mapping(contract.get("non_goals"), "non_goals")
    for key, value in non_goals.items():
        require_equal(value, False, f"non_goals.{key}")

    return {
        "path": str(path),
        "profile": contract["profile"],
    }


def validate_rust_core() -> None:
    """Require the Rust core to expose the expected CC5 boundary functions."""
    missing = missing_confidential_rust_terms("tee-attestation")
    if missing:
        raise TeeAttestationAssuranceError(
            "kiva-confidential is missing TEE terms: " + ", ".join(missing),
        )


def validate_profile(profile: dict[str, Any], contract_path: Path) -> dict[str, Any]:
    """Validate profile invariants and return a report payload."""
    require_equal(profile.get("schema_version"), 1, "schema_version")
    require_equal(profile.get("kind"), "tee-attestation-assurance-profile", "kind")
    require_equal(profile.get("status"), "published", "status")
    require_equal(profile.get("profile"), PROFILE, "profile")

    supported = require_mapping(profile.get("supported_tee"), "supported_tee")
    require_equal(supported.get("family"), FAMILY, "supported_tee.family")
    require_equal(supported.get("format"), FORMAT, "supported_tee.format")
    require_equal(
        supported.get("vendor_quote_verification"),
        False,
        "supported_tee.vendor_quote_verification",
    )

    attestation = require_mapping(profile.get("attestation"), "attestation")
    require_equal(attestation.get("digest"), "sha256", "attestation.digest")
    require_equal(attestation.get("domain"), DOMAIN, "attestation.domain")
    require_equal(attestation.get("profile"), PROFILE, "attestation.profile")
    for key in (
        "records_measurement",
        "records_image_digest",
        "records_policy_binding",
        "records_evidence_digest",
        "records_evidence_sequence",
    ):
        require_equal(attestation.get(key), True, f"attestation.{key}")

    admission = require_mapping(profile.get("admission"), "admission")
    for key in (
        "requires_policy_approval",
        "requires_supported_format",
        "requires_job_binding",
        "requires_measurement_match",
        "requires_image_digest_match",
        "requires_policy_binding_match",
        "requires_fresh_evidence_window",
        "rejects_rollback_sequence",
        "rejects_replay",
        "rejects_digest_mismatch",
    ):
        require_equal(admission.get(key), True, f"admission.{key}")

    for section_name in ("secrets", "result_release"):
        section = require_mapping(profile.get(section_name), section_name)
        for key, value in section.items():
            require_equal(value, True, f"{section_name}.{key}")

    non_goals = require_mapping(profile.get("non_goals"), "non_goals")
    for key, value in non_goals.items():
        require_equal(value, False, f"non_goals.{key}")

    evidence = require_mapping(profile.get("evidence"), "evidence")
    missing_paths = sorted(
        path for path in evidence.values() if isinstance(path, str) and not Path(path).exists()
    )
    if missing_paths:
        raise TeeAttestationAssuranceError(
            "missing evidence files: " + ", ".join(missing_paths),
        )

    validate_rust_core()
    contract_report = validate_job_contract(contract_path)
    vector_report = validate_vectors(profile)
    return {
        "kind": "tee-attestation-assurance-validation",
        "status": "passed",
        "profile": profile["profile"],
        "job_contract": contract_report,
        "vectors": vector_report,
    }


def main() -> int:
    """Run the TEE attestation assurance validator."""
    parser = argparse.ArgumentParser()
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
    print(
        "validated TEE attestation assurance profile: "
        f"{args.profile}, profile={report['profile']}",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
