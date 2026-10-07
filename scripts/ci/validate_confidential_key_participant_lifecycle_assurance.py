#!/usr/bin/env python3
"""Validate Kiva's CC10 confidential key/participant lifecycle profile."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import yaml

from source_scan import (
    confidential_rust_source,
    missing_kani_harnesses,
    missing_rust_tests,
    registry_string_list,
    server_rust_source,
)


DEFAULT_PROFILE = Path("spec/confidential/confidential-key-participant-lifecycle-assurance.yaml")
EXPECTED_STATUSES = {"invited", "active", "suspended", "frozen", "removed"}
EXPECTED_ROLES = {
    "additive-share-contributor",
    "data-custodian",
    "policy-authority",
    "psi-participant",
    "tee-operator",
    "fhe-key-authority",
    "auditor",
}
EXPECTED_ENDPOINTS = {
    "/confidential/participants": {"get", "post"},
    "/confidential/participant-transitions": {"post"},
    "/confidential/participant-audit-events": {"get"},
    "/confidential/additive-share-signing-keys": {"get", "post"},
    "/confidential/additive-share-signing-key-rotations": {"post"},
    "/confidential/additive-share-signing-key-revocations": {"post"},
}
REQUIRED_PARTICIPANT_FIELDS = {
    "tenant_id",
    "owner_subject",
    "participant_subject",
    "participant_index",
    "status",
    "mechanism_roles",
    "authorization_scope",
    "key_ceremony_ref",
    "recovery_contact_ref",
    "evidence_refs",
    "created_at",
    "updated_at",
}
FORBIDDEN_METADATA_FIELDS = {
    "rawInput",
    "rawInputs",
    "share",
    "shares",
    "signedEnvelope",
    "plaintext",
    "secret",
    "signature",
    "noise",
    "noiseValue",
    "privateKey",
    "thresholdShare",
    "oprfSeed",
    "teeQuote",
}


class ConfidentialKeyParticipantLifecycleAssuranceError(RuntimeError):
    """Raised when the CC10 assurance profile is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise ConfidentialKeyParticipantLifecycleAssuranceError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise ConfidentialKeyParticipantLifecycleAssuranceError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a non-empty list or raise a validation error."""
    if not isinstance(value, list) or not value:
        raise ConfidentialKeyParticipantLifecycleAssuranceError(f"{label} must be a non-empty list")
    return value


def require_string_list(value: Any, label: str) -> list[str]:
    """Require a non-empty list of unique strings."""
    values = require_list(value, label)
    if not all(isinstance(item, str) and item for item in values):
        raise ConfidentialKeyParticipantLifecycleAssuranceError(
            f"{label} must contain only non-empty strings",
        )
    if len(values) != len(set(values)):
        raise ConfidentialKeyParticipantLifecycleAssuranceError(f"{label} must not contain duplicates")
    return values


def require_path(value: Any, label: str) -> Path:
    """Require a repository path to exist."""
    if not isinstance(value, str) or not value:
        raise ConfidentialKeyParticipantLifecycleAssuranceError(f"{label} must be a non-empty path")
    path = Path(value)
    if not path.exists():
        raise ConfidentialKeyParticipantLifecycleAssuranceError(f"{label} does not exist: {path}")
    return path


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality."""
    if actual != expected:
        raise ConfidentialKeyParticipantLifecycleAssuranceError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def validate_participant_model(profile: dict[str, Any]) -> None:
    """Validate participant fields, statuses, roles, and metadata exclusions."""
    model = require_mapping(profile.get("participant_model"), "participant_model")
    fields = set(require_string_list(model.get("required_fields"), "participant_model.required_fields"))
    missing_fields = sorted(REQUIRED_PARTICIPANT_FIELDS - fields)
    if missing_fields:
        raise ConfidentialKeyParticipantLifecycleAssuranceError(
            "participant_model.required_fields is missing: " + ", ".join(missing_fields),
        )
    statuses = set(require_string_list(model.get("status_enum"), "participant_model.status_enum"))
    require_equal(statuses, EXPECTED_STATUSES, "participant_model.status_enum")
    roles = set(require_string_list(model.get("mechanism_roles"), "participant_model.mechanism_roles"))
    require_equal(roles, EXPECTED_ROLES, "participant_model.mechanism_roles")
    active_operations = set(
        require_string_list(
            model.get("active_only_operations"),
            "participant_model.active_only_operations",
        ),
    )
    for operation in {
        "confidential-job-registration",
        "additive-share-signing-key-registration",
        "additive-share-submission",
    }:
        if operation not in active_operations:
            raise ConfidentialKeyParticipantLifecycleAssuranceError(
                f"active_only_operations is missing {operation}",
            )
    forbidden = set(
        require_string_list(
            model.get("metadata_only_forbidden_fields"),
            "participant_model.metadata_only_forbidden_fields",
        ),
    )
    missing_forbidden = sorted(FORBIDDEN_METADATA_FIELDS - forbidden)
    if missing_forbidden:
        raise ConfidentialKeyParticipantLifecycleAssuranceError(
            "metadata_only_forbidden_fields is missing: " + ", ".join(missing_forbidden),
        )


