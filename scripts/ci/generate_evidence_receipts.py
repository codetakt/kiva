#!/usr/bin/env python3
"""Generate the EvidenceReceipt and VerificationReceipt index."""

from __future__ import annotations

import argparse
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


DEFAULT_SOURCE = Path("spec/evidence/evidence-receipts.source.yaml")
DEFAULT_OUTPUT = Path("generated/evidence/evidence-receipts.yaml")
DEFAULT_MARKDOWN = Path("docs/architecture/generated/evidence-receipts.md")
DEFAULT_SCHEMA = Path("spec/evidence/schemas/evidence-receipts.schema.json")


class EvidenceReceiptGenerationError(RuntimeError):
    """Raised when receipt index generation fails."""


def evidence_receipts(evidence_set: dict[str, Any]) -> list[dict[str, Any]]:
    """Return declared EvidenceReceipt rows from the generated EvidenceSet."""
    receipts: list[dict[str, Any]] = []
    for group_index, raw_group in enumerate(
        require_list(evidence_set.get("groups"), "evidence_set.groups", EvidenceReceiptGenerationError)
    ):
        group = require_mapping(raw_group, f"evidence_set.groups[{group_index}]", EvidenceReceiptGenerationError)
        group_id = require_string(group.get("id"), f"evidence_set.groups[{group_index}].id", EvidenceReceiptGenerationError)
        for check_index, raw_check in enumerate(
            require_list(group.get("checks"), f"evidence_set.groups[{group_index}].checks", EvidenceReceiptGenerationError)
        ):
            check = require_mapping(
                raw_check,
                f"evidence_set.groups[{group_index}].checks[{check_index}]",
                EvidenceReceiptGenerationError,
            )
            check_id = require_string(check.get("id"), f"{group_id}.checks[{check_index}].id", EvidenceReceiptGenerationError)
            receipts.append(
                {
                    "id": require_string(check.get("receipt_id"), f"{group_id}.{check_id}.receipt_id", EvidenceReceiptGenerationError),
                    "root_type": "EvidenceReceipt",
                    "status": "declared",
                    "check_ref": {
                        "group": group_id,
                        "check_id": check_id,
                    },
                    "script": require_string(check.get("script"), f"{group_id}.{check_id}.script", EvidenceReceiptGenerationError),
                    "artifact_class": require_string(
                        check.get("artifact_class"),
                        f"{group_id}.{check_id}.artifact_class",
                        EvidenceReceiptGenerationError,
                    ),
                    "input_count": len(
                        require_list(
                            check.get("effective_inputs"),
                            f"{group_id}.{check_id}.effective_inputs",
                            EvidenceReceiptGenerationError,
                        )
                    ),
                    "output_count": len(
                        require_list(check.get("outputs"), f"{group_id}.{check_id}.outputs", EvidenceReceiptGenerationError)
                    ),
                    "ready_claim_count": len(
                        require_list(
                            check.get("ready_claims"),
                            f"{group_id}.{check_id}.ready_claims",
                            EvidenceReceiptGenerationError,
                        )
                    ),
                    "receipt_policy": "execution-receipt-required-for-release",
                }
            )
    return receipts


def verification_receipts(obligations: dict[str, Any]) -> list[dict[str, Any]]:
    """Return declared VerificationReceipt rows from the generated obligations."""
    receipts: list[dict[str, Any]] = []
    for obligation_index, raw_obligation in enumerate(
        require_list(obligations.get("obligations"), "verification_obligations.obligations", EvidenceReceiptGenerationError)
    ):
        obligation = require_mapping(
            raw_obligation,
            f"verification_obligations.obligations[{obligation_index}]",
            EvidenceReceiptGenerationError,
        )
        obligation_id = require_string(
            obligation.get("id"),
            f"verification_obligations.obligations[{obligation_index}].id",
            EvidenceReceiptGenerationError,
        )
        for discharge_index, raw_discharge in enumerate(
            require_list(obligation.get("declared_discharges"), f"{obligation_id}.declared_discharges", EvidenceReceiptGenerationError)
        ):
            discharge = require_mapping(
                raw_discharge,
                f"{obligation_id}.declared_discharges[{discharge_index}]",
                EvidenceReceiptGenerationError,
            )
            receipts.append(
                {
                    "id": require_string(discharge.get("receipt_id"), f"{obligation_id}.receipt_id", EvidenceReceiptGenerationError),
                    "root_type": "VerificationReceipt",
                    "status": "declared",
                    "obligation_id": obligation_id,
                    "method": require_string(discharge.get("method"), f"{obligation_id}.method", EvidenceReceiptGenerationError),
                    "source_field": require_string(
                        discharge.get("source_field"),
                        f"{obligation_id}.source_field",
                        EvidenceReceiptGenerationError,
                    ),
                    "declared_discharge_count": int(discharge.get("count", 0)),
                    "receipt_policy": "execution-receipt-required-for-release",
                }
            )
    return receipts


