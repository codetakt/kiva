#!/usr/bin/env python3
"""Generate a release-specific executed receipt overlay."""

from __future__ import annotations

import argparse
import hashlib
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


DEFAULT_SOURCE = Path("spec/evidence/executed-receipts.source.yaml")
DEFAULT_OUTPUT = Path("generated/evidence/executed-receipt-overlay.yaml")
DEFAULT_MARKDOWN = Path("docs/architecture/generated/executed-receipt-overlay.md")
DEFAULT_SCHEMA = Path("spec/evidence/schemas/executed-receipt-overlay.schema.json")


class ExecutedReceiptOverlayError(RuntimeError):
    """Raised when the executed receipt overlay cannot be generated."""


def sha256_file(path: Path) -> str:
    """Return the sha256 digest of one file."""
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def declared_evidence_receipts(receipt_index: dict[str, Any]) -> dict[tuple[str, str], str]:
    """Return declared EvidenceReceipt ids keyed by CI group/check."""
    receipts: dict[tuple[str, str], str] = {}
    for index, raw_receipt in enumerate(
        require_list(receipt_index.get("evidence_receipts"), "receipt_index.evidence_receipts", ExecutedReceiptOverlayError)
    ):
        receipt = require_mapping(raw_receipt, f"receipt_index.evidence_receipts[{index}]", ExecutedReceiptOverlayError)
        check_ref = require_mapping(
            receipt.get("check_ref"),
            f"receipt_index.evidence_receipts[{index}].check_ref",
            ExecutedReceiptOverlayError,
        )
        key = (
            require_string(check_ref.get("group"), f"receipt_index.evidence_receipts[{index}].check_ref.group", ExecutedReceiptOverlayError),
            require_string(
                check_ref.get("check_id"),
                f"receipt_index.evidence_receipts[{index}].check_ref.check_id",
                ExecutedReceiptOverlayError,
            ),
        )
        receipts[key] = require_string(receipt.get("id"), f"receipt_index.evidence_receipts[{index}].id", ExecutedReceiptOverlayError)
    return receipts


def report_checks(report: dict[str, Any], label: str) -> list[dict[str, str]]:
    """Return group/check rows from a design-review or evidence-check report."""
    kind = require_string(report.get("kind"), f"{label}.kind", ExecutedReceiptOverlayError)
    if kind == "kiva-ci-evidence-check-run":
        group_id = require_string(report.get("group"), f"{label}.group", ExecutedReceiptOverlayError)
        return [
            {
                "group": group_id,
                "check_id": require_string(
                    require_mapping(raw_check, f"{label}.checks[]", ExecutedReceiptOverlayError).get("id"),
                    f"{label}.checks[].id",
                    ExecutedReceiptOverlayError,
                ),
            }
            for raw_check in require_list(report.get("checks"), f"{label}.checks", ExecutedReceiptOverlayError)
        ]
    if kind == "kiva-design-review-suite-run":
        rows: list[dict[str, str]] = []
        for group_index, raw_group in enumerate(
            require_list(report.get("groups"), f"{label}.groups", ExecutedReceiptOverlayError)
        ):
            group = require_mapping(raw_group, f"{label}.groups[{group_index}]", ExecutedReceiptOverlayError)
            group_id = require_string(group.get("group"), f"{label}.groups[{group_index}].group", ExecutedReceiptOverlayError)
            for raw_check in require_list(
                group.get("checks"),
                f"{label}.groups[{group_index}].checks",
                ExecutedReceiptOverlayError,
            ):
                check = require_mapping(raw_check, f"{label}.groups[{group_index}].checks[]", ExecutedReceiptOverlayError)
                rows.append(
                    {
                        "group": group_id,
                        "check_id": require_string(check.get("id"), f"{label}.groups[{group_index}].checks[].id", ExecutedReceiptOverlayError),
                    }
                )
        return rows
    raise ExecutedReceiptOverlayError(f"{label}.kind is not a supported run report kind: {kind}")


