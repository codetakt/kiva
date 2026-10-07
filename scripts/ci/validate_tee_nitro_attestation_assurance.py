#!/usr/bin/env python3
"""Validate Kiva's review-gated AWS Nitro attestation assurance profile."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

import yaml

from source_scan import missing_confidential_rust_terms


DEFAULT_PROFILE = Path("spec/confidential/tee-nitro-attestation-assurance.yaml")
DEFAULT_CONTRACT = Path("spec/confidential/tee-nitro-attestation-job-contract.yaml")
PROFILE = "kiva-tee-aws-nitro-review-gated-v1"
DOCUMENT_DOMAIN = "kiva-confidential:tee-aws-nitro-attestation-document:v1"
FAMILY = "aws-nitro-enclaves"
FORMAT = "verified-nitro-attestation-document-v1"


class TeeNitroAttestationAssuranceError(RuntimeError):
    """Raised when the AWS Nitro attestation assurance profile is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise TeeNitroAttestationAssuranceError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise TeeNitroAttestationAssuranceError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a list or raise a validation error."""
    if not isinstance(value, list) or not value:
        raise TeeNitroAttestationAssuranceError(f"{label} must be a non-empty list")
    return value


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality with a stable error message."""
    if actual != expected:
        raise TeeNitroAttestationAssuranceError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def require_path(path_value: Any, label: str) -> Path:
    """Require a repository path to exist."""
    if not isinstance(path_value, str) or not path_value:
        raise TeeNitroAttestationAssuranceError(f"{label} must be a non-empty path")
    path = Path(path_value)
    if not path.exists():
        raise TeeNitroAttestationAssuranceError(f"{label} does not exist: {path}")
    return path


def update_len_prefixed(digest: Any, value: str) -> None:
    """Add Rust-compatible 128-bit-length-prefixed UTF-8 bytes."""
    encoded = value.encode("utf-8")
    digest.update(len(encoded).to_bytes(16, "big"))
    digest.update(encoded)


def seed_bytes(vector: dict[str, Any], key: str) -> bytes:
    """Return the 32-byte deterministic fixture value for a seed field."""
    return bytes([int(vector[key])]) * 32


def derive_document_digest_hex(vector: dict[str, Any]) -> str:
    """Derive the expected verified AWS Nitro document digest."""
    digest = hashlib.sha256()
    digest.update(DOCUMENT_DOMAIN.encode("utf-8"))
    for value in (PROFILE, FAMILY, FORMAT):
        update_len_prefixed(digest, value)
    for key in (
        "job_seed",
        "pcr0_seed",
        "pcr1_seed",
        "pcr2_seed",
        "pcr3_seed",
        "pcr8_seed",
        "image_digest_seed",
        "policy_binding_seed",
        "nonce_digest_seed",
        "certificate_chain_digest_seed",
        "cose_signing_key_digest_seed",
    ):
        digest.update(seed_bytes(vector, key))
    digest.update(int(vector["not_before_epoch_seconds"]).to_bytes(8, "big"))
    digest.update(int(vector["not_after_epoch_seconds"]).to_bytes(8, "big"))
    digest.update(int(vector["evidence_sequence"]).to_bytes(8, "big"))
    digest.update(bytes([1 if vector.get("debug_mode") else 0]))
    return digest.hexdigest()


def validate_vectors(profile: dict[str, Any]) -> dict[str, Any]:
    """Validate deterministic AWS Nitro attestation vectors."""
    vectors_config = require_mapping(profile.get("vectors"), "vectors")
    vectors_path = require_path(vectors_config.get("path"), "vectors.path")
    vectors = load_yaml(vectors_path)
    entries = require_list(vectors.get("vectors"), "vectors.vectors")
    for entry_value in entries:
        entry = require_mapping(entry_value, "vectors[]")
        require_equal(entry.get("profile"), PROFILE, "vector.profile")
        require_equal(entry.get("document_domain"), DOCUMENT_DOMAIN, "vector.document_domain")
        require_equal(entry.get("family"), FAMILY, "vector.family")
        require_equal(entry.get("format"), FORMAT, "vector.format")
        require_equal(entry.get("debug_mode"), False, "vector.debug_mode")
        if int(entry["not_after_epoch_seconds"]) < int(entry["not_before_epoch_seconds"]):
            raise TeeNitroAttestationAssuranceError("vector validity window is invalid")
        if int(entry["evidence_sequence"]) < 1:
            raise TeeNitroAttestationAssuranceError("vector evidence_sequence must be non-zero")
        require_equal(
            entry.get("expected_document_digest_hex"),
            derive_document_digest_hex(entry),
            "vector.expected_document_digest_hex",
        )
    return {"vectors_path": str(vectors_path), "vector_count": len(entries)}


def require_true_fields(section: dict[str, Any], label: str, fields: tuple[str, ...]) -> None:
    """Require a set of boolean fields to be true."""
    for field in fields:
        require_equal(section.get(field), True, f"{label}.{field}")


