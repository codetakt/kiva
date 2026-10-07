"""Shared helpers for Kiva CI evidence-check manifests."""

from __future__ import annotations

from pathlib import Path
from typing import Any, Iterable, TypeVar

from kiva_ci.manifest import (
    ManifestError,
    load_yaml,
    relative_path,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
    validation_error,
)


EVIDENCE_CHECK_MANIFEST_KIND = "kiva-ci-evidence-check-manifest"
EVIDENCE_CHECK_KINDS = {
    "contract-validator",
    "evidence-validator",
    "generator-check",
    "ledger-validator",
    "schema-artifact-check",
}
EVIDENCE_ARTIFACT_CLASSES = {
    "architecture-guard",
    "claim-catalog",
    "confidential-assurance",
    "confidential-mechanism",
    "contract",
    "dto-boundary",
    "generated-catalog",
    "generated-evidence",
    "generated-ledger",
    "mechanism-maturity",
    "provider-contract",
    "schema-artifact",
}

ErrorT = TypeVar("ErrorT", bound=RuntimeError)


def _require_bool(value: Any, label: str, error_cls: type[ErrorT]) -> bool:
    """Return a boolean manifest field."""
    if not isinstance(value, bool):
        raise validation_error(error_cls, f"{label} must be a boolean")
    return value


def evidence_check_input_model(
    check: dict[str, Any],
    projection_inputs: Iterable[str],
    label: str,
    error_cls: type[ErrorT],
) -> dict[str, Any]:
    """Return normalized explicit and projection-derived input metadata."""
    inputs = require_string_list(
        check.get("inputs"),
        f"{label}.inputs",
        error_cls,
        allow_empty=True,
    )
    derived_input_policy = check.get("derived_input_policy", "explicit-inputs")
    if not isinstance(derived_input_policy, str) or derived_input_policy not in {
        "explicit-inputs",
        "projection-graph",
    }:
        raise validation_error(
            error_cls,
            f"{label}.derived_input_policy must be explicit-inputs or projection-graph",
        )
    if not inputs and derived_input_policy != "projection-graph":
        raise validation_error(
            error_cls,
            f"{label}.inputs must not be empty unless derived_input_policy is projection-graph",
        )
    derived_projection_inputs = sorted(set(projection_inputs))
    return {
        "inputs": inputs,
        "derived_input_policy": derived_input_policy,
        "derived_projection_inputs": derived_projection_inputs,
        "effective_inputs": sorted(set(inputs).union(derived_projection_inputs)),
    }


def evidence_check_effective_inputs(
    check: dict[str, Any],
    projection_inputs: Iterable[str],
    label: str,
    error_cls: type[ErrorT],
    *,
    require_projection_graph_inputs: bool = False,
    explicit_input_must_include: str | None = None,
) -> dict[str, Any]:
    """Return normalized check inputs and enforce derivation-policy invariants."""
    model = evidence_check_input_model(check, projection_inputs, label, error_cls)
    if (
        require_projection_graph_inputs
        and model["derived_input_policy"] == "projection-graph"
        and not model["derived_projection_inputs"]
    ):
        raise validation_error(
            error_cls,
            f"{label}.derived_input_policy requires projection graph inputs",
        )
    if (
        explicit_input_must_include is not None
        and model["derived_input_policy"] == "explicit-inputs"
        and explicit_input_must_include not in set(model["inputs"])
    ):
        raise validation_error(
            error_cls,
            f"{label}.inputs must include {explicit_input_must_include}",
        )
    return model


def _validate_check_metadata(
    check: dict[str, Any],
    label: str,
    error_cls: type[ErrorT],
) -> None:
    """Validate typed CI dependency and execution metadata for one check."""
    evidence_check_input_model(check, [], label, error_cls)
    require_string_list(check.get("outputs", []), f"{label}.outputs", error_cls, allow_empty=True)
    _require_bool(check.get("mutates_repository"), f"{label}.mutates_repository", error_cls)
    _require_bool(check.get("requires_network"), f"{label}.requires_network", error_cls)
    artifact_class = require_string(check.get("artifact_class"), f"{label}.artifact_class", error_cls)
    if artifact_class not in EVIDENCE_ARTIFACT_CLASSES:
        raise validation_error(
            error_cls,
            f"{label}.artifact_class is invalid: {artifact_class}",
        )


