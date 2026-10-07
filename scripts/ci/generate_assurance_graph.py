#!/usr/bin/env python3
"""Generate a unified AssuranceGraph from claim/evidence/receipt projections."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_CLAIM_GRAPH = Path("generated/architecture/claim-graph.yaml")
DEFAULT_ASSURANCE_PRIMITIVE = Path("spec/evidence/assurance-primitive.source.yaml")
DEFAULT_OUTPUT = Path("generated/architecture/assurance-graph.yaml")
DEFAULT_MARKDOWN = Path("docs/architecture/generated/assurance-graph.md")
class AssuranceGraphError(RuntimeError):
    """Raised when AssuranceGraph generation fails."""


def authority_policy(source: dict[str, Any]) -> dict[str, dict[str, str]]:
    """Return source-owned authority policy rows keyed by node type."""
    model = require_mapping(source.get("authority_model"), "assurance_primitive.authority_model", AssuranceGraphError)
    rows: dict[str, dict[str, str]] = {}
    for index, raw_row in enumerate(require_list(model.get("node_types"), "authority_model.node_types", AssuranceGraphError)):
        row = require_mapping(raw_row, f"authority_model.node_types[{index}]", AssuranceGraphError)
        node_type = require_string(row.get("node_type"), f"authority_model.node_types[{index}].node_type", AssuranceGraphError)
        if node_type in rows:
            raise AssuranceGraphError(f"duplicate authority node type: {node_type}")
        rows[node_type] = {
            "policy_key": require_string(row.get("policy_key"), f"authority_model.{node_type}.policy_key", AssuranceGraphError),
            "authority_level": require_string(row.get("authority_level"), f"authority_model.{node_type}.authority_level", AssuranceGraphError),
            "rule": require_string(row.get("rule"), f"authority_model.{node_type}.rule", AssuranceGraphError),
        }
    required = {"claim", "evidence-receipt", "capability", "verification-obligation"}
    if set(rows) != required:
        raise AssuranceGraphError(f"authority node types must be {sorted(required)}")
    return rows


def node(id: str, node_type: str, status: object, policy: dict[str, dict[str, str]]) -> dict[str, Any]:
    """Return one AssuranceGraph node with its authority level."""
    return {
        "id": id,
        "node_type": node_type,
        "authority_level": policy[node_type]["authority_level"],
        "status": status,
    }


def graph_document(root: Path, claim_graph_path: Path, assurance_primitive_path: Path, output_path: Path, markdown_path: Path) -> dict[str, Any]:
    """Return the generated AssuranceGraph document."""
    claim_graph = load_yaml(root / claim_graph_path, AssuranceGraphError)
    assurance_primitive = load_yaml(root / assurance_primitive_path, AssuranceGraphError)
    policy = authority_policy(assurance_primitive)
    claims = require_list(claim_graph.get("claims"), "claim_graph.claims", AssuranceGraphError)
    capabilities = require_list(claim_graph.get("capability_bindings"), "claim_graph.capability_bindings", AssuranceGraphError)
    verifications = require_list(claim_graph.get("verification_obligations"), "claim_graph.verification_obligations", AssuranceGraphError)
    nodes: list[dict[str, Any]] = []
    edges: list[dict[str, Any]] = []
    for raw_claim in claims:
        claim = require_mapping(raw_claim, "claim_graph.claims[]", AssuranceGraphError)
        claim_id = require_string(claim.get("id"), "claim.id", AssuranceGraphError)
        nodes.append(node(f"claim:{claim_id}", "claim", claim.get("readiness"), policy))
        for raw_edge in require_list(claim.get("evidence_edges"), f"claim.{claim_id}.evidence_edges", AssuranceGraphError):
            edge = require_mapping(raw_edge, "claim.evidence_edges[]", AssuranceGraphError)
            receipt_id = require_string(edge.get("receipt_id"), "evidence_edge.receipt_id", AssuranceGraphError)
            nodes.append(node(f"receipt:{receipt_id}", "evidence-receipt", "executed" if edge.get("executed") else "declared", policy))
            edges.append({"from": f"claim:{claim_id}", "to": f"receipt:{receipt_id}", "edge_type": "supported-by"})
    for raw_capability in capabilities:
        capability = require_mapping(raw_capability, "claim_graph.capability_bindings[]", AssuranceGraphError)
        capability_id = require_string(capability.get("id"), "capability.id", AssuranceGraphError)
        nodes.append(node(f"capability:{capability_id}", "capability", capability.get("binding_kind"), policy))
    for raw_obligation in verifications:
        obligation = require_mapping(raw_obligation, "claim_graph.verification_obligations[]", AssuranceGraphError)
        obligation_id = require_string(obligation.get("id"), "verification_obligation.id", AssuranceGraphError)
        nodes.append(node(f"verification:{obligation_id}", "verification-obligation", "complete" if obligation.get("complete") else "incomplete", policy))
        mechanism = require_string(obligation.get("mechanism"), "verification_obligation.mechanism", AssuranceGraphError)
        edges.append({"from": f"verification:{obligation_id}", "to": f"mechanism:{mechanism}", "edge_type": "verifies"})
    unique_nodes = {node["id"]: node for node in nodes}
    return {
        "schema_version": 1,
        "kind": "kiva-assurance-graph",
        "status": "active",
        "description": "Primary generated AssuranceGraph review surface over claims, declared evidence receipts, capability bindings, and verification obligations.",
        "primary_review_surface": True,
        "authority_level_policy": {
            row["policy_key"]: {"authority_level": row["authority_level"], "rule": row["rule"]}
            for row in policy.values()
        },
        "derived_view_policy": {
            "rule": "EvidenceSet, EvidenceReceipt, ClaimGraph, and VerificationObligation remain materialized inputs to the AssuranceGraph review surface.",
            "materialized_inputs": [claim_graph_path.as_posix()],
        },
        "generated_from": [claim_graph_path.as_posix(), assurance_primitive_path.as_posix()],
        "generated_artifacts": {
            "assurance_graph": output_path.as_posix(),
            "markdown": markdown_path.as_posix(),
        },
        "summary": {
            "node_count": len(unique_nodes),
            "edge_count": len(edges),
            "claim_count": len(claims),
            "capability_count": len(capabilities),
            "verification_obligation_count": len(verifications),
        },
        "nodes": sorted(unique_nodes.values(), key=lambda row: row["id"]),
        "edges": sorted(edges, key=lambda row: (row["from"], row["to"], row["edge_type"])),
    }


def render_markdown(document: dict[str, Any]) -> str:
    """Render AssuranceGraph summary Markdown."""
    summary = require_mapping(document.get("summary"), "assurance_graph.summary", AssuranceGraphError)
    authority_policy = require_mapping(document.get("authority_level_policy"), "assurance_graph.authority_level_policy", AssuranceGraphError)
    authority_rows = "\n".join(
        f"| `{node_type}` | `{require_mapping(policy, f'authority.{node_type}', AssuranceGraphError)['authority_level']}` |"
        for node_type, policy in authority_policy.items()
    )
    return f"""# Generated AssuranceGraph

