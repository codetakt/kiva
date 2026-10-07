"""Common manifest validation helpers for Kiva CI scripts."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, TypeVar

import yaml


class ManifestError(RuntimeError):
    """Base error for shared manifest validation helpers."""


ErrorT = TypeVar("ErrorT", bound=RuntimeError)


def validation_error(error_cls: type[ErrorT], message: str) -> ErrorT:
    """Construct a validator-specific error."""
    return error_cls(message)


def load_yaml(path: Path, error_cls: type[ErrorT] = ManifestError) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        document = yaml.safe_load(handle)
    if not isinstance(document, dict):
        raise validation_error(error_cls, f"{path} root must be a mapping")
    return document


def load_json(path: Path, error_cls: type[ErrorT] = ManifestError) -> dict[str, Any]:
    """Load a JSON mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        document = json.load(handle)
    if not isinstance(document, dict):
        raise validation_error(error_cls, f"{path} root must be a mapping")
    return document


def require_mapping(
    value: Any,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise validation_error(error_cls, f"{label} must be a mapping")
    return value


def require_list(
    value: Any,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> list[Any]:
    """Return a list or raise a validation error."""
    if not isinstance(value, list):
        raise validation_error(error_cls, f"{label} must be a list")
    return value


def require_string(
    value: Any,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> str:
    """Return a non-empty string or raise a validation error."""
    if not isinstance(value, str) or not value:
        raise validation_error(error_cls, f"{label} must be a non-empty string")
    return value


def require_bool(
    value: Any,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> bool:
    """Return a boolean or raise a validation error."""
    if not isinstance(value, bool):
        raise validation_error(error_cls, f"{label} must be a boolean")
    return value


def require_string_list(
    value: Any,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
    *,
    allow_empty: bool = False,
) -> list[str]:
    """Return a list of non-empty strings."""
    values = require_list(value, label, error_cls)
    if not allow_empty and not values:
        raise validation_error(error_cls, f"{label} must be a non-empty list")
    if not all(isinstance(item, str) and item for item in values):
        raise validation_error(error_cls, f"{label} entries must be non-empty strings")
    return values


def mapping_by_unique_string_key(
    values: Any,
    *,
    key: str,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, dict[str, Any]]:
    """Return mapping rows keyed by a unique non-empty string field."""
    rows = require_list(values, label, error_cls)
    result: dict[str, dict[str, Any]] = {}
    for index, value in enumerate(rows):
        row = require_mapping(value, f"{label}[{index}]", error_cls)
        row_key = require_string(row.get(key), f"{label}[{index}].{key}", error_cls)
        if row_key in result:
            raise validation_error(error_cls, f"{label} duplicates {key}: {row_key}")
        result[row_key] = row
    return result


def optional_string_list(
    value: Any,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
    *,
    allow_empty: bool = True,
) -> list[str]:
    """Return an optional list of non-empty strings."""
    if value is None:
        return []
    return require_string_list(value, label, error_cls, allow_empty=allow_empty)


def generated_artifact_values(
    source_key: str,
    source: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> list[str]:
    """Return concrete generated artifact paths declared by a source entry."""
    targets: list[str] = []
    if "generated_artifacts" in source:
        targets.extend(
            require_string_list(
                source.get("generated_artifacts"),
                f"sources.{source_key}.generated_artifacts",
                error_cls,
            )
        )
    for field in ("bundle", "index"):
        if field in source:
            targets.append(
                require_string(
                    source.get(field),
                    f"sources.{source_key}.{field}",
                    error_cls,
                )
            )
    return sorted(set(targets))


def generated_target_values(
    source_key: str,
    source: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> list[str]:
    """Return generated graph targets declared by a source entry."""
    targets = generated_artifact_values(source_key, source, error_cls)
    if "projection_targets" in source:
        targets.extend(
            require_string_list(
                source.get("projection_targets"),
                f"sources.{source_key}.projection_targets",
                error_cls,
            )
        )
    return sorted(set(targets))


def source_reference_artifact_values(
    source_key: str,
    source: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> list[str]:
    """Return paths exported when another source entry references this source."""
    if "projection_exports" in source:
        return require_string_list(
            source.get("projection_exports"),
            f"sources.{source_key}.projection_exports",
            error_cls,
        )
    if "artifacts" in source:
        return require_string_list(
            source.get("artifacts"),
            f"sources.{source_key}.artifacts",
            error_cls,
        )
    targets = generated_target_values(source_key, source, error_cls)
    if targets:
        return targets
    raise validation_error(
        error_cls,
        f"sources.{source_key} cannot be used as a projection source ref without artifacts, projection_exports, or generated targets",
    )


def resolve_projection_sources(
    source_key: str,
    source: dict[str, Any],
    sources: dict[str, Any],
    error_cls: type[ErrorT] = ManifestError,
) -> list[str]:
    """Resolve direct projection source paths and source-key references."""
    resolved: list[str] = []
    for ref in optional_string_list(
        source.get("projection_source_refs"),
        f"sources.{source_key}.projection_source_refs",
        error_cls,
    ):
        referenced = sources.get(ref)
        if referenced is None:
            raise validation_error(
                error_cls,
                f"sources.{source_key}.projection_source_refs references missing source: {ref}",
            )
        referenced_source = require_mapping(
            referenced,
            f"sources.{ref}",
            error_cls,
        )
        resolved.extend(source_reference_artifact_values(ref, referenced_source, error_cls))
    resolved.extend(
        optional_string_list(
            source.get("projection_sources"),
            f"sources.{source_key}.projection_sources",
            error_cls,
        )
    )
    if not resolved:
        raise validation_error(
            error_cls,
            f"sources.{source_key} must declare projection_sources or projection_source_refs",
        )
    return sorted(set(resolved))


def validate_projection_check_refs(
    checks: Any,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> list[dict[str, Any]]:
    """Validate projection check references and return normalized mappings."""
    validators = require_list(checks, label, error_cls)
    if not validators:
        raise validation_error(error_cls, f"{label} must not be empty")
    normalized: list[dict[str, Any]] = []
    for index, raw_validator in enumerate(validators):
        validator = require_mapping(raw_validator, f"{label}[{index}]", error_cls)
        require_string(validator.get("group"), f"{label}[{index}].group", error_cls)
        require_string(validator.get("check_id"), f"{label}[{index}].check_id", error_cls)
        normalized.append(validator)
    return normalized


def relative_path(
    raw_path: str,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> Path:
    """Parse a repository-relative path."""
    path = Path(raw_path)
    if path.is_absolute():
        raise validation_error(error_cls, f"{label} must be repository-relative: {raw_path}")
    return path


def require_existing_path(
    root: Path,
    raw_path: str,
    label: str,
    error_cls: type[ErrorT] = ManifestError,
) -> Path:
    """Validate and return an existing repository-relative path."""
    path = relative_path(raw_path, label, error_cls)
    if not (root / path).exists():
        raise validation_error(error_cls, f"{label} path is missing: {raw_path}")
    return path
