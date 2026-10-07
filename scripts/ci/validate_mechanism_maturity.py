#!/usr/bin/env python3
"""Validate Kiva confidential mechanism maturity registry."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import yaml


EXPECTED_MECHANISMS = {
    "additive-secure-aggregation",
    "differential-privacy-release",
    "shamir-vss-admission",
    "psi-cohort-hash-token",
    "tee-portable-attestation",
    "fhe-sum-feasibility",
    "selected-mechanism-assurance-package",
    "multi-agency-research-aggregate-v1",
    "confidential-job-lifecycle",
    "confidential-participant-key-lifecycle",
    "psi-oprf-dh-voprf",
    "tee-aws-nitro-attestation",
    "threshold-frost-signing",
    "fhe-sum-runtime",
    "external-production-assurance-readiness",
    "secret-compute-provider-contract",
}
PDS_RELEASE_ARTIFACT_KINDS = {
    "threshold-release",
    "differential-privacy-release",
    "shamir-threshold-reconstruction",
}
PDS_ARTIFACT_MECHANISMS = {
    "additive-secure-aggregation": "threshold-release",
    "differential-privacy-release": "differential-privacy-release",
    "shamir-vss-admission": "shamir-threshold-reconstruction",
}
EXTERNAL_REVIEW_PENDING = {
    "psi-oprf-dh-voprf",
    "tee-aws-nitro-attestation",
    "threshold-frost-signing",
    "fhe-sum-runtime",
}


class MaturityError(RuntimeError):
    """Raised when the maturity registry violates a Kiva invariant."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise MaturityError("mechanism maturity root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a maturity error."""
    if not isinstance(value, dict):
        raise MaturityError(f"{label} must be a mapping")
    return value


def require_string(value: Any, label: str) -> str:
    """Return a non-empty string or raise a maturity error."""
    if not isinstance(value, str) or not value:
        raise MaturityError(f"{label} must be a non-empty string")
    return value


def require_bool(value: Any, label: str) -> bool:
    """Return a bool or raise a maturity error."""
    if not isinstance(value, bool):
        raise MaturityError(f"{label} must be a bool")
    return value


def validate(document: dict[str, Any]) -> None:
    """Validate the mechanism maturity registry."""
    if document.get("kind") != "kiva-confidential-mechanism-maturity":
        raise MaturityError("kind must be kiva-confidential-mechanism-maturity")
    if document.get("version") != 1:
        raise MaturityError("version must be 1")

    status_levels = document.get("status_levels")
    if not isinstance(status_levels, list) or not all(isinstance(item, str) for item in status_levels):
        raise MaturityError("status_levels must be a string list")
    status_level_set = set(status_levels)

    mechanisms = document.get("mechanisms")
    if not isinstance(mechanisms, list):
        raise MaturityError("mechanisms must be a list")

    seen: dict[str, dict[str, Any]] = {}
    for index, mechanism_value in enumerate(mechanisms):
        mechanism = require_mapping(mechanism_value, f"mechanisms[{index}]")
        mechanism_id = require_string(mechanism.get("id"), f"mechanisms[{index}].id")
        if mechanism_id in seen:
            raise MaturityError(f"duplicate mechanism id: {mechanism_id}")
        seen[mechanism_id] = mechanism

        milestone = require_string(mechanism.get("milestone"), f"{mechanism_id}.milestone")
        if not milestone.startswith("CC"):
            raise MaturityError(f"{mechanism_id}.milestone must reference a CC milestone")
        require_string(mechanism.get("kind"), f"{mechanism_id}.kind")
        current_level = require_string(mechanism.get("current_level"), f"{mechanism_id}.current_level")
        if current_level not in status_level_set:
            raise MaturityError(f"{mechanism_id}.current_level is not declared in status_levels")
        if require_bool(mechanism.get("production_claim"), f"{mechanism_id}.production_claim"):
            raise MaturityError(f"{mechanism_id} must not claim production security in this registry")
        require_string(mechanism.get("claim_boundary"), f"{mechanism_id}.claim_boundary")

        artifact_kind = mechanism.get("release_artifact_kind")
        if artifact_kind is not None and artifact_kind not in PDS_RELEASE_ARTIFACT_KINDS:
            raise MaturityError(f"{mechanism_id}.release_artifact_kind is unknown: {artifact_kind!r}")

    missing = EXPECTED_MECHANISMS.difference(seen)
    if missing:
        raise MaturityError("mechanism maturity registry is missing: " + ", ".join(sorted(missing)))

    for mechanism_id, artifact_kind in PDS_ARTIFACT_MECHANISMS.items():
        if seen[mechanism_id].get("release_artifact_kind") != artifact_kind:
            raise MaturityError(f"{mechanism_id}.release_artifact_kind must be {artifact_kind}")

    for mechanism_id in EXPECTED_MECHANISMS.difference(PDS_ARTIFACT_MECHANISMS):
        if seen[mechanism_id].get("release_artifact_kind") is not None:
            raise MaturityError(f"{mechanism_id} must not declare a PDS release artifact kind")

    for mechanism_id in EXTERNAL_REVIEW_PENDING:
        if seen[mechanism_id].get("current_level") != "external-review-pending":
            raise MaturityError(f"{mechanism_id}.current_level must be external-review-pending")


def main() -> int:
    """Run the validator."""
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "registry",
        type=Path,
        nargs="?",
        default=Path("spec/confidential/mechanism-maturity.yaml"),
    )
    args = parser.parse_args()

    validate(load_yaml(args.registry))
    print(f"validated {args.registry}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
