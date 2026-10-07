#!/usr/bin/env python3
"""Validate Kiva's CC14 review-gated FHE sum runtime assurance profile."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

import yaml

from source_scan import missing_confidential_rust_terms, server_rust_source


DEFAULT_PROFILE = Path("spec/confidential/fhe-runtime-assurance.yaml")
DEFAULT_CONTRACT = Path("spec/confidential/fhe-runtime-job-contract.yaml")
PROFILE = "kiva-fhe-sum-runtime-review-gated-v1"
SCHEME_FAMILY = "bfv-integer-sum"
OPERATION = "sum"
CIPHERTEXT_TRANSCRIPT_DOMAIN = "kiva-confidential:fhe-runtime-ciphertext-transcript:v1"
EVALUATION_DOMAIN = "kiva-confidential:fhe-runtime-evaluation:v1"
FUTURE_PDS_SOURCE_KIND = "fhe-sum-runtime-result"


class FheRuntimeAssuranceError(RuntimeError):
    """Raised when the FHE runtime assurance profile is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise FheRuntimeAssuranceError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise FheRuntimeAssuranceError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a non-empty list or raise a validation error."""
    if not isinstance(value, list) or not value:
        raise FheRuntimeAssuranceError(f"{label} must be a non-empty list")
    return value


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality with a stable error message."""
    if actual != expected:
        raise FheRuntimeAssuranceError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def validate_pds_boundary(pds: dict[str, Any], label: str) -> None:
    """Validate the current non-dedicated PDS boundary for FHE runtime."""
    require_equal(
        pds.get("dedicated_data_product_source_kind_implemented"),
        False,
        f"{label}.dedicated_data_product_source_kind_implemented",
    )
    require_equal(pds.get("future_source_kind"), FUTURE_PDS_SOURCE_KIND, f"{label}.future_source_kind")
    require_equal(pds.get("resource_class"), "aggregate-insights", f"{label}.resource_class")
    require_equal(pds.get("raw_record_read"), False, f"{label}.raw_record_read")
    require_equal(
        pds.get("raw_ciphertext_or_plaintext_publication"),
        False,
        f"{label}.raw_ciphertext_or_plaintext_publication",
    )
    require_equal(
        pds.get("governed_release_fact_required"),
        True,
        f"{label}.governed_release_fact_required",
    )
    require_equal(
        pds.get("current_release_boundary"),
        "generic_confidential_job_evidence",
        f"{label}.current_release_boundary",
    )
    for key in ("pds_openapi_source_kind", "pds_postgresql_source_kind", "pds_policy_source_kind"):
        require_equal(pds.get(key), "absent", f"{label}.{key}")


def validate_no_public_pds_source_kind() -> None:
    """Require the future FHE source kind to remain absent from PDS public surfaces."""
    checks = {
        Path("spec/openapi/kiva-pds.yaml"): FUTURE_PDS_SOURCE_KIND,
        Path("db/schema.sql"): FUTURE_PDS_SOURCE_KIND,
        Path("crates/kiva-policy/src/lib.rs"): FUTURE_PDS_SOURCE_KIND,
        Path("crates/kiva-policy/src/policy/data_products.rs"): FUTURE_PDS_SOURCE_KIND,
        Path("crates/kiva-policy/src/policy/decision.rs"): FUTURE_PDS_SOURCE_KIND,
    }
    for path, token in checks.items():
        if token in path.read_text(encoding="utf-8"):
            raise FheRuntimeAssuranceError(
                f"dedicated FHE runtime PDS source kind is unexpectedly implemented in {path}",
            )
    if FUTURE_PDS_SOURCE_KIND in server_rust_source():
        raise FheRuntimeAssuranceError(
            "dedicated FHE runtime PDS source kind is unexpectedly implemented in kiva-server",
        )


def require_path(path_value: Any, label: str) -> Path:
    """Require a repository path to exist."""
    if not isinstance(path_value, str) or not path_value:
        raise FheRuntimeAssuranceError(f"{label} must be a non-empty path")
    path = Path(path_value)
    if not path.exists():
        raise FheRuntimeAssuranceError(f"{label} does not exist: {path}")
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


def derive_ciphertext_transcript_digest_hex(vector: dict[str, Any]) -> str:
    """Derive the expected FHE runtime ciphertext transcript digest."""
    digest = hashlib.sha256()
    digest.update(CIPHERTEXT_TRANSCRIPT_DOMAIN.encode("utf-8"))
    update_len_prefixed(digest, PROFILE)
    update_len_prefixed(digest, SCHEME_FAMILY)
    update_len_prefixed(digest, OPERATION)
    digest.update(seed_bytes(vector, "job_seed"))
    digest.update(int(vector["participant_count"]).to_bytes(16, "big"))
    digest.update(int(vector["key_epoch"]).to_bytes(8, "big"))
    digest.update(seed_bytes(vector, "runtime_artifact_seed"))
    digest.update(seed_bytes(vector, "parameter_set_seed"))
    digest.update(seed_bytes(vector, "review_evidence_seed"))
    participants = sorted(
        require_list(vector.get("participants"), "vector.participants"),
        key=lambda item: int(require_mapping(item, "vector.participants[]")["participant_index"]),
    )
    for participant_value in participants:
        participant = require_mapping(participant_value, "vector.participants[]")
        digest.update(int(participant["participant_index"]).to_bytes(16, "big"))
        digest.update(seed_bytes(participant, "ciphertext_seed"))
        digest.update(seed_bytes(participant, "parameter_set_seed"))
        digest.update(seed_bytes(participant, "key_custody_seed"))
    return digest.hexdigest()


