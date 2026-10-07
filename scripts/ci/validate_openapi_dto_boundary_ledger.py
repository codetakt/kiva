#!/usr/bin/env python3
"""Validate the OpenAPI-to-Rust DTO boundary ledger."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from kiva_ci.dto_boundary_ledger import (
    DEFAULT_DTO_MAP,
    DEFAULT_LEDGER,
    DtoBoundaryLedgerError,
    validate_boundary_ledger,
)
from kiva_ci.manifest import relative_path


def parse_args() -> argparse.Namespace:
    """Parse CLI arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--ledger", type=Path, default=DEFAULT_LEDGER)
    parser.add_argument("--dto-map", type=Path, default=DEFAULT_DTO_MAP)
    parser.add_argument("--json-report", type=Path)
    return parser.parse_args()


def main() -> int:
    """CLI entrypoint."""
    args = parse_args()
    root = args.root.resolve()
    ledger_path = relative_path(str(args.ledger), "--ledger", DtoBoundaryLedgerError)
    dto_map_path = relative_path(str(args.dto_map), "--dto-map", DtoBoundaryLedgerError)
    try:
        report = validate_boundary_ledger(root, ledger_path, dto_map_path)
    except DtoBoundaryLedgerError as error:
        print(f"OpenAPI DTO boundary-ledger validation failed: {error}", file=sys.stderr)
        return 1

    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.json_report is not None:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
