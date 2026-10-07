#!/usr/bin/env python3
"""Validate Kiva Shamir VSS assurance profile and vectors."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

import yaml


DEFAULT_PROFILE = Path("spec/confidential/shamir-vss-assurance.yaml")
DEFAULT_CONTRACT = Path("spec/openapi/kiva-confidential.yaml")
FIELD_MODULUS = 4_294_967_291


class ShamirVssAssuranceError(RuntimeError):
    """Raised when the Shamir VSS profile is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise ShamirVssAssuranceError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise ShamirVssAssuranceError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a list or raise a validation error."""
    if not isinstance(value, list):
        raise ShamirVssAssuranceError(f"{label} must be a list")
    return value


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality with a stable error message."""
    if actual != expected:
        raise ShamirVssAssuranceError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def derive_commitment_hex(
    domain: str,
    transcript_seed: int,
    participant_index: int,
    value: int,
    opening_nonce_seed: int,
) -> str:
    """Derive the expected SHA-256 share-opening commitment hex."""
    digest = hashlib.sha256()
    digest.update(domain.encode("utf-8"))
    digest.update(bytes([transcript_seed]) * 32)
    digest.update(participant_index.to_bytes(16, "big"))
    digest.update(value.to_bytes(8, "big"))
    digest.update(bytes([opening_nonce_seed]) * 32)
    return digest.hexdigest()


def validate_vectors(profile: dict[str, Any]) -> dict[str, Any]:
    """Validate deterministic Shamir VSS vectors."""
    vectors_config = require_mapping(profile.get("vectors"), "vectors")
    vectors_path = Path(str(vectors_config.get("path")))
    vectors = load_yaml(vectors_path)
    entries = require_list(vectors.get("vectors"), "vectors.vectors")
    required_profile = str(vectors_config.get("required_profile"))
    checked_openings = 0
    for entry in entries:
        vector = require_mapping(entry, "vectors[]")
        require_equal(vector.get("field_modulus"), FIELD_MODULUS, "vector.field_modulus")
        require_equal(
            vector.get("verification_profile"),
            required_profile,
            "vector.verification_profile",
        )
        commitment_domain = str(vector.get("commitment_domain"))
        transcript_seed = int(vector.get("transcript_seed"))
        participant_count = int(vector.get("participant_count"))
        threshold = int(vector.get("threshold"))
        if participant_count < 2 or threshold < 2 or threshold > participant_count:
            raise ShamirVssAssuranceError("vector threshold policy is invalid")
        openings = require_list(vector.get("share_openings"), "vector.share_openings")
        if len(openings) < threshold:
            raise ShamirVssAssuranceError("vector has fewer openings than threshold")
        seen: set[int] = set()
        for opening_value in openings:
            opening = require_mapping(opening_value, "vector.share_openings[]")
            participant_index = int(opening.get("participant_index"))
            share_value = int(opening.get("value"))
            nonce_seed = int(opening.get("opening_nonce_seed"))
            if participant_index in seen:
                raise ShamirVssAssuranceError("vector contains duplicate participant index")
            seen.add(participant_index)
            if not 1 <= participant_index <= participant_count:
                raise ShamirVssAssuranceError("vector participant index is out of range")
            if not 0 <= share_value < FIELD_MODULUS:
                raise ShamirVssAssuranceError("vector share value is outside the field")
            expected = derive_commitment_hex(
                commitment_domain,
                transcript_seed,
                participant_index,
                share_value,
                nonce_seed,
            )
            require_equal(opening.get("commitment_hex"), expected, "opening.commitment_hex")
            checked_openings += 1
    return {
        "vectors_path": str(vectors_path),
        "vector_count": len(entries),
        "checked_openings": checked_openings,
    }