def derive_evaluation_digest_hex(
    vector: dict[str, Any],
    ciphertext_transcript_digest_hex: str,
) -> str:
    """Derive the expected FHE runtime evaluation digest."""
    digest = hashlib.sha256()
    digest.update(EVALUATION_DOMAIN.encode("utf-8"))
    update_len_prefixed(digest, PROFILE)
    update_len_prefixed(digest, SCHEME_FAMILY)
    update_len_prefixed(digest, OPERATION)
    digest.update(bytes.fromhex(ciphertext_transcript_digest_hex))
    digest.update(int(vector["participant_count"]).to_bytes(16, "big"))
    digest.update(int(vector["key_epoch"]).to_bytes(8, "big"))
    digest.update(seed_bytes(vector, "runtime_artifact_seed"))
    digest.update(seed_bytes(vector, "parameter_set_seed"))
    digest.update(seed_bytes(vector, "evaluated_ciphertext_seed"))
    digest.update(seed_bytes(vector, "evaluation_proof_seed"))
    return digest.hexdigest()


def validate_vectors(profile: dict[str, Any]) -> dict[str, Any]:
    """Validate deterministic FHE runtime metadata vectors."""
    vectors_config = require_mapping(profile.get("vectors"), "vectors")
    vectors_path = require_path(vectors_config.get("path"), "vectors.path")
    vectors = load_yaml(vectors_path)
    require_equal(vectors.get("schema_version"), 1, "vectors.schema_version")
    require_equal(vectors.get("kind"), "fhe-runtime-vectors", "vectors.kind")
    entries = require_list(vectors.get("vectors"), "vectors.vectors")
    checked_ciphertexts = 0
    for entry_value in entries:
        vector = require_mapping(entry_value, "vectors[]")
        require_equal(vector.get("profile"), PROFILE, "vector.profile")
        require_equal(
            vector.get("ciphertext_transcript_domain"),
            CIPHERTEXT_TRANSCRIPT_DOMAIN,
            "vector.ciphertext_transcript_domain",
        )
        require_equal(vector.get("evaluation_domain"), EVALUATION_DOMAIN, "vector.evaluation_domain")
        require_equal(vector.get("scheme_family"), SCHEME_FAMILY, "vector.scheme_family")
        require_equal(vector.get("operation"), OPERATION, "vector.operation")
        require_equal(
            vector.get("external_review_completed"),
            True,
            "vector.external_review_completed",
        )
        participants = require_list(vector.get("participants"), "vector.participants")
        require_equal(len(participants), int(vector["participant_count"]), "vector.participant_count")
        seen: set[int] = set()
        for participant_value in participants:
            participant = require_mapping(participant_value, "vector.participants[]")
            participant_index = int(participant["participant_index"])
            if participant_index in seen:
                raise FheRuntimeAssuranceError("vector contains duplicate participant index")
            seen.add(participant_index)
            if not 1 <= participant_index <= int(vector["participant_count"]):
                raise FheRuntimeAssuranceError("vector participant index is out of range")
            require_equal(
                int(participant["parameter_set_seed"]),
                int(vector["parameter_set_seed"]),
                "participant.parameter_set_seed",
            )
            checked_ciphertexts += 1
        transcript_digest = derive_ciphertext_transcript_digest_hex(vector)
        require_equal(
            vector.get("expected_ciphertext_transcript_digest_hex"),
            transcript_digest,
            "vector.expected_ciphertext_transcript_digest_hex",
        )
        require_equal(
            vector.get("expected_evaluation_digest_hex"),
            derive_evaluation_digest_hex(vector, transcript_digest),
            "vector.expected_evaluation_digest_hex",
        )
    return {
        "vectors_path": str(vectors_path),
        "vector_count": len(entries),
        "checked_ciphertexts": checked_ciphertexts,
    }


