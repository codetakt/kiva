#!/usr/bin/env python3
"""Validate Kiva PSI cohort assurance profile, job contract, and vectors."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

import yaml

from source_scan import server_rust_source


DEFAULT_PROFILE = Path("spec/confidential/psi-cohort-assurance.yaml")
DEFAULT_CONTRACT = Path("spec/confidential/psi-cohort-job-contract.yaml")
PROFILE = "kiva-psi-cohort-hash-v1"
DOMAIN = "kiva-confidential:psi-cohort-identifier:v1"
FUTURE_PDS_SOURCE_KIND = "private-set-intersection-result"


class PsiCohortAssuranceError(RuntimeError):
    """Raised when the PSI cohort assurance profile is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise PsiCohortAssuranceError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise PsiCohortAssuranceError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a list or raise a validation error."""
    if not isinstance(value, list):
        raise PsiCohortAssuranceError(f"{label} must be a list")
    return value


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality with a stable error message."""
    if actual != expected:
        raise PsiCohortAssuranceError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def validate_pds_boundary(pds: dict[str, Any], label: str) -> None:
    """Validate that PSI cohort has no current dedicated PDS source kind."""
    require_equal(
        pds.get("dedicated_data_product_source_kind_implemented"),
        False,
        f"{label}.dedicated_data_product_source_kind_implemented",
    )
    require_equal(pds.get("future_source_kind"), FUTURE_PDS_SOURCE_KIND, f"{label}.future_source_kind")
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
            raise PsiCohortAssuranceError(
                f"dedicated PSI PDS source kind is unexpectedly implemented in {path}",
            )
    if FUTURE_PDS_SOURCE_KIND in server_rust_source():
        raise PsiCohortAssuranceError(
            "dedicated PSI PDS source kind is unexpectedly implemented in kiva-server",
        )


def canonicalize_identifier(identifier: str) -> str:
    """Apply the Rust PSI cohort canonicalization rule."""
    canonical = identifier.strip().lower()
    if not canonical:
        raise PsiCohortAssuranceError("identifier became empty after canonicalization")
    if any(ord(character) < 0x20 or ord(character) == 0x7F for character in canonical):
        raise PsiCohortAssuranceError("identifier contains an ASCII control character")
    return canonical


def derive_digest_hex(job_seed: int, namespace: str, identifier: str) -> str:
    """Derive the expected profile digest."""
    namespace = namespace.strip()
    if not namespace:
        raise PsiCohortAssuranceError("namespace became empty after canonicalization")
    if any(ord(character) < 0x20 or ord(character) == 0x7F for character in namespace):
        raise PsiCohortAssuranceError("namespace contains an ASCII control character")
    canonical = canonicalize_identifier(identifier)

    digest = hashlib.sha256()
    digest.update(DOMAIN.encode("utf-8"))
    digest.update(bytes([job_seed]) * 32)
    namespace_bytes = namespace.encode("utf-8")
    identifier_bytes = canonical.encode("utf-8")
    digest.update(len(namespace_bytes).to_bytes(16, "big"))
    digest.update(namespace_bytes)
    digest.update(len(identifier_bytes).to_bytes(16, "big"))
    digest.update(identifier_bytes)
    return digest.hexdigest()


