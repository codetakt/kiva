#!/usr/bin/env python3
"""Validate Kiva FHE sum feasibility assurance profile and vectors."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

import yaml

from source_scan import missing_confidential_rust_terms, server_rust_source


DEFAULT_PROFILE = Path("spec/confidential/fhe-sum-feasibility-assurance.yaml")
DEFAULT_CONTRACT = Path("spec/confidential/fhe-sum-feasibility-job-contract.yaml")
PROFILE = "kiva-fhe-sum-bfv-feasibility-v1"
CIPHERTEXT_DOMAIN = "kiva-confidential:fhe-sum-feasibility-ciphertext:v1"
EVALUATION_DOMAIN = "kiva-confidential:fhe-sum-feasibility-evaluation:v1"
SCHEME_FAMILY = "bfv-style-integer-sum"
OPERATION = "sum"
POLYNOMIAL_MODULUS_DEGREE = 4096
PLAINTEXT_MODULUS = 1_048_583
CIPHERTEXT_MODULUS_BITS = 109
SECURITY_LEVEL_BITS = 128
MAX_PARTICIPANT_COUNT = 16
MAX_PLAINTEXT_VALUE = 65_000
FUTURE_PDS_SOURCE_KIND = "fhe-sum-feasibility-result"


class FheFeasibilityAssuranceError(RuntimeError):
    """Raised when the FHE feasibility assurance profile is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise FheFeasibilityAssuranceError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise FheFeasibilityAssuranceError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a list or raise a validation error."""
    if not isinstance(value, list):
        raise FheFeasibilityAssuranceError(f"{label} must be a list")
    return value


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality with a stable error message."""
    if actual != expected:
        raise FheFeasibilityAssuranceError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def validate_pds_boundary(pds: dict[str, Any], label: str) -> None:
    """Validate the current non-dedicated PDS boundary for FHE feasibility."""
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
        "rust_feasibility_release_gate",
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
            raise FheFeasibilityAssuranceError(
                f"dedicated FHE feasibility PDS source kind is unexpectedly implemented in {path}",
            )
    if FUTURE_PDS_SOURCE_KIND in server_rust_source():
        raise FheFeasibilityAssuranceError(
            "dedicated FHE feasibility PDS source kind is unexpectedly implemented in kiva-server",
        )


def length_prefixed(value: str) -> bytes:
    """Return the Rust update_len_prefixed projection for a UTF-8 string."""
    encoded = value.encode("utf-8")
    return len(encoded).to_bytes(16, "big") + encoded


def validate_parameter_mapping(value: dict[str, Any], label: str) -> None:
    """Validate the strict CC6 parameter set."""
    require_equal(
        value.get("polynomial_modulus_degree"),
        POLYNOMIAL_MODULUS_DEGREE,
        f"{label}.polynomial_modulus_degree",
    )
    require_equal(value.get("plaintext_modulus"), PLAINTEXT_MODULUS, f"{label}.plaintext_modulus")
    require_equal(
        value.get("ciphertext_modulus_bits"),
        CIPHERTEXT_MODULUS_BITS,
        f"{label}.ciphertext_modulus_bits",
    )
    require_equal(
        value.get("security_level_bits"),
        SECURITY_LEVEL_BITS,
        f"{label}.security_level_bits",
    )
    require_equal(
        value.get("max_participant_count"),
        MAX_PARTICIPANT_COUNT,
        f"{label}.max_participant_count",
    )
    require_equal(
        value.get("max_plaintext_value"),
        MAX_PLAINTEXT_VALUE,
        f"{label}.max_plaintext_value",
    )
    if MAX_PARTICIPANT_COUNT * MAX_PLAINTEXT_VALUE >= PLAINTEXT_MODULUS:
        raise FheFeasibilityAssuranceError("strict FHE parameter set permits plaintext wrap")