def validate_job_contract(path: Path) -> dict[str, Any]:
    """Validate the FHE runtime job contract."""
    contract = load_yaml(path)
    require_equal(contract.get("schema_version"), 1, "job_contract.schema_version")
    require_equal(contract.get("kind"), "fhe-runtime-job-contract", "job_contract.kind")
    require_equal(contract.get("status"), "review-gated", "job_contract.status")
    require_equal(contract.get("profile"), PROFILE, "job_contract.profile")
    require_equal(contract.get("milestone"), "CC14", "job_contract.milestone")

    runtime = require_mapping(contract.get("runtime_adapter"), "job_contract.runtime_adapter")
    require_equal(runtime.get("scheme_family"), SCHEME_FAMILY, "job_contract.runtime_adapter.scheme_family")
    require_equal(runtime.get("operation"), OPERATION, "job_contract.runtime_adapter.operation")
    require_true_fields(
        runtime,
        "job_contract.runtime_adapter",
        (
            "artifact_digest_required",
            "parameter_set_digest_required",
            "external_review_evidence_digest_required",
            "external_review_required_before_production_claim",
            "performs_key_generation",
            "performs_encryption",
            "performs_homomorphic_evaluation",
            "performs_decryption",
            "reports_key_custody",
            "reports_evaluation_proof_digest",
        ),
    )
    for section_name in (
        "ciphertext_transcript",
        "evaluation",
        "release",
        "operational_assumptions",
    ):
        section = require_mapping(contract.get(section_name), f"job_contract.{section_name}")
        for key, value in section.items():
            require_equal(value, True, f"job_contract.{section_name}.{key}")

    validate_pds_boundary(require_mapping(contract.get("pds"), "job_contract.pds"), "job_contract.pds")

    for path_value in require_mapping(contract.get("evidence"), "job_contract.evidence").values():
        if isinstance(path_value, str):
            require_path(path_value, f"job_contract.evidence[{path_value}]")
    for key, value in require_mapping(contract.get("non_goals"), "job_contract.non_goals").items():
        require_equal(value, False, f"job_contract.non_goals.{key}")
    return {"path": str(path), "profile": contract["profile"]}


def validate_rust_core() -> None:
    """Require the Rust core to expose the expected CC14 boundary terms."""
    missing = missing_confidential_rust_terms("fhe-runtime")
    if missing:
        raise FheRuntimeAssuranceError(
            "kiva-confidential is missing FHE runtime terms: " + ", ".join(missing),
        )


def validate_profile(profile: dict[str, Any], contract_path: Path) -> dict[str, Any]:
    """Validate profile invariants and return a report payload."""
    require_equal(profile.get("schema_version"), 1, "schema_version")
    require_equal(profile.get("kind"), "fhe-runtime-assurance-profile", "kind")
    require_equal(profile.get("status"), "review-gated", "status")
    require_equal(profile.get("profile"), PROFILE, "profile")
    require_equal(profile.get("milestone"), "CC14", "milestone")

    target = require_mapping(profile.get("target_runtime"), "target_runtime")
    require_equal(target.get("scheme_family"), SCHEME_FAMILY, "target_runtime.scheme_family")
    require_equal(target.get("operation"), OPERATION, "target_runtime.operation")
    require_equal(target.get("digest"), "sha256", "target_runtime.digest")
    require_equal(
        target.get("runtime_adapter"),
        "review-gated-external-fhe-runtime",
        "target_runtime.runtime_adapter",
    )
    require_equal(
        target.get("production_fhe_claim_allowed"),
        False,
        "target_runtime.production_fhe_claim_allowed",
    )
    require_equal(
        target.get("malicious_participant_robustness_claim_allowed"),
        False,
        "target_runtime.malicious_participant_robustness_claim_allowed",
    )

    transcript = require_mapping(profile.get("ciphertext_transcript"), "ciphertext_transcript")
    require_equal(transcript.get("profile"), PROFILE, "ciphertext_transcript.profile")
    require_equal(transcript.get("domain"), CIPHERTEXT_TRANSCRIPT_DOMAIN, "ciphertext_transcript.domain")
    require_equal(transcript.get("digest"), "sha256", "ciphertext_transcript.digest")
    require_true_fields(
        transcript,
        "ciphertext_transcript",
        (
            "records_runtime_artifact_digest",
            "records_parameter_set_digest",
            "records_review_evidence_digest",
            "records_key_epoch",
            "records_ciphertext_digest",
            "records_key_custody_digest",
            "records_transcript_digest",
        ),
    )
    evaluation = require_mapping(profile.get("evaluation"), "evaluation")
    require_equal(evaluation.get("evaluation_domain"), EVALUATION_DOMAIN, "evaluation.evaluation_domain")
    for section_name in ("admission", "evaluation", "release"):
        section = require_mapping(profile.get(section_name), section_name)
        for key, value in section.items():
            if key.endswith("_domain"):
                continue
            require_equal(value, True, f"{section_name}.{key}")
    validate_pds_boundary(require_mapping(profile.get("pds_boundary"), "pds_boundary"), "pds_boundary")

    review = require_mapping(profile.get("external_cryptographic_review"), "external_cryptographic_review")
    require_equal(
        review.get("required_before_production_claim"),
        True,
        "external_cryptographic_review.required_before_production_claim",
    )
    require_equal(
        review.get("production_fhe_claim_allowed"),
        False,
        "external_cryptographic_review.production_fhe_claim_allowed",
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
    validate_no_public_pds_source_kind()
    return {
        "kind": "fhe-runtime-assurance-validation",
        "status": "passed",
        "profile": profile["profile"],
        "job_contract": validate_job_contract(contract_path),
        "vectors": validate_vectors(profile),
    }


def main() -> int:
    """Run the FHE runtime assurance validator."""
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