def validate_key_ceremonies(profile: dict[str, Any]) -> None:
    """Validate key ceremony non-claim boundaries."""
    ceremonies = require_mapping(profile.get("key_ceremony_model"), "key_ceremony_model")
    additive = require_mapping(ceremonies.get("additive_signing_keys"), "key_ceremony_model.additive_signing_keys")
    require_equal(additive.get("active_participant_required"), True, "additive_signing_keys.active_participant_required")
    for key, claim_field in {
        "psi_tokens_oprf_keys": "production_cryptographic_ceremony_claimed",
        "shamir_threshold_shares": "production_dkg_claimed",
        "tee_trust_anchors": "vendor_quote_verification_claimed",
        "fhe_keys": "production_fhe_key_management_claimed",
    }.items():
        section = require_mapping(ceremonies.get(key), f"key_ceremony_model.{key}")
        require_equal(section.get(claim_field), False, f"key_ceremony_model.{key}.{claim_field}")


def validate_api_contract(profile: dict[str, Any]) -> None:
    """Validate API endpoint declarations against OpenAPI."""
    api = require_mapping(profile.get("api_contract"), "api_contract")
    openapi_path = require_path(api.get("openapi"), "api_contract.openapi")
    openapi = load_yaml(openapi_path)
    paths = require_mapping(openapi.get("paths"), "openapi.paths")
    declared: dict[str, set[str]] = {}
    for endpoint_value in require_list(api.get("endpoints"), "api_contract.endpoints"):
        endpoint = require_mapping(endpoint_value, "api_contract.endpoints[]")
        path = str(endpoint.get("path"))
        methods = set(require_string_list(endpoint.get("methods"), f"{path}.methods"))
        declared[path] = methods
        path_item = require_mapping(paths.get(path), f"openapi.paths.{path}")
        missing = sorted(methods - set(path_item))
        if missing:
            raise ConfidentialKeyParticipantLifecycleAssuranceError(
                f"{path} is missing OpenAPI methods: " + ", ".join(missing),
            )
        scopes = require_mapping(endpoint.get("scopes"), f"{path}.scopes")
        for method in methods:
            operation = require_mapping(path_item.get(method), f"{path}.{method}")
            expected_scope = scopes.get(method)
            if operation.get("x-kiva-required-scopes") != expected_scope:
                raise ConfidentialKeyParticipantLifecycleAssuranceError(
                    f"{path}.{method}.x-kiva-required-scopes must be {expected_scope!r}",
                )
    require_equal(declared, EXPECTED_ENDPOINTS, "api_contract.endpoints")


def validate_persistence(profile: dict[str, Any]) -> None:
    """Validate persistence artifacts and table declarations."""
    persistence = require_mapping(profile.get("persistence"), "persistence")
    migration = require_path(persistence.get("migration"), "persistence.migration")
    schema = require_path(persistence.get("schema"), "persistence.schema")
    require_path(persistence.get("atlas_sum"), "persistence.atlas_sum")
    tables = set(require_string_list(persistence.get("tables"), "persistence.tables"))
    require_equal(
        tables,
        {"kiva.confidential_participants", "kiva.confidential_participant_audit_events"},
        "persistence.tables",
    )
    combined_sql = migration.read_text(encoding="utf-8") + "\n" + schema.read_text(encoding="utf-8")
    for table in tables:
        if table not in combined_sql:
            raise ConfidentialKeyParticipantLifecycleAssuranceError(
                f"{table} is missing from SQL artifacts",
            )
    boundaries = require_mapping(
        persistence.get("transaction_boundaries"),
        "persistence.transaction_boundaries",
    )
    require_equal(
        boundaries.get("transition_participant_and_audit_event"),
        "select-for-update-single-postgresql-transaction",
        "persistence.transaction_boundaries.transition_participant_and_audit_event",
    )
    require_equal(
        boundaries.get("active_participant_gate"),
        "fail-closed-before-key-or-job-write",
        "persistence.transaction_boundaries.active_participant_gate",
    )