def receipt_document(source_path: Path, output_path: Path, markdown_path: Path) -> dict[str, Any]:
    """Return the generated receipt index document."""
    source = load_yaml(source_path, EvidenceReceiptGenerationError)
    if source.get("schema_version") != 1:
        raise EvidenceReceiptGenerationError(f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-evidence-receipts-source":
        raise EvidenceReceiptGenerationError(f"{source_path}.kind must be kiva-evidence-receipts-source")
    if source.get("status") != "active":
        raise EvidenceReceiptGenerationError(f"{source_path}.status must be active")
    inputs = require_mapping(source.get("inputs"), f"{source_path}.inputs", EvidenceReceiptGenerationError)
    evidence_set_path = Path(
        require_string(inputs.get("evidence_set"), f"{source_path}.inputs.evidence_set", EvidenceReceiptGenerationError)
    )
    verification_obligations_path = Path(
        require_string(
            inputs.get("verification_obligations"),
            f"{source_path}.inputs.verification_obligations",
            EvidenceReceiptGenerationError,
        )
    )
    evidence_rows = evidence_receipts(load_yaml(evidence_set_path, EvidenceReceiptGenerationError))
    verification_rows = verification_receipts(load_yaml(verification_obligations_path, EvidenceReceiptGenerationError))
    return {
        "schema_version": 1,
        "kind": "kiva-evidence-receipt-index",
        "status": "active",
        "description": require_string(source.get("description"), f"{source_path}.description", EvidenceReceiptGenerationError),
        "generated_from": [
            source_path.as_posix(),
            evidence_set_path.as_posix(),
            verification_obligations_path.as_posix(),
        ],
        "generated_artifacts": {
            "receipt_index": output_path.as_posix(),
            "markdown": markdown_path.as_posix(),
        },
        "receipt_model": require_mapping(
            source.get("receipt_model"),
            f"{source_path}.receipt_model",
            EvidenceReceiptGenerationError,
        ),
        "summary": {
            "evidence_receipt_count": len(evidence_rows),
            "verification_receipt_count": len(verification_rows),
            "receipt_count": len(evidence_rows) + len(verification_rows),
        },
        "evidence_receipts": evidence_rows,
        "verification_receipts": verification_rows,
    }


def validate_schema(document: dict[str, Any], schema_path: Path) -> None:
    """Validate the generated receipt index."""
    validate_json_schema(
        document,
        load_json(schema_path, EvidenceReceiptGenerationError),
        label=schema_path.as_posix(),
        error_cls=EvidenceReceiptGenerationError,
    )


def markdown_cell(value: object) -> str:
    """Render a Markdown table cell."""
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_markdown(document: dict[str, Any]) -> str:
    """Render a compact receipt summary."""
    summary = require_mapping(document.get("summary"), "summary", EvidenceReceiptGenerationError)
    rows = [
        "| Receipt Type | Count |",
        "| --- | ---: |",
        f"| `EvidenceReceipt` | {summary['evidence_receipt_count']} |",
        f"| `VerificationReceipt` | {summary['verification_receipt_count']} |",
    ]
    sample_rows = [
        "| Receipt | Type | Status |",
        "| --- | --- | --- |",
    ]
    for receipt in document["evidence_receipts"][:8]:
        sample_rows.append(
            f"| `{markdown_cell(receipt['id'])}` | `EvidenceReceipt` | `{markdown_cell(receipt['status'])}` |"
        )
    for receipt in document["verification_receipts"][:8]:
        sample_rows.append(
            f"| `{markdown_cell(receipt['id'])}` | `VerificationReceipt` | `{markdown_cell(receipt['status'])}` |"
        )
    return f"""# Generated Evidence Receipts

This file is generated by `scripts/ci/generate_evidence_receipts.py`.

Receipt rows establish stable evidence identities for CI checks and formal
verification discharges. Generated rows are `declared`; release-specific
executed receipts can refine the same ids with command output hashes.

{chr(10).join(rows)}

## Sample Receipts

{chr(10).join(sample_rows)}
"""


def main() -> int:
    """Generate or validate the receipt index."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    document = receipt_document(args.source, args.out, args.markdown_out)
    validate_schema(document, args.schema)
    rendered_yaml = dump_yaml(document)
    rendered_markdown = render_markdown(document)
    command_hint = "scripts/ci/generate_evidence_receipts.py"
    try:
        write_or_check_generated_artifacts(
            [
                GeneratedArtifact(args.out, rendered_yaml),
                GeneratedArtifact(args.markdown_out, rendered_markdown),
            ],
            check=args.check,
            command_hint=command_hint,
            error_cls=EvidenceReceiptGenerationError,
        )
    except EvidenceReceiptGenerationError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated EvidenceReceipt index: {args.out}, {args.markdown_out}")
        return 0
    print(f"generated EvidenceReceipt index: {args.out}, {args.markdown_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