def executed_rows_for_report(
    root: Path,
    report_path: Path,
    run_id: str,
    declared_receipts: dict[tuple[str, str], str],
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    """Return one ReceiptRun row and executed receipts for a run report."""
    resolved = (root / report_path).resolve()
    report = load_yaml(resolved, ExecutedReceiptOverlayError)
    report_kind = require_string(report.get("kind"), f"{report_path}.kind", ExecutedReceiptOverlayError)
    digest = sha256_file(resolved)
    checks = report_checks(report, report_path.as_posix())
    executed: list[dict[str, Any]] = []
    for check in checks:
        key = (check["group"], check["check_id"])
        receipt_id = declared_receipts.get(key)
        if receipt_id is None:
            raise ExecutedReceiptOverlayError(
                f"{report_path} references undeclared EvidenceReceipt check: {check['group']}.{check['check_id']}"
            )
        executed.append(
            {
                "id": f"executed-receipt.{run_id}.{check['group']}.{check['check_id']}",
                "root_type": "ExecutedReceipt",
                "status": "executed",
                "receipt_id": receipt_id,
                "check_ref": check,
                "run_id": run_id,
                "report_sha256": digest,
            }
        )
    return (
        {
            "id": run_id,
            "root_type": "ReceiptRun",
            "status": "executed",
            "report_path": report_path.as_posix(),
            "report_sha256": digest,
            "report_kind": report_kind,
            "check_count": len(checks),
        },
        executed,
    )


def overlay_document(
    root: Path,
    source_path: Path,
    output_path: Path,
    markdown_path: Path,
    run_reports: list[Path],
) -> dict[str, Any]:
    """Return the executed receipt overlay document."""
    source = load_yaml(root / source_path, ExecutedReceiptOverlayError)
    if source.get("schema_version") != 1:
        raise ExecutedReceiptOverlayError(f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-executed-receipts-source":
        raise ExecutedReceiptOverlayError(f"{source_path}.kind must be kiva-executed-receipts-source")
    if source.get("status") != "active":
        raise ExecutedReceiptOverlayError(f"{source_path}.status must be active")
    inputs = require_mapping(source.get("inputs"), f"{source_path}.inputs", ExecutedReceiptOverlayError)
    receipt_index_path = Path(
        require_string(inputs.get("declared_receipts"), f"{source_path}.inputs.declared_receipts", ExecutedReceiptOverlayError)
    )
    manifest_path = Path(
        require_string(inputs.get("evidence_manifest"), f"{source_path}.inputs.evidence_manifest", ExecutedReceiptOverlayError)
    )
    receipt_index = load_yaml(root / receipt_index_path, ExecutedReceiptOverlayError)
    declared = declared_evidence_receipts(receipt_index)
    receipt_runs: list[dict[str, Any]] = []
    executed_receipts: list[dict[str, Any]] = []
    for index, report in enumerate(run_reports, start=1):
        run_id = f"receipt-run-{index:03d}"
        run, rows = executed_rows_for_report(root, report, run_id, declared)
        receipt_runs.append(run)
        executed_receipts.extend(rows)
    generated_from = [source_path.as_posix(), receipt_index_path.as_posix(), manifest_path.as_posix()]
    generated_from.extend(report.as_posix() for report in run_reports)
    return {
        "schema_version": 1,
        "kind": "kiva-executed-receipt-overlay",
        "status": "active",
        "description": require_string(source.get("description"), f"{source_path}.description", ExecutedReceiptOverlayError),
        "generated_from": generated_from,
        "generated_artifacts": {
            "overlay": output_path.as_posix(),
            "markdown": markdown_path.as_posix(),
        },
        "run_model": require_mapping(source.get("run_model"), f"{source_path}.run_model", ExecutedReceiptOverlayError),
        "summary": {
            "declared_evidence_receipt_count": len(declared),
            "declared_verification_receipt_count": len(
                require_list(receipt_index.get("verification_receipts"), "receipt_index.verification_receipts", ExecutedReceiptOverlayError)
            ),
            "receipt_run_count": len(receipt_runs),
            "executed_receipt_count": len(executed_receipts),
        },
        "receipt_runs": receipt_runs,
        "executed_receipts": sorted(executed_receipts, key=lambda row: row["id"]),
    }


def validate_schema(document: dict[str, Any], schema_path: Path) -> None:
    """Validate the generated overlay."""
    validate_json_schema(
        document,
        load_json(schema_path, ExecutedReceiptOverlayError),
        label=schema_path.as_posix(),
        error_cls=ExecutedReceiptOverlayError,
    )


def markdown_cell(value: object) -> str:
    """Render a Markdown table cell."""
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_markdown(document: dict[str, Any]) -> str:
    """Render usage instructions, including run details only for local reports."""
    run_rows = [
        "| Run | Report | SHA-256 | Checks |",
        "| --- | --- | --- | ---: |",
    ]
    for run in require_list(document.get("receipt_runs"), "receipt_runs", ExecutedReceiptOverlayError):
        row = require_mapping(run, "receipt_runs[]", ExecutedReceiptOverlayError)
        run_rows.append(
            "| "
            + " | ".join(
                [
                    f"`{markdown_cell(row['id'])}`",
                    f"`{markdown_cell(row['report_path'])}`",
                    f"`{markdown_cell(row['report_sha256'])}`",
                    str(row["check_count"]),
                ]
            )
            + " |"
        )
    return f"""# Generated Executed Receipt Overlay

This file is generated by `scripts/ci/generate_executed_receipt_overlay.py`.

Use `--run-report` to bind declared receipt ids to concrete report hashes.
For executed reports, select both `--out` and `--markdown-out` under the
Git-ignored `target/` directory. Execution results and review decisions remain
local.

{('## Local Receipt Runs' + chr(10) * 2 + chr(10).join(run_rows) + chr(10)) if document['receipt_runs'] else ''}
""".rstrip() + "\n"


def main() -> int:
    """Generate or validate the executed receipt overlay."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--run-report", type=Path, action="append", default=[])
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    if args.run_report:
        local_root = (root / "target").resolve()
        if not all((root / path).resolve().is_relative_to(local_root) for path in (args.out, args.markdown_out)):
            raise SystemExit("executed receipt outputs must remain local under target/; set --out and --markdown-out")
    document = overlay_document(root, args.source, args.out, args.markdown_out, args.run_report)
    validate_schema(document, root / args.schema)
    rendered_yaml = dump_yaml(document)
    rendered_markdown = render_markdown(document)
    if args.check:
        if args.run_report:
            raise SystemExit("--check validates only the committed no-run overlay; omit --run-report")
    command_hint = "scripts/ci/generate_executed_receipt_overlay.py"
    try:
        write_or_check_generated_artifacts(
            [
                GeneratedArtifact(root / args.out, rendered_yaml),
                GeneratedArtifact(root / args.markdown_out, rendered_markdown),
            ],
            check=args.check,
            command_hint=command_hint,
            error_cls=ExecutedReceiptOverlayError,
        )
    except ExecutedReceiptOverlayError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated executed receipt overlay: {args.out}, {args.markdown_out}")
        return 0
    print(f"generated executed receipt overlay: {args.out}, {args.markdown_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
