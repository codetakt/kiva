#!/usr/bin/env python3
"""Validate Kiva differential privacy assurance profile and sampler evidence."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

import yaml


DEFAULT_PROFILE = Path("spec/confidential/differential-privacy-assurance.yaml")
DEFAULT_CONTRACT = Path("spec/openapi/kiva-confidential.yaml")
MICROUNITS_PER_UNIT = 1_000_000
U64_MASK = (1 << 64) - 1


class DifferentialPrivacyAssuranceError(RuntimeError):
    """Raised when the differential privacy assurance profile is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise DifferentialPrivacyAssuranceError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise DifferentialPrivacyAssuranceError(f"{label} must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a list or raise a validation error."""
    if not isinstance(value, list):
        raise DifferentialPrivacyAssuranceError(f"{label} must be a list")
    return value


def require_equal(actual: Any, expected: Any, label: str) -> None:
    """Require exact equality with a stable error message."""
    if actual != expected:
        raise DifferentialPrivacyAssuranceError(
            f"{label} must be {expected!r}, got {actual!r}",
        )


def splitmix64(seed: int) -> int:
    """Return one SplitMix64 output word for deterministic sampler evidence."""
    value = (seed + 0x9E37_79B9_7F4A_7C15) & U64_MASK
    value = ((value ^ (value >> 30)) * 0xBF58_476D_1CE4_E5B9) & U64_MASK
    value = ((value ^ (value >> 27)) * 0x94D0_49BB_1331_11EB) & U64_MASK
    return (value ^ (value >> 31)) & U64_MASK


def uniform_open_closed_unit(random_word: int) -> float:
    """Mirror the Rust u64-to-f64 open-closed unit interval adapter."""
    mantissa_bits = 53
    denominator = 9_007_199_254_740_992.0
    mantissa = (random_word >> (64 - mantissa_bits)) + 1
    return mantissa / denominator


def sample_geometric_magnitude(alpha: float, random_word: int) -> int:
    """Mirror the Rust geometric magnitude sampler."""
    if alpha <= 0.0:
        return 0
    if not math.isfinite(alpha) or alpha >= 1.0:
        raise DifferentialPrivacyAssuranceError("sampler alpha is outside the supported range")
    uniform = uniform_open_closed_unit(random_word)
    magnitude = math.floor(math.log(uniform) / math.log(alpha))
    if not math.isfinite(magnitude) or magnitude >= 9_223_372_036_854_775_808.0:
        raise DifferentialPrivacyAssuranceError("sampled magnitude exceeded i64 bounds")
    return int(magnitude)


def sample_noise(epsilon_microunits: int, sensitivity: int, left: int, right: int) -> int:
    """Sample deterministic two-sided geometric noise from two random words."""
    epsilon = epsilon_microunits / MICROUNITS_PER_UNIT
    alpha = math.exp(-epsilon / sensitivity)
    return sample_geometric_magnitude(alpha, left) - sample_geometric_magnitude(alpha, right)


def validate_sampler(profile: dict[str, Any]) -> dict[str, Any]:
    """Validate deterministic sampler distribution evidence."""
    validation = require_mapping(profile.get("statistical_validation"), "statistical_validation")
    sample_count = int(validation.get("sample_count"))
    epsilon_microunits = int(validation.get("epsilon_microunits"))
    sensitivity = int(validation.get("l1_sensitivity"))
    seed = int(validation.get("splitmix_seed"))
    zero_tolerance = float(validation.get("zero_probability_tolerance"))
    mean_tolerance = float(validation.get("mean_absolute_tolerance"))

    if sample_count < 1024:
        raise DifferentialPrivacyAssuranceError("statistical_validation.sample_count is too small")
    if epsilon_microunits <= 0:
        raise DifferentialPrivacyAssuranceError("statistical_validation.epsilon_microunits must be positive")
    if sensitivity <= 0:
        raise DifferentialPrivacyAssuranceError("statistical_validation.l1_sensitivity must be positive")

    alpha = math.exp(-(epsilon_microunits / MICROUNITS_PER_UNIT) / sensitivity)
    expected_zero_probability = (1.0 - alpha) / (1.0 + alpha)
    expected_mean_absolute = 2.0 * alpha / (1.0 - alpha * alpha)

    zero_count = 0
    absolute_sum = 0
    for offset in range(sample_count):
        left = splitmix64(seed + offset)
        right = splitmix64(seed + offset ^ 0x9E37_79B9_7F4A_7C15)
        noise = sample_noise(epsilon_microunits, sensitivity, left, right)
        if noise == 0:
            zero_count += 1
        absolute_sum += abs(noise)

    observed_zero_probability = zero_count / sample_count
    observed_mean_absolute = absolute_sum / sample_count
    zero_error = abs(observed_zero_probability - expected_zero_probability)
    mean_error = abs(observed_mean_absolute - expected_mean_absolute)
    if zero_error > zero_tolerance:
        raise DifferentialPrivacyAssuranceError(
            "sampler zero-probability smoke check failed: "
            f"observed={observed_zero_probability:.6f}, "
            f"expected={expected_zero_probability:.6f}, tolerance={zero_tolerance}",
        )
    if mean_error > mean_tolerance:
        raise DifferentialPrivacyAssuranceError(
            "sampler mean-absolute smoke check failed: "
            f"observed={observed_mean_absolute:.6f}, "
            f"expected={expected_mean_absolute:.6f}, tolerance={mean_tolerance}",
        )

    return {
        "sample_count": sample_count,
        "expected_zero_probability": expected_zero_probability,
        "observed_zero_probability": observed_zero_probability,
        "zero_probability_error": zero_error,
        "expected_mean_absolute": expected_mean_absolute,
        "observed_mean_absolute": observed_mean_absolute,
        "mean_absolute_error": mean_error,
    }


def validate_lifecycle(profile: dict[str, Any]) -> None:
    """Validate DP budget policy lifecycle declarations."""
    lifecycle = require_mapping(profile.get("budget_policy_lifecycle"), "budget_policy_lifecycle")
    statuses = require_list(lifecycle.get("statuses"), "budget_policy_lifecycle.statuses")
    require_equal(statuses, ["draft", "active", "superseded", "retired"], "budget policy statuses")
    transitions = require_mapping(
        lifecycle.get("allowed_transitions"),
        "budget_policy_lifecycle.allowed_transitions",
    )
    expected = {
        "draft": ["draft", "active", "retired"],
        "active": ["active", "superseded", "retired"],
        "superseded": ["superseded", "retired"],
        "retired": ["retired"],
    }
    require_equal(transitions, expected, "budget policy allowed transitions")


def validate_profile(profile: dict[str, Any]) -> dict[str, Any]:
    """Validate profile invariants and return a report payload."""
    require_equal(profile.get("schema_version"), 1, "schema_version")
    require_equal(profile.get("kind"), "differential-privacy-assurance-profile", "kind")
    require_equal(profile.get("status"), "published", "status")

    mechanism = require_mapping(profile.get("mechanism"), "mechanism")
    require_equal(mechanism.get("label"), "discreteLaplace", "mechanism.label")
    require_equal(mechanism.get("distribution"), "two-sided-geometric", "mechanism.distribution")
    require_equal(mechanism.get("adjacency_model"), "addRemoveOneSubject", "mechanism.adjacency_model")
    require_equal(mechanism.get("accounting_scope"), "subjectBudget", "mechanism.accounting_scope")
    require_equal(mechanism.get("random_source"), "os-csprng", "mechanism.random_source")

    release_gate = require_mapping(profile.get("release_gate"), "release_gate")
    for key in (
        "requires_persisted_aggregate",
        "requires_nonzero_epsilon",
        "requires_epsilon_within_subject_budget",
        "requires_applied_noise",
        "hides_raw_aggregate",
        "hides_noise_sample",
    ):
        require_equal(release_gate.get(key), True, f"release_gate.{key}")
    minimum_population = release_gate.get("requires_minimum_population_at_least")
    if not isinstance(minimum_population, int) or minimum_population < 2:
        raise DifferentialPrivacyAssuranceError(
            "release_gate.requires_minimum_population_at_least must be at least 2",
        )

    composition = require_mapping(profile.get("composition"), "composition")
    require_equal(composition.get("enforced_rule"), "basicSequential", "composition.enforced_rule")
    require_equal(composition.get("advanced_rule"), "drvPureDp", "composition.advanced_rule")
    require_equal(composition.get("advanced_delta_microunits"), 1, "composition.advanced_delta_microunits")
    require_equal(
        composition.get("scope_dimensions"),
        ["subjectBudget", "release", "timeWindow"],
        "composition.scope_dimensions",
    )
    windows = require_list(composition.get("release_windows"), "composition.release_windows")
    if not windows:
        raise DifferentialPrivacyAssuranceError("composition.release_windows must not be empty")
    for window in windows:
        window_map = require_mapping(window, "composition.release_windows[]")
        if int(window_map.get("max_days", 0)) <= 0:
            raise DifferentialPrivacyAssuranceError("composition release window max_days must be positive")
        require_equal(window_map.get("lifecycle"), "active", "composition release window lifecycle")

    validate_lifecycle(profile)
    sampler_report = validate_sampler(profile)

    evidence = require_mapping(profile.get("evidence"), "evidence")
    missing = sorted(path for path in evidence.values() if isinstance(path, str) and not Path(path).exists())
    if missing:
        raise DifferentialPrivacyAssuranceError(f"evidence files are missing: {', '.join(missing)}")

    return {
        "schema_version": 1,
        "kind": "differential-privacy-assurance-validation",
        "status": "passed",
        "profile": str(DEFAULT_PROFILE),
        "mechanism": mechanism["id"],
        "adjacency_model": mechanism["adjacency_model"],
        "accounting_scope": mechanism["accounting_scope"],
        "enforced_composition_rule": composition["enforced_rule"],
        "advanced_composition_rule": composition["advanced_rule"],
        "advanced_delta_microunits": composition["advanced_delta_microunits"],
        "sampler": sampler_report,
    }


def validate_openapi_projection(contract: dict[str, Any]) -> None:
    """Validate DP assurance fields projected into the confidential OpenAPI contract."""
    schemas = require_mapping(require_mapping(contract.get("components"), "components").get("schemas"), "schemas")
    release_metadata = require_mapping(
        schemas.get("DifferentialPrivacyReleaseMetadata"),
        "DifferentialPrivacyReleaseMetadata",
    )
    release_required = set(require_list(release_metadata.get("required"), "DifferentialPrivacyReleaseMetadata.required"))
    if "adjacencyModel" not in release_required:
        raise DifferentialPrivacyAssuranceError("DifferentialPrivacyReleaseMetadata must require adjacencyModel")
    release_properties = require_mapping(
        release_metadata.get("properties"),
        "DifferentialPrivacyReleaseMetadata.properties",
    )
    adjacency = require_mapping(release_properties.get("adjacencyModel"), "adjacencyModel")
    require_equal(adjacency.get("enum"), ["addRemoveOneSubject"], "adjacencyModel.enum")

    summary = require_mapping(
        schemas.get("DifferentialPrivacyCompositionSummary"),
        "DifferentialPrivacyCompositionSummary",
    )
    summary_required = set(require_list(summary.get("required"), "DifferentialPrivacyCompositionSummary.required"))
    for field in (
        "advancedCompositionRule",
        "advancedCompositionDeltaMicrounits",
        "advancedCompositionEpsilonMicrounits",
        "releaseEpsilonSquareSumMicrounits2",
    ):
        if field not in summary_required:
            raise DifferentialPrivacyAssuranceError(
                f"DifferentialPrivacyCompositionSummary must require {field}",
            )


def main() -> int:
    """Run the differential privacy assurance validator."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", type=Path, default=DEFAULT_PROFILE)
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    profile = load_yaml(args.profile)
    report = validate_profile(profile)
    validate_openapi_projection(load_yaml(args.contract))
    report["profile"] = str(args.profile)
    report["contract"] = str(args.contract)

    if args.json_report is not None:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    print(
        "validated differential privacy assurance profile: "
        f"{args.profile}, mechanism={report['mechanism']}",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