def validate_job_contract(path: Path) -> dict[str, Any]:
    """Validate the PSI job contract boundary."""
    contract = load_yaml(path)
    require_equal(contract.get("schema_version"), 1, "job_contract.schema_version")
    require_equal(contract.get("kind"), "psi-cohort-job-contract", "job_contract.kind")
    require_equal(contract.get("profile"), PROFILE, "job_contract.profile")
    require_equal(contract.get("use_case"), "cross-school-cohort-matching", "job_contract.use_case")

    protocol = require_mapping(contract.get("protocol"), "job_contract.protocol")
    require_equal(protocol.get("family"), "hash-token-n-of-n-intersection", "protocol.family")
    require_equal(protocol.get("digest"), "sha256", "protocol.digest")
    require_equal(protocol.get("profile_domain"), DOMAIN, "protocol.profile_domain")
    require_equal(protocol.get("minimum_participants"), 2, "protocol.minimum_participants")
    require_equal(protocol.get("requires_all_participants"), True, "protocol.requires_all_participants")

    canonicalization = require_mapping(contract.get("canonicalization"), "canonicalization")
    require_equal(
        canonicalization.get("raw_identifiers_admitted_to_kiva"),
        False,
        "canonicalization.raw_identifiers_admitted_to_kiva",
    )

    request = require_mapping(contract.get("request"), "request")
    forbidden = set(require_list(request.get("forbidden_fields"), "request.forbidden_fields"))
    for field in ("rawIdentifier", "raw_identifier", "identifier", "identifiers", "studentId"):
        if field not in forbidden:
            raise PsiCohortAssuranceError(f"request.forbidden_fields is missing {field}")

    response = require_mapping(contract.get("response"), "response")
    required = set(require_list(response.get("required_fields"), "response.required_fields"))
    for field in ("intersection_digests", "intersection_count"):
        if field not in required:
            raise PsiCohortAssuranceError(f"response.required_fields is missing {field}")
    forbidden_response = set(require_list(response.get("forbidden_fields"), "response.forbidden_fields"))
    for field in ("nonMatchingDigests", "participantInputs", "rawIdentifier", "identifier"):
        if field not in forbidden_response:
            raise PsiCohortAssuranceError(f"response.forbidden_fields is missing {field}")

    pds = require_mapping(contract.get("pds"), "pds")
    validate_pds_boundary(pds, "pds")
    require_equal(pds.get("resource_class"), "aggregate-insights", "pds.resource_class")
    require_equal(pds.get("data_product_admission_requires"), [], "pds.data_product_admission_requires")

    return {
        "path": str(path),
        "profile": contract["profile"],
    }


def validate_vectors(profile: dict[str, Any]) -> dict[str, Any]:
    """Validate deterministic PSI cohort vectors."""
    vectors_config = require_mapping(profile.get("vectors"), "vectors")
    vectors_path = Path(str(vectors_config.get("path")))
    vectors = load_yaml(vectors_path)
    entries = require_list(vectors.get("vectors"), "vectors.vectors")
    checked_digests = 0
    for entry in entries:
        vector = require_mapping(entry, "vectors[]")
        require_equal(vector.get("profile"), PROFILE, "vector.profile")
        require_equal(vector.get("domain"), DOMAIN, "vector.domain")
        job_seed = int(vector.get("job_seed"))
        namespace = str(vector.get("namespace"))
        participant_count = int(vector.get("participant_count"))
        if participant_count < 2:
            raise PsiCohortAssuranceError("vector participant_count must be at least two")
        participants = require_list(vector.get("participants"), "vector.participants")
        if len(participants) != participant_count:
            raise PsiCohortAssuranceError("vector participant count does not match participants")
        seen_participants: set[int] = set()
        participant_digest_sets: list[set[str]] = []
        for participant_value in participants:
            participant = require_mapping(participant_value, "vector.participants[]")
            participant_index = int(participant.get("participant_index"))
            if participant_index in seen_participants:
                raise PsiCohortAssuranceError("vector contains duplicate participant index")
            seen_participants.add(participant_index)
            if not 1 <= participant_index <= participant_count:
                raise PsiCohortAssuranceError("vector participant index is out of range")
            identifiers = require_list(participant.get("identifiers"), "participant.identifiers")
            digests = require_list(participant.get("digest_hex"), "participant.digest_hex")
            if len(identifiers) != len(digests):
                raise PsiCohortAssuranceError("participant identifiers and digests differ in length")
            if not identifiers:
                raise PsiCohortAssuranceError("participant identifiers must not be empty")
            seen_digests: set[str] = set()
            for identifier, digest_hex in zip(identifiers, digests, strict=True):
                expected = derive_digest_hex(job_seed, namespace, str(identifier))
                require_equal(digest_hex, expected, "participant.digest_hex[]")
                if digest_hex in seen_digests:
                    raise PsiCohortAssuranceError("participant contains duplicate digest")
                seen_digests.add(str(digest_hex))
                checked_digests += 1
            participant_digest_sets.append(seen_digests)
        intersection = sorted(set.intersection(*participant_digest_sets))
        require_equal(
            vector.get("expected_intersection_hex"),
            intersection,
            "vector.expected_intersection_hex",
        )
    return {
        "vectors_path": str(vectors_path),
        "vector_count": len(entries),
        "checked_digests": checked_digests,
    }


