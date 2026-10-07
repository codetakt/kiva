#!/usr/bin/env python3
"""Validate Kiva's CC9 confidential job lifecycle assurance profile."""

from __future__ import annotations

import argparse
import json
import re
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


DEFAULT_PROFILE = Path("spec/confidential/confidential-job-lifecycle-assurance.yaml")
EXPECTED_STATUSES = {
    "draft",
    "admitted",
    "running",
    "awaitingRelease",
    "released",
    "failed",
    "cancelled",
    "expired",
    "retired",
}
EXPECTED_ENDPOINTS = {
    "/confidential/jobs": {"get", "post"},
    "/confidential/job-transitions": {"post"},
    "/confidential/job-audit-events": {"get"},
}
EXPECTED_MECHANISMS = {
    "additive-secure-aggregation",
    "differential-privacy-release",
    "shamir-vss-admission",
    "threshold-frost-signing",
    "psi-cohort-hash-token",
    "psi-oprf-dh-voprf",
    "tee-portable-attestation",
    "tee-aws-nitro-attestation",
    "fhe-sum-feasibility",
    "fhe-sum-runtime",
}
REQUIRED_JOB_FIELDS = {
    "tenant_id",
    "job_id",
    "owner_subject",
    "mechanism",
    "mechanism_profile",
    "workload_id",
    "participant_subjects",
    "policy_binding_id",
    "status",
    "retry_count",
    "max_retry_count",
    "deadline_at",
    "admission_quota_key",
    "evidence_refs",
    "created_at",
    "updated_at",
}
FORBIDDEN_METADATA_FIELDS = {
    "rawInput",
    "rawInputs",
    "share",
    "signedEnvelope",
    "plaintext",
    "secret",
    "signature",
    "noise",
    "noiseValue",
}


class ConfidentialJobLifecycleAssuranceError(RuntimeError):
    """Raised when the CC9 assurance profile is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise ConfidentialJobLifecycleAssuranceError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise ConfidentialJobLifecycleAssuranceError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a non-empty list or raise a validation error."""
    if not isinstance(value, list) or not value:
        raise ConfidentialJobLifecycleAssuranceError(f"{label} must be a non-empty list")
    return value


def require_string_list(value: Any, label: str) -> list[str]:
    """Require a non-empty list of unique strings."""
    values = require_list(value, label)
    if not all(isinstance(item, str) and item for item in values):
        raise ConfidentialJobLifecycleAssuranceError(
            f"{label} must contain only non-empty strings",
        )
    if len(values) != len(set(values)):
        raise ConfidentialJobLifecycleAssuranceError(f"{label} must not contain duplicates")
    return values