def derive_ciphertext_digest_hex(vector: dict[str, Any], participant: dict[str, Any]) -> str:
    """Derive a deterministic ciphertext fixture digest."""
    digest = hashlib.sha256()
    digest.update(CIPHERTEXT_DOMAIN.encode("utf-8"))
    digest.update(length_prefixed(PROFILE))
    digest.update(length_prefixed(SCHEME_FAMILY))
    digest.update(length_prefixed(OPERATION))
    digest.update(bytes([int(vector["job_seed"])]) * 32)
    digest.update(int(vector["polynomial_modulus_degree"]).to_bytes(4, "big"))
    digest.update(int(vector["plaintext_modulus"]).to_bytes(8, "big"))
    digest.update(int(vector["ciphertext_modulus_bits"]).to_bytes(2, "big"))
    digest.update(int(vector["security_level_bits"]).to_bytes(2, "big"))
    digest.update(MAX_PARTICIPANT_COUNT.to_bytes(16, "big"))
    digest.update(int(vector["max_plaintext_value"]).to_bytes(8, "big"))
    digest.update(int(participant["participant_index"]).to_bytes(16, "big"))
    digest.update(int(participant["plaintext_value"]).to_bytes(8, "big"))
    digest.update(bytes([int(participant["nonce_seed"])]) * 32)
    return digest.hexdigest()


def derive_evaluation_digest_hex(
    vector: dict[str, Any],
    participant_digests: list[tuple[int, str]],
) -> str:
    """Derive a deterministic evaluated ciphertext fixture digest."""
    digest = hashlib.sha256()
    digest.update(EVALUATION_DOMAIN.encode("utf-8"))
    digest.update(length_prefixed(PROFILE))
    digest.update(length_prefixed(SCHEME_FAMILY))
    digest.update(length_prefixed(OPERATION))
    digest.update(bytes([int(vector["job_seed"])]) * 32)
    digest.update(int(vector["polynomial_modulus_degree"]).to_bytes(4, "big"))
    digest.update(int(vector["plaintext_modulus"]).to_bytes(8, "big"))
    digest.update(int(vector["ciphertext_modulus_bits"]).to_bytes(2, "big"))
    digest.update(int(vector["security_level_bits"]).to_bytes(2, "big"))
    digest.update(MAX_PARTICIPANT_COUNT.to_bytes(16, "big"))
    digest.update(int(vector["max_plaintext_value"]).to_bytes(8, "big"))
    digest.update(len(participant_digests).to_bytes(16, "big"))
    for participant_index, digest_hex in sorted(participant_digests):
        digest.update(participant_index.to_bytes(16, "big"))
        digest.update(bytes.fromhex(digest_hex))
    digest.update(int(vector["expected_sum"]).to_bytes(8, "big"))
    return digest.hexdigest()


