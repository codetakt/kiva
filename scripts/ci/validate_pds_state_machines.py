#!/usr/bin/env python3
"""Validate the PDS state-machine source ledger."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.evidence import load_evidence_check_manifest
from kiva_ci.manifest import (
    load_yaml,
    mapping_by_unique_string_key,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
    validation_error,
)


DEFAULT_SOURCE = Path("spec/pds/state-machines.source.yaml")
DEFAULT_CORE_PLANES = Path("spec/pds/core-planes.source.yaml")
DEFAULT_INVARIANTS = Path("spec/pds/invariants.source.yaml")
DEFAULT_EVIDENCE_MANIFEST = Path("spec/ci/evidence-checks.yaml")


class PdsStateMachineError(RuntimeError):
    """Raised when the PDS state-machine ledger is inconsistent."""


def evidence_check_ids(manifest: dict[str, Any]) -> set[str]:
    """Return all evidence check ids from the generated manifest."""
    ids: set[str] = set()
    for group_index, raw_group in enumerate(require_list(manifest.get("groups"), "groups", PdsStateMachineError)):
        group = require_mapping(raw_group, f"groups[{group_index}]", PdsStateMachineError)
        group_id = require_string(group.get("id"), f"groups[{group_index}].id", PdsStateMachineError)
        for check_index, raw_check in enumerate(
            require_list(group.get("checks"), f"groups.{group_id}.checks", PdsStateMachineError)
        ):
            check = require_mapping(raw_check, f"groups.{group_id}.checks[{check_index}]", PdsStateMachineError)
            ids.add(require_string(check.get("id"), f"{group_id}.checks[{check_index}].id", PdsStateMachineError))
    return ids


def core_plane_ids(core_planes: dict[str, Any]) -> set[str]:
    """Return PDS core-plane ids."""
    rows = mapping_by_unique_string_key(
        core_planes.get("planes"),
        key="id",
        label="core_planes.planes",
        error_cls=PdsStateMachineError,
    )
    return set(rows)


def invariant_ids(invariants: dict[str, Any]) -> set[str]:
    """Return PDS invariant ids."""
    rows = mapping_by_unique_string_key(
        invariants.get("invariants"),
        key="id",
        label="pds_invariants.invariants",
        error_cls=PdsStateMachineError,
    )
    return set(rows)


def validate_envelope(source: dict[str, Any], source_path: Path) -> list[str]:
    """Validate source envelope and return required machine ids."""
    if source.get("schema_version") != 1:
        raise validation_error(PdsStateMachineError, f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-pds-state-machines-source":
        raise validation_error(PdsStateMachineError, f"{source_path}.kind must be kiva-pds-state-machines-source")
    if source.get("status") != "active":
        raise validation_error(PdsStateMachineError, f"{source_path}.status must be active")
    model = require_mapping(source.get("state_machine_model"), f"{source_path}.state_machine_model", PdsStateMachineError)
    if model.get("root_type") != "PdsStateMachine":
        raise validation_error(PdsStateMachineError, f"{source_path}.state_machine_model.root_type must be PdsStateMachine")
    if model.get("parent_root_type") != "PdsCorePlane":
        raise validation_error(PdsStateMachineError, f"{source_path}.state_machine_model.parent_root_type must be PdsCorePlane")
    if model.get("parent_model") != "operation":
        raise validation_error(PdsStateMachineError, f"{source_path}.state_machine_model.parent_model must be operation")
    require_string(model.get("rule"), f"{source_path}.state_machine_model.rule", PdsStateMachineError)
    return require_string_list(
        model.get("required_machine_ids"),
        f"{source_path}.state_machine_model.required_machine_ids",
        PdsStateMachineError,
    )


def validate_transition(machine_id: str, transition: dict[str, Any], states: set[str], terminal_states: set[str], label: str) -> None:
    """Validate one state transition row."""
    from_state = require_string(transition.get("from"), f"{label}.from", PdsStateMachineError)
    to_state = require_string(transition.get("to"), f"{label}.to", PdsStateMachineError)
    require_string(transition.get("event"), f"{label}.event", PdsStateMachineError)
    if from_state != "none" and from_state not in states:
        raise validation_error(PdsStateMachineError, f"{machine_id} transition from unknown state: {from_state}")
    if to_state not in states:
        raise validation_error(PdsStateMachineError, f"{machine_id} transition to unknown state: {to_state}")
    if from_state in terminal_states and from_state != to_state:
        raise validation_error(PdsStateMachineError, f"{machine_id} terminal state {from_state} must be idempotent")


def validate_required_terms(root: Path, anchors: list[str], terms: list[str], machine_id: str) -> None:
    """Validate required implementation terms appear in at least one anchor."""
    anchor_text = ""
    for anchor in anchors:
        anchor_path = root / anchor
        if not anchor_path.exists():
            raise validation_error(PdsStateMachineError, f"{machine_id} missing implementation anchor: {anchor}")
        anchor_text += anchor_path.read_text(encoding="utf-8")
        anchor_text += "\n"
    for term in terms:
        if term not in anchor_text:
            raise validation_error(PdsStateMachineError, f"{machine_id} required term is missing from anchors: {term}")


def validate_machine(
    root: Path,
    machine_id: str,
    machine: dict[str, Any],
    known_planes: set[str],
    known_invariants: set[str],
    known_checks: set[str],
) -> set[str]:
    """Validate one PDS state-machine row and return covered planes."""
    require_string(machine.get("domain_object"), f"{machine_id}.domain_object", PdsStateMachineError)
    states = set(require_string_list(machine.get("states"), f"{machine_id}.states", PdsStateMachineError))
    if len(states) < 2:
        raise validation_error(PdsStateMachineError, f"{machine_id}.states must contain at least two states")
    initial_states = set(require_string_list(machine.get("initial_states"), f"{machine_id}.initial_states", PdsStateMachineError))
    terminal_states = set(require_string_list(machine.get("terminal_states"), f"{machine_id}.terminal_states", PdsStateMachineError))
    idempotent_terminal_states = set(
        require_string_list(machine.get("idempotent_terminal_states"), f"{machine_id}.idempotent_terminal_states", PdsStateMachineError)
    )
    for label, subset in (
        ("initial_states", initial_states),
        ("terminal_states", terminal_states),
        ("idempotent_terminal_states", idempotent_terminal_states),
    ):
        unknown = sorted(subset.difference(states))
        if unknown:
            raise validation_error(PdsStateMachineError, f"{machine_id}.{label} references unknown states: {unknown}")
    missing_idempotent = sorted(terminal_states.difference(idempotent_terminal_states))
    if missing_idempotent:
        raise validation_error(PdsStateMachineError, f"{machine_id} terminal states must be idempotent: {missing_idempotent}")

    covered_planes = set(require_string_list(machine.get("plane_refs"), f"{machine_id}.plane_refs", PdsStateMachineError))
    unknown_planes = sorted(covered_planes.difference(known_planes))
    if unknown_planes:
        raise validation_error(PdsStateMachineError, f"{machine_id} references unknown PDS planes: {unknown_planes}")
    unknown_invariants = sorted(
        set(require_string_list(machine.get("invariant_refs"), f"{machine_id}.invariant_refs", PdsStateMachineError)).difference(known_invariants)
    )
    if unknown_invariants:
        raise validation_error(PdsStateMachineError, f"{machine_id} references unknown PDS invariants: {unknown_invariants}")
    for check_id in require_string_list(machine.get("required_checks"), f"{machine_id}.required_checks", PdsStateMachineError):
        if check_id not in known_checks:
            raise validation_error(PdsStateMachineError, f"{machine_id} references missing evidence check: {check_id}")

    transitions = require_list(machine.get("transitions"), f"{machine_id}.transitions", PdsStateMachineError)
    if not transitions:
        raise validation_error(PdsStateMachineError, f"{machine_id}.transitions must not be empty")
    transition_pairs: set[tuple[str, str, str]] = set()
    for index, raw_transition in enumerate(transitions):
        transition = require_mapping(raw_transition, f"{machine_id}.transitions[{index}]", PdsStateMachineError)
        validate_transition(machine_id, transition, states, terminal_states, f"{machine_id}.transitions[{index}]")
        key = (
            str(transition.get("from")),
            str(transition.get("to")),
            str(transition.get("event")),
        )
        if key in transition_pairs:
            raise validation_error(PdsStateMachineError, f"{machine_id} duplicates transition: {key}")
        transition_pairs.add(key)
    from_none_targets = {to_state for from_state, to_state, _ in transition_pairs if from_state == "none"}
    missing_initial_transitions = sorted(initial_states.difference(from_none_targets))
    if missing_initial_transitions:
        raise validation_error(PdsStateMachineError, f"{machine_id} initial states have no none transition: {missing_initial_transitions}")

    anchors = require_string_list(machine.get("implementation_anchors"), f"{machine_id}.implementation_anchors", PdsStateMachineError)
    terms = require_string_list(machine.get("required_terms"), f"{machine_id}.required_terms", PdsStateMachineError)
    validate_required_terms(root, anchors, terms, machine_id)
    return covered_planes


def validate_pds_state_machines(root: Path, source_path: Path) -> tuple[int, int]:
    """Validate the PDS state-machine ledger."""
    source = load_yaml(root / source_path, PdsStateMachineError)
    core_planes_path = Path(require_string(source.get("core_planes"), f"{source_path}.core_planes", PdsStateMachineError))
    invariants_path = Path(require_string(source.get("invariant_source"), f"{source_path}.invariant_source", PdsStateMachineError))
    manifest_path = Path(require_string(source.get("evidence_manifest"), f"{source_path}.evidence_manifest", PdsStateMachineError))
    core_planes = load_yaml(root / core_planes_path, PdsStateMachineError)
    invariants = load_yaml(root / invariants_path, PdsStateMachineError)
    manifest = load_evidence_check_manifest(root / manifest_path, PdsStateMachineError)
    required_ids = validate_envelope(source, source_path)
    machines = mapping_by_unique_string_key(
        source.get("machines"),
        key="id",
        label=f"{source_path}.machines",
        error_cls=PdsStateMachineError,
    )
    if list(machines) != required_ids:
        raise validation_error(PdsStateMachineError, f"PDS state-machine ids must be {required_ids}")
    known_planes = core_plane_ids(core_planes)
    known_invariants = invariant_ids(invariants)
    known_checks = evidence_check_ids(manifest)
    covered_planes: set[str] = set()
    for machine_id, machine in machines.items():
        covered_planes.update(validate_machine(root, machine_id, machine, known_planes, known_invariants, known_checks))
    missing_planes = sorted(known_planes.difference(covered_planes))
    if missing_planes:
        raise validation_error(PdsStateMachineError, f"PDS state machines do not cover planes: {missing_planes}")
    return len(machines), len(covered_planes)


def main() -> int:
    """Run PDS state-machine validation."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    args = parser.parse_args()

    machine_count, plane_count = validate_pds_state_machines(args.root.resolve(), args.source)
    print(f"validated PDS state machines: {machine_count} machines, {plane_count} planes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
