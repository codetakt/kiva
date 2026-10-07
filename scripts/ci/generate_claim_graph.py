#!/usr/bin/env python3
"""Generate the Kiva ClaimGraph review projection."""

from __future__ import annotations

import argparse
from collections import defaultdict
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.ledger import validate_json_schema
from kiva_ci.manifest import (
    load_json,
    load_yaml,
    require_list,
    require_mapping,
    require_string,
)
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_SOURCE = Path("spec/architecture/claim-graph.source.yaml")
DEFAULT_OUTPUT = Path("generated/architecture/claim-graph.yaml")
DEFAULT_MARKDOWN = Path("docs/architecture/generated/claim-graph.md")
DEFAULT_SCHEMA = Path("spec/architecture/schemas/claim-graph.schema.json")


class ClaimGraphGenerationError(RuntimeError):
    """Raised when ClaimGraph generation fails."""


def evidence_checks_by_ref(evidence_set: dict[str, Any]) -> dict[tuple[str, str], dict[str, Any]]:
    """Return EvidenceSet check rows keyed by group/check."""
    checks: dict[tuple[str, str], dict[str, Any]] = {}
    for group_index, raw_group in enumerate(
        require_list(evidence_set.get("groups"), "evidence_set.groups", ClaimGraphGenerationError)
    ):
        group = require_mapping(raw_group, f"evidence_set.groups[{group_index}]", ClaimGraphGenerationError)
        group_id = require_string(group.get("id"), f"evidence_set.groups[{group_index}].id", ClaimGraphGenerationError)
        for check_index, raw_check in enumerate(
            require_list(group.get("checks"), f"evidence_set.groups[{group_index}].checks", ClaimGraphGenerationError)
        ):
            check = require_mapping(
                raw_check,
                f"evidence_set.groups[{group_index}].checks[{check_index}]",
                ClaimGraphGenerationError,
            )
            check_id = require_string(check.get("id"), f"{group_id}.checks[{check_index}].id", ClaimGraphGenerationError)
            checks[(group_id, check_id)] = check
    return checks


def executed_receipt_ids(overlay: dict[str, Any]) -> set[str]:
    """Return receipt ids with an executed overlay row."""
    ids: set[str] = set()
    for index, raw_receipt in enumerate(
        require_list(overlay.get("executed_receipts"), "executed_overlay.executed_receipts", ClaimGraphGenerationError)
    ):
        receipt = require_mapping(raw_receipt, f"executed_overlay.executed_receipts[{index}]", ClaimGraphGenerationError)
        ids.add(
            require_string(
                receipt.get("receipt_id"),
                f"executed_overlay.executed_receipts[{index}].receipt_id",
                ClaimGraphGenerationError,
            )
        )
    return ids


def verification_nodes(obligations: dict[str, Any]) -> list[dict[str, Any]]:
    """Return compact verification obligation graph nodes."""
    nodes: list[dict[str, Any]] = []
    for index, raw_obligation in enumerate(
        require_list(obligations.get("obligations"), "verification_obligations.obligations", ClaimGraphGenerationError)
    ):
        obligation = require_mapping(
            raw_obligation,
            f"verification_obligations.obligations[{index}]",
            ClaimGraphGenerationError,
        )
        discharges = []
        for raw_discharge in require_list(
            obligation.get("declared_discharges"),
            f"verification_obligations.obligations[{index}].declared_discharges",
            ClaimGraphGenerationError,
        ):
            discharge = require_mapping(raw_discharge, "declared_discharges[]", ClaimGraphGenerationError)
            discharges.append(
                {
                    "method": require_string(discharge.get("method"), "declared_discharges[].method", ClaimGraphGenerationError),
                    "receipt_id": require_string(discharge.get("receipt_id"), "declared_discharges[].receipt_id", ClaimGraphGenerationError),
                    "status": require_string(discharge.get("status"), "declared_discharges[].status", ClaimGraphGenerationError),
                }
            )
        nodes.append(
            {
                "id": require_string(obligation.get("id"), f"verification_obligations.obligations[{index}].id", ClaimGraphGenerationError),
                "mechanism": require_string(obligation.get("mechanism"), f"verification_obligations.obligations[{index}].mechanism", ClaimGraphGenerationError),
                "complete": bool(obligation.get("complete")),
                "declared_discharges": discharges,
            }
        )
    return nodes