def validate_openapi(contract_path: Path) -> None:
    """Validate OpenAPI projection for Shamir VSS metadata."""
    contract = load_yaml(contract_path)
    schemas = require_mapping(
        require_mapping(contract.get("components"), "components").get("schemas"),
        "components.schemas",
    )
    reconstruction = require_mapping(
        schemas.get("ShamirThresholdReconstruction"),
        "ShamirThresholdReconstruction",
    )
    require_equal(
        "share_openings" in reconstruction.get("required", []),
        True,
        "ShamirThresholdReconstruction.required.share_openings",
    )
    require_equal(
        "shares" in reconstruction.get("required", []),
        False,
        "ShamirThresholdReconstruction.required.shares",
    )
    require_mapping(
        schemas.get("VerifiableShamirShare"),
        "VerifiableShamirShare",
    )
    metadata = require_mapping(
        schemas.get("ShamirThresholdReconstructionMetadata"),
        "ShamirThresholdReconstructionMetadata",
    )
    required = set(require_list(metadata.get("required"), "Shamir metadata required"))
    expected = {"verificationProfile", "verifiedShareCount", "complaintCount"}
    missing = expected.difference(required)
    if missing:
        raise ShamirVssAssuranceError(
            "Shamir metadata is missing required fields: " + ", ".join(sorted(missing)),
        )
    properties = set(require_mapping(metadata.get("properties"), "Shamir metadata properties"))
    forbidden = {"share", "shares", "shareOpenings", "commitment", "openingNonce"}
    leaked = forbidden.intersection(properties)
    if leaked:
        raise ShamirVssAssuranceError(
            "Shamir metadata exposes forbidden fields: " + ", ".join(sorted(leaked)),
        )


def validate_profile(profile: dict[str, Any], contract_path: Path) -> dict[str, Any]:
    """Validate profile invariants and return a report payload."""
    require_equal(profile.get("schema_version"), 1, "schema_version")
    require_equal(profile.get("kind"), "shamir-vss-assurance-profile", "kind")
    require_equal(profile.get("status"), "published", "status")
    require_equal(
        profile.get("verification_profile"),
        "kiva-shamir-hash-vss-v1",
        "verification_profile",
    )

    mechanism = require_mapping(profile.get("mechanism"), "mechanism")
    require_equal(mechanism.get("secret_sharing"), "shamir-threshold", "mechanism.secret_sharing")
    require_equal(mechanism.get("field_modulus"), FIELD_MODULUS, "mechanism.field_modulus")
    require_equal(
        mechanism.get("commitment_scheme"),
        "sha256-share-opening",
        "mechanism.commitment_scheme",
    )
    require_equal(int(mechanism.get("minimum_threshold")), 2, "mechanism.minimum_threshold")

    admission = require_mapping(profile.get("admission"), "admission")
    for key in (
        "requires_threshold_plan",
        "requires_threshold_at_least_two",
        "requires_threshold_within_participants",
        "requires_unique_participant_indexes",
        "requires_verified_share_openings",
        "rejects_commitment_mismatch",
        "excludes_failed_openings_from_reconstruction",
    ):
        require_equal(admission.get(key), True, f"admission.{key}")

    persistence = require_mapping(profile.get("persistence"), "persistence")
    for key in ("persist_share_values", "persist_opening_nonces"):
        require_equal(persistence.get(key), False, f"persistence.{key}")
    for key in ("expose_share_values_in_metadata", "expose_opening_nonces_in_metadata"):
        require_equal(persistence.get(key), False, f"persistence.{key}")

    evidence = require_mapping(profile.get("evidence"), "evidence")
    missing = sorted(path for path in evidence.values() if isinstance(path, str) and not Path(path).exists())
    if missing:
        raise ShamirVssAssuranceError("missing evidence files: " + ", ".join(missing))

    vector_report = validate_vectors(profile)
    validate_openapi(contract_path)
    return {
        "kind": "shamir-vss-assurance-validation",
        "status": "passed",
        "verification_profile": profile["verification_profile"],
        "vectors": vector_report,
    }


def main() -> int:
    """Run the Shamir VSS assurance validator."""
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
        "validated Shamir VSS assurance profile: "
        f"{args.profile}, profile={report['verification_profile']}",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
