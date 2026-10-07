#!/usr/bin/env python3
"""Validate generated ClaimGraph reference integrity."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string
from kiva_ci.projection import require_envelope


DEFAULT_SOURCE = Path("spec/architecture/claim-graph.source.yaml")
DEFAULT_GRAPH = Path("generated/architecture/claim-graph.yaml")
DEFAULT_RECEIPTS = Path("generated/evidence/evidence-receipts.yaml")
DEFAULT_EXECUTED = Path("generated/evidence/executed-receipt-overlay.yaml")


class ClaimGraphValidationError(RuntimeError):
    """Raised when ClaimGraph reference validation fails."""


def receipt_ids(receipts: dict[str, Any], field: str) -> set[str]:
    """Return receipt ids from one receipt index field."""
    ids: set[str] = set()
    for index, raw_receipt in enumerate(
        require_list(receipts.get(field), f"receipts.{field}", ClaimGraphValidationError)
    ):
        receipt = require_mapping(raw_receipt, f"receipts.{field}[{index}]", ClaimGraphValidationError)
        receipt_id = require_string(receipt.get("id"), f"receipts.{field}[{index}].id", ClaimGraphValidationError)
        if receipt_id in ids:
            raise ClaimGraphValidationError(f"duplicate {field} id: {receipt_id}")
        ids.add(receipt_id)
    return ids


def executed_receipt_ids(overlay: dict[str, Any]) -> set[str]:
    """Return receipt ids marked executed by the overlay."""
    ids: set[str] = set()
    for index, raw_receipt in enumerate(
        require_list(overlay.get("executed_receipts"), "executed_overlay.executed_receipts", ClaimGraphValidationError)
    ):
        receipt = require_mapping(raw_receipt, f"executed_overlay.executed_receipts[{index}]", ClaimGraphValidationError)
        receipt_id = require_string(
            receipt.get("receipt_id"),
            f"executed_overlay.executed_receipts[{index}].receipt_id",
            ClaimGraphValidationError,
        )
        if receipt_id in ids:
            raise ClaimGraphValidationError(f"duplicate executed receipt id: {receipt_id}")
        ids.add(receipt_id)
    return ids


def verification_aliases(source: dict[str, Any]) -> tuple[dict[str, str], set[str]]:
    """Return mechanism verification aliases and explicit exceptions."""
    aliases: dict[str, str] = {}
    for key, value in require_mapping(
        source.get("verification_aliases", {}),
        "claim_graph_source.verification_aliases",
        ClaimGraphValidationError,
    ).items():
        if not isinstance(key, str) or not key:
            raise ClaimGraphValidationError("verification_aliases keys must be non-empty strings")
        aliases[key] = require_string(value, f"verification_aliases.{key}", ClaimGraphValidationError)
    exceptions = {
        require_string(
            require_mapping(row, f"verification_exceptions[{index}]", ClaimGraphValidationError).get("mechanism"),
            f"verification_exceptions[{index}].mechanism",
            ClaimGraphValidationError,
        )
        for index, row in enumerate(
            require_list(source.get("verification_exceptions", []), "verification_exceptions", ClaimGraphValidationError)
        )
    }
    return aliases, exceptions


def validate_claims(
    graph: dict[str, Any],
    evidence_receipts: set[str],
    executed: set[str],
) -> None:
    """Validate claim-to-evidence edges."""
    for index, raw_claim in enumerate(require_list(graph.get("claims"), "graph.claims", ClaimGraphValidationError)):
        claim = require_mapping(raw_claim, f"graph.claims[{index}]", ClaimGraphValidationError)
        claim_id = require_string(claim.get("id"), f"graph.claims[{index}].id", ClaimGraphValidationError)
        readiness = require_string(claim.get("readiness"), f"graph.claims[{index}].readiness", ClaimGraphValidationError)
        edges = require_list(claim.get("evidence_edges"), f"graph.claims[{index}].evidence_edges", ClaimGraphValidationError)
        if readiness == "ready" and not edges:
            raise ClaimGraphValidationError(f"ready claim has no evidence edges: {claim_id}")
        for edge_index, raw_edge in enumerate(edges):
            edge = require_mapping(raw_edge, f"graph.claims[{index}].evidence_edges[{edge_index}]", ClaimGraphValidationError)
            receipt_id = require_string(
                edge.get("receipt_id"),
                f"graph.claims[{index}].evidence_edges[{edge_index}].receipt_id",
                ClaimGraphValidationError,
            )
            if receipt_id not in evidence_receipts:
                raise ClaimGraphValidationError(f"claim edge references undeclared EvidenceReceipt: {claim_id} -> {receipt_id}")
            if bool(edge.get("executed")) and receipt_id not in executed:
                raise ClaimGraphValidationError(f"claim edge is executed but missing overlay receipt: {claim_id} -> {receipt_id}")


def validate_verification_receipts(graph: dict[str, Any], verification_receipts: set[str]) -> None:
    """Validate verification discharge receipt ids."""
    for index, raw_obligation in enumerate(
        require_list(graph.get("verification_obligations"), "graph.verification_obligations", ClaimGraphValidationError)
    ):
        obligation = require_mapping(raw_obligation, f"graph.verification_obligations[{index}]", ClaimGraphValidationError)
        obligation_id = require_string(obligation.get("id"), f"graph.verification_obligations[{index}].id", ClaimGraphValidationError)
        for discharge_index, raw_discharge in enumerate(
            require_list(
                obligation.get("declared_discharges"),
                f"graph.verification_obligations[{index}].declared_discharges",
                ClaimGraphValidationError,
            )
        ):
            discharge = require_mapping(
                raw_discharge,
                f"graph.verification_obligations[{index}].declared_discharges[{discharge_index}]",
                ClaimGraphValidationError,
            )
            receipt_id = require_string(
                discharge.get("receipt_id"),
                f"graph.verification_obligations[{index}].declared_discharges[{discharge_index}].receipt_id",
                ClaimGraphValidationError,
            )
            if receipt_id not in verification_receipts:
                raise ClaimGraphValidationError(f"verification discharge references undeclared VerificationReceipt: {obligation_id} -> {receipt_id}")


def validate_capability_verification_links(
    source: dict[str, Any],
    graph: dict[str, Any],
) -> None:
    """Validate that capability-bound mechanisms have verification coverage or explicit exceptions."""
    aliases, exceptions = verification_aliases(source)
    verification_mechanisms = {
        require_string(
            require_mapping(raw, f"graph.verification_obligations[{index}]", ClaimGraphValidationError).get("mechanism"),
            f"graph.verification_obligations[{index}].mechanism",
            ClaimGraphValidationError,
        )
        for index, raw in enumerate(
            require_list(graph.get("verification_obligations"), "graph.verification_obligations", ClaimGraphValidationError)
        )
    }
    for index, raw_binding in enumerate(
        require_list(graph.get("capability_bindings"), "graph.capability_bindings", ClaimGraphValidationError)
    ):
        binding = require_mapping(raw_binding, f"graph.capability_bindings[{index}]", ClaimGraphValidationError)
        mechanism = require_string(
            binding.get("maturity_mechanism"),
            f"graph.capability_bindings[{index}].maturity_mechanism",
            ClaimGraphValidationError,
        )
        if mechanism in exceptions:
            continue
        verification_key = aliases.get(mechanism, mechanism)
        if verification_key not in verification_mechanisms:
            raise ClaimGraphValidationError(
                f"capability binding has no verification obligation or explicit exception: {mechanism}"
            )


def validate_claim_graph(source_path: Path, graph_path: Path, receipts_path: Path, executed_path: Path) -> None:
    """Validate the generated ClaimGraph."""
    source = load_yaml(source_path, ClaimGraphValidationError)
    require_envelope(
        source,
        path=source_path,
        kind="kiva-claim-graph-source",
        error_cls=ClaimGraphValidationError,
    )
    graph = load_yaml(graph_path, ClaimGraphValidationError)
    require_envelope(
        graph,
        path=graph_path,
        kind="kiva-claim-graph",
        error_cls=ClaimGraphValidationError,
    )
    receipts = load_yaml(receipts_path, ClaimGraphValidationError)
    overlay = load_yaml(executed_path, ClaimGraphValidationError)
    evidence_receipts = receipt_ids(receipts, "evidence_receipts")
    verification_receipts = receipt_ids(receipts, "verification_receipts")
    executed = executed_receipt_ids(overlay)
    unknown_executed = sorted(executed.difference(evidence_receipts).difference(verification_receipts))
    if unknown_executed:
        raise ClaimGraphValidationError(f"executed overlay references unknown receipts: {unknown_executed}")
    validate_claims(graph, evidence_receipts, executed)
    validate_verification_receipts(graph, verification_receipts)
    validate_capability_verification_links(source, graph)


def main() -> int:
    """Validate generated ClaimGraph reference integrity."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--graph", type=Path, default=DEFAULT_GRAPH)
    parser.add_argument("--receipts", type=Path, default=DEFAULT_RECEIPTS)
    parser.add_argument("--executed", type=Path, default=DEFAULT_EXECUTED)
    args = parser.parse_args()

    validate_claim_graph(args.source, args.graph, args.receipts, args.executed)
    print("validated ClaimGraph reference integrity")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