def capability_nodes(bindings: dict[str, Any]) -> list[dict[str, Any]]:
    """Return compact capability binding graph nodes."""
    nodes: list[dict[str, Any]] = []
    for index, raw_binding in enumerate(
        require_list(bindings.get("bindings"), "capability_bindings.bindings", ClaimGraphGenerationError)
    ):
        binding = require_mapping(raw_binding, f"capability_bindings.bindings[{index}]", ClaimGraphGenerationError)
        nodes.append(
            {
                "id": require_string(binding.get("id"), f"capability_bindings.bindings[{index}].id", ClaimGraphGenerationError),
                "maturity_mechanism": require_string(
                    binding.get("maturity_mechanism"),
                    f"capability_bindings.bindings[{index}].maturity_mechanism",
                    ClaimGraphGenerationError,
                ),
                "capability": require_string(
                    binding.get("capability"),
                    f"capability_bindings.bindings[{index}].capability",
                    ClaimGraphGenerationError,
                ),
                "binding_kind": require_string(
                    binding.get("binding_kind"),
                    f"capability_bindings.bindings[{index}].binding_kind",
                    ClaimGraphGenerationError,
                ),
                "operation_count": int(binding.get("operation_count", 0)),
            }
        )
    return nodes


def claim_nodes(
    readiness: dict[str, Any],
    checks_by_ref: dict[tuple[str, str], dict[str, Any]],
    executed: set[str],
) -> list[dict[str, Any]]:
    """Return claim graph nodes enriched with evidence receipt edges."""
    claims: list[dict[str, Any]] = []
    for index, raw_claim in enumerate(
        require_list(readiness.get("claims"), "claim_readiness.claims", ClaimGraphGenerationError)
    ):
        claim = require_mapping(raw_claim, f"claim_readiness.claims[{index}]", ClaimGraphGenerationError)
        evidence_edges = []
        for raw_check in require_list(
            claim.get("matched_evidence_checks", []),
            f"claim_readiness.claims[{index}].matched_evidence_checks",
            ClaimGraphGenerationError,
        ):
            check = require_mapping(raw_check, "matched_evidence_checks[]", ClaimGraphGenerationError)
            group_id = require_string(check.get("group"), "matched_evidence_checks[].group", ClaimGraphGenerationError)
            check_id = require_string(check.get("check_id"), "matched_evidence_checks[].check_id", ClaimGraphGenerationError)
            evidence_row = checks_by_ref.get((group_id, check_id), {})
            receipt_id = evidence_row.get("receipt_id", f"evidence-receipt.{group_id}.{check_id}")
            evidence_edges.append(
                {
                    "group": group_id,
                    "check_id": check_id,
                    "receipt_id": receipt_id,
                    "check_kind": require_string(check.get("check_kind"), "matched_evidence_checks[].check_kind", ClaimGraphGenerationError),
                    "artifact_class": require_string(check.get("artifact_class"), "matched_evidence_checks[].artifact_class", ClaimGraphGenerationError),
                    "executed": receipt_id in executed,
                }
            )
        claims.append(
            {
                "id": require_string(claim.get("id"), f"claim_readiness.claims[{index}].id", ClaimGraphGenerationError),
                "boundary": require_string(claim.get("boundary"), f"claim_readiness.claims[{index}].boundary", ClaimGraphGenerationError),
                "readiness": require_string(claim.get("readiness"), f"claim_readiness.claims[{index}].readiness", ClaimGraphGenerationError),
                "claim_maturity": require_string(
                    claim.get("claim_maturity"),
                    f"claim_readiness.claims[{index}].claim_maturity",
                    ClaimGraphGenerationError,
                ),
                "matched_projection_ids": [
                    str(value)
                    for value in require_list(
                        claim.get("matched_projection_ids", []),
                        f"claim_readiness.claims[{index}].matched_projection_ids",
                        ClaimGraphGenerationError,
                    )
                ],
                "evidence_edges": evidence_edges,
            }
        )
    return claims


