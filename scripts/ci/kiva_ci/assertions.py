"""Small validation assertions shared by Kiva CI manifest checks."""

from __future__ import annotations

from pathlib import Path
from typing import Any, Callable, Iterable, TypeVar

from .manifest import (
    ManifestError,
    require_existing_path,
    require_mapping,
    require_string,
    validation_error,
)


ErrorT = TypeVar("ErrorT", bound=RuntimeError)


def assert_kind(
    document: dict[str, Any],
    expected_kind: str,
    error_cls: type[ErrorT] = ManifestError,
) -> None:
    """Assert that a manifest has the expected kind marker."""
    actual_kind = document.get("kind")
    if actual_kind != expected_kind:
        raise validation_error(error_cls, f"kind must be {expected_kind}: {actual_kind!r}")


def assert_required_fields(
    mapping: dict[str, Any],
    required_fields: Iterable[str],
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> None:
    """Assert that a mapping contains required non-empty fields."""
    for field in required_fields:
        value = mapping.get(field)
        if value is None or value == "":
            raise validation_error(error_cls, f"{label}.{field} is required")


def assert_unique_by(
    values: Iterable[dict[str, Any]],
    key_fn: Callable[[dict[str, Any]], tuple[object, ...]],
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> None:
    """Assert that mapping values are unique under a derived tuple key."""
    seen: set[tuple[object, ...]] = set()
    for value in values:
        key = key_fn(value)
        if key in seen:
            formatted = ", ".join(str(part) for part in key)
            raise validation_error(error_cls, f"{label} duplicates key: {formatted}")
        seen.add(key)


def assert_existing_manifest_path(
    root: Path,
    mapping: dict[str, Any],
    field: str,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> Path:
    """Validate one repository-relative path field."""
    raw_path = require_string(mapping.get(field), f"{label}.{field}", error_cls)
    return require_existing_path(root, raw_path, f"{label}.{field}", error_cls)


def require_mapping_field(
    mapping: dict[str, Any],
    field: str,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Return a nested mapping field."""
    return require_mapping(mapping.get(field), f"{label}.{field}", error_cls)
