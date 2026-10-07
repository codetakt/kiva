"""Shared helpers for CI CheckCatalog/GatePlan normal form."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any, Literal, TypeVar

from .manifest import require_list, require_mapping, require_string, validation_error
from .projection import require_envelope


ErrorT = TypeVar("ErrorT", bound=RuntimeError)
CommandFamily = Literal["generate", "validate"]
CI_NORMAL_FORM_SYNC_POLICY = "normal-form-primary-with-evidence-check-compatibility-view"
DEFAULT_CI_ENTRYPOINT = "scripts/ci/kiva-ci"


@dataclass(frozen=True)
class CheckCatalogRow:
    """Typed primary CheckCatalog row."""

    group: str
    check_id: str
    row: dict[str, Any]

    @classmethod
    def from_mapping(cls, check: dict[str, Any], label: str, error_cls: type[ErrorT]) -> "CheckCatalogRow":
        """Parse and validate one CheckCatalog row."""
        return cls(
            group=require_string(check.get("group"), f"{label}.group", error_cls),
            check_id=require_string(check.get("id"), f"{label}.id", error_cls),
            row=check,
        )

    def manifest_check(self, error_cls: type[ErrorT]) -> dict[str, Any]:
        """Return this row as a generated evidence-check manifest row."""
        row = {key: value for key, value in self.row.items() if key != "group"}
        row["ci_command"] = ci_command_for_check(row, f"{self.group}.{self.check_id}", error_cls)
        return row

    def compatibility_check(self) -> dict[str, Any]:
        """Return this row as it appears inside the compatibility source view."""
        return {key: value for key, value in self.row.items() if key != "group"}


@dataclass(frozen=True)
class GatePlanGroup:
    """Typed primary GatePlan group row."""

    group_id: str
    description: str
    nix_gate: str
    claim_boundary: str
    check_order: list[str]

    @classmethod
    def from_mapping(cls, group: dict[str, Any], label: str, error_cls: type[ErrorT]) -> "GatePlanGroup":
        """Parse and validate one GatePlan group row."""
        group_id = require_string(group.get("id"), f"{label}.id", error_cls)
        return cls(
            group_id=group_id,
            description=require_string(group.get("description"), f"{label}.description", error_cls),
            nix_gate=require_string(group.get("nix_gate"), f"{label}.nix_gate", error_cls),
            claim_boundary=require_string(group.get("claim_boundary"), f"{label}.claim_boundary", error_cls),
            check_order=[
                require_string(check_id, f"{label}.check_order[]", error_cls)
                for check_id in require_list(group.get("check_order"), f"{label}.check_order", error_cls)
            ],
        )


def ci_command_family(check_kind: str) -> CommandFamily:
    """Return the `kiva-ci` command family for one check kind."""
    return "generate" if check_kind == "generator-check" else "validate"


def ci_command_for_check(check: dict[str, Any], label: str, error_cls: type[ErrorT]) -> dict[str, str]:
    """Return the primary `kiva-ci` command identity for one check row."""
    check_id = require_string(check.get("id"), f"{label}.id", error_cls)
    check_kind = require_string(check.get("check_kind"), f"{label}.check_kind", error_cls)
    return {
        "entrypoint": DEFAULT_CI_ENTRYPOINT,
        "family": ci_command_family(check_kind),
        "name": check_id,
    }


def validate_check_catalog(
    catalog: dict[str, Any],
    catalog_path: Path,
    error_cls: type[ErrorT],
) -> list[CheckCatalogRow]:
    """Validate and return primary CheckCatalog rows."""
    require_envelope(
        catalog,
        path=catalog_path,
        kind="kiva-ci-evidence-check-catalog-source",
        error_cls=error_cls,
    )
    if catalog.get("synchronization_policy") != CI_NORMAL_FORM_SYNC_POLICY:
        raise validation_error(
            error_cls,
            f"{catalog_path}.synchronization_policy must be {CI_NORMAL_FORM_SYNC_POLICY}",
        )
    rows: list[CheckCatalogRow] = []
    seen: set[tuple[str, str]] = set()
    for index, raw_check in enumerate(require_list(catalog.get("checks"), f"{catalog_path}.checks", error_cls)):
        check = require_mapping(raw_check, f"{catalog_path}.checks[{index}]", error_cls)
        row = CheckCatalogRow.from_mapping(check, f"{catalog_path}.checks[{index}]", error_cls)
        key = (row.group, row.check_id)
        if key in seen:
            raise validation_error(error_cls, f"{catalog_path}.checks duplicates row: {row.group}/{row.check_id}")
        seen.add(key)
        rows.append(row)
    return rows


def validate_gate_plan(
    gate_plan: dict[str, Any],
    gate_plan_path: Path,
    error_cls: type[ErrorT],
) -> list[GatePlanGroup]:
    """Validate and return primary GatePlan groups."""
    require_envelope(
        gate_plan,
        path=gate_plan_path,
        kind="kiva-ci-evidence-gate-plan-source",
        error_cls=error_cls,
    )
    if gate_plan.get("synchronization_policy") != CI_NORMAL_FORM_SYNC_POLICY:
        raise validation_error(
            error_cls,
            f"{gate_plan_path}.synchronization_policy must be {CI_NORMAL_FORM_SYNC_POLICY}",
        )
    groups: list[GatePlanGroup] = []
    seen: set[str] = set()
    for index, raw_group in enumerate(require_list(gate_plan.get("groups"), f"{gate_plan_path}.groups", error_cls)):
        group = require_mapping(raw_group, f"{gate_plan_path}.groups[{index}]", error_cls)
        row = GatePlanGroup.from_mapping(group, f"{gate_plan_path}.groups[{index}]", error_cls)
        if row.group_id in seen:
            raise validation_error(error_cls, f"{gate_plan_path}.groups duplicates id: {row.group_id}")
        seen.add(row.group_id)
        groups.append(row)
    return groups


def checks_by_group(
    checks: list[CheckCatalogRow],
    error_cls: type[ErrorT],
) -> dict[str, dict[str, CheckCatalogRow]]:
    """Return check rows keyed by group and check id."""
    grouped: dict[str, dict[str, CheckCatalogRow]] = {}
    for index, check in enumerate(checks):
        if not check.group or not check.check_id:
            raise validation_error(error_cls, f"checks[{index}] must have group and id")
        grouped.setdefault(check.group, {})[check.check_id] = check
    return grouped


def manifest_groups_from_normal_form(
    checks: list[CheckCatalogRow],
    gate_groups: list[GatePlanGroup],
    error_cls: type[ErrorT],
) -> list[dict[str, Any]]:
    """Project CheckCatalog and GatePlan rows into generated manifest groups."""
    grouped_checks = checks_by_group(checks, error_cls)
    materialized: list[dict[str, Any]] = []
    covered: set[tuple[str, str]] = set()
    for group_index, group in enumerate(gate_groups):
        group_id = group.group_id
        check_order = group.check_order
        group_checks = grouped_checks.get(group_id, {})
        checks_in_order: list[dict[str, Any]] = []
        covered_group: set[str] = set()
        for check_id in check_order:
            check = group_checks.get(check_id)
            if check is None:
                raise validation_error(error_cls, f"GatePlan references missing CheckCatalog row: {group_id}/{check_id}")
            checks_in_order.append(check.manifest_check(error_cls))
            covered.add((group_id, check_id))
            covered_group.add(check_id)
        extra = sorted(set(group_checks).difference(covered_group))
        if extra:
            raise validation_error(
                error_cls,
                f"CheckCatalog rows are absent from GatePlan order for {group_id}: {extra}",
            )
        materialized.append(
            {
                "id": group_id,
                "description": group.description,
                "nix_gate": group.nix_gate,
                "claim_boundary": group.claim_boundary,
                "checks": checks_in_order,
                "required_checks": check_order,
                "required_check_derivation": "generated-from-check-catalog-and-gate-plan",
            }
        )
    all_keys = {(check.group, check.check_id) for check in checks}
    missing = sorted(all_keys.difference(covered))
    if missing:
        raise validation_error(error_cls, f"CheckCatalog rows are not covered by any GatePlan group: {missing}")
    return materialized


def required_groups_from_gate_plan(gate_groups: list[GatePlanGroup], error_cls: type[ErrorT]) -> list[dict[str, str]]:
    """Return generated manifest required_groups from primary GatePlan rows."""
    return [
        {
            "id": group.group_id,
            "nix_gate": group.nix_gate,
            "claim_boundary": group.claim_boundary,
        }
        for group in gate_groups
    ]


def validate_evidence_check_source_compatibility(
    source: dict[str, Any],
    source_path: Path,
    checks: list[CheckCatalogRow],
    gate_groups: list[GatePlanGroup],
    error_cls: type[ErrorT],
) -> None:
    """Validate legacy evidence-check source remains a compatibility view."""
    source_groups = require_list(source.get("groups"), f"{source_path}.groups", error_cls)
    expected_groups = manifest_groups_from_normal_form(checks, gate_groups, error_cls)
    if len(source_groups) != len(expected_groups):
        raise validation_error(error_cls, f"{source_path}.groups length drifts from GatePlan")
    for group_index, expected in enumerate(expected_groups):
        actual = require_mapping(source_groups[group_index], f"{source_path}.groups[{group_index}]", error_cls)
        for key in ("id", "description"):
            if actual.get(key) != expected.get(key):
                raise validation_error(error_cls, f"{source_path}.groups[{group_index}].{key} drifts from GatePlan")
        actual_checks = require_list(actual.get("checks"), f"{source_path}.groups[{group_index}].checks", error_cls)
        expected_checks = [{key: value for key, value in check.items() if key != "ci_command"} for check in expected["checks"]]
        if actual_checks != expected_checks:
            raise validation_error(error_cls, f"{source_path}.groups[{group_index}].checks drifts from CheckCatalog")