def validate_vectors(profile: dict[str, Any]) -> dict[str, Any]:
    """Validate deterministic FHE feasibility vectors."""
    vectors_config = require_mapping(profile.get("vectors"), "vectors")
    vectors_path = Path(str(vectors_config.get("path")))
    vectors = load_yaml(vectors_path)
    require_equal(vectors.get("schema_version"), 1, "vectors.schema_version")
    require_equal(vectors.get("kind"), "fhe-sum-feasibility-vectors", "vectors.kind")
    entries = require_list(vectors.get("vectors"), "vectors.vectors")
    checked_ciphertexts = 0
    for entry in entries:
        vector = require_mapping(entry, "vectors[]")
        require_equal(vector.get("profile"), PROFILE, "vector.profile")
        require_equal(
            vector.get("ciphertext_domain"),
            CIPHERTEXT_DOMAIN,
            "vector.ciphertext_domain",
        )
        require_equal(
            vector.get("evaluation_domain"),
            EVALUATION_DOMAIN,
            "vector.evaluation_domain",
        )
        require_equal(vector.get("scheme_family"), SCHEME_FAMILY, "vector.scheme_family")
        require_equal(vector.get("operation"), OPERATION, "vector.operation")
        validate_parameter_mapping(vector, "vector")

        participants = require_list(vector.get("participants"), "vector.participants")
        require_equal(len(participants), int(vector["participant_count"]), "vector.participant_count")
        seen_participants: set[int] = set()
        expected_sum = 0
        participant_digests: list[tuple[int, str]] = []
        for participant_value in participants:
            participant = require_mapping(participant_value, "vector.participants[]")
            participant_index = int(participant["participant_index"])
            if participant_index in seen_participants:
                raise FheFeasibilityAssuranceError("vector contains duplicate participant index")
            seen_participants.add(participant_index)
            if not 1 <= participant_index <= int(vector["participant_count"]):
                raise FheFeasibilityAssuranceError("vector participant index is out of range")
            plaintext_value = int(participant["plaintext_value"])
            if plaintext_value > MAX_PLAINTEXT_VALUE:
                raise FheFeasibilityAssuranceError("vector plaintext exceeds profile maximum")
            expected_sum += plaintext_value
            digest_hex = derive_ciphertext_digest_hex(vector, participant)
            require_equal(
                participant.get("expected_ciphertext_digest_hex"),
                digest_hex,
                "participant.expected_ciphertext_digest_hex",
            )
            participant_digests.append((participant_index, digest_hex))
            checked_ciphertexts += 1
        if expected_sum >= PLAINTEXT_MODULUS:
            raise FheFeasibilityAssuranceError("vector sum wraps plaintext modulus")
        require_equal(vector.get("expected_sum"), expected_sum, "vector.expected_sum")
        require_equal(
            vector.get("expected_evaluated_ciphertext_digest_hex"),
            derive_evaluation_digest_hex(vector, participant_digests),
            "vector.expected_evaluated_ciphertext_digest_hex",
        )
    return {
        "vectors_path": str(vectors_path),
        "vector_count": len(entries),
        "checked_ciphertexts": checked_ciphertexts,
    }


def validate_job_contract(path: Path) -> dict[str, Any]:
    """Validate the FHE feasibility job contract."""
    contract = load_yaml(path)
    require_equal(contract.get("schema_version"), 1, "job_contract.schema_version")
    require_equal(contract.get("kind"), "fhe-sum-feasibility-job-contract", "job_contract.kind")
    require_equal(contract.get("status"), "published", "job_contract.status")
    require_equal(contract.get("profile"), PROFILE, "job_contract.profile")

    operation = require_mapping(contract.get("operation"), "operation")
    require_equal(operation.get("family"), SCHEME_FAMILY, "operation.family")
    require_equal(operation.get("operation"), OPERATION, "operation.operation")
    require_equal(operation.get("deterministic_fixture"), True, "operation.deterministic_fixture")
    require_equal(operation.get("production_encryption"), False, "operation.production_encryption")
    require_equal(operation.get("requires_all_participants"), True, "operation.requires_all_participants")
    require_equal(operation.get("ciphertext_domain"), CIPHERTEXT_DOMAIN, "operation.ciphertext_domain")
    require_equal(operation.get("evaluation_domain"), EVALUATION_DOMAIN, "operation.evaluation_domain")

    validate_parameter_mapping(require_mapping(contract.get("parameters"), "parameters"), "parameters")

    request = require_mapping(contract.get("request"), "request")
    forbidden = set(require_list(request.get("forbidden_fields"), "request.forbidden_fields"))
    for field in ("plaintext", "plaintextValue", "rawPlaintext", "decryptionKey", "secretKey"):
        if field not in forbidden:
            raise FheFeasibilityAssuranceError(f"request.forbidden_fields is missing {field}")

    evaluation = require_mapping(contract.get("evaluation"), "evaluation")
    for key in (
        "rejects_missing_participants",
        "rejects_duplicate_participants",
        "rejects_job_mismatch",
        "rejects_parameter_mismatch",
        "rejects_plaintext_modulus_wrap",
    ):
        require_equal(evaluation.get(key), True, f"evaluation.{key}")

    release = require_mapping(contract.get("release"), "release")
    for key in (
        "requires_completed_evaluation",
        "requires_policy_approval",
        "requires_governed_release_treatment",
        "requires_clear_plaintext_boundary",
    ):
        require_equal(release.get(key), True, f"release.{key}")

    pds = require_mapping(contract.get("pds"), "pds")
    validate_pds_boundary(pds, "pds")

    non_goals = require_mapping(contract.get("non_goals"), "non_goals")
    for key, value in non_goals.items():
        require_equal(value, False, f"non_goals.{key}")

    return {"path": str(path), "profile": contract["profile"]}


