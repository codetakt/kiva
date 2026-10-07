"""Validate the OpenAPI-to-Rust DTO boundary ledger."""

from __future__ import annotations

import copy
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from kiva_ci.manifest import (
    load_yaml,
    mapping_by_unique_string_key,
    require_bool,
    require_existing_path,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
)
from kiva_ci.ledger import rows_by_unique_string_field
from kiva_ci.openapi_dto_map import (
    dto_map_contracts_by_name,
    load_openapi_dto_map,
    rust_source_paths,
)
from kiva_ci.openapi import collect_request_body_schema_names
from kiva_ci.openapi import collect_request_body_dto_schema_closure_names
from kiva_ci.openapi import collect_success_response_schema_names
from kiva_ci.openapi import collect_success_response_dto_schema_closure_names


DEFAULT_LEDGER = Path("spec/openapi/dto-boundary-ledger.yaml")
DEFAULT_DTO_MAP = Path("spec/openapi/semantic-compatibility-map.yaml")


class DtoBoundaryLedgerError(RuntimeError):
    """Raised when OpenAPI DTO boundary metadata drifts."""


@dataclass(frozen=True)
class ActiveDtoGuards:
    """Typed active DTO guard set for the OpenAPI HTTP boundary."""

    generated_contract_dto_map: str
    semantic_compatibility_map: str
    semantic_compatibility_map_validator: str
    generated_contract_dto_map_generator: str
    dto_map_projection: str
    retirement_milestone: str
    retired_dto_map_archive: str

    def as_mapping(self) -> dict[str, str]:
        """Return the stable report mapping."""
        return {
            "generated_contract_dto_map": self.generated_contract_dto_map,
            "semantic_compatibility_map": self.semantic_compatibility_map,
            "semantic_compatibility_map_validator": self.semantic_compatibility_map_validator,
            "generated_contract_dto_map_generator": self.generated_contract_dto_map_generator,
            "dto_map_projection": self.dto_map_projection,
            "retirement_milestone": self.retirement_milestone,
            "retired_dto_map_archive": self.retired_dto_map_archive,
        }


@dataclass(frozen=True)
class DtoBoundaryLedger:
    """Typed envelope for the canonical OpenAPI DTO boundary ledger."""

    path: Path
    document: dict[str, Any]

    @classmethod
    def from_document(cls, path: Path, document: dict[str, Any]) -> "DtoBoundaryLedger":
        """Parse and validate the stable DTO boundary-ledger envelope."""
        ledger = cls(path=path, document=document)
        ledger.validate_envelope()
        return ledger

    def validate_envelope(self) -> None:
        """Validate ledger identity and canonical/legacy path metadata."""
        if self.document.get("version") != 1:
            raise DtoBoundaryLedgerError("version must be 1")
        if self.document.get("kind") != "kiva-openapi-dto-boundary-ledger":
            raise DtoBoundaryLedgerError("kind must be kiva-openapi-dto-boundary-ledger")
        if self.document.get("status") not in {
            "generated-boundary-adopted-with-compatibility-guard",
            "complete",
        }:
            raise DtoBoundaryLedgerError("status is invalid")
        if self.document.get("source_of_truth") != "openapi-contracts":
            raise DtoBoundaryLedgerError("source_of_truth must be openapi-contracts")
        canonical_name = require_string(
            self.document.get("canonical_name"),
            "canonical_name",
            DtoBoundaryLedgerError,
        )
        if canonical_name != "openapi-dto-boundary-ledger":
            raise DtoBoundaryLedgerError("canonical_name must be openapi-dto-boundary-ledger")
        canonical_path = require_string(
            self.document.get("canonical_path"),
            "canonical_path",
            DtoBoundaryLedgerError,
        )
        if canonical_path != self.path.as_posix():
            raise DtoBoundaryLedgerError(
                f"canonical_path must match --ledger path: {self.path.as_posix()}"
            )
        legacy_path = require_string(
            self.document.get("legacy_path"),
            "legacy_path",
            DtoBoundaryLedgerError,
        )
        if legacy_path != "spec/openapi/dto-generation-plan.yaml":
            raise DtoBoundaryLedgerError(
                "legacy_path must record the retired dto-generation-plan path"
            )


def _require_split_source(
    root: Path,
    split_sources: dict[str, Any],
    key: str,
    expected_kind: str,
) -> dict[str, Any]:
    """Load one DTO boundary-ledger split source."""
    raw_path = require_string(
        split_sources.get(key),
        f"split_sources.{key}",
        DtoBoundaryLedgerError,
    )
    require_existing_path(root, raw_path, f"split_sources.{key}", DtoBoundaryLedgerError)
    document = load_yaml(root / raw_path, DtoBoundaryLedgerError)
    if document.get("version") != 1:
        raise DtoBoundaryLedgerError(f"{raw_path}.version must be 1")
    if document.get("kind") != expected_kind:
        raise DtoBoundaryLedgerError(f"{raw_path}.kind must be {expected_kind}")
    if document.get("source_of_truth") != "openapi-contracts":
        raise DtoBoundaryLedgerError(f"{raw_path}.source_of_truth must be openapi-contracts")
    return document


def _overlay_exact_field(
    aggregate: dict[str, Any],
    source: dict[str, Any],
    field: str,
    source_label: str,
) -> None:
    """Overlay a split-source field after checking aggregate drift."""
    if field not in source:
        return
    if field in aggregate and aggregate[field] != source[field]:
        raise DtoBoundaryLedgerError(
            f"{source_label}.{field} drifts from DTO boundary ledger"
        )
    aggregate[field] = source[field]


def _overlay_boundary_policy_source(
    root: Path,
    aggregate: dict[str, Any],
    split_sources: dict[str, Any],
) -> None:
    """Overlay the current DTO boundary policy split source."""
    source = _require_split_source(
        root,
        split_sources,
        "boundary_policy",
        "kiva-openapi-dto-boundary-policy",
    )
    for field in (
        "target_state",
        "boundary_status_policy",
        "handler_adoption",
        "boundary_parity_policy",
        "contracts",
        "invariants",
    ):
        _overlay_exact_field(aggregate, source, field, "dto-boundary-policy")

    if "active_guards" not in source:
        return
    source_guard = require_mapping(
        source.get("active_guards"),
        "dto-boundary-policy.active_guards",
        DtoBoundaryLedgerError,
    )
    ledger_guard = require_mapping(
        aggregate.get("active_guards"),
        "active_guards",
        DtoBoundaryLedgerError,
    )
    merged_guard = dict(ledger_guard)
    for key, value in source_guard.items():
        if key in ledger_guard and ledger_guard[key] != value:
            raise DtoBoundaryLedgerError(
                f"dto-boundary-policy.active_guards.{key} drifts from DTO boundary ledger"
            )
        merged_guard[key] = value
    aggregate["active_guards"] = merged_guard


def _overlay_completion_milestone_rows(
    aggregate: dict[str, Any],
    source: dict[str, Any],
) -> None:
    """Validate retirement-ledger milestone rows against the aggregate."""
    if "completion_milestones" not in source:
        return
    aggregate_rows = mapping_by_unique_string_key(
        aggregate.get("completion_milestones"),
        key="id",
        label="completion_milestones",
        error_cls=DtoBoundaryLedgerError,
    )
    for milestone_id, row in mapping_by_unique_string_key(
        source.get("completion_milestones"),
        key="id",
        label="semantic-compatibility-retirement.completion_milestones",
        error_cls=DtoBoundaryLedgerError,
    ).items():
        aggregate_row = aggregate_rows.get(milestone_id)
        if aggregate_row is None:
            raise DtoBoundaryLedgerError(
                "semantic-compatibility-retirement.completion_milestones "
                f"references unknown aggregate milestone: {milestone_id}"
            )
        if aggregate_row != row:
            raise DtoBoundaryLedgerError(
                "semantic-compatibility-retirement.completion_milestones "
                f"drifts from aggregate row: {milestone_id}"
            )


