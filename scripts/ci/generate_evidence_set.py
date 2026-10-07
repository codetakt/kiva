#!/usr/bin/env python3
"""Generate the unified Kiva EvidenceSet index."""

from __future__ import annotations

import argparse
from collections import defaultdict
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.evidence import (
    evidence_check_effective_inputs,
    load_evidence_check_manifest,
    validate_evidence_check_manifest_metadata,
)
from kiva_ci.manifest import load_json, load_yaml, require_list, require_mapping, require_string
from kiva_ci.projection_graph import derived_inputs_by_check, load_projection_graph
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_SOURCE = Path("spec/evidence/evidence-set.source.yaml")
DEFAULT_EVIDENCE_MANIFEST = Path("spec/ci/evidence-checks.yaml")
DEFAULT_PROJECTION_GRAPH = Path("spec/ci/projection-graph.yaml")
DEFAULT_CLAIM_READINESS = Path("generated/claim-readiness-matrix.yaml")
DEFAULT_OUTPUT = Path("generated/evidence/evidence-set.yaml")
DEFAULT_MARKDOWN = Path("docs/architecture/generated/evidence-set.md")
DEFAULT_SCHEMA = Path("spec/evidence/schemas/evidence-set.schema.json")


class EvidenceSetError(RuntimeError):
    """Raised when the EvidenceSet projection cannot be generated."""