def validate_benchmarks(profile: dict[str, Any]) -> dict[str, Any]:
    """Validate FHE feasibility benchmark fixtures."""
    benchmarks_config = require_mapping(profile.get("benchmarks"), "benchmarks")
    benchmarks_path = Path(str(benchmarks_config.get("path")))
    benchmarks = load_yaml(benchmarks_path)
    require_equal(benchmarks.get("schema_version"), 1, "benchmarks.schema_version")
    require_equal(
        benchmarks.get("kind"),
        "fhe-sum-feasibility-benchmark-fixtures",
        "benchmarks.kind",
    )
    require_equal(benchmarks.get("status"), "published", "benchmarks.status")
    require_equal(benchmarks.get("profile"), PROFILE, "benchmarks.profile")

    fixture_scope = require_mapping(benchmarks.get("fixture_scope"), "fixture_scope")
    require_equal(
        fixture_scope.get("deterministic_fixture_only"),
        True,
        "fixture_scope.deterministic_fixture_only",
    )
    require_equal(
        fixture_scope.get("measured_with_real_fhe_library"),
        False,
        "fixture_scope.measured_with_real_fhe_library",
    )
    require_equal(
        fixture_scope.get("production_sizing_claim"),
        False,
        "fixture_scope.production_sizing_claim",
    )
    validate_parameter_mapping(
        require_mapping(benchmarks.get("parameter_set"), "parameter_set"),
        "parameter_set",
    )
    cost_model = require_mapping(benchmarks.get("cost_model"), "cost_model")
    require_equal(cost_model.get("operation"), OPERATION, "cost_model.operation")
    require_equal(cost_model.get("bootstrapping_required"), False, "cost_model.bootstrapping_required")
    require_equal(cost_model.get("supported_for_interactive_bi"), False, "cost_model.supported_for_interactive_bi")

    fixtures = require_list(benchmarks.get("fixtures"), "fixtures")
    if not fixtures:
        raise FheFeasibilityAssuranceError("benchmark fixtures must not be empty")
    for fixture_value in fixtures:
        fixture = require_mapping(fixture_value, "fixtures[]")
        if int(fixture["participant_count"]) > MAX_PARTICIPANT_COUNT:
            raise FheFeasibilityAssuranceError("benchmark fixture exceeds max participants")
        if int(fixture["expected_sum"]) >= PLAINTEXT_MODULUS:
            raise FheFeasibilityAssuranceError("benchmark fixture wraps plaintext modulus")
    return {"path": str(benchmarks_path), "fixture_count": len(fixtures)}


def validate_rust_core() -> None:
    """Require the Rust core to expose the expected CC6 boundary functions."""
    missing = missing_confidential_rust_terms("fhe-feasibility")
    if missing:
        raise FheFeasibilityAssuranceError(
            "kiva-confidential is missing FHE terms: " + ", ".join(missing),
        )