def _overlay_phase_rows(aggregate: dict[str, Any], source: dict[str, Any]) -> None:
    """Validate retirement-ledger phase rows against the aggregate."""
    if "phases" not in source:
        return
    aggregate_rows = mapping_by_unique_string_key(
        aggregate.get("phases"),
        key="id",
        label="phases",
        error_cls=DtoBoundaryLedgerError,
    )
    for phase_id, row in mapping_by_unique_string_key(
        source.get("phases"),
        key="id",
        label="semantic-compatibility-retirement.phases",
        error_cls=DtoBoundaryLedgerError,
    ).items():
        aggregate_row = aggregate_rows.get(phase_id)
        if aggregate_row is None:
            raise DtoBoundaryLedgerError(
                "semantic-compatibility-retirement.phases "
                f"references unknown aggregate phase: {phase_id}"
            )
        if aggregate_row != row:
            raise DtoBoundaryLedgerError(
                f"semantic-compatibility-retirement.phases drifts from DTO boundary ledger row: {phase_id}"
            )


def _overlay_semantic_compatibility_retirement_source(
    root: Path,
    aggregate: dict[str, Any],
    split_sources: dict[str, Any],
) -> None:
    """Overlay the semantic compatibility retirement split source."""
    source = _require_split_source(
        root,
        split_sources,
        "semantic_compatibility_retirement",
        "kiva-openapi-dto-semantic-compatibility-retirement",
    )
    _overlay_completion_milestone_rows(aggregate, source)
    _overlay_phase_rows(aggregate, source)
    _overlay_exact_field(
        aggregate,
        source,
        "remaining_compatibility_work_queue",
        "semantic-compatibility-retirement",
    )
    for field in ("final_contract_boundary", "retirement_exit_criteria"):
        _overlay_exact_field(
            aggregate,
            source,
            field,
            "semantic-compatibility-retirement",
        )

    retired_archives = source.get("retired_archives")
    if retired_archives is None:
        return
    retired_archives = require_mapping(
        retired_archives,
        "semantic-compatibility-retirement.retired_archives",
        DtoBoundaryLedgerError,
    )
    existing_archives = aggregate.get("retired_archives")
    if existing_archives is not None and existing_archives != retired_archives:
        raise DtoBoundaryLedgerError(
            "semantic-compatibility-retirement.retired_archives "
            "drifts from DTO boundary ledger"
        )
    aggregate["retired_archives"] = retired_archives


def load_boundary_ledger(root: Path, ledger_path: Path) -> dict[str, Any]:
    """Load the DTO boundary ledger, expanding validated split sources."""
    ledger = load_yaml(root / ledger_path, DtoBoundaryLedgerError)
    split_sources_value = ledger.get("split_sources")
    if split_sources_value is None:
        return ledger
    split_sources = require_mapping(
        split_sources_value,
        "split_sources",
        DtoBoundaryLedgerError,
    )
    aggregate = copy.deepcopy(ledger)
    _overlay_boundary_policy_source(root, aggregate, split_sources)
    _overlay_semantic_compatibility_retirement_source(root, aggregate, split_sources)
    return aggregate


def validate_referenced_dto_map_contracts(
    root: Path,
    openapi_path: str,
    generation_target: str,
    referenced_contract_names: list[str],
    dto_contracts: dict[str, dict[str, Any]],
) -> tuple[set[str], set[str]]:
    """Validate DTO map contract references and return covered schema names."""
    covered_schemas: set[str] = set()
    generation_target_schemas: set[str] = set()
    for contract_name in referenced_contract_names:
        contract = dto_contracts.get(contract_name)
        if contract is None:
            raise DtoBoundaryLedgerError(f"dto_map_contracts references missing contract: {contract_name}")
        mapped_openapi = require_string(
            contract.get("openapi"),
            f"{contract_name}.openapi",
            DtoBoundaryLedgerError,
        )
        if mapped_openapi != openapi_path:
            raise DtoBoundaryLedgerError(
                f"{contract_name}.openapi must match ledger contract {openapi_path}, got {mapped_openapi}"
            )
        schemas = require_mapping(
            contract.get("schemas"),
            f"{contract_name}.schemas",
            DtoBoundaryLedgerError,
        )
        schema_names = {str(schema_name) for schema_name in schemas}
        covered_schemas.update(schema_names)
        contract_has_generation_target_source = False
        for raw_source in rust_source_paths(contract, contract_name, DtoBoundaryLedgerError):
            require_existing_path(root, raw_source, f"{contract_name}.rust_sources", DtoBoundaryLedgerError)
            contract_has_generation_target_source = (
                contract_has_generation_target_source
                or (
                    raw_source == generation_target
                    or raw_source.startswith(f"{generation_target}/")
                )
            )
        if contract_has_generation_target_source:
            generation_target_schemas.update(schema_names)
    return covered_schemas, generation_target_schemas


def validate_generated_slice_schema_set(
    *,
    openapi_path: str,
    label: str,
    slice_info: dict[str, Any],
    schemas: dict[str, Any],
    generation_target_schemas: set[str],
    coverage_schema_sets: dict[str, set[str]],
) -> set[str]:
    """Validate a generated DTO slice, deriving schema sets when requested."""
    coverage_value = slice_info.get("coverage")
    coverage_label = None
    if coverage_value is not None:
        coverage_label = require_string(
            coverage_value,
            f"{openapi_path}.{label}.coverage",
            DtoBoundaryLedgerError,
        )
        if coverage_label not in coverage_schema_sets:
            raise DtoBoundaryLedgerError(
                f"{openapi_path}.{label}.coverage is invalid: {coverage_label}"
            )

    raw_schemas = slice_info.get("schemas")
    if raw_schemas is None:
        if coverage_label is None:
            raise DtoBoundaryLedgerError(
                f"{openapi_path}.{label}.schemas is required unless coverage derives the slice"
            )
        slice_schemas = set(coverage_schema_sets[coverage_label])
    else:
        slice_schemas = set(
            require_string_list(
                raw_schemas,
                f"{openapi_path}.{label}.schemas",
                DtoBoundaryLedgerError,
            )
        )

    if coverage_label is not None:
        expected = coverage_schema_sets[coverage_label]
        if coverage_label.endswith("-schema-closure"):
            missing = sorted(expected.difference(slice_schemas))
            unexpected = sorted(slice_schemas.difference(expected))
            if missing or unexpected:
                raise DtoBoundaryLedgerError(
                    f"{openapi_path}.{label}.schemas drift from {coverage_label}; "
                    f"missing={missing}, unexpected={unexpected}"
                )
        else:
            missing = sorted(expected.difference(slice_schemas))
            if missing:
                raise DtoBoundaryLedgerError(
                    f"{openapi_path}.{label}.schemas missing {coverage_label}: "
                    + ", ".join(missing)
                )

    missing_openapi_schemas = sorted(slice_schemas.difference(schemas))
    if missing_openapi_schemas:
        raise DtoBoundaryLedgerError(
            f"{openapi_path}.{label}.schemas missing from OpenAPI: "
            + ", ".join(missing_openapi_schemas)
        )
    missing_dto_map_schemas = sorted(slice_schemas.difference(generation_target_schemas))
    if missing_dto_map_schemas:
        raise DtoBoundaryLedgerError(
            f"{openapi_path}.{label}.schemas missing from DTO map: "
            + ", ".join(missing_dto_map_schemas)
        )
    return slice_schemas