def require_path(value: Any, label: str) -> Path:
    """Require a repository path to exist."""
    if not isinstance(value, str) or not value:
        raise ConfidentialJobLifecycleAssuranceError(f"{label} must be a non-empty path")
    path = Path(value)
    if not path.exists():
        raise ConfidentialJobLifecycleAssuranceError(f"{label} does not exist: {path}")
    return path


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality."""
    if actual != expected:
        raise ConfidentialJobLifecycleAssuranceError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def openapi_confidential_job_mechanisms(openapi: dict[str, Any]) -> set[str]:
    """Return the ConfidentialJobMechanism enum values from OpenAPI."""
    components = require_mapping(openapi.get("components"), "openapi.components")
    schemas = require_mapping(components.get("schemas"), "openapi.components.schemas")
    schema = require_mapping(
        schemas.get("ConfidentialJobMechanism"),
        "openapi.components.schemas.ConfidentialJobMechanism",
    )
    return set(require_string_list(schema.get("enum"), "ConfidentialJobMechanism.enum"))


def sql_confidential_job_mechanisms(sql: str, label: str) -> set[str]:
    """Return the SQL CHECK values accepted by confidential_jobs.mechanism."""
    table_start = sql.find("CREATE TABLE IF NOT EXISTS kiva.confidential_jobs")
    if table_start < 0:
        raise ConfidentialJobLifecycleAssuranceError(
            f"{label} is missing kiva.confidential_jobs table",
        )
    table_sql = sql[table_start:]
    match = re.search(r"mechanism\s+IN\s*\((?P<body>.*?)\)", table_sql, re.DOTALL)
    if match is None:
        raise ConfidentialJobLifecycleAssuranceError(
            f"{label} is missing confidential_jobs.mechanism CHECK",
        )
    values = set(re.findall(r"'([^']+)'", match.group("body")))
    if not values:
        raise ConfidentialJobLifecycleAssuranceError(
            f"{label} confidential_jobs.mechanism CHECK has no values",
        )
    return values


def validate_job_model(profile: dict[str, Any]) -> None:
    """Validate model fields, statuses, and metadata-only exclusions."""
    model = require_mapping(profile.get("job_model"), "job_model")
    fields = set(require_string_list(model.get("required_fields"), "job_model.required_fields"))
    missing_fields = sorted(REQUIRED_JOB_FIELDS - fields)
    if missing_fields:
        raise ConfidentialJobLifecycleAssuranceError(
            "job_model.required_fields is missing: " + ", ".join(missing_fields),
        )
    statuses = set(require_string_list(model.get("status_enum"), "job_model.status_enum"))
    require_equal(statuses, EXPECTED_STATUSES, "job_model.status_enum")
    mechanisms = set(require_string_list(model.get("mechanisms"), "job_model.mechanisms"))
    require_equal(mechanisms, EXPECTED_MECHANISMS, "job_model.mechanisms")
    forbidden = set(
        require_string_list(
            model.get("metadata_only_forbidden_fields"),
            "job_model.metadata_only_forbidden_fields",
        ),
    )
    missing_forbidden = sorted(FORBIDDEN_METADATA_FIELDS - forbidden)
    if missing_forbidden:
        raise ConfidentialJobLifecycleAssuranceError(
            "metadata_only_forbidden_fields is missing: " + ", ".join(missing_forbidden),
        )


def validate_api_contract(profile: dict[str, Any]) -> None:
    """Validate API endpoint declarations against OpenAPI."""
    api = require_mapping(profile.get("api_contract"), "api_contract")
    openapi_path = require_path(api.get("openapi"), "api_contract.openapi")
    openapi = load_yaml(openapi_path)
    require_equal(
        openapi_confidential_job_mechanisms(openapi),
        EXPECTED_MECHANISMS,
        "openapi.ConfidentialJobMechanism.enum",
    )
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
            raise ConfidentialJobLifecycleAssuranceError(
                f"{path} is missing OpenAPI methods: " + ", ".join(missing),
            )
        scopes = require_mapping(endpoint.get("scopes"), f"{path}.scopes")
        for method in methods:
            operation = require_mapping(path_item.get(method), f"{path}.{method}")
            expected_scope = scopes.get(method)
            if operation.get("x-kiva-required-scopes") != expected_scope:
                raise ConfidentialJobLifecycleAssuranceError(
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
        {"kiva.confidential_jobs", "kiva.confidential_job_audit_events"},
        "persistence.tables",
    )
    combined_sql = migration.read_text(encoding="utf-8") + "\n" + schema.read_text(encoding="utf-8")
    for table in tables:
        if table not in combined_sql:
            raise ConfidentialJobLifecycleAssuranceError(f"{table} is missing from SQL artifacts")
    for path in (migration, schema):
        require_equal(
            sql_confidential_job_mechanisms(path.read_text(encoding="utf-8"), str(path)),
            EXPECTED_MECHANISMS,
            f"{path}.confidential_jobs.mechanism CHECK",
        )
    transaction_boundaries = require_mapping(
        persistence.get("transaction_boundaries"),
        "persistence.transaction_boundaries",
    )
    require_equal(
        transaction_boundaries.get("transition_job_and_audit_event"),
        "select-for-update-single-postgresql-transaction",
        "persistence.transaction_boundaries.transition_job_and_audit_event",
    )


def validate_scheduler(profile: dict[str, Any]) -> None:
    """Validate bounded scheduler/admission profile."""
    scheduler = require_mapping(profile.get("scheduler"), "scheduler")
    quota = require_mapping(scheduler.get("admission_quota"), "scheduler.admission_quota")
    require_equal(quota.get("field"), "admission_quota_key", "scheduler.admission_quota.field")
    if int(quota.get("max_active_per_quota_key", 0)) != 64:
        raise ConfidentialJobLifecycleAssuranceError("max_active_per_quota_key must be 64")
    require_equal(
        set(require_string_list(quota.get("active_statuses"), "scheduler.admission_quota.active_statuses")),
        {"admitted", "running", "awaitingRelease"},
        "scheduler.admission_quota.active_statuses",
    )
    concurrency = require_mapping(
        scheduler.get("bounded_concurrency"),
        "scheduler.bounded_concurrency",
    )
    if int(concurrency.get("max_running_per_tenant", 0)) != 16:
        raise ConfidentialJobLifecycleAssuranceError("max_running_per_tenant must be 16")
    require_mapping(scheduler.get("timeout_handling"), "scheduler.timeout_handling")
    require_mapping(scheduler.get("backpressure"), "scheduler.backpressure")


def validate_verification(profile: dict[str, Any]) -> None:
    """Validate verification artifact references."""
    verification = require_mapping(profile.get("verification"), "verification")
    for key in ("validators", "docs", "tamarin_models"):
        for path_value in require_string_list(verification.get(key), f"verification.{key}"):
            require_path(path_value, f"verification.{key}[{path_value}]")
    kani = require_string_list(verification.get("kani_harnesses"), "verification.kani_harnesses")
    require_equal(
        set(kani),
        set(registry_string_list("confidential-job-lifecycle", "kani_harnesses")),
        "verification.kani_harnesses registry match",
    )
    missing_harnesses = missing_kani_harnesses("confidential-job-lifecycle")
    if missing_harnesses:
        raise ConfidentialJobLifecycleAssuranceError(
            "Kani runner does not reference: " + ", ".join(missing_harnesses),
        )
    rust_tests = require_string_list(verification.get("rust_tests"), "verification.rust_tests")
    require_equal(
        set(rust_tests),
        set(registry_string_list("confidential-job-lifecycle", "rust_tests")),
        "verification.rust_tests registry match",
    )
    source_text = "\n".join((confidential_rust_source(), server_rust_source()))
    missing_tests = missing_rust_tests("confidential-job-lifecycle", source_text)
    if missing_tests:
        raise ConfidentialJobLifecycleAssuranceError(
            "Rust test is missing: " + ", ".join(missing_tests),
        )


def validate_non_claims(profile: dict[str, Any]) -> None:
    """Validate explicit non-claim language."""
    non_claims = require_mapping(profile.get("non_claims"), "non_claims")
    for key, value in non_claims.items():
        if value is not False:
            raise ConfidentialJobLifecycleAssuranceError(f"non_claims.{key} must be false")


def validate_profile(profile: dict[str, Any]) -> None:
    """Validate the complete CC9 assurance profile."""
    require_equal(profile.get("schema_version"), 1, "schema_version")
    require_equal(profile.get("kind"), "confidential-job-lifecycle-assurance", "kind")
    require_equal(profile.get("status"), "published", "status")
    require_equal(profile.get("milestone"), "CC9", "milestone")
    validate_job_model(profile)
    validate_api_contract(profile)
    validate_persistence(profile)
    validate_scheduler(profile)
    validate_verification(profile)
    validate_non_claims(profile)


def main() -> int:
    """Run the CC9 validator."""
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
                    "kind": "confidential-job-lifecycle-assurance-validation",
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