def validate_profile(profile: dict[str, Any], contract_path: Path) -> dict[str, Any]:
    """Validate profile invariants and return a report payload."""
    require_equal(profile.get("schema_version"), 1, "schema_version")
    require_equal(profile.get("kind"), "fhe-sum-feasibility-assurance-profile", "kind")
    require_equal(profile.get("status"), "published", "status")
    require_equal(profile.get("profile"), PROFILE, "profile")

    claim_boundary = require_mapping(profile.get("claim_boundary"), "claim_boundary")
    require_equal(
        claim_boundary.get("deterministic_fixture_only"),
        True,
        "claim_boundary.deterministic_fixture_only",
    )
    require_equal(
        claim_boundary.get("production_fhe_encryption"),
        False,
        "claim_boundary.production_fhe_encryption",
    )
    require_equal(
        claim_boundary.get("cryptographic_confidentiality_claim"),
        False,
        "claim_boundary.cryptographic_confidentiality_claim",
    )
    require_equal(claim_boundary.get("supported_operation"), OPERATION, "claim_boundary.supported_operation")
    require_equal(
        claim_boundary.get("supported_scheme_family"),
        SCHEME_FAMILY,
        "claim_boundary.supported_scheme_family",
    )
    require_equal(claim_boundary.get("requires_all_participants"), True, "claim_boundary.requires_all_participants")

    parameters = require_mapping(profile.get("parameters"), "parameters")
    validate_parameter_mapping(parameters, "parameters")
    require_equal(
        parameters.get("max_plaintext_sum_below_plaintext_modulus"),
        True,
        "parameters.max_plaintext_sum_below_plaintext_modulus",
    )
    require_equal(parameters.get("rejects_parameter_variants"), True, "parameters.rejects_parameter_variants")

    boundaries = require_mapping(profile.get("boundaries"), "boundaries")
    ciphertext_plaintext = require_mapping(
        boundaries.get("ciphertext_plaintext"),
        "boundaries.ciphertext_plaintext",
    )
    for key in (
        "ciphertext_digest_public",
        "public_ciphertext_contract_excludes_plaintext",
        "fixture_plaintext_private_to_rust_tests",
        "repository_persistence_not_enabled",
    ):
        require_equal(ciphertext_plaintext.get(key), True, f"ciphertext_plaintext.{key}")

    evaluation = require_mapping(boundaries.get("evaluation"), "boundaries.evaluation")
    require_equal(evaluation.get("operation"), OPERATION, "evaluation.operation")
    for key in (
        "rejects_missing_participants",
        "rejects_duplicate_participants",
        "rejects_plaintext_overflow_or_modular_wrap",
    ):
        require_equal(evaluation.get(key), True, f"evaluation.{key}")

    release = require_mapping(boundaries.get("release"), "boundaries.release")
    for key in (
        "requires_completed_evaluation",
        "requires_policy_approval",
        "requires_governed_release_treatment",
        "requires_clear_plaintext_boundary",
    ):
        require_equal(release.get(key), True, f"release.{key}")

    validate_pds_boundary(
        require_mapping(boundaries.get("pds_boundary"), "boundaries.pds_boundary"),
        "boundaries.pds_boundary",
    )

    evidence = require_mapping(profile.get("evidence"), "evidence")
    missing_paths = sorted(
        path for path in evidence.values() if isinstance(path, str) and not Path(path).exists()
    )
    if missing_paths:
        raise FheFeasibilityAssuranceError("missing evidence files: " + ", ".join(missing_paths))

    non_goals = require_mapping(profile.get("non_goals"), "non_goals")
    for key, value in non_goals.items():
        require_equal(value, False, f"non_goals.{key}")

    validate_rust_core()
    validate_no_public_pds_source_kind()
    return {
        "kind": "fhe-feasibility-assurance-validation",
        "status": "passed",
        "profile": profile["profile"],
        "job_contract": validate_job_contract(contract_path),
        "vectors": validate_vectors(profile),
        "benchmarks": validate_benchmarks(profile),
    }


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", type=Path, default=DEFAULT_PROFILE)
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    parser.add_argument("--json-report", type=Path)
    return parser.parse_args()


def main() -> int:
    """Run the FHE feasibility assurance validator."""
    args = parse_args()
    profile = load_yaml(args.profile)
    report = validate_profile(profile, args.contract)
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
