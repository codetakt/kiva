#!/usr/bin/env python3
"""Export Kiva PDS OpenAPI component schemas as generated JSON artifacts."""

from __future__ import annotations

import argparse
import json
import shutil
import tempfile
from pathlib import Path
from typing import Any

import yaml


class PdsSchemaExportError(RuntimeError):
    """Raised when PDS schema export cannot be completed."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise PdsSchemaExportError(f"{path} root must be a mapping")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise an export error."""
    if not isinstance(value, dict):
        raise PdsSchemaExportError(f"{label} must be a mapping")
    return value


def write_json(path: Path, value: Any) -> None:
    """Write stable JSON."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def normalize_schema_refs(value: Any) -> Any:
    """Return a JSON-Schema artifact value with local OpenAPI component refs normalized."""
    if isinstance(value, dict):
        normalized: dict[str, Any] = {}
        for key, item in value.items():
            if key == "$ref" and isinstance(item, str):
                normalized[key] = item.replace("#/components/schemas/", "#/$defs/")
            else:
                normalized[key] = normalize_schema_refs(item)
        return normalized
    if isinstance(value, list):
        return [normalize_schema_refs(item) for item in value]
    return value


def generated_files(root: Path) -> dict[str, bytes]:
    """Read generated files for comparison."""
    return {
        path.relative_to(root).as_posix(): path.read_bytes()
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


def export_schemas(contract: Path, out_dir: Path) -> None:
    """Export PDS OpenAPI schemas to an output directory."""
    document = load_yaml(contract)
    info = require_mapping(document.get("info"), "info")
    components = require_mapping(document.get("components"), "components")
    schemas = require_mapping(components.get("schemas"), "components.schemas")

    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True)

    normalized_schemas = normalize_schema_refs(schemas)
    bundled = {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "urn:kiva:pds:openapi-component-schemas",
        "title": "Kiva PDS OpenAPI component schemas",
        "$defs": normalized_schemas,
    }
    write_json(out_dir / "components.schemas.json", bundled)

    schema_names = sorted(schemas)
    for schema_name in schema_names:
        schema = normalized_schemas[schema_name]
        if not isinstance(schema, dict):
            raise PdsSchemaExportError(f"components.schemas.{schema_name} must be a mapping")
        artifact = {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": f"urn:kiva:pds:schema:{schema_name}",
            "$ref": f"urn:kiva:pds:openapi-component-schemas#/$defs/{schema_name}",
        }
        write_json(out_dir / f"{schema_name}.schema.json", artifact)

    write_json(
        out_dir / "index.json",
        {
            "schema_version": 1,
            "kind": "pds-openapi-schema-export",
            "source_contract": contract.as_posix(),
            "contract_title": info.get("title"),
            "contract_version": info.get("version"),
            "schema_count": len(schema_names),
            "schemas": schema_names,
            "bundle": "components.schemas.json",
            "artifact_strategy": "bundle-plus-thin-schema-refs",
        },
    )


def check_export(contract: Path, out_dir: Path) -> None:
    """Check that an existing export directory is current."""
    if not out_dir.is_dir():
        raise PdsSchemaExportError(f"generated schema directory is missing: {out_dir}")
    with tempfile.TemporaryDirectory() as temp:
        temp_dir = Path(temp) / "schemas"
        export_schemas(contract, temp_dir)
        expected = generated_files(temp_dir)
        actual = generated_files(out_dir)
    if actual != expected:
        missing = sorted(set(expected) - set(actual))
        extra = sorted(set(actual) - set(expected))
        changed = sorted(
            path for path in set(expected).intersection(actual) if expected[path] != actual[path]
        )
        details = []
        if missing:
            details.append(f"missing: {', '.join(missing)}")
        if extra:
            details.append(f"extra: {', '.join(extra)}")
        if changed:
            details.append(f"changed: {', '.join(changed)}")
        raise PdsSchemaExportError(
            "generated PDS schema artifacts are not current; "
            + "; ".join(details)
            + f". Regenerate with scripts/ci/export_pds_openapi_schemas.py --out {out_dir}"
        )


def main() -> int:
    """Run PDS schema export or currentness check."""
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--contract",
        type=Path,
        default=Path("spec/openapi/kiva-pds.yaml"),
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("generated/pds-openapi-schemas"),
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    if args.check:
        check_export(args.contract, args.out)
        print(f"validated generated PDS schema artifacts: {args.out}")
    else:
        export_schemas(args.contract, args.out)
        print(f"exported PDS schema artifacts: {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