def validate_profile(profile: dict[str, Any], contract_path: Path) -> dict[str, Any]:
    """Validate profile invariants and return a report payload."""
    require_equal(profile.get("schema_version"), 1, "schema_version")
    require_equal(profile.get("kind"), "psi-cohort-assurance-profile", "kind")
    require_equal(profile.get("status"), "published", "status")
    require_equal(profile.get("profile"), PROFILE, "profile")

    protocol = require_mapping(profile.get("protocol"), "protocol")
    require_equal(protocol.get("family"), "hash-token-n-of-n-intersection", "protocol.family")
    require_equal(protocol.get("digest"), "sha256", "protocol.digest")
    require_equal(protocol.get("domain"), DOMAIN, "protocol.domain")
    require_equal(protocol.get("minimum_participants"), 2, "protocol.minimum_participants")
    require_equal(protocol.get("requires_all_participants"), True, "protocol.requires_all_participants")
    require_equal(
        protocol.get("raw_identifiers_admitted_to_kiva"),
        False,
        "protocol.raw_identifiers_admitted_to_kiva",
    )

    admission = require_mapping(profile.get("admission"), "admission")
    for key in (
        "requires_job_id",
        "rejects_replayed_job_id",
        "requires_exactly_one_dataset_per_participant",
        "requires_non_empty_participant_datasets",
        "rejects_duplicate_participant_indexes",
        "rejects_duplicate_identifier_digests_per_participant",
        "rejects_out_of_range_participant_indexes",
    ):
        require_equal(admission.get(key), True, f"admission.{key}")

    output = require_mapping(profile.get("output"), "output")
    require_equal(output.get("exposes_intersection_digests"), True, "output.exposes_intersection_digests")
    for key in ("exposes_raw_identifiers", "exposes_non_matching_digests", "exposes_participant_inputs"):
        require_equal(output.get(key), False, f"output.{key}")

    pds = require_mapping(profile.get("pds"), "pds")
    validate_pds_boundary(pds, "pds")
    require_equal(pds.get("aggregate_insights_only"), True, "pds.aggregate_insights_only")
    require_equal(pds.get("pds_openapi_source_kind"), "absent", "pds.pds_openapi_source_kind")
    require_equal(pds.get("pds_postgresql_source_kind"), "absent", "pds.pds_postgresql_source_kind")
    require_equal(pds.get("pds_policy_source_kind"), "absent", "pds.pds_policy_source_kind")

    evidence = require_mapping(profile.get("evidence"), "evidence")
    missing = sorted(
        path for path in evidence.values() if isinstance(path, str) and not Path(path).exists()
    )
    if missing:
        raise PsiCohortAssuranceError("missing evidence files: " + ", ".join(missing))

    validate_no_public_pds_source_kind()
    contract_report = validate_job_contract(contract_path)
    vector_report = validate_vectors(profile)
    return {
        "kind": "psi-cohort-assurance-validation",
        "status": "passed",
        "profile": profile["profile"],
        "job_contract": contract_report,
        "vectors": vector_report,
    }


def main() -> int:
    """Run the PSI cohort assurance validator."""
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
        "validated PSI cohort assurance profile: "
        f"{args.profile}, profile={report['profile']}",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