def graph_document(root: Path, source_path: Path, output_path: Path, markdown_path: Path) -> dict[str, Any]:
    """Return the ClaimGraph document."""
    source = load_yaml(root / source_path, ClaimGraphGenerationError)
    if source.get("schema_version") != 1:
        raise ClaimGraphGenerationError(f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-claim-graph-source":
        raise ClaimGraphGenerationError(f"{source_path}.kind must be kiva-claim-graph-source")
    if source.get("status") != "active":
        raise ClaimGraphGenerationError(f"{source_path}.status must be active")
    inputs = require_mapping(source.get("inputs"), f"{source_path}.inputs", ClaimGraphGenerationError)
    input_paths = {
        key: Path(require_string(inputs.get(key), f"{source_path}.inputs.{key}", ClaimGraphGenerationError))
        for key in (
            "claim_readiness",
            "evidence_set",
            "evidence_receipts",
            "executed_receipt_overlay",
            "verification_obligations",
            "capability_bindings",
        )
    }
    readiness = load_yaml(root / input_paths["claim_readiness"], ClaimGraphGenerationError)
    evidence_set = load_yaml(root / input_paths["evidence_set"], ClaimGraphGenerationError)
    evidence_receipts = load_yaml(root / input_paths["evidence_receipts"], ClaimGraphGenerationError)
    executed_overlay = load_yaml(root / input_paths["executed_receipt_overlay"], ClaimGraphGenerationError)
    obligations = load_yaml(root / input_paths["verification_obligations"], ClaimGraphGenerationError)
    bindings = load_yaml(root / input_paths["capability_bindings"], ClaimGraphGenerationError)
    checks_by_ref = evidence_checks_by_ref(evidence_set)
    claims = claim_nodes(readiness, checks_by_ref, executed_receipt_ids(executed_overlay))
    capabilities = capability_nodes(bindings)
    verification = verification_nodes(obligations)
    verification_by_mechanism: dict[str, int] = defaultdict(int)
    for node in verification:
        verification_by_mechanism[node["mechanism"]] += 1
    return {
        "schema_version": 1,
        "kind": "kiva-claim-graph",
        "status": "active",
        "description": require_string(source.get("description"), f"{source_path}.description", ClaimGraphGenerationError),
        "generated_from": [source_path.as_posix(), *(path.as_posix() for path in input_paths.values())],
        "generated_artifacts": {
            "claim_graph": output_path.as_posix(),
            "markdown": markdown_path.as_posix(),
        },
        "graph_model": require_mapping(source.get("graph_model"), f"{source_path}.graph_model", ClaimGraphGenerationError),
        "summary": {
            "claim_count": len(claims),
            "ready_claim_count": sum(1 for claim in claims if claim["readiness"] == "ready"),
            "evidence_edge_count": sum(len(claim["evidence_edges"]) for claim in claims),
            "declared_evidence_receipt_count": len(
                require_list(evidence_receipts.get("evidence_receipts"), "evidence_receipts.evidence_receipts", ClaimGraphGenerationError)
            ),
            "executed_evidence_receipt_count": len(executed_receipt_ids(executed_overlay)),
            "capability_binding_count": len(capabilities),
            "verification_obligation_count": len(verification),
            "verified_mechanism_count": len(verification_by_mechanism),
        },
        "claims": claims,
        "capability_bindings": capabilities,
        "verification_obligations": verification,
    }


def validate_schema(document: dict[str, Any], schema_path: Path) -> None:
    """Validate the generated ClaimGraph."""
    validate_json_schema(
        document,
        load_json(schema_path, ClaimGraphGenerationError),
        label=schema_path.as_posix(),
        error_cls=ClaimGraphGenerationError,
    )


def markdown_cell(value: object) -> str:
    """Render a Markdown table cell."""
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_markdown(document: dict[str, Any]) -> str:
    """Render a compact ClaimGraph review table."""
    summary = require_mapping(document.get("summary"), "summary", ClaimGraphGenerationError)
    rows = [
        "| Contract | Boundary | Evidence Edges |",
        "| --- | --- | ---: |",
    ]
    for claim in require_list(document.get("claims"), "claims", ClaimGraphGenerationError):
        row = require_mapping(claim, "claims[]", ClaimGraphGenerationError)
        rows.append(
            "| "
            + " | ".join(
                [
                    f"`{markdown_cell(row['id'])}`",
                    f"`{markdown_cell(row['boundary'])}`",
                    str(len(require_list(row.get("evidence_edges"), "claims[].evidence_edges", ClaimGraphGenerationError))),
                ]
            )
            + " |"
        )
    return f"""# Generated ClaimGraph

This file is generated by `scripts/ci/generate_claim_graph.py`.

ClaimGraph joins public claims, CI evidence rows, declared receipts, optional
executed receipt overlays, confidential capability bindings, and verification
obligations into one read-only review projection.

- Claims: {summary['claim_count']}
- Evidence edges: {summary['evidence_edge_count']}
- Capability bindings: {summary['capability_binding_count']}
- Verification obligations: {summary['verification_obligation_count']}

{chr(10).join(rows)}
"""


def main() -> int:
    """Generate or validate ClaimGraph."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    document = graph_document(root, args.source, args.out, args.markdown_out)
    validate_schema(document, root / args.schema)
    rendered_yaml = dump_yaml(document)
    rendered_markdown = render_markdown(document)
    command_hint = "scripts/ci/generate_claim_graph.py"
    write_or_check_generated_artifacts(
        [
            GeneratedArtifact(root / args.out, rendered_yaml),
            GeneratedArtifact(root / args.markdown_out, rendered_markdown),
        ],
        check=args.check,
        command_hint=command_hint,
        error_cls=ClaimGraphGenerationError,
    )
    if args.check:
        print(f"validated generated ClaimGraph: {args.out}, {args.markdown_out}")
        return 0
    print(f"generated ClaimGraph: {args.out}, {args.markdown_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
