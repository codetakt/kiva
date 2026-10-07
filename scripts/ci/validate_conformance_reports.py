#!/usr/bin/env python3
"""Validate machine-readable conformance reports against JSON Schemas."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator


DEFAULT_RUN_SCHEMA = Path("spec/conformance/schemas/conformance-run-report.schema.json")
DEFAULT_CASE_MATRIX_SCHEMA = Path(
    "spec/conformance/schemas/conformance-case-matrix-report.schema.json"
)
DEFAULT_COVERAGE_SCHEMA = Path("spec/conformance/schemas/conformance-coverage-report.schema.json")
DEFAULT_ADL_SCHEMA = Path("spec/conformance/schemas/adl-compatibility-report.schema.json")
DEFAULT_ADL_RUN_SCHEMA = Path("spec/conformance/schemas/adl-run-report.schema.json")
DEFAULT_REQUIREMENT_AUDIT_SCHEMA = Path(
    "spec/conformance/schemas/requirement-catalog-audit-report.schema.json"
)
DEFAULT_POSTGRES_VERIFICATION_SCHEMA = Path(
    "spec/conformance/schemas/postgres-verification-report.schema.json"
)


class ReportSchemaError(RuntimeError):
    """Raised when a report or schema violates the report contract."""


def load_json(path: Path) -> Any:
    """Load JSON from a file path, or stdin when the path is '-'."""
    if str(path) == "-":
        return json.load(sys.stdin)
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def validate_instance(schema_path: Path, instance_path: Path, label: str) -> None:
    """Validate one report instance against one JSON Schema."""
    schema = validate_schema(schema_path)
    instance = load_json(instance_path)
    validator = Draft202012Validator(schema)
    errors = sorted(
        validator.iter_errors(instance),
        key=lambda error: list(error.absolute_path),
    )
    if errors:
        details = "\n".join(format_validation_error(error) for error in errors)
        raise ReportSchemaError(f"{label} report violates {schema_path}:\n{details}")
    print(f"validated {label} report: {instance_path}")


def validate_schema(schema_path: Path) -> dict[str, Any]:
    """Validate one JSON Schema and return it."""
    schema = load_json(schema_path)
    if not isinstance(schema, dict):
        raise ReportSchemaError(f"{schema_path} root must be a JSON object")
    Draft202012Validator.check_schema(schema)
    return schema


def format_validation_error(error: Any) -> str:
    """Format a jsonschema validation error with a stable JSON path."""
    path = "$"
    for part in error.absolute_path:
        if isinstance(part, int):
            path += f"[{part}]"
        else:
            path += f".{part}"
    return f"- {path}: {error.message}"


def main() -> int:
    """Run report schema validation."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-report", type=Path)
    parser.add_argument("--run-schema", type=Path, default=DEFAULT_RUN_SCHEMA)
    parser.add_argument("--case-matrix-report", type=Path)
    parser.add_argument("--case-matrix-schema", type=Path, default=DEFAULT_CASE_MATRIX_SCHEMA)
    parser.add_argument("--coverage-report", type=Path)
    parser.add_argument("--coverage-schema", type=Path, default=DEFAULT_COVERAGE_SCHEMA)
    parser.add_argument("--adl-report", type=Path)
    parser.add_argument("--adl-schema", type=Path, default=DEFAULT_ADL_SCHEMA)
    parser.add_argument("--adl-run-report", type=Path)
    parser.add_argument("--adl-run-schema", type=Path, default=DEFAULT_ADL_RUN_SCHEMA)
    parser.add_argument("--requirement-audit-report", type=Path)
    parser.add_argument(
        "--requirement-audit-schema",
        type=Path,
        default=DEFAULT_REQUIREMENT_AUDIT_SCHEMA,
    )
    parser.add_argument("--postgres-verification-report", type=Path)
    parser.add_argument(
        "--postgres-verification-schema",
        type=Path,
        default=DEFAULT_POSTGRES_VERIFICATION_SCHEMA,
    )
    parser.add_argument(
        "--check-schemas",
        action="store_true",
        help="validate all configured JSON Schemas even when an instance is not supplied",
    )
    args = parser.parse_args()

    selected = [
        (args.run_report, args.run_schema, "conformance run"),
        (args.case_matrix_report, args.case_matrix_schema, "conformance case matrix"),
        (args.coverage_report, args.coverage_schema, "conformance coverage"),
        (args.adl_report, args.adl_schema, "ADL compatibility"),
        (args.adl_run_report, args.adl_run_schema, "ADL execution compatibility"),
        (
            args.requirement_audit_report,
            args.requirement_audit_schema,
            "requirement catalog audit",
        ),
        (
            args.postgres_verification_report,
            args.postgres_verification_schema,
            "PostgreSQL verification",
        ),
    ]
    if args.check_schemas:
        for schema_path in [
            args.run_schema,
            args.case_matrix_schema,
            args.coverage_schema,
            args.adl_schema,
            args.adl_run_schema,
            args.requirement_audit_schema,
            args.postgres_verification_schema,
        ]:
            validate_schema(schema_path)
            print(f"validated JSON Schema: {schema_path}")

    if not args.check_schemas and not any(instance_path is not None for instance_path, _, _ in selected):
        raise ReportSchemaError("at least one report path must be supplied")

    for instance_path, schema_path, label in selected:
        if instance_path is not None:
            validate_instance(schema_path, instance_path, label)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