def validate_lifecycle_controls(profile: dict[str, Any]) -> None:
    """Validate lifecycle control declarations."""
    controls = require_mapping(profile.get("lifecycle_controls"), "lifecycle_controls")
    onboarding = require_mapping(controls.get("onboarding"), "lifecycle_controls.onboarding")
    require_equal(onboarding.get("initial_status"), "invited", "lifecycle_controls.onboarding.initial_status")
    require_equal(
        onboarding.get("activation_required_for_jobs_and_keys"),
        True,
        "lifecycle_controls.onboarding.activation_required_for_jobs_and_keys",
    )
    for section_name in ("suspension", "emergency_freeze"):
        section = require_mapping(controls.get(section_name), f"lifecycle_controls.{section_name}")
        require_equal(section.get("blocks_new_jobs"), True, f"lifecycle_controls.{section_name}.blocks_new_jobs")
        require_equal(section.get("blocks_new_keys"), True, f"lifecycle_controls.{section_name}.blocks_new_keys")
    removal = require_mapping(controls.get("removal"), "lifecycle_controls.removal")
    require_equal(removal.get("terminal_for_new_work"), True, "lifecycle_controls.removal.terminal_for_new_work")
    require_mapping(controls.get("replacement"), "lifecycle_controls.replacement")


def validate_verification(profile: dict[str, Any]) -> None:
    """Validate verification artifact references."""
    verification = require_mapping(profile.get("verification"), "verification")
    for key in ("validators", "docs", "tamarin_models"):
        for path_value in require_string_list(verification.get(key), f"verification.{key}"):
            require_path(path_value, f"verification.{key}[{path_value}]")
    kani = require_string_list(verification.get("kani_harnesses"), "verification.kani_harnesses")
    require_equal(
        set(kani),
        set(registry_string_list("confidential-key-participant-lifecycle", "kani_harnesses")),
        "verification.kani_harnesses registry match",
    )
    missing_harnesses = missing_kani_harnesses("confidential-key-participant-lifecycle")
    if missing_harnesses:
        raise ConfidentialKeyParticipantLifecycleAssuranceError(
            "Kani runner does not reference: " + ", ".join(missing_harnesses),
        )
    rust_tests = require_string_list(verification.get("rust_tests"), "verification.rust_tests")
    require_equal(
        set(rust_tests),
        set(registry_string_list("confidential-key-participant-lifecycle", "rust_tests")),
        "verification.rust_tests registry match",
    )
    source_text = "\n".join((confidential_rust_source(), server_rust_source()))
    missing_tests = missing_rust_tests("confidential-key-participant-lifecycle", source_text)
    if missing_tests:
        raise ConfidentialKeyParticipantLifecycleAssuranceError(
            "Rust test is missing: " + ", ".join(missing_tests),
        )


def validate_non_claims(profile: dict[str, Any]) -> None:
    """Validate explicit non-claim language."""
    non_claims = require_mapping(profile.get("non_claims"), "non_claims")
    for key, value in non_claims.items():
        if value is not False:
            raise ConfidentialKeyParticipantLifecycleAssuranceError(
                f"non_claims.{key} must be false",
            )


def validate_profile(profile: dict[str, Any]) -> None:
    """Validate the complete CC10 assurance profile."""
    require_equal(profile.get("schema_version"), 1, "schema_version")
    require_equal(profile.get("kind"), "confidential-key-participant-lifecycle-assurance", "kind")
    require_equal(profile.get("status"), "published", "status")
    require_equal(profile.get("milestone"), "CC10", "milestone")
    validate_participant_model(profile)
    validate_key_ceremonies(profile)
    validate_api_contract(profile)
    validate_persistence(profile)
    validate_lifecycle_controls(profile)
    validate_verification(profile)
    validate_non_claims(profile)


def main() -> int:
    """Run the CC10 validator."""
    parser = argparse.ArgumentParser()
    parser.add_argument("profile", nargs="?", type=Path, default=DEFAULT_PROFILE)
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    validate_profile(load_yaml(args.profile))
    if args.json_report:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "kind": "confidential-key-participant-lifecycle-assurance-validation",
                    "status": "passed",
                    "profile": str(args.profile),
                },
                indent=2,
                sort_keys=True,
            )
            + "\n",
            encoding="utf-8",
        )
    print(f"validated {args.profile}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