This file is generated by `scripts/ci/generate_assurance_graph.py`.

| Metric | Count |
| --- | ---: |
| Nodes | {summary['node_count']} |
| Edges | {summary['edge_count']} |
| Claims | {summary['claim_count']} |
| Capabilities | {summary['capability_count']} |
| Verification obligations | {summary['verification_obligation_count']} |

| Node | Authority level |
| --- | --- |
{authority_rows}
"""


def rendered_artifacts(root: Path, claim_graph_path: Path, assurance_primitive_path: Path, output_path: Path, markdown_path: Path) -> list[GeneratedArtifact]:
    """Return rendered AssuranceGraph artifacts."""
    document = graph_document(root, claim_graph_path, assurance_primitive_path, output_path, markdown_path)
    return [
        GeneratedArtifact(root / output_path, dump_yaml(document)),
        GeneratedArtifact(root / markdown_path, render_markdown(document)),
    ]


def main() -> int:
    """Generate or validate AssuranceGraph artifacts."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--claim-graph", type=Path, default=DEFAULT_CLAIM_GRAPH)
    parser.add_argument("--assurance-primitive", type=Path, default=DEFAULT_ASSURANCE_PRIMITIVE)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    artifacts = rendered_artifacts(root, args.claim_graph, args.assurance_primitive, args.out, args.markdown_out)
    write_or_check_generated_artifacts(
        artifacts,
        check=args.check,
        command_hint="scripts/ci/generate_assurance_graph.py",
        error_cls=AssuranceGraphError,
    )
    verb = "validated" if args.check else "generated"
    print(f"{verb} AssuranceGraph: {args.out}, {args.markdown_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
