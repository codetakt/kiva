#!/usr/bin/env python3
"""Validate formal verification reports and evidence manifests."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker


DEFAULT_SCHEMA = Path("spec/verification/schemas/formal-verification-report.schema.json")
DEFAULT_MANIFEST_SCHEMA = Path(
    "spec/verification/schemas/formal-verification-evidence-manifest.schema.json"
)


class FormalVerificationReportError(RuntimeError):
    """Raised when a formal verification report violates its schema."""


def load_json(path: Path) -> Any:
    """Load JSON from a path."""
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def format_validation_error(error: Any) -> str:
    """Format a JSON Schema validation error with a stable JSON path."""
    path = "$"
    for part in error.absolute_path:
        if isinstance(part, int):
            path += f"[{part}]"
        else:
            path += f".{part}"
    return f"- {path}: {error.message}"


def main() -> int:
    """Run report validation."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--manifest-schema", type=Path, default=DEFAULT_MANIFEST_SCHEMA)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument(
        "--check-schemas",
        action="store_true",
        help="validate formal verification JSON Schemas even without report instances",
    )
    parser.add_argument("reports", nargs="*", type=Path)
    args = parser.parse_args()

    schema = load_json(args.schema)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    manifest_schema = load_json(args.manifest_schema)
    Draft202012Validator.check_schema(manifest_schema)
    manifest_validator = Draft202012Validator(
        manifest_schema,
        format_checker=FormatChecker(),
    )

    if args.check_schemas:
        print(f"validated JSON Schema: {args.schema}")
        print(f"validated JSON Schema: {args.manifest_schema}")
    if not args.check_schemas and args.manifest is None and not args.reports:
        raise FormalVerificationReportError(
            "at least one report path, --manifest, or --check-schemas must be supplied"
        )

    for report_path in args.reports:
        report = load_json(report_path)
        errors = sorted(
            validator.iter_errors(report),
            key=lambda error: list(error.absolute_path),
        )
        if errors:
            details = "\n".join(format_validation_error(error) for error in errors)
            raise FormalVerificationReportError(
                f"{report_path} violates {args.schema}:\n{details}"
            )
        print(f"validated formal verification report: {report_path}")

    if args.manifest is not None:
        manifest = load_json(args.manifest)
        errors = sorted(
            manifest_validator.iter_errors(manifest),
            key=lambda error: list(error.absolute_path),
        )
        if errors:
            details = "\n".join(format_validation_error(error) for error in errors)
            raise FormalVerificationReportError(
                f"{args.manifest} violates {args.manifest_schema}:\n{details}"
            )
        print(f"validated formal verification evidence manifest: {args.manifest}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
