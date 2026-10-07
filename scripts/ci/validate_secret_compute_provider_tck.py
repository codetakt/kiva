#!/usr/bin/env python3
"""Validate the secret/confidential-compute provider TCK source ledger."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from kiva_ci.manifest import (
    load_yaml,
    mapping_by_unique_string_key,
    require_bool,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
)


DEFAULT_TCK = Path("spec/confidential/secret-compute-provider-tck.source.yaml")


class SecretComputeProviderTckError(RuntimeError):
    """Raised when the provider TCK source ledger is inconsistent."""


def require_envelope(document: dict[str, Any], path: Path, kind: str) -> None:
    """Validate a simple source-ledger envelope."""
    if document.get("schema_version") != 1:
        raise SecretComputeProviderTckError(f"{path}.schema_version must be 1")
    if document.get("kind") != kind:
        raise SecretComputeProviderTckError(f"{path}.kind must be {kind}")
    if document.get("status") != "active":
        raise SecretComputeProviderTckError(f"{path}.status must be active")


def mechanism_rows(mechanism_catalog: dict[str, Any]) -> dict[str, dict[str, Any]]:
    """Return mechanism-catalog rows keyed by mechanism id."""
    return mapping_by_unique_string_key(
        mechanism_catalog.get("mechanisms"),
        key="id",
        label="mechanism_catalog.mechanisms",
        error_cls=SecretComputeProviderTckError,
    )


def contract_provider_kinds(contract: dict[str, Any]) -> set[str]:
    """Return provider-kind labels declared by the provider contract."""
    variants = require_mapping(
        contract.get("provider_kind_rust_variants"),
        "contract.provider_kind_rust_variants",
        SecretComputeProviderTckError,
    )
    return set(variants)


def runtime_parameter_kinds(contract: dict[str, Any]) -> set[str]:
    """Return provider kinds that require parameter-set evidence."""
    admission_model = require_mapping(
        contract.get("admission_model"),
        "contract.admission_model",
        SecretComputeProviderTckError,
    )
    return set(
        require_string_list(
            admission_model.get("runtime_parameter_set_required_for"),
            "contract.admission_model.runtime_parameter_set_required_for",
            SecretComputeProviderTckError,
        )
    )


def validate_tck_coverage(tck: dict[str, Any], contract: dict[str, Any]) -> None:
    """Validate provider-kind and profile coverage declared by the TCK."""
    coverage = require_mapping(
        tck.get("provider_kind_coverage"),
        "provider_kind_coverage",
        SecretComputeProviderTckError,
    )
    compute_kinds = set(
        require_string_list(
            coverage.get("compute_case_provider_kinds"),
            "provider_kind_coverage.compute_case_provider_kinds",
            SecretComputeProviderTckError,
        )
    )
    metadata_only_kinds = set(
        require_string_list(
            coverage.get("metadata_only_provider_kinds"),
            "provider_kind_coverage.metadata_only_provider_kinds",
            SecretComputeProviderTckError,
        )
    )
    if compute_kinds | metadata_only_kinds != contract_provider_kinds(contract):
        raise SecretComputeProviderTckError(
            "provider_kind_coverage must partition contract provider kinds"
        )
    if compute_kinds & metadata_only_kinds:
        raise SecretComputeProviderTckError(
            "provider_kind_coverage compute and metadata-only kinds must be disjoint"
        )
    expected_profiles = {
        require_string(
            require_mapping(contract.get("reference_provider"), "contract.reference_provider", SecretComputeProviderTckError).get("profile"),
            "contract.reference_provider.profile",
            SecretComputeProviderTckError,
        ),
        require_string(contract.get("profile"), "contract.profile", SecretComputeProviderTckError),
    }
    profiles = set(
        require_string_list(
            coverage.get("required_profiles"),
            "provider_kind_coverage.required_profiles",
            SecretComputeProviderTckError,
        )
    )
    if profiles != expected_profiles:
        raise SecretComputeProviderTckError(
            f"provider_kind_coverage.required_profiles must be {sorted(expected_profiles)}"
        )


def expected_case_result(
    case: dict[str, Any],
    mechanism: dict[str, Any],
    parameter_required_kinds: set[str],
    reference_profile: str,
    external_profile: str,
) -> dict[str, str]:
    """Derive the expected admission result from provider contract rules."""
    provider_kind = require_string(case.get("provider_kind"), "case.provider_kind", SecretComputeProviderTckError)
    capability = require_string(case.get("capability"), "case.capability", SecretComputeProviderTckError)
    profile = require_string(case.get("profile"), "case.profile", SecretComputeProviderTckError)
    parameter_present = require_bool(
        case.get("parameter_set_digest_present"),
        "case.parameter_set_digest_present",
        SecretComputeProviderTckError,
    )
    review_present = require_bool(
        case.get("review_evidence_digest_present"),
        "case.review_evidence_digest_present",
        SecretComputeProviderTckError,
    )
    review_completed = require_bool(
        case.get("external_review_completed"),
        "case.external_review_completed",
        SecretComputeProviderTckError,
    )
    production = require_bool(
        case.get("production_claim_requested"),
        "case.production_claim_requested",
        SecretComputeProviderTckError,
    )
    mechanism_id = require_string(mechanism.get("id"), "mechanism.id", SecretComputeProviderTckError)
    required_capability = require_string(
        mechanism.get("required_provider_capability"),
        f"mechanism.{mechanism_id}.required_provider_capability",
        SecretComputeProviderTckError,
    )
    accepted_capability = capability == required_capability or (
        mechanism_id != "additive-n-of-n" and capability == "reference-fixtures"
    )
    if not accepted_capability:
        return {"expected_result": "rejected", "expected_error": "SecretComputeProviderCapabilityMismatch"}
    if profile == reference_profile:
        if provider_kind != "reference-primitive":
            return {"expected_result": "rejected", "expected_error": "ReferencePrimitiveProviderKindMismatch"}
        if production:
            return {
                "expected_result": "rejected",
                "expected_error": "ReferencePrimitiveProviderProductionClaimRejected",
            }
        return {
            "expected_result": "admitted",
            "expected_evidence_gate": "reference-fixture",
            "expected_release_gate": "fixture-only",
            "expected_runtime_class": "reference",
        }
    if profile != external_profile:
        return {"expected_result": "rejected", "expected_error": "UnknownProviderProfile"}
    if provider_kind in parameter_required_kinds and not parameter_present:
        return {
            "expected_result": "rejected",
            "expected_error": "SecretComputeProviderParameterSetRequired",
        }
    if production and (not review_completed or not review_present):
        return {"expected_result": "rejected", "expected_error": "ExternalProviderReviewRequired"}
    if production:
        return {
            "expected_result": "admitted",
            "expected_evidence_gate": "external-reviewed",
            "expected_release_gate": "production",
            "expected_runtime_class": "external-reviewed-production",
        }
    evidence_gate = "external-reviewed" if review_completed and review_present else "external-non-production"
    return {
        "expected_result": "admitted",
        "expected_evidence_gate": evidence_gate,
        "expected_release_gate": "non-production",
        "expected_runtime_class": "external-non-production",
    }


def validate_case(
    case: dict[str, Any],
    mechanisms: dict[str, dict[str, Any]],
    contract: dict[str, Any],
) -> dict[str, str]:
    """Validate one TCK case and return its derived outcome."""
    case_id = require_string(case.get("id"), "case.id", SecretComputeProviderTckError)
    mechanism_id = require_string(
        case.get("mechanism_catalog_id"),
        f"{case_id}.mechanism_catalog_id",
        SecretComputeProviderTckError,
    )
    mechanism = mechanisms.get(mechanism_id)
    if mechanism is None:
        raise SecretComputeProviderTckError(f"{case_id} references unknown mechanism: {mechanism_id}")
    provider_kind = require_string(case.get("provider_kind"), f"{case_id}.provider_kind", SecretComputeProviderTckError)
    if provider_kind not in contract_provider_kinds(contract):
        raise SecretComputeProviderTckError(f"{case_id} provider_kind is not in provider contract: {provider_kind}")
    reference_profile = require_string(
        require_mapping(contract.get("reference_provider"), "contract.reference_provider", SecretComputeProviderTckError).get("profile"),
        "contract.reference_provider.profile",
        SecretComputeProviderTckError,
    )
    external_profile = require_string(contract.get("profile"), "contract.profile", SecretComputeProviderTckError)
    expected = expected_case_result(
        case,
        mechanism,
        runtime_parameter_kinds(contract),
        reference_profile,
        external_profile,
    )
    for key, expected_value in expected.items():
        actual = require_string(case.get(key), f"{case_id}.{key}", SecretComputeProviderTckError)
        if actual != expected_value:
            raise SecretComputeProviderTckError(
                f"{case_id}.{key} must be {expected_value!r}, got {actual!r}"
            )
    return {"id": case_id, **expected}


def validate_tck(tck: dict[str, Any], tck_path: Path, root: Path = Path(".")) -> dict[str, Any]:
    """Validate the provider TCK source ledger and return a report."""
    require_envelope(tck, tck_path, "kiva-secret-compute-provider-tck-source")
    contract_path = Path(
        require_string(tck.get("provider_contract"), "provider_contract", SecretComputeProviderTckError)
    )
    mechanism_path = Path(
        require_string(tck.get("mechanism_catalog"), "mechanism_catalog", SecretComputeProviderTckError)
    )
    contract = load_yaml(root / contract_path, SecretComputeProviderTckError)
    mechanism_catalog = load_yaml(root / mechanism_path, SecretComputeProviderTckError)
    validate_tck_coverage(tck, contract)
    mechanisms = mechanism_rows(mechanism_catalog)
    cases = [
        validate_case(
            require_mapping(raw_case, f"cases[{index}]", SecretComputeProviderTckError),
            mechanisms,
            contract,
        )
        for index, raw_case in enumerate(
            require_list(tck.get("cases"), "cases", SecretComputeProviderTckError)
        )
    ]
    case_ids = {case["id"] for case in cases}
    required_cases = {
        "reference-additive-fixture-admitted",
        "reference-fhe-fixture-admitted",
        "reference-production-claim-rejected",
        "fhe-reviewed-production-admitted",
        "fhe-production-missing-parameter-rejected",
        "fhe-production-missing-review-rejected",
        "psi-non-production-parameterized-admitted",
        "psi-non-production-missing-parameter-rejected",
        "tee-reviewed-production-admitted",
        "threshold-reviewed-production-admitted",
    }
    missing = sorted(required_cases.difference(case_ids))
    if missing:
        raise SecretComputeProviderTckError(f"TCK missing required case ids: {missing}")
    return {
        "schema_version": 1,
        "kind": "secret-compute-provider-tck-validation",
        "status": "passed",
        "case_count": len(cases),
        "admitted_count": sum(1 for case in cases if case["expected_result"] == "admitted"),
        "rejected_count": sum(1 for case in cases if case["expected_result"] == "rejected"),
        "cases": cases,
    }


def main() -> int:
    """Run the provider TCK validator."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tck", type=Path, default=DEFAULT_TCK)
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    report = validate_tck(load_yaml(args.tck, SecretComputeProviderTckError), args.tck)
    if args.json_report is not None:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