def validate_job_contract(path: Path) -> dict[str, Any]:
    """Validate the AWS Nitro attestation job contract."""
    contract = load_yaml(path)
    require_equal(contract.get("schema_version"), 1, "job_contract.schema_version")
    require_equal(contract.get("kind"), "tee-nitro-attestation-job-contract", "job_contract.kind")
    require_equal(contract.get("status"), "review-gated", "job_contract.status")
    require_equal(contract.get("profile"), PROFILE, "job_contract.profile")
    require_equal(contract.get("milestone"), "CC12", "job_contract.milestone")

    target = require_mapping(contract.get("target_tee"), "job_contract.target_tee")
    require_equal(target.get("family"), FAMILY, "job_contract.target_tee.family")
    require_equal(
        target.get("normalized_format"),
        FORMAT,
        "job_contract.target_tee.normalized_format",
    )
    require_equal(
        target.get("production_claim_allowed"),
        False,
        "job_contract.target_tee.production_claim_allowed",
    )

    require_true_fields(
        require_mapping(contract.get("verifier_adapter"), "job_contract.verifier_adapter"),
        "job_contract.verifier_adapter",
        (
            "artifact_digest_required",
            "trust_anchor_digest_required",
            "verifier_key_epoch_required",
            "external_review_evidence_digest_required",
            "external_security_review_required_before_production_claim",
            "verifies_certificate_chain",
            "verifies_cose_signature",
            "verifies_trust_anchor",
            "verifies_nonce",
            "rejects_debug_mode",
        ),
    )
    require_true_fields(
        require_mapping(contract.get("measurement_policy"), "job_contract.measurement_policy"),
        "job_contract.measurement_policy",
        (
            "requires_pcr0_image_measurement",
            "requires_pcr1_kernel_bootstrap_measurement",
            "requires_pcr2_application_measurement",
            "requires_pcr3_parent_iam_role_measurement",
            "requires_pcr8_eif_signing_certificate_measurement",
            "rejects_all_zero_debug_pcrs",
            "records_image_digest",
            "records_policy_binding",
        ),
    )
    for section_name in (
        "freshness_and_replay",
        "provisioning",
        "result_release",
        "rollout_and_rollback",
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
    """Require the Rust core to expose the expected CC12 boundary functions."""
    missing = missing_confidential_rust_terms("tee-nitro-attestation")
    if missing:
        raise TeeNitroAttestationAssuranceError(
            "kiva-confidential is missing AWS Nitro TEE terms: " + ", ".join(missing),
        )


def validate_profile(profile: dict[str, Any], contract_path: Path) -> dict[str, Any]:
    """Validate profile invariants and return a report payload."""
    require_equal(profile.get("schema_version"), 1, "schema_version")
    require_equal(profile.get("kind"), "tee-nitro-attestation-assurance-profile", "kind")
    require_equal(profile.get("status"), "review-gated", "status")
    require_equal(profile.get("profile"), PROFILE, "profile")
    require_equal(profile.get("milestone"), "CC12", "milestone")

    target = require_mapping(profile.get("target_tee"), "target_tee")
    require_equal(target.get("family"), FAMILY, "target_tee.family")
    require_equal(target.get("normalized_format"), FORMAT, "target_tee.normalized_format")
    require_equal(
        target.get("vendor_quote_verification"),
        "review-gated-external-adapter",
        "target_tee.vendor_quote_verification",
    )
    require_equal(target.get("production_claim_allowed"), False, "target_tee.production_claim_allowed")

    attestation = require_mapping(profile.get("attestation"), "attestation")
    require_equal(attestation.get("digest"), "sha256", "attestation.digest")
    require_equal(attestation.get("document_domain"), DOCUMENT_DOMAIN, "attestation.document_domain")
    require_equal(attestation.get("profile"), PROFILE, "attestation.profile")
    require_true_fields(
        attestation,
        "attestation",
        (
            "records_pcr0",
            "records_pcr1",
            "records_pcr2",
            "records_pcr3",
            "records_pcr8",
            "records_image_digest",
            "records_policy_binding",
            "records_nonce_digest",
            "records_certificate_chain_digest",
            "records_cose_signing_key_digest",
            "records_document_digest",
            "records_evidence_sequence",
        ),
    )
    for section_name in ("admission", "provisioning", "result_release", "rollout_and_rollback"):
        section = require_mapping(profile.get(section_name), section_name)
        for key, value in section.items():
            require_equal(value, True, f"{section_name}.{key}")

    review = require_mapping(profile.get("external_security_review"), "external_security_review")
    require_equal(
        review.get("required_before_production_claim"),
        True,
        "external_security_review.required_before_production_claim",
    )
    require_equal(
        review.get("production_claim_allowed"),
        False,
        "external_security_review.production_claim_allowed",
    )
    require_equal(
        review.get("review_evidence_digest_required"),
        True,
        "external_security_review.review_evidence_digest_required",
    )

    for path_value in require_mapping(profile.get("evidence"), "evidence").values():
        if isinstance(path_value, str):
            require_path(path_value, f"evidence[{path_value}]")
    for key, value in require_mapping(profile.get("non_goals"), "non_goals").items():
        require_equal(value, False, f"non_goals.{key}")

    validate_rust_core()
    return {
        "kind": "tee-nitro-attestation-assurance-validation",
        "status": "passed",
        "profile": profile["profile"],
        "job_contract": validate_job_contract(contract_path),
        "vectors": validate_vectors(profile),
    }


def main() -> int:
    """Run the AWS Nitro attestation assurance validator."""
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
    print(
        "validated AWS Nitro TEE attestation assurance profile: "
        f"{args.profile}, profile={report['profile']}",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