def load_evidence_check_manifest(
    path: Path,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Load and validate the CI evidence-check manifest envelope."""
    manifest = load_yaml(path, error_cls)
    if manifest.get("schema_version") != 1:
        raise validation_error(error_cls, f"{path}.schema_version must be 1")
    if manifest.get("kind") != EVIDENCE_CHECK_MANIFEST_KIND:
        raise validation_error(
            error_cls,
            f"{path}.kind must be {EVIDENCE_CHECK_MANIFEST_KIND}",
        )
    return manifest


def _required_group_matrix(
    manifest: dict[str, Any],
    label: str,
    error_cls: type[ErrorT],
) -> dict[str, dict[str, str]]:
    """Return the top-level required CI group metadata matrix."""
    matrix: dict[str, dict[str, str]] = {}
    entries = require_list(manifest.get("required_groups"), f"{label}.required_groups", error_cls)
    if not entries:
        raise validation_error(error_cls, f"{label}.required_groups must not be empty")
    for index, value in enumerate(entries):
        entry = require_mapping(value, f"{label}.required_groups[{index}]", error_cls)
        group_id = require_string(
            entry.get("id"),
            f"{label}.required_groups[{index}].id",
            error_cls,
        )
        if group_id in matrix:
            raise validation_error(error_cls, f"{label} duplicate required group id: {group_id}")
        matrix[group_id] = {
            "nix_gate": require_string(
                entry.get("nix_gate"),
                f"{label}.required_groups[{index}].nix_gate",
                error_cls,
            ),
            "claim_boundary": require_string(
                entry.get("claim_boundary"),
                f"{label}.required_groups[{index}].claim_boundary",
                error_cls,
            ),
        }
    return matrix


def evidence_check_groups(
    manifest: dict[str, Any],
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, dict[str, Any]]:
    """Return CI evidence-check groups keyed by id."""
    groups: dict[str, dict[str, Any]] = {}
    for index, value in enumerate(require_list(manifest.get("groups"), f"{label}.groups", error_cls)):
        group = require_mapping(value, f"{label}.groups[{index}]", error_cls)
        group_id = require_string(group.get("id"), f"{label}.groups[{index}].id", error_cls)
        if group_id in groups:
            raise validation_error(error_cls, f"{label} duplicate CI check group id: {group_id}")
        groups[group_id] = group
    return groups


def evidence_check_ids(
    group: dict[str, Any],
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> set[str]:
    """Return validated check ids for one group."""
    check_ids: set[str] = set()
    for index, value in enumerate(require_list(group.get("checks"), f"{label}.checks", error_cls)):
        check = require_mapping(value, f"{label}.checks[{index}]", error_cls)
        check_id = require_string(check.get("id"), f"{label}.checks[{index}].id", error_cls)
        check_kind = require_string(
            check.get("check_kind"),
            f"{label}.checks[{index}].check_kind",
            error_cls,
        )
        if check_kind not in EVIDENCE_CHECK_KINDS:
            raise validation_error(
                error_cls,
                f"{label}.checks[{index}].check_kind is invalid: {check_kind}",
            )
        _validate_check_metadata(check, f"{label}.checks[{index}]", error_cls)
        if check_id in check_ids:
            raise validation_error(error_cls, f"{label} duplicate check id: {check_id}")
        check_ids.add(check_id)
    return check_ids


def validate_evidence_check_manifest_metadata(
    manifest: dict[str, Any],
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, dict[str, Any]]:
    """Validate group metadata and required-check self-description."""
    required_groups = _required_group_matrix(manifest, label, error_cls)
    groups = evidence_check_groups(manifest, label, error_cls)
    missing_groups = sorted(set(required_groups).difference(groups))
    unexpected_groups = sorted(set(groups).difference(required_groups))
    if missing_groups or unexpected_groups:
        raise validation_error(
            error_cls,
            f"{label} CI group drift; missing={missing_groups}, unexpected={unexpected_groups}",
        )

    for group_id, group in groups.items():
        group_label = f"{label}.groups.{group_id}"
        metadata = required_groups[group_id]
        for field in ("nix_gate", "claim_boundary"):
            value = require_string(group.get(field), f"{group_label}.{field}", error_cls)
            if value != metadata[field]:
                raise validation_error(
                    error_cls,
                    f"{group_label}.{field} drift: group={value} required={metadata[field]}",
                )
        actual_check_ids = evidence_check_ids(group, group_label, error_cls)
        required_check_ids = set(
            require_string_list(
                group.get("required_checks"),
                f"{group_label}.required_checks",
                error_cls,
            )
        )
        if actual_check_ids != required_check_ids:
            missing = sorted(required_check_ids.difference(actual_check_ids))
            unexpected = sorted(actual_check_ids.difference(required_check_ids))
            raise validation_error(
                error_cls,
                f"{group_label} check ids drift; missing={missing}, unexpected={unexpected}",
            )
    return groups


def select_evidence_check_group(
    manifest: dict[str, Any],
    group_id: str,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Return one validated evidence-check group."""
    groups = validate_evidence_check_manifest_metadata(manifest, label, error_cls)
    group = groups.get(group_id)
    if group is None:
        raise validation_error(error_cls, f"{label} evidence-check group is not declared: {group_id}")
    return group


def validate_evidence_check_scripts(
    root: Path,
    group_id: str,
    group: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> list[str]:
    """Validate check scripts, args, and JSON report declarations."""
    checked: list[str] = []
    for index, value in enumerate(require_list(group.get("checks"), f"{group_id}.checks", error_cls)):
        check = require_mapping(value, f"{group_id}.checks[{index}]", error_cls)
        check_id = require_string(check.get("id"), f"{group_id}.checks[{index}].id", error_cls)
        check_kind = require_string(
            check.get("check_kind"),
            f"{group_id}.{check_id}.check_kind",
            error_cls,
        )
        if check_kind not in EVIDENCE_CHECK_KINDS:
            raise validation_error(
                error_cls,
                f"{group_id}.{check_id}.check_kind is invalid: {check_kind}",
            )
        _validate_check_metadata(check, f"{group_id}.{check_id}", error_cls)
        script = root / relative_path(
            require_string(check.get("script"), f"{group_id}.{check_id}.script", error_cls),
            f"{group_id}.{check_id}.script",
            error_cls,
        )
        if not script.exists():
            raise validation_error(error_cls, f"{group_id}.{check_id}.script is missing: {script}")
        require_string_list(check.get("args", []), f"{group_id}.{check_id}.args", error_cls, allow_empty=True)
        if "json_report" in check:
            require_string(check.get("json_report"), f"{group_id}.{check_id}.json_report", error_cls)
        checked.append(f"{group_id}:{check_id}")
    return checked
