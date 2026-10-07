#!/usr/bin/env python3
"""Validate committed and release-mode executed receipt overlays."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string
from kiva_ci.projection import require_envelope


DEFAULT_SOURCE = Path("spec/evidence/executed-receipts.source.yaml")
DEFAULT_OVERLAY = Path("generated/evidence/executed-receipt-overlay.yaml")
DEFAULT_RECEIPTS = Path("generated/evidence/evidence-receipts.yaml")


class ExecutedReceiptReleaseError(RuntimeError):
    """Raised when executed receipt release policy validation fails."""


def declared_receipt_ids(receipts: dict[str, Any]) -> set[str]:
    """Return all declared evidence and verification receipt ids."""
    ids: set[str] = set()
    for field in ("evidence_receipts", "verification_receipts"):
        for index, raw_receipt in enumerate(
            require_list(receipts.get(field), f"receipts.{field}", ExecutedReceiptReleaseError)
        ):
            receipt = require_mapping(raw_receipt, f"receipts.{field}[{index}]", ExecutedReceiptReleaseError)
            receipt_id = require_string(
                receipt.get("id"),
                f"receipts.{field}[{index}].id",
                ExecutedReceiptReleaseError,
            )
            if receipt_id in ids:
                raise ExecutedReceiptReleaseError(f"duplicate declared receipt id: {receipt_id}")
            ids.add(receipt_id)
    return ids


def overlay_summary(overlay: dict[str, Any]) -> dict[str, Any]:
    """Return the overlay summary mapping."""
    return require_mapping(overlay.get("summary"), "overlay.summary", ExecutedReceiptReleaseError)


def executed_ids(overlay: dict[str, Any]) -> set[str]:
    """Return executed receipt ids in an overlay."""
    ids: set[str] = set()
    for index, raw_receipt in enumerate(
        require_list(overlay.get("executed_receipts"), "overlay.executed_receipts", ExecutedReceiptReleaseError)
    ):
        receipt = require_mapping(raw_receipt, f"overlay.executed_receipts[{index}]", ExecutedReceiptReleaseError)
        receipt_id = require_string(
            receipt.get("receipt_id"),
            f"overlay.executed_receipts[{index}].receipt_id",
            ExecutedReceiptReleaseError,
        )
        if receipt_id in ids:
            raise ExecutedReceiptReleaseError(f"duplicate executed receipt id: {receipt_id}")
        ids.add(receipt_id)
        if receipt.get("status") != "executed":
            raise ExecutedReceiptReleaseError(f"executed receipt must have status=executed: {receipt_id}")
        require_string(
            receipt.get("report_sha256"),
            f"overlay.executed_receipts[{index}].report_sha256",
            ExecutedReceiptReleaseError,
        )
    return ids


def validate_overlay(root: Path, source_path: Path, overlay_path: Path, receipts_path: Path, *, release: bool) -> None:
    """Validate an executed receipt overlay against declared receipts."""
    source = load_yaml(root / source_path, ExecutedReceiptReleaseError)
    require_envelope(
        source,
        path=source_path,
        kind="kiva-executed-receipts-source",
        error_cls=ExecutedReceiptReleaseError,
    )
    overlay = load_yaml(root / overlay_path, ExecutedReceiptReleaseError)
    require_envelope(
        overlay,
        path=overlay_path,
        kind="kiva-executed-receipt-overlay",
        error_cls=ExecutedReceiptReleaseError,
    )
    receipts = load_yaml(root / receipts_path, ExecutedReceiptReleaseError)
    declared = declared_receipt_ids(receipts)
    executed = executed_ids(overlay)
    unknown_executed = sorted(executed.difference(declared))
    if unknown_executed:
        raise ExecutedReceiptReleaseError(f"executed overlay references undeclared receipts: {unknown_executed}")

    summary = overlay_summary(overlay)
    run_count = summary.get("receipt_run_count")
    executed_count = summary.get("executed_receipt_count")
    if run_count != len(require_list(overlay.get("receipt_runs"), "overlay.receipt_runs", ExecutedReceiptReleaseError)):
        raise ExecutedReceiptReleaseError("overlay.summary.receipt_run_count does not match receipt_runs")
    if executed_count != len(executed):
        raise ExecutedReceiptReleaseError("overlay.summary.executed_receipt_count does not match executed_receipts")

    if release:
        if run_count == 0 or executed_count == 0:
            raise ExecutedReceiptReleaseError("release mode requires at least one ReceiptRun and executed receipt")
    else:
        if run_count != 0 or executed_count != 0:
            raise ExecutedReceiptReleaseError("committed non-release overlay must not claim executed receipts")


def main() -> int:
    """Validate the executed receipt release policy."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--overlay", type=Path, default=DEFAULT_OVERLAY)
    parser.add_argument("--receipts", type=Path, default=DEFAULT_RECEIPTS)
    parser.add_argument("--release", action="store_true")
    args = parser.parse_args()

    validate_overlay(
        args.root.resolve(),
        args.source,
        args.overlay,
        args.receipts,
        release=args.release,
    )
    mode = "release" if args.release else "committed no-run"
    print(f"validated executed receipt overlay policy: {mode}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