def validate_source(source: dict[str, Any], source_path: Path) -> None:
    """Validate the EvidenceSet source policy envelope."""
    if source.get("schema_version") != 1:
        raise EvidenceSetError(f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-evidence-set-source":
        raise EvidenceSetError(f"{source_path}.kind must be kiva-evidence-set-source")
    if source.get("status") != "active":
        raise EvidenceSetError(f"{source_path}.status must be active")
    require_mapping(source.get("source_model"), f"{source_path}.source_model", EvidenceSetError)
    require_mapping(source.get("assurance_meta_model"), f"{source_path}.assurance_meta_model", EvidenceSetError)
    require_mapping(source.get("generated_artifacts"), f"{source_path}.generated_artifacts", EvidenceSetError)
    require_mapping(source.get("projection_policy"), f"{source_path}.projection_policy", EvidenceSetError)


def ready_claims_by_check(readiness: dict[str, Any]) -> dict[tuple[str, str], list[dict[str, str]]]:
    """Return ready public claims keyed by matched evidence group/check id."""
    if readiness.get("kind") != "kiva-claim-readiness-matrix":
        raise EvidenceSetError("claim readiness kind must be kiva-claim-readiness-matrix")
    claims_by_check: dict[tuple[str, str], list[dict[str, str]]] = defaultdict(list)
    for claim_index, raw_claim in enumerate(
        require_list(readiness.get("claims"), "claim-readiness.claims", EvidenceSetError)
    ):
        claim = require_mapping(raw_claim, f"claim-readiness.claims[{claim_index}]", EvidenceSetError)
        if claim.get("readiness") != "ready":
            continue
        claim_id = require_string(claim.get("id"), f"claim-readiness.claims[{claim_index}].id", EvidenceSetError)
        boundary = require_string(
            claim.get("boundary"),
            f"claim-readiness.claims[{claim_index}].boundary",
            EvidenceSetError,
        )
        maturity = require_string(
            claim.get("claim_maturity"),
            f"claim-readiness.claims[{claim_index}].claim_maturity",
            EvidenceSetError,
        )
        for check_index, raw_check in enumerate(
            require_list(
                claim.get("matched_evidence_checks", []),
                f"claim-readiness.claims[{claim_index}].matched_evidence_checks",
                EvidenceSetError,
            )
        ):
            check = require_mapping(
                raw_check,
                f"claim-readiness.claims[{claim_index}].matched_evidence_checks[{check_index}]",
                EvidenceSetError,
            )
            claims_by_check[
                (
                    require_string(check.get("group"), "matched_evidence_check.group", EvidenceSetError),
                    require_string(check.get("check_id"), "matched_evidence_check.check_id", EvidenceSetError),
                )
            ].append(
                {
                    "id": claim_id,
                    "boundary": boundary,
                    "claim_maturity": maturity,
                }
            )
    return {
        key: sorted(value, key=lambda item: item["id"])
        for key, value in claims_by_check.items()
    }


def evidence_set_document(
    root: Path,
    source_path: Path,
    manifest_path: Path,
    projection_graph_path: Path,
    readiness_path: Path,
    output_path: Path,
    markdown_path: Path,
) -> dict[str, Any]:
    """Generate the unified EvidenceSet document."""
    source = load_yaml(root / source_path, EvidenceSetError)
    validate_source(source, source_path)
    manifest = load_evidence_check_manifest(root / manifest_path, EvidenceSetError)
    groups = validate_evidence_check_manifest_metadata(
        manifest,
        manifest_path.as_posix(),
        EvidenceSetError,
    )
    projection_graph = load_projection_graph(root / projection_graph_path, EvidenceSetError)
    derived_inputs = derived_inputs_by_check(projection_graph, EvidenceSetError)
    readiness = load_yaml(root / readiness_path, EvidenceSetError)
    claim_matches = ready_claims_by_check(readiness)

    evidence_groups: list[dict[str, Any]] = []
    total_checks = 0
    for group_id, group in groups.items():
        checks: list[dict[str, Any]] = []
        for check_index, raw_check in enumerate(
            require_list(group.get("checks"), f"{group_id}.checks", EvidenceSetError)
        ):
            check = require_mapping(raw_check, f"{group_id}.checks[{check_index}]", EvidenceSetError)
            check_id = require_string(check.get("id"), f"{group_id}.checks[{check_index}].id", EvidenceSetError)
            input_model = evidence_check_effective_inputs(
                check,
                derived_inputs.get((group_id, check_id), set()),
                f"{group_id}.{check_id}",
                EvidenceSetError,
                require_projection_graph_inputs=check.get("derived_input_policy") == "projection-graph",
            )
            checks.append(
                {
                    "id": check_id,
                    "receipt_id": f"evidence-receipt.{group_id}.{check_id}",
                    "check_kind": require_string(
                        check.get("check_kind"),
                        f"{group_id}.{check_id}.check_kind",
                        EvidenceSetError,
                    ),
                    "artifact_class": require_string(
                        check.get("artifact_class"),
                        f"{group_id}.{check_id}.artifact_class",
                        EvidenceSetError,
                    ),
                    "script": require_string(
                        check.get("script"),
                        f"{group_id}.{check_id}.script",
                        EvidenceSetError,
                    ),
                    "ci_command": require_mapping(
                        check.get("ci_command"),
                        f"{group_id}.{check_id}.ci_command",
                        EvidenceSetError,
                    ),
                    "effective_inputs": input_model["effective_inputs"],
                    "derived_input_policy": input_model["derived_input_policy"],
                    "outputs": check.get("outputs", []),
                    "ready_claims": claim_matches.get((group_id, check_id), []),
                }
            )
        total_checks += len(checks)
        evidence_groups.append(
            {
                "id": group_id,
                "claim_boundary": require_string(
                    group.get("claim_boundary"),
                    f"{group_id}.claim_boundary",
                    EvidenceSetError,
                ),
                "nix_gate": require_string(group.get("nix_gate"), f"{group_id}.nix_gate", EvidenceSetError),
                "description": require_string(
                    group.get("description"),
                    f"{group_id}.description",
                    EvidenceSetError,
                ),
                "check_count": len(checks),
                "checks": checks,
            }
        )

    return {
        "schema_version": 1,
        "kind": "kiva-evidence-set",
        "status": "active",
        "description": require_string(source.get("description"), f"{source_path}.description", EvidenceSetError),
        "generated_from": [
            source_path.as_posix(),
            manifest_path.as_posix(),
            projection_graph_path.as_posix(),
            readiness_path.as_posix(),
        ],
        "generated_artifacts": {
            "evidence_set": output_path.as_posix(),
            "markdown": markdown_path.as_posix(),
        },
        "source_model": require_mapping(source.get("source_model"), f"{source_path}.source_model", EvidenceSetError),
        "assurance_meta_model": require_mapping(
            source.get("assurance_meta_model"),
            f"{source_path}.assurance_meta_model",
            EvidenceSetError,
        ),
        "projection_policy": require_mapping(
            source.get("projection_policy"),
            f"{source_path}.projection_policy",
            EvidenceSetError,
        ),
        "group_count": len(evidence_groups),
        "check_count": total_checks,
        "ready_claim_match_count": sum(
            len(check["ready_claims"])
            for group in evidence_groups
            for check in group["checks"]
        ),
        "groups": evidence_groups,
    }


def validate_schema(document: dict[str, Any], schema_path: Path) -> None:
    """Validate the generated EvidenceSet document against its JSON Schema."""
    schema = load_json(schema_path, EvidenceSetError)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)
    errors = sorted(validator.iter_errors(document), key=lambda error: list(error.path))
    if errors:
        details = "\n".join(
            f"- {'/'.join(str(part) for part in error.path) or '<root>'}: {error.message}"
            for error in errors
        )
        raise EvidenceSetError(f"EvidenceSet violates {schema_path}:\n{details}")


