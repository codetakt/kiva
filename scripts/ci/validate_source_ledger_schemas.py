#!/usr/bin/env python3
"""Validate canonical source ledgers against registered JSON Schemas."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.ledger import validate_json_schema
from kiva_ci.manifest import load_json, load_yaml, require_list, require_mapping, require_string


DEFAULT_REGISTRY = Path("spec/architecture/source-ledger-schemas.yaml")


class SourceLedgerSchemaError(RuntimeError):
    """Raised when a source ledger schema validation fails."""


def validate_registry(registry: dict[str, Any], registry_path: Path) -> list[tuple[Path, Path]]:
    """Return source/schema path pairs from the registry."""
    if registry.get("schema_version") != 1:
        raise SourceLedgerSchemaError(f"{registry_path}.schema_version must be 1")
    if registry.get("kind") != "kiva-source-ledger-schema-registry":
        raise SourceLedgerSchemaError(f"{registry_path}.kind must be kiva-source-ledger-schema-registry")
    if registry.get("status") != "active":
        raise SourceLedgerSchemaError(f"{registry_path}.status must be active")
    pairs: list[tuple[Path, Path]] = []
    for index, value in enumerate(require_list(registry.get("schemas"), f"{registry_path}.schemas", SourceLedgerSchemaError)):
        row = require_mapping(value, f"{registry_path}.schemas[{index}]", SourceLedgerSchemaError)
        pairs.append(
            (
                Path(require_string(row.get("source"), f"{registry_path}.schemas[{index}].source", SourceLedgerSchemaError)),
                Path(require_string(row.get("schema"), f"{registry_path}.schemas[{index}].schema", SourceLedgerSchemaError)),
            )
        )
    if not pairs:
        raise SourceLedgerSchemaError(f"{registry_path}.schemas must not be empty")
    return pairs


def validate_sources(root: Path, registry_path: Path) -> list[str]:
    """Validate every source ledger declared by the registry."""
    registry = load_yaml(root / registry_path, SourceLedgerSchemaError)
    validated: list[str] = []
    for source_path, schema_path in validate_registry(registry, registry_path):
        source = load_yaml(root / source_path, SourceLedgerSchemaError)
        schema = load_json(root / schema_path, SourceLedgerSchemaError)
        validate_json_schema(
            source,
            schema,
            label=f"{source_path} against {schema_path}",
            error_cls=SourceLedgerSchemaError,
        )
        validated.append(source_path.as_posix())
    return validated


def main() -> int:
    """Validate registered source ledgers."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--registry", type=Path, default=DEFAULT_REGISTRY)
    args = parser.parse_args()

    validated = validate_sources(args.root.resolve(), args.registry)
    print(f"validated source ledger schemas: {len(validated)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