def validate_contract(
    root: Path,
    contract: dict[str, Any],
    index: int,
    dto_contracts: dict[str, dict[str, Any]],
    required_contract_targets: dict[str, str],
    boundary_status_policy: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    """Validate one OpenAPI contract entry from the DTO boundary ledger."""
    openapi_path = require_string(
        contract.get("openapi"),
        f"contracts[{index}].openapi",
        DtoBoundaryLedgerError,
    )
    require_existing_path(root, openapi_path, f"contracts[{index}].openapi", DtoBoundaryLedgerError)
    openapi = load_yaml(root / openapi_path, DtoBoundaryLedgerError)
    components = require_mapping(
        openapi.get("components"),
        f"{openapi_path}.components",
        DtoBoundaryLedgerError,
    )
    schemas = require_mapping(
        components.get("schemas"),
        f"{openapi_path}.components.schemas",
        DtoBoundaryLedgerError,
    )

    generation_target = require_string(
        contract.get("generation_target"),
        f"{openapi_path}.generation_target",
        DtoBoundaryLedgerError,
    )
    external_contract_boundary = require_string(
        contract.get("external_contract_boundary"),
        f"{openapi_path}.external_contract_boundary",
        DtoBoundaryLedgerError,
    )
    if external_contract_boundary != generation_target:
        raise DtoBoundaryLedgerError(
            f"{openapi_path}.external_contract_boundary must match generation_target"
        )
    require_existing_path(
        root,
        external_contract_boundary,
        f"{openapi_path}.external_contract_boundary",
        DtoBoundaryLedgerError,
    )
    internal_semantic_boundaries = require_string_list(
        contract.get("internal_semantic_boundaries"),
        f"{openapi_path}.internal_semantic_boundaries",
        DtoBoundaryLedgerError,
    )
    for semantic_boundary in internal_semantic_boundaries:
        require_existing_path(
            root,
            semantic_boundary,
            f"{openapi_path}.internal_semantic_boundaries",
            DtoBoundaryLedgerError,
        )
    expected_generation_target = required_contract_targets.get(openapi_path)
    if expected_generation_target is None:
        raise DtoBoundaryLedgerError(
            f"{openapi_path} is missing from the DTO generation contract target set"
        )
    if generation_target != expected_generation_target:
        raise DtoBoundaryLedgerError(
            f"{openapi_path}.generation_target must match contract target "
            f"{expected_generation_target}, got {generation_target}"
        )
    require_existing_path(root, generation_target, f"{openapi_path}.generation_target", DtoBoundaryLedgerError)

    status = require_string(
        contract.get("boundary_status"),
        f"{openapi_path}.boundary_status",
        DtoBoundaryLedgerError,
    )
    status_policy = boundary_status_policy.get(status)
    if status_policy is None:
        raise DtoBoundaryLedgerError(f"{openapi_path}.boundary_status is invalid: {status}")

    referenced_contract_names = require_string_list(
        contract.get("dto_map_contracts"),
        f"{openapi_path}.dto_map_contracts",
        DtoBoundaryLedgerError,
        allow_empty=True,
    )
    generated_contract_names = require_string_list(
        contract.get("generated_contract_dto_map_contracts"),
        f"{openapi_path}.generated_contract_dto_map_contracts",
        DtoBoundaryLedgerError,
        allow_empty=True,
    )
    semantic_compatibility_names = require_string_list(
        contract.get("semantic_compatibility_map_contracts"),
        f"{openapi_path}.semantic_compatibility_map_contracts",
        DtoBoundaryLedgerError,
        allow_empty=True,
    )
    if set(generated_contract_names).intersection(semantic_compatibility_names):
        raise DtoBoundaryLedgerError(
            f"{openapi_path} generated and semantic compatibility DTO map contracts must be disjoint"
        )
    split_names = set(generated_contract_names).union(semantic_compatibility_names)
    if split_names != set(referenced_contract_names):
        raise DtoBoundaryLedgerError(
            f"{openapi_path}.dto_map_contracts must equal generated plus semantic compatibility contracts"
        )
    covered_schemas, generation_target_schemas = validate_referenced_dto_map_contracts(
        root,
        openapi_path,
        generation_target,
        referenced_contract_names,
        dto_contracts,
    )

    dto_map_policy = require_string(
        status_policy.get("dto_map_contracts"),
        f"boundary_status_policy.{status}.dto_map_contracts",
        DtoBoundaryLedgerError,
    )
    if dto_map_policy == "empty" and referenced_contract_names:
        raise DtoBoundaryLedgerError(f"{openapi_path} must not claim DTO map contracts before generation")
    if dto_map_policy == "non_empty" and not referenced_contract_names:
        raise DtoBoundaryLedgerError(f"{openapi_path} must reference at least one DTO map contract")
    if dto_map_policy not in {"empty", "non_empty"}:
        raise DtoBoundaryLedgerError(
            f"boundary_status_policy.{status}.dto_map_contracts is invalid: {dto_map_policy}"
        )

    if (
        require_bool(
            status_policy.get("dto_maps_must_live_under_generation_target"),
            f"boundary_status_policy.{status}.dto_maps_must_live_under_generation_target",
            DtoBoundaryLedgerError,
        )
        and covered_schemas != generation_target_schemas
    ):
        raise DtoBoundaryLedgerError(
            f"{openapi_path} DTO maps must live under {generation_target} for boundary status {status}"
        )

    active_slice_policy = require_string(
        status_policy.get("active_generated_slice"),
        f"boundary_status_policy.{status}.active_generated_slice",
        DtoBoundaryLedgerError,
    )
    has_active_slice = "active_generated_slice" in contract
    if active_slice_policy == "required" and not has_active_slice:
        raise DtoBoundaryLedgerError(f"{openapi_path}.active_generated_slice is required")
    if active_slice_policy == "forbidden" and has_active_slice:
        raise DtoBoundaryLedgerError(f"{openapi_path}.active_generated_slice is forbidden")
    if active_slice_policy not in {"required", "forbidden", "optional"}:
        raise DtoBoundaryLedgerError(
            f"boundary_status_policy.{status}.active_generated_slice is invalid: {active_slice_policy}"
        )
    if has_active_slice:
        slice_info = require_mapping(
            contract.get("active_generated_slice"),
            f"{openapi_path}.active_generated_slice",
            DtoBoundaryLedgerError,
        )
        slice_crate = require_string(
            slice_info.get("crate"),
            f"{openapi_path}.active_generated_slice.crate",
            DtoBoundaryLedgerError,
        )
        if slice_crate != generation_target:
            raise DtoBoundaryLedgerError(f"{openapi_path}.active_generated_slice.crate must match generation_target")
        validate_generated_slice_schema_set(
            openapi_path=openapi_path,
            label="active_generated_slice",
            slice_info=slice_info,
            schemas=schemas,
            generation_target_schemas=generation_target_schemas,
            coverage_schema_sets={
                "all-request-body-schemas": collect_request_body_schema_names(
                    openapi,
                    openapi_path,
                    DtoBoundaryLedgerError,
                ),
                "all-request-body-schema-closure": collect_request_body_dto_schema_closure_names(
                    openapi,
                    openapi_path,
                    DtoBoundaryLedgerError,
                ),
            },
        )

    if "generated_response_slice" in contract:
        response_slice = require_mapping(
            contract.get("generated_response_slice"),
            f"{openapi_path}.generated_response_slice",
            DtoBoundaryLedgerError,
        )
        response_crate = require_string(
            response_slice.get("crate"),
            f"{openapi_path}.generated_response_slice.crate",
            DtoBoundaryLedgerError,
        )
        if response_crate != generation_target:
            raise DtoBoundaryLedgerError(
                f"{openapi_path}.generated_response_slice.crate must match generation_target"
            )
        validate_generated_slice_schema_set(
            openapi_path=openapi_path,
            label="generated_response_slice",
            slice_info=response_slice,
            schemas=schemas,
            generation_target_schemas=generation_target_schemas,
            coverage_schema_sets={
                "all-success-response-schemas": collect_success_response_schema_names(
                    openapi,
                    openapi_path,
                    DtoBoundaryLedgerError,
                ),
                "all-success-response-schema-closure": collect_success_response_dto_schema_closure_names(
                    openapi,
                    openapi_path,
                    DtoBoundaryLedgerError,
                ),
            },
        )

    return {
        "openapi": openapi_path,
        "boundary_status": status,
        "dto_map_contracts": referenced_contract_names,
        "generated_contract_dto_map_contracts": generated_contract_names,
        "semantic_compatibility_map_contracts": semantic_compatibility_names,
        "covered_schema_count": len(covered_schemas),
        "external_contract_boundary": external_contract_boundary,
        "internal_semantic_boundaries": internal_semantic_boundaries,
    }


def validate_target_state(ledger: dict[str, Any]) -> None:
    """Validate the final OpenAPI-to-DTO generation target state."""
    target_state = require_mapping(
        ledger.get("target_state"),
        "target_state",
        DtoBoundaryLedgerError,
    )
    for key in (
        "external_http_dtos",
        "internal_domain_types",
        "source_of_truth",
        "completion_rule",
    ):
        require_string(
            target_state.get(key),
            f"target_state.{key}",
            DtoBoundaryLedgerError,
        )
    if target_state["source_of_truth"] != ledger.get("source_of_truth"):
        raise DtoBoundaryLedgerError(
            "target_state.source_of_truth must match ledger.source_of_truth"
        )


def contract_targets(root: Path, ledger: dict[str, Any]) -> dict[str, str]:
    """Return OpenAPI contract generation targets from the canonical contract records."""
    contracts = mapping_by_unique_string_key(
        ledger.get("contracts"),
        key="openapi",
        label="contracts",
        error_cls=DtoBoundaryLedgerError,
    )
    if not contracts:
        raise DtoBoundaryLedgerError("contracts must not be empty")
    targets: dict[str, str] = {}
    for openapi, contract in contracts.items():
        target = require_string(
            contract.get("generation_target"),
            f"contracts[{openapi}].generation_target",
            DtoBoundaryLedgerError,
        )
        require_existing_path(
            root,
            openapi,
            f"contracts[{openapi}].openapi",
            DtoBoundaryLedgerError,
        )
        require_existing_path(
            root,
            target,
            f"contracts[{openapi}].generation_target",
            DtoBoundaryLedgerError,
        )
        targets[openapi] = target
    if len(targets) != len(contracts):
        raise DtoBoundaryLedgerError("contracts must not contain duplicate OpenAPI paths")
    return targets


def validate_boundary_status_policy(ledger: dict[str, Any]) -> dict[str, dict[str, Any]]:
    """Validate the boundary-status policy table and return it keyed by status."""
    raw_policy = require_mapping(
        ledger.get("boundary_status_policy"),
        "boundary_status_policy",
        DtoBoundaryLedgerError,
    )
    if not raw_policy:
        raise DtoBoundaryLedgerError("boundary_status_policy must not be empty")
    policy: dict[str, dict[str, Any]] = {}
    for status, value in raw_policy.items():
        if not isinstance(status, str) or not status:
            raise DtoBoundaryLedgerError("boundary_status_policy keys must be non-empty strings")
        status_policy = require_mapping(
            value,
            f"boundary_status_policy.{status}",
            DtoBoundaryLedgerError,
        )
        for key in (
            "dto_map_contracts",
            "active_generated_slice",
            "dto_maps_must_live_under_generation_target",
        ):
            if key not in status_policy:
                raise DtoBoundaryLedgerError(f"boundary_status_policy.{status}.{key} is required")
        policy[status] = status_policy
    return policy


def validate_completion_milestones(
    root: Path,
    ledger: dict[str, Any],
    required_contract_targets: dict[str, str],
) -> list[dict[str, Any]]:
    """Validate DTO generation completion milestones against required contracts."""
    milestones = rows_by_unique_string_field(
        ledger.get("completion_milestones"),
        label="completion_milestones",
        error_cls=DtoBoundaryLedgerError,
        duplicate_label="completion milestone id",
    )
    if not milestones:
        raise DtoBoundaryLedgerError("completion_milestones must not be empty")
    required_contracts = set(required_contract_targets)
    summaries: list[dict[str, Any]] = []
    for milestone_id, milestone in milestones.items():
        status = require_string(
            milestone.get("status"),
            f"completion_milestones.{milestone_id}.status",
            DtoBoundaryLedgerError,
        )
        if status not in {"active", "planned", "complete"}:
            raise DtoBoundaryLedgerError(f"{milestone_id}.status is invalid: {status}")
        require_string(
            milestone.get("completion_rule"),
            f"completion_milestones.{milestone_id}.completion_rule",
            DtoBoundaryLedgerError,
        )
        contract_set = require_string(
            milestone.get("contract_set"),
            f"completion_milestones.{milestone_id}.contract_set",
            DtoBoundaryLedgerError,
        )
        if contract_set != "all-openapi-contracts":
            raise DtoBoundaryLedgerError(
                f"{milestone_id}.contract_set must be all-openapi-contracts"
            )
        contracts = set(required_contracts)
        missing = sorted(required_contracts.difference(contracts))
        unexpected = sorted(contracts.difference(required_contracts))
        if missing or unexpected:
            raise DtoBoundaryLedgerError(
                f"{milestone_id}.contracts drift; missing={missing}, unexpected={unexpected}"
            )
        for openapi_path in contracts:
            require_existing_path(
                root,
                openapi_path,
                f"{milestone_id}.contract_set",
                DtoBoundaryLedgerError,
            )
        summaries.append(
            {
                "id": milestone_id,
                "status": status,
                "contract_count": len(contracts),
            }
        )
    return summaries


def validate_handler_adoption(
    root: Path,
    ledger: dict[str, Any],
    required_contract_targets: dict[str, str],
    ledger_contracts_by_openapi: dict[str, dict[str, Any]],
) -> list[dict[str, Any]]:
    """Validate generated DTO adoption status at server handler boundaries."""
    adoption = require_mapping(
        ledger.get("handler_adoption"),
        "handler_adoption",
        DtoBoundaryLedgerError,
    )
    status = require_string(
        adoption.get("status"),
        "handler_adoption.status",
        DtoBoundaryLedgerError,
    )
    if status != "complete":
        raise DtoBoundaryLedgerError(f"handler_adoption.status is invalid: {status}")
    completion_rule = require_string(
        adoption.get("completion_rule"),
        "handler_adoption.completion_rule",
        DtoBoundaryLedgerError,
    )
    if "generated-boundary-adoption" not in completion_rule:
        raise DtoBoundaryLedgerError(
            "handler_adoption.completion_rule must bind generated-boundary-adoption"
        )
    source = require_string(
        adoption.get("source"),
        "handler_adoption.source",
        DtoBoundaryLedgerError,
    )
    if source != "contracts[].handler_adoption":
        raise DtoBoundaryLedgerError(
            "handler_adoption.source must be contracts[].handler_adoption"
        )
    expected_contracts = set(required_contract_targets)
    summaries: list[dict[str, Any]] = []
    seen_openapi: set[str] = set()
    allowed_statuses = {
        "generated-contract-slice-not-handler-adopted",
        "generated-request-response-handler-adopted",
        "generated-response-handler-adopted",
    }
    for index, (openapi_path, ledger_contract) in enumerate(ledger_contracts_by_openapi.items()):
        entry = require_mapping(
            ledger_contract.get("handler_adoption"),
            f"contracts[{index}].handler_adoption",
            DtoBoundaryLedgerError,
        )
        if openapi_path in seen_openapi:
            raise DtoBoundaryLedgerError(
                f"duplicate handler adoption contract: {openapi_path}"
            )
        seen_openapi.add(openapi_path)
        if openapi_path not in expected_contracts:
            raise DtoBoundaryLedgerError(
                f"handler_adoption references unknown OpenAPI contract: {openapi_path}"
            )
        adoption_status = require_string(
            entry.get("adoption_status"),
            f"contracts[{index}].handler_adoption.adoption_status",
            DtoBoundaryLedgerError,
        )
        if adoption_status not in allowed_statuses:
            raise DtoBoundaryLedgerError(
                f"{openapi_path}.handler_adoption.adoption_status is invalid: {adoption_status}"
            )
        generated_contract_crate = required_contract_targets[openapi_path]
        require_existing_path(
            root,
            generated_contract_crate,
            f"contracts[{index}].generation_target",
            DtoBoundaryLedgerError,
        )
        for handler_boundary in require_string_list(
            entry.get("handler_boundaries"),
            f"contracts[{index}].handler_adoption.handler_boundaries",
            DtoBoundaryLedgerError,
        ):
            require_existing_path(
                root,
                handler_boundary,
                f"contracts[{index}].handler_adoption.handler_boundaries",
                DtoBoundaryLedgerError,
            )
        generated_request_parsing = require_bool(
            entry.get("generated_request_parsing"),
            f"contracts[{index}].handler_adoption.generated_request_parsing",
            DtoBoundaryLedgerError,
        )
        generated_response_projection = require_bool(
            entry.get("generated_response_projection"),
            f"contracts[{index}].handler_adoption.generated_response_projection",
            DtoBoundaryLedgerError,
        )
        has_runtime_gate = "runtime_adoption_gate" in entry
        if not generated_request_parsing or not generated_response_projection:
            gate = require_mapping(
                entry.get("runtime_adoption_gate"),
                f"contracts[{index}].handler_adoption.runtime_adoption_gate",
                DtoBoundaryLedgerError,
            )
            status = require_string(
                gate.get("status"),
                f"contracts[{index}].handler_adoption.runtime_adoption_gate.status",
                DtoBoundaryLedgerError,
            )
            if not status.startswith("blocked-by-"):
                raise DtoBoundaryLedgerError(
                    f"{openapi_path} runtime_adoption_gate.status must start with blocked-by-"
                )
            require_string(
                gate.get("next_step"),
                f"contracts[{index}].handler_adoption.runtime_adoption_gate.next_step",
                DtoBoundaryLedgerError,
            )
            validator = require_string(
                gate.get("validator"),
                f"contracts[{index}].handler_adoption.runtime_adoption_gate.validator",
                DtoBoundaryLedgerError,
            )
            if validator != "scripts/ci/validate_openapi_dto_boundary_ledger.py":
                raise DtoBoundaryLedgerError(
                    f"{openapi_path} runtime_adoption_gate.validator must be this validator"
                )
        elif has_runtime_gate:
            raise DtoBoundaryLedgerError(
                f"{openapi_path} must not keep runtime_adoption_gate after generated handler adoption is complete"
            )
        boundary_status = require_string(
            ledger_contract.get("boundary_status"),
            f"{openapi_path}.boundary_status",
            DtoBoundaryLedgerError,
        )
        boundary_type = require_string(
            ledger_contract.get("boundary_type"),
            f"{openapi_path}.boundary_type",
            DtoBoundaryLedgerError,
        )
        if boundary_type not in {"GeneratedContractBoundary", "CompatibilityHttpBoundary"}:
            raise DtoBoundaryLedgerError(
                f"{openapi_path}.boundary_type is invalid: {boundary_type}"
            )
        target_boundary_type = require_string(
            ledger_contract.get("target_boundary_type"),
            f"{openapi_path}.target_boundary_type",
            DtoBoundaryLedgerError,
        )
        if target_boundary_type != "GeneratedContractBoundary":
            raise DtoBoundaryLedgerError(
                f"{openapi_path}.target_boundary_type must be GeneratedContractBoundary"
            )
        generated_request_parsing_target = require_bool(
            entry.get("generated_request_parsing_target"),
            f"contracts[{index}].handler_adoption.generated_request_parsing_target",
            DtoBoundaryLedgerError,
        )
        generated_response_projection_target = require_bool(
            entry.get("generated_response_projection_target"),
            f"contracts[{index}].handler_adoption.generated_response_projection_target",
            DtoBoundaryLedgerError,
        )
        if not generated_request_parsing_target or not generated_response_projection_target:
            raise DtoBoundaryLedgerError(
                f"{openapi_path} handler targets must require generated request parsing and response projection"
            )
        if generated_request_parsing and generated_response_projection:
            if boundary_type != "GeneratedContractBoundary":
                raise DtoBoundaryLedgerError(
                    f"{openapi_path}.boundary_type must be GeneratedContractBoundary after full generated handler adoption"
                )
        elif boundary_type != "CompatibilityHttpBoundary":
            raise DtoBoundaryLedgerError(
                f"{openapi_path}.boundary_type must be CompatibilityHttpBoundary while any handwritten handler boundary remains"
            )
        if boundary_status == "active-generated-contract-slice":
            if adoption_status != "generated-contract-slice-not-handler-adopted":
                raise DtoBoundaryLedgerError(
                    f"{openapi_path} active-generated-contract-slice must remain not-handler-adopted"
                )
            if generated_request_parsing or generated_response_projection:
                raise DtoBoundaryLedgerError(
                    f"{openapi_path} must not claim generated handler adoption while runtime parser is handwritten"
                )
        elif boundary_status == "active-generated-request-slice":
            if not generated_request_parsing:
                raise DtoBoundaryLedgerError(
                    f"{openapi_path} active-generated-request-slice must claim generated request parsing"
                )
        elif boundary_status == "generated-contract-crate":
            if not generated_request_parsing and not generated_response_projection:
                raise DtoBoundaryLedgerError(
                    f"{openapi_path} generated-contract-crate must claim at least one generated handler projection"
                )
        if adoption_status == "generated-response-handler-adopted" and not generated_response_projection:
            raise DtoBoundaryLedgerError(
                f"{openapi_path} response handler adoption requires generated_response_projection"
            )
        if (
            adoption_status == "generated-request-response-handler-adopted"
            and (not generated_request_parsing or not generated_response_projection)
        ):
            raise DtoBoundaryLedgerError(
                f"{openapi_path} request/response handler adoption requires both generated flags"
            )
        summaries.append(
            {
                "openapi": openapi_path,
                "boundary_type": boundary_type,
                "boundary_status": boundary_status,
                "adoption_status": adoption_status,
                "generated_request_parsing": generated_request_parsing,
                "generated_response_projection": generated_response_projection,
                "target_boundary_type": target_boundary_type,
                "generated_request_parsing_target": generated_request_parsing_target,
                "generated_response_projection_target": generated_response_projection_target,
            }
        )
    missing = sorted(expected_contracts.difference(seen_openapi))
    unexpected = sorted(seen_openapi.difference(expected_contracts))
    if missing or unexpected:
        raise DtoBoundaryLedgerError(
            f"contracts[].handler_adoption drift; missing={missing}, unexpected={unexpected}"
        )
    return summaries


def validate_boundary_parity_policy(ledger: dict[str, Any]) -> None:
    """Validate the final generated-boundary parity policy."""
    policy = require_mapping(
        ledger.get("boundary_parity_policy"),
        "boundary_parity_policy",
        DtoBoundaryLedgerError,
    )
    target = require_string(
        policy.get("target_boundary_type"),
        "boundary_parity_policy.target_boundary_type",
        DtoBoundaryLedgerError,
    )
    if target != "GeneratedContractBoundary":
        raise DtoBoundaryLedgerError(
            "boundary_parity_policy.target_boundary_type must be GeneratedContractBoundary"
        )
    contract_set = require_string(
        policy.get("contract_set"),
        "boundary_parity_policy.contract_set",
        DtoBoundaryLedgerError,
    )
    if contract_set != "all-openapi-contracts":
        raise DtoBoundaryLedgerError(
            "boundary_parity_policy.contract_set must be all-openapi-contracts"
        )
    rule = require_string(
        policy.get("compatibility_boundary_rule"),
        "boundary_parity_policy.compatibility_boundary_rule",
        DtoBoundaryLedgerError,
    )
    for token in (
        "CompatibilityHttpBoundary",
        "target_boundary_type",
        "runtime_adoption_gate",
    ):
        if token not in rule:
            raise DtoBoundaryLedgerError(
                f"boundary_parity_policy.compatibility_boundary_rule must mention {token}"
            )


def validate_remaining_compatibility_work_queue(ledger: dict[str, Any]) -> list[dict[str, str]]:
    """Validate the remaining generated-boundary compatibility work queue."""
    adoption_dimensions = {
        "generated_request_parsing",
        "generated_response_projection",
        "boundary_retirement",
    }
    queue = require_mapping(
        ledger.get("remaining_compatibility_work_queue"),
        "remaining_compatibility_work_queue",
        DtoBoundaryLedgerError,
    )
    source = require_string(
        queue.get("source_of_truth"),
        "remaining_compatibility_work_queue.source_of_truth",
        DtoBoundaryLedgerError,
    )
    if source != "contracts[].handler_adoption":
        raise DtoBoundaryLedgerError(
            "remaining_compatibility_work_queue.source_of_truth must be contracts[].handler_adoption"
        )
    completion_order = require_string_list(
        queue.get("completion_order"),
        "remaining_compatibility_work_queue.completion_order",
        DtoBoundaryLedgerError,
        allow_empty=True,
    )
    items = rows_by_unique_string_field(
        queue.get("items"),
        label="remaining_compatibility_work_queue.items",
        error_cls=DtoBoundaryLedgerError,
        duplicate_label="compatibility work item",
    )
    item_dimensions: set[tuple[str, str]] = set()
    summaries: list[dict[str, str]] = []
    for item_id, item in items.items():
        target = require_string(
            item.get("target_boundary_type"),
            f"{item_id}.target_boundary_type",
            DtoBoundaryLedgerError,
        )
        if target != "GeneratedContractBoundary":
            raise DtoBoundaryLedgerError(f"{item_id}.target_boundary_type must be GeneratedContractBoundary")
        current = require_string(
            item.get("current_boundary_type"),
            f"{item_id}.current_boundary_type",
            DtoBoundaryLedgerError,
        )
        if current not in {"CompatibilityHttpBoundary", "GeneratedContractBoundary"}:
            raise DtoBoundaryLedgerError(f"{item_id}.current_boundary_type is invalid: {current}")
        contract = require_string(item.get("contract"), f"{item_id}.contract", DtoBoundaryLedgerError)
        adoption_dimension = require_string(
            item.get("adoption_dimension"),
            f"{item_id}.adoption_dimension",
            DtoBoundaryLedgerError,
        )
        if adoption_dimension not in adoption_dimensions:
            raise DtoBoundaryLedgerError(
                f"{item_id}.adoption_dimension is invalid: {adoption_dimension}"
            )
        if adoption_dimension == "boundary_retirement" and contract != "all-openapi-contracts":
            raise DtoBoundaryLedgerError(
                f"{item_id}.contract must be all-openapi-contracts for boundary_retirement"
            )
        if adoption_dimension != "boundary_retirement" and contract == "all-openapi-contracts":
            raise DtoBoundaryLedgerError(
                f"{item_id}.contract must name one OpenAPI contract for {adoption_dimension}"
            )
        pair = (contract, adoption_dimension)
        if pair in item_dimensions:
            raise DtoBoundaryLedgerError(
                f"duplicate compatibility work item dimension: {contract}.{adoption_dimension}"
            )
        item_dimensions.add(pair)
        summaries.append(
            {
                "id": item_id,
                "contract": contract,
                "adoption_dimension": adoption_dimension,
                "implementation_target": require_string(
                    item.get("implementation_target"),
                    f"{item_id}.implementation_target",
                    DtoBoundaryLedgerError,
                ),
                "completion_gate": require_string(
                    item.get("completion_gate"),
                    f"{item_id}.completion_gate",
                    DtoBoundaryLedgerError,
                ),
            }
        )
    item_ids = set(items)
    if set(completion_order) != item_ids:
        raise DtoBoundaryLedgerError(
            "remaining_compatibility_work_queue completion_order drift: "
            f"items={sorted(item_ids)} order={sorted(completion_order)}"
        )
    archive = require_retired_dto_map_archive(ledger)
    retirement_status = require_string(
        archive.get("status"),
        "retired_archives.rust_dto_map_archive.status",
        DtoBoundaryLedgerError,
    )
    expected_dimensions = (
        set()
        if retirement_status == "complete"
        else {("all-openapi-contracts", "boundary_retirement")}
    )
    for index, raw_contract in enumerate(
        require_list(ledger.get("contracts"), "contracts", DtoBoundaryLedgerError)
    ):
        contract = require_mapping(raw_contract, f"contracts[{index}]", DtoBoundaryLedgerError)
        openapi = require_string(contract.get("openapi"), f"contracts[{index}].openapi", DtoBoundaryLedgerError)
        adoption = require_mapping(
            contract.get("handler_adoption"),
            f"{openapi}.handler_adoption",
            DtoBoundaryLedgerError,
        )
        for dimension in ("generated_request_parsing", "generated_response_projection"):
            target = adoption.get(f"{dimension}_target")
            current = adoption.get(dimension)
            if target is True and current is False:
                expected_dimensions.add((openapi, dimension))
            if target is not True and current is False:
                raise DtoBoundaryLedgerError(
                    f"{openapi}.handler_adoption.{dimension} is false without a target flag"
                )
    if item_dimensions != expected_dimensions:
        missing = sorted(expected_dimensions.difference(item_dimensions))
        unexpected = sorted(item_dimensions.difference(expected_dimensions))
        raise DtoBoundaryLedgerError(
            "remaining_compatibility_work_queue dimension drift: "
            f"missing={missing} unexpected={unexpected}"
        )
    return summaries


def require_retired_dto_map_archive(ledger: dict[str, Any]) -> dict[str, Any]:
    """Return the retired DTO map archive row from the boundary ledger."""
    retired_archives = require_mapping(
        ledger.get("retired_archives"),
        "retired_archives",
        DtoBoundaryLedgerError,
    )
    return require_mapping(
        retired_archives.get("rust_dto_map_archive"),
        "retired_archives.rust_dto_map_archive",
        DtoBoundaryLedgerError,
    )


def validate_active_guards(
    root: Path,
    ledger: dict[str, Any],
    dto_map_path: Path,
) -> ActiveDtoGuards:
    """Validate active split DTO-map guards after aggregate retirement."""
    guard = require_mapping(ledger.get("active_guards"), "active_guards", DtoBoundaryLedgerError)
    generated_contract_dto_map = require_string(
        guard.get("generated_contract_dto_map"),
        "active_guards.generated_contract_dto_map",
        DtoBoundaryLedgerError,
    )
    semantic_compatibility_map = require_string(
        guard.get("semantic_compatibility_map"),
        "active_guards.semantic_compatibility_map",
        DtoBoundaryLedgerError,
    )
    if semantic_compatibility_map != str(dto_map_path):
        raise DtoBoundaryLedgerError(
            f"active_guards.semantic_compatibility_map must be {dto_map_path}, "
            f"got {semantic_compatibility_map}"
        )
    for guard_map in (generated_contract_dto_map, semantic_compatibility_map):
        require_existing_path(root, guard_map, "active_guards split DTO map", DtoBoundaryLedgerError)

    dto_map_projection = require_string(
        guard.get("dto_map_projection"),
        "active_guards.dto_map_projection",
        DtoBoundaryLedgerError,
    )
    if dto_map_projection != "generated/openapi-dto-map-projection.yaml":
        raise DtoBoundaryLedgerError(
            "active_guards.dto_map_projection must be generated/openapi-dto-map-projection.yaml"
        )
    require_existing_path(
        root,
        dto_map_projection,
        "active_guards.dto_map_projection",
        DtoBoundaryLedgerError,
    )
    semantic_map_validator = require_string(
        guard.get("semantic_compatibility_map_validator"),
        "active_guards.semantic_compatibility_map_validator",
        DtoBoundaryLedgerError,
    )
    if semantic_map_validator != "scripts/ci/validate_openapi_rust_dto_map.py":
        raise DtoBoundaryLedgerError(
            "active_guards.semantic_compatibility_map_validator must be validate_openapi_rust_dto_map.py"
        )
    require_existing_path(
        root,
        semantic_map_validator,
        "active_guards.semantic_compatibility_map_validator",
        DtoBoundaryLedgerError,
    )
    generated_map_generator = require_string(
        guard.get("generated_contract_dto_map_generator"),
        "active_guards.generated_contract_dto_map_generator",
        DtoBoundaryLedgerError,
    )
    if generated_map_generator != "scripts/ci/generate_generated_contract_dto_map.py":
        raise DtoBoundaryLedgerError(
            "active_guards.generated_contract_dto_map_generator must be "
            "generate_generated_contract_dto_map.py"
        )
    require_existing_path(
        root,
        generated_map_generator,
        "active_guards.generated_contract_dto_map_generator",
        DtoBoundaryLedgerError,
    )
    plan_validator = require_string(
        guard.get("boundary_ledger_validator"),
        "active_guards.boundary_ledger_validator",
        DtoBoundaryLedgerError,
    )
    if plan_validator != "scripts/ci/validate_openapi_dto_boundary_ledger.py":
        raise DtoBoundaryLedgerError(
            "active_guards.boundary_ledger_validator must be validate_openapi_dto_boundary_ledger.py"
        )
    projection_generator = require_string(
        guard.get("dto_map_projection_generator"),
        "active_guards.dto_map_projection_generator",
        DtoBoundaryLedgerError,
    )
    if projection_generator != "scripts/ci/generate_openapi_dto_map_projection.py":
        raise DtoBoundaryLedgerError(
            "active_guards.dto_map_projection_generator must be generate_openapi_dto_map_projection.py"
        )
    require_existing_path(
        root,
        projection_generator,
        "active_guards.dto_map_projection_generator",
        DtoBoundaryLedgerError,
    )
    projection_check = require_string(
        guard.get("dto_map_projection_check"),
        "active_guards.dto_map_projection_check",
        DtoBoundaryLedgerError,
    )
    if projection_check != "openapi-dto-map-projection":
        raise DtoBoundaryLedgerError(
            "active_guards.dto_map_projection_check must be openapi-dto-map-projection"
        )

    retirement = require_retired_dto_map_archive(ledger)
    status = require_string(
        retirement.get("status"),
        "retired_archives.rust_dto_map_archive.status",
        DtoBoundaryLedgerError,
    )
    if status != "complete":
        raise DtoBoundaryLedgerError("retired_archives.rust_dto_map_archive.status must be complete")
    retired_guard = require_string(
        retirement.get("path"),
        "retired_archives.rust_dto_map_archive.path",
        DtoBoundaryLedgerError,
    )
    if retired_guard != "spec/openapi/rust-dto-map.yaml":
        raise DtoBoundaryLedgerError(
            "retired_archives.rust_dto_map_archive.path must be spec/openapi/rust-dto-map.yaml"
        )
    former_role = require_string(
        retirement.get("former_role"),
        "retired_archives.rust_dto_map_archive.former_role",
        DtoBoundaryLedgerError,
    )
    if former_role != "retired-openapi-rust-dto-map":
        raise DtoBoundaryLedgerError(
            "retired_archives.rust_dto_map_archive.former_role must be retired-openapi-rust-dto-map"
        )
    milestone = require_string(
        retirement.get("retirement_milestone"),
        "retired_archives.rust_dto_map_archive.retirement_milestone",
        DtoBoundaryLedgerError,
    )
    if milestone != "semantic-compatibility-guard-retirement":
        raise DtoBoundaryLedgerError(
            "retired_archives.rust_dto_map_archive.retirement_milestone must be "
            "semantic-compatibility-guard-retirement"
        )
    completion_rule = require_string(
        retirement.get("completion_rule"),
        "retired_archives.rust_dto_map_archive.completion_rule",
        DtoBoundaryLedgerError,
    )
    if "generated-boundary-adoption" not in completion_rule:
        raise DtoBoundaryLedgerError(
            "retired_archives.rust_dto_map_archive.completion_rule must bind generated-boundary-adoption"
        )
    return ActiveDtoGuards(
        generated_contract_dto_map=generated_contract_dto_map,
        semantic_compatibility_map=semantic_compatibility_map,
        semantic_compatibility_map_validator=semantic_map_validator,
        generated_contract_dto_map_generator=generated_map_generator,
        dto_map_projection=dto_map_projection,
        retirement_milestone=milestone,
        retired_dto_map_archive=retired_guard,
    )


def validate_retirement_exit_criteria(
    root: Path,
    ledger: dict[str, Any],
    active_guards: ActiveDtoGuards,
) -> dict[str, Any]:
    """Validate the final DTO boundary after semantic compatibility retirement."""
    final_boundary = require_mapping(
        ledger.get("final_contract_boundary"),
        "final_contract_boundary",
        DtoBoundaryLedgerError,
    )
    if require_string(
        final_boundary.get("archive_after_retirement"),
        "final_contract_boundary.archive_after_retirement",
        DtoBoundaryLedgerError,
    ) != "retired":
        raise DtoBoundaryLedgerError(
            "final_contract_boundary.archive_after_retirement must be retired"
        )
    retired_guard = require_string(
        final_boundary.get("retired_dto_map_archive"),
        "final_contract_boundary.retired_dto_map_archive",
        DtoBoundaryLedgerError,
    )
    if retired_guard != active_guards.retired_dto_map_archive:
        raise DtoBoundaryLedgerError(
            "final_contract_boundary.retired_dto_map_archive must match retired DTO archive"
        )
    active_guard = require_string(
        final_boundary.get("active_external_guard_after_retirement"),
        "final_contract_boundary.active_external_guard_after_retirement",
        DtoBoundaryLedgerError,
    )
    if active_guard != active_guards.generated_contract_dto_map:
        raise DtoBoundaryLedgerError(
            "final_contract_boundary.active_external_guard_after_retirement must match "
            "active_guards.generated_contract_dto_map"
        )
    temporary_semantic_guard = require_string(
        final_boundary.get("temporary_semantic_guard"),
        "final_contract_boundary.temporary_semantic_guard",
        DtoBoundaryLedgerError,
    )
    if temporary_semantic_guard != active_guards.semantic_compatibility_map:
        raise DtoBoundaryLedgerError(
            "final_contract_boundary.temporary_semantic_guard must match "
            "active_guards.semantic_compatibility_map"
        )
    if require_string(
        final_boundary.get("external_http_dtos"),
        "final_contract_boundary.external_http_dtos",
        DtoBoundaryLedgerError,
    ) != "generated-contract-crates":
        raise DtoBoundaryLedgerError(
            "final_contract_boundary.external_http_dtos must be generated-contract-crates"
        )
    if require_string(
        final_boundary.get("internal_domain_types"),
        "final_contract_boundary.internal_domain_types",
        DtoBoundaryLedgerError,
    ) != "handwritten-domain-and-application-types":
        raise DtoBoundaryLedgerError(
            "final_contract_boundary.internal_domain_types must be handwritten-domain-and-application-types"
        )

    criteria = require_mapping(
        ledger.get("retirement_exit_criteria"),
        "retirement_exit_criteria",
        DtoBoundaryLedgerError,
    )
    required_checks = set(
        require_string_list(
            criteria.get("required_evidence_checks"),
            "retirement_exit_criteria.required_evidence_checks",
            DtoBoundaryLedgerError,
        )
    )
    expected_checks = {
        "openapi-generated-contract-dto-map",
        "openapi-semantic-compatibility-map",
        "openapi-dto-boundary-ledger",
        "openapi-dto-map-projection",
    }
    if required_checks != expected_checks:
        raise DtoBoundaryLedgerError(
            "retirement_exit_criteria.required_evidence_checks drift: "
            f"expected={sorted(expected_checks)}, actual={sorted(required_checks)}"
        )
    for artifact in require_string_list(
        criteria.get("required_artifacts"),
        "retirement_exit_criteria.required_artifacts",
        DtoBoundaryLedgerError,
    ):
        require_existing_path(root, artifact, "retirement_exit_criteria.required_artifacts", DtoBoundaryLedgerError)
    forbidden_guards = set(
        require_string_list(
            criteria.get("forbidden_post_retirement_active_guards"),
            "retirement_exit_criteria.forbidden_post_retirement_active_guards",
            DtoBoundaryLedgerError,
        )
    )
    if forbidden_guards != {active_guards.retired_dto_map_archive}:
        raise DtoBoundaryLedgerError(
            "retirement_exit_criteria.forbidden_post_retirement_active_guards "
            "must contain only the retired DTO map archive"
        )
    return {
        "retired_dto_map_archive": retired_guard,
        "active_external_guard_after_retirement": active_guard,
        "required_evidence_check_count": len(required_checks),
    }


def validate_boundary_ledger(root: Path, ledger_path: Path, dto_map_path: Path) -> dict[str, Any]:
    """Validate the OpenAPI DTO boundary ledger."""
    ledger = load_boundary_ledger(root, ledger_path)
    DtoBoundaryLedger.from_document(ledger_path, ledger)
    validate_target_state(ledger)
    validate_boundary_parity_policy(ledger)
    remaining_compatibility_work_queue = validate_remaining_compatibility_work_queue(ledger)
    required_contract_targets = contract_targets(root, ledger)
    boundary_status_policy = validate_boundary_status_policy(ledger)
    completion_milestones = validate_completion_milestones(
        root,
        ledger,
        required_contract_targets,
    )

    active_guards = validate_active_guards(root, ledger, dto_map_path)
    retirement_exit_criteria = validate_retirement_exit_criteria(root, ledger, active_guards)

    dto_contracts: dict[str, dict[str, Any]] = {}
    for guard_map in (
        active_guards.generated_contract_dto_map,
        active_guards.semantic_compatibility_map,
    ):
        dto_map = load_openapi_dto_map(root, Path(guard_map), DtoBoundaryLedgerError)
        for name, contract in dto_map_contracts_by_name(
            dto_map,
            guard_map,
            DtoBoundaryLedgerError,
        ).items():
            if name in dto_contracts:
                raise DtoBoundaryLedgerError(f"duplicate DTO map contract name: {name}")
            dto_contracts[name] = contract

    contracts = require_list(ledger.get("contracts"), "contracts", DtoBoundaryLedgerError)
    ledger_contracts_by_openapi: dict[str, dict[str, Any]] = {}
    for index, value in enumerate(contracts):
        contract = require_mapping(value, f"contracts[{index}]", DtoBoundaryLedgerError)
        openapi_path = require_string(
            contract.get("openapi"),
            f"contracts[{index}].openapi",
            DtoBoundaryLedgerError,
        )
        previous = ledger_contracts_by_openapi.setdefault(openapi_path, contract)
        if previous is not contract:
            raise DtoBoundaryLedgerError(
                f"duplicate OpenAPI contract in DTO boundary ledger: {openapi_path}"
            )
    handler_adoption = validate_handler_adoption(
        root,
        ledger,
        required_contract_targets,
        ledger_contracts_by_openapi,
    )
    seen_openapi: set[str] = set()
    summaries: list[dict[str, Any]] = []
    for index, value in enumerate(contracts):
        contract = require_mapping(value, f"contracts[{index}]", DtoBoundaryLedgerError)
        summary = validate_contract(
            root,
            contract,
            index,
            dto_contracts,
            required_contract_targets,
            boundary_status_policy,
        )
        openapi_path = summary["openapi"]
        if openapi_path in seen_openapi:
            raise DtoBoundaryLedgerError(f"duplicate OpenAPI contract in DTO boundary ledger: {openapi_path}")
        seen_openapi.add(openapi_path)
        summaries.append(summary)

    expected = set(required_contract_targets)
    if seen_openapi != expected:
        raise DtoBoundaryLedgerError(
            "DTO boundary ledger OpenAPI contract set drift: "
            f"expected={sorted(expected)} actual={sorted(seen_openapi)}"
        )

    return {
        "ledger": str(ledger_path),
        "semantic_compatibility_map": str(dto_map_path),
        "contract_count": len(summaries),
        "contracts": summaries,
        "completion_milestones": completion_milestones,
        "handler_adoption": handler_adoption,
        "remaining_compatibility_work_queue": remaining_compatibility_work_queue,
        "active_guards": active_guards.as_mapping(),
        "retirement_exit_criteria": retirement_exit_criteria,
    }