def markdown_cell(value: object) -> str:
    """Render a Markdown table cell."""
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_markdown(document: dict[str, Any]) -> str:
    """Render a concise EvidenceSet review table."""
    rows = [
        "| Group | Check | CI Command | Receipt | Kind | Artifact Class | Ready Claims |",
        "| --- | --- | --- | --- | --- | --- | --- |",
    ]
    for raw_group in require_list(document.get("groups"), "evidence_set.groups", EvidenceSetError):
        group = require_mapping(raw_group, "evidence_set.groups[]", EvidenceSetError)
        group_id = require_string(group.get("id"), "evidence_set.groups[].id", EvidenceSetError)
        for raw_check in require_list(group.get("checks"), f"{group_id}.checks", EvidenceSetError):
            check = require_mapping(raw_check, f"{group_id}.checks[]", EvidenceSetError)
            ready_claims = ", ".join(
                f"`{markdown_cell(claim['id'])}`"
                for claim in require_list(
                    check.get("ready_claims"),
                    f"{group_id}.ready_claims",
                    EvidenceSetError,
                )
            )
            rows.append(
                "| "
                + " | ".join(
                    [
                        f"`{markdown_cell(group_id)}`",
                        f"`{markdown_cell(check['id'])}`",
                        f"`{markdown_cell(check['ci_command']['family'])} {markdown_cell(check['ci_command']['name'])}`",
                        f"`{markdown_cell(check['receipt_id'])}`",
                        f"`{markdown_cell(check['check_kind'])}`",
                        f"`{markdown_cell(check['artifact_class'])}`",
                        ready_claims,
                    ]
                )
                + " |"
            )
    return f"""# Generated EvidenceSet

This file is generated by `scripts/ci/generate_evidence_set.py`.

`EvidenceSet` is the common evidence review plane. It binds manifest-declared CI
checks, projection-derived inputs, generated outputs, and ready public claims
without replacing domain-specific conformance, PDS, confidential-computation, or
formal-verification packages.

AssurancePrimitive flow: Claim -> Obligation -> Discharge -> Receipt.

{chr(10).join(rows)}
"""


def parse_args() -> argparse.Namespace:
    """Parse CLI arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_EVIDENCE_MANIFEST)
    parser.add_argument("--projection-graph", type=Path, default=DEFAULT_PROJECTION_GRAPH)
    parser.add_argument("--claim-readiness", type=Path, default=DEFAULT_CLAIM_READINESS)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def main() -> int:
    """Generate or validate the EvidenceSet index."""
    args = parse_args()
    root = args.root.resolve()
    document = evidence_set_document(
        root,
        args.source,
        args.manifest,
        args.projection_graph,
        args.claim_readiness,
        args.out,
        args.markdown_out,
    )
    validate_schema(document, root / args.schema)
    rendered_yaml = dump_yaml(document)
    rendered_markdown = render_markdown(document)
    output_path = root / args.out
    markdown_path = root / args.markdown_out
    command_hint = "scripts/ci/generate_evidence_set.py"
    try:
        write_or_check_generated_artifacts(
            [
                GeneratedArtifact(output_path, rendered_yaml),
                GeneratedArtifact(markdown_path, rendered_markdown),
            ],
            check=args.check,
            command_hint=command_hint,
            error_cls=EvidenceSetError,
        )
    except EvidenceSetError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated EvidenceSet: {args.out}, {args.markdown_out}")
        return 0
    print(f"generated EvidenceSet: {args.out}, {args.markdown_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
