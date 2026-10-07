"""Typed row helpers for machine-readable Kiva ledgers."""

from __future__ import annotations

from pathlib import Path
from typing import Any, TypeVar

from jsonschema import Draft202012Validator

from .manifest import require_list, require_mapping, require_string, validation_error


ErrorT = TypeVar("ErrorT", bound=RuntimeError)


def mapping_rows(
    values: Any,
    *,
    label: str,
    error_cls: type[ErrorT],
) -> list[tuple[int, dict[str, Any]]]:
    """Return indexed mapping rows from a YAML/JSON ledger list."""
    return [
        (index, require_mapping(value, f"{label}[{index}]", error_cls))
        for index, value in enumerate(require_list(values, label, error_cls))
    ]


def rows_by_unique_string_field(
    values: Any,
    *,
    key: str = "id",
    label: str,
    error_cls: type[ErrorT],
    duplicate_label: str | None = None,
) -> dict[str, dict[str, Any]]:
    """Return ledger rows keyed by one unique non-empty string field."""
    rows: dict[str, dict[str, Any]] = {}
    for index, row in mapping_rows(values, label=label, error_cls=error_cls):
        row_key = require_string(row.get(key), f"{label}[{index}].{key}", error_cls)
        if row_key in rows:
            noun = duplicate_label if duplicate_label is not None else key
            raise validation_error(error_cls, f"duplicate {noun}: {row_key}")
        rows[row_key] = row
    return rows


def require_declared_count(
    actual_count: int,
    declared_count: Any,
    *,
    label: str,
    error_cls: type[ErrorT],
) -> None:
    """Validate a declared ledger count against computed row cardinality."""
    if declared_count != actual_count:
        raise validation_error(error_cls, f"{label} must be {actual_count}")


def validate_json_schema(
    document: dict[str, Any],
    schema: dict[str, Any],
    *,
    label: str,
    error_cls: type[ErrorT],
) -> None:
    """Validate one generated document against a JSON Schema mapping."""
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document),
        key=lambda error: list(error.path),
    )
    if errors:
        details = "\n".join(
            f"- {'/'.join(str(part) for part in error.path) or '<root>'}: {error.message}"
            for error in errors
        )
        raise validation_error(error_cls, f"{label} schema validation failed:\n{details}")


def read_expected_text(path: Path, *, hint: str, error_cls: type[ErrorT]) -> str:
    """Read an expected generated artifact for a --check comparison."""
    if not path.exists():
        raise validation_error(error_cls, f"{path} is missing; {hint}")
    return path.read_text(encoding="utf-8")
