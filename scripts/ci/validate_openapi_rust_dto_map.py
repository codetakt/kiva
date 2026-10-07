#!/usr/bin/env python3
"""Validate OpenAPI schema to Rust DTO boundary mappings."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from kiva_ci.manifest import load_yaml as manifest_load_yaml
from kiva_ci.manifest import require_mapping as manifest_require_mapping
from kiva_ci.openapi_dto_map import load_openapi_dto_map, rust_source_paths
from kiva_ci.rust_dto import (
    RustDtoError,
    public_rust_items,
    rust_enum_labels,
    rust_source_text,
    rust_struct_fields,
    sorted_list,
)


class MappingError(RuntimeError):
    """Raised when the OpenAPI/Rust DTO mapping is inconsistent."""


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a YAML mapping or raise a mapping-specific validation error."""
    return manifest_require_mapping(value, label, MappingError)


def schema_rust_type(mapping_value: Any, name: str, schema_name: str) -> tuple[str, str]:
    """Return Rust type and shape validation mode for one schema mapping."""
    if isinstance(mapping_value, str) and mapping_value:
        return mapping_value, "presence"
    if not isinstance(mapping_value, dict):
        raise MappingError(f"{name}.{schema_name} maps to an invalid Rust type")
    rust_type = mapping_value.get("rust")
    if not isinstance(rust_type, str) or not rust_type:
        raise MappingError(f"{name}.{schema_name}.rust must be a non-empty string")
    shape = mapping_value.get("shape", "presence")
    if shape not in {"presence", "object", "enum"}:
        raise MappingError(f"{name}.{schema_name}.shape is invalid: {shape}")
    return rust_type, shape


def validate_object_shape(
    name: str,
    schema_name: str,
    rust_type: str,
    schema: dict[str, Any],
    source: str,
) -> None:
    """Validate an OpenAPI object schema against a Rust struct field shape."""
    if schema.get("type") != "object":
        raise MappingError(f"{name}.{schema_name}: OpenAPI schema is not an object")
    properties = require_mapping(schema.get("properties"), f"{name}.{schema_name}.properties")
    required = schema.get("required", [])
    if not isinstance(required, list) or not all(isinstance(item, str) for item in required):
        raise MappingError(f"{name}.{schema_name}.required must be a string list")

    openapi_fields = set(properties)
    openapi_required = set(required)
    rust_fields = rust_struct_fields(source, rust_type)
    rust_field_names = set(rust_fields)
    rust_required = {field for field, is_required in rust_fields.items() if is_required}

    if openapi_fields != rust_field_names:
        raise MappingError(
            f"{name}.{schema_name}: object fields differ; "
            f"OpenAPI={sorted_list(openapi_fields)} Rust={sorted_list(rust_field_names)}"
        )
    if openapi_required != rust_required:
        raise MappingError(
            f"{name}.{schema_name}: required fields differ; "
            f"OpenAPI={sorted_list(openapi_required)} Rust={sorted_list(rust_required)}"
        )


def validate_enum_shape(
    name: str,
    schema_name: str,
    rust_type: str,
    schema: dict[str, Any],
    source: str,
) -> None:
    """Validate an OpenAPI enum schema against a Rust serde enum shape."""
    values = schema.get("enum")
    if not isinstance(values, list) or not all(isinstance(item, str) for item in values):
        raise MappingError(f"{name}.{schema_name}.enum must be a string list")
    openapi_values = set(values)
    rust_values = rust_enum_labels(source, rust_type)
    if openapi_values != rust_values:
        raise MappingError(
            f"{name}.{schema_name}: enum values differ; "
            f"OpenAPI={sorted_list(openapi_values)} Rust={sorted_list(rust_values)}"
        )


def validate_contract(root: Path, contract: dict[str, Any], index: int) -> None:
    """Validate one OpenAPI/Rust mapping contract."""
    name = contract.get("name")
    if not isinstance(name, str) or not name:
        raise MappingError(f"contracts[{index}].name must be a non-empty string")

    openapi_path = contract.get("openapi")
    if not isinstance(openapi_path, str) or not openapi_path:
        raise MappingError(f"{name}.openapi must be a non-empty string")

    openapi = manifest_load_yaml(root / openapi_path, MappingError)
    components = require_mapping(openapi.get("components"), f"{name}.components")
    schemas = require_mapping(components.get("schemas"), f"{name}.components.schemas")
    rust_paths = rust_source_paths(contract, name, MappingError)
    rust_items = set()
    for rust_path in rust_paths:
        rust_items.update(public_rust_items(root / rust_path))
    rust_source = rust_source_text(root, rust_paths)
    mappings = require_mapping(contract.get("schemas"), f"{name}.schemas")

    for schema_name, mapping_value in sorted(mappings.items()):
        if not isinstance(schema_name, str) or not schema_name:
            raise MappingError(f"{name}.schemas contains an invalid schema name")
        rust_type, shape = schema_rust_type(mapping_value, name, schema_name)
        if schema_name not in schemas:
            raise MappingError(f"{name}: OpenAPI schema is missing: {schema_name}")
        if rust_type not in rust_items:
            raise MappingError(f"{name}: Rust DTO is missing: {rust_type}")
        schema = require_mapping(schemas[schema_name], f"{name}.{schema_name}")
        if shape == "object":
            validate_object_shape(name, schema_name, rust_type, schema, rust_source)
        elif shape == "enum":
            validate_enum_shape(name, schema_name, rust_type, schema, rust_source)


def validate_mapping(path: Path) -> None:
    """Validate all mapping contracts in a mapping file."""
    resolved_path = path.resolve()
    root = resolved_path.parent.parent.parent
    document = load_openapi_dto_map(root, resolved_path.relative_to(root), MappingError)
    contracts = document.get("contracts")
    if not isinstance(contracts, list) or not contracts:
        raise MappingError("contracts must be a non-empty list after source map expansion")
    for index, contract in enumerate(contracts):
        validate_contract(root, require_mapping(contract, f"contracts[{index}]"), index)


def main() -> int:
    """CLI entry point."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "mapping",
        nargs="?",
        default="spec/openapi/semantic-compatibility-map.yaml",
        type=Path,
        help="OpenAPI/Rust DTO mapping file",
    )
    args = parser.parse_args()

    try:
        validate_mapping(args.mapping)
    except (MappingError, RustDtoError) as error:
        print(f"error: {error}")
        return 1

    print(f"validated {args.mapping}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
