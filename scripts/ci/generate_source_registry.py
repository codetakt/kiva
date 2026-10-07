#!/usr/bin/env python3
"""Generate a unified SourceRegistry view from compiled architecture."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_COMPILED = Path("generated/architecture/compiled-architecture.yaml")
DEFAULT_SCHEMA_REGISTRY = Path("spec/architecture/source-ledger-schemas.yaml")
DEFAULT_LEDGER_CATALOG = Path("spec/architecture/ledger-catalog.source.yaml")
DEFAULT_OUTPUT = Path("generated/architecture/source-registry.yaml")
DEFAULT_MARKDOWN = Path("docs/architecture/generated/source-registry.md")


class SourceRegistryError(RuntimeError):
    """Raised when SourceRegistry generation fails."""


def schema_by_source(registry: dict[str, Any], registry_path: Path) -> dict[str, str]:
    """Return schema paths keyed by source path."""
    rows: dict[str, str] = {}
    for index, raw_row in enumerate(require_list(registry.get("schemas"), f"{registry_path}.schemas", SourceRegistryError)):
        row = require_mapping(raw_row, f"{registry_path}.schemas[{index}]", SourceRegistryError)
        source = require_string(row.get("source"), f"{registry_path}.schemas[{index}].source", SourceRegistryError)
        schema = require_string(row.get("schema"), f"{registry_path}.schemas[{index}].schema", SourceRegistryError)
        rows[source] = schema
    return rows


def catalog_rows_by_path(catalog: dict[str, Any], catalog_path: Path) -> dict[str, dict[str, Any]]:
    """Return LedgerCatalog rows keyed by ledger path."""
    rows: dict[str, dict[str, Any]] = {}
    if catalog.get("kind") != "kiva-ledger-catalog-source":
        raise SourceRegistryError(f"{catalog_path}.kind must be kiva-ledger-catalog-source")
    for index, raw_row in enumerate(require_list(catalog.get("ledgers"), f"{catalog_path}.ledgers", SourceRegistryError)):
        row = require_mapping(raw_row, f"{catalog_path}.ledgers[{index}]", SourceRegistryError)
        path = require_string(row.get("path"), f"{catalog_path}.ledgers[{index}].path", SourceRegistryError)
        if path in rows:
            raise SourceRegistryError(f"duplicate LedgerCatalog path: {path}")
        rows[path] = row
    return rows


def list_strings(value: Any) -> list[str]:
    """Return stringified list values."""
    if not isinstance(value, list):
        return []
    return [str(item) for item in value]


def source_rows(
    compiled: dict[str, Any],
    schemas: dict[str, str],
    ledger_catalog: dict[str, dict[str, Any]],
) -> list[dict[str, Any]]:
    """Return SourceRegistry rows."""
    sources = require_mapping(compiled.get("sources"), "compiled.sources", SourceRegistryError)
    rows: list[dict[str, Any]] = []
    for source_id in sorted(sources):
        source = require_mapping(sources[source_id], f"compiled.sources.{source_id}", SourceRegistryError)
        artifacts = list_strings(source.get("artifacts"))
        source_schema = next((schemas[artifact] for artifact in artifacts if artifact in schemas), None)
        row: dict[str, Any] = {
            "id": source_id,
            "source_layer": require_string(source.get("source_layer"), f"{source_id}.source_layer", SourceRegistryError),
            "source_kind": require_string(source.get("source_kind"), f"{source_id}.source_kind", SourceRegistryError),
            "owner": require_string(source.get("owner"), f"{source_id}.owner", SourceRegistryError),
            "purpose": require_string(source.get("purpose"), f"{source_id}.purpose", SourceRegistryError),
            "artifacts": artifacts,
            "schema": source_schema,
        }
        if "projection_id" in source:
            row["projection"] = {
                "id": require_string(source.get("projection_id"), f"{source_id}.projection_id", SourceRegistryError),
                "kind": require_string(source.get("projection_kind"), f"{source_id}.projection_kind", SourceRegistryError),
                "targets": list_strings(source.get("generated_artifacts")) + list_strings(source.get("projection_targets")),
            }
        catalog_row = next((ledger_catalog[artifact] for artifact in artifacts if artifact in ledger_catalog), None)
        if catalog_row is not None:
            row["lifecycle"] = require_string(catalog_row.get("lifecycle"), f"ledger_catalog.{source_id}.lifecycle", SourceRegistryError)
            source_of_truth = catalog_row.get("source_of_truth")
            if not isinstance(source_of_truth, bool):
                raise SourceRegistryError(f"ledger_catalog.{source_id}.source_of_truth must be a boolean")
            row["source_of_truth"] = source_of_truth
            row["review_surface"] = require_string(catalog_row.get("review_surface"), f"ledger_catalog.{source_id}.review_surface", SourceRegistryError)
            if "generated_by" in catalog_row:
                row["generated_by"] = require_string(catalog_row.get("generated_by"), f"ledger_catalog.{source_id}.generated_by", SourceRegistryError)
            if "validates_with" in catalog_row:
                row["validates_with"] = require_string(catalog_row.get("validates_with"), f"ledger_catalog.{source_id}.validates_with", SourceRegistryError)
        rows.append(row)
    return rows


def registry_document(
    root: Path,
    compiled_path: Path,
    schema_registry_path: Path,
    ledger_catalog_path: Path,
    output_path: Path,
    markdown_path: Path,
) -> dict[str, Any]:
    """Return the generated SourceRegistry document."""
    compiled = load_yaml(root / compiled_path, SourceRegistryError)
    schema_registry = load_yaml(root / schema_registry_path, SourceRegistryError)
    ledger_catalog_source = load_yaml(root / ledger_catalog_path, SourceRegistryError)
    schemas = schema_by_source(schema_registry, schema_registry_path)
    ledger_catalog = catalog_rows_by_path(ledger_catalog_source, ledger_catalog_path)
    rows = source_rows(compiled, schemas, ledger_catalog)
    return {
        "schema_version": 1,
        "kind": "kiva-source-registry",
        "status": "active",
        "description": "Unified generated SourceRegistry view for architecture source nodes, layer classification, schemas, and projection roles.",
        "generated_from": [
            compiled_path.as_posix(),
            schema_registry_path.as_posix(),
            ledger_catalog_path.as_posix(),
        ],
        "generated_artifacts": {
            "source_registry": output_path.as_posix(),
            "markdown": markdown_path.as_posix(),
        },
        "source_count": len(rows),
        "schema_covered_source_count": sum(1 for row in rows if row.get("schema") is not None),
        "source_of_truth_count": sum(1 for row in rows if row.get("source_of_truth") is True),
        "sources": rows,
    }


def render_markdown(document: dict[str, Any]) -> str:
    """Render SourceRegistry as Markdown."""
    lines = [
        "# Generated SourceRegistry",
        "",
        "This file is generated by `scripts/ci/generate_source_registry.py`.",
        "",
        "| Source | Layer | Kind | Lifecycle | Source of truth | Owner | Schema |",
        "| --- | --- | --- | --- | --- | --- | --- |",
    ]
    for row in require_list(document.get("sources"), "source_registry.sources", SourceRegistryError):
        source = require_mapping(row, "source_registry.sources[]", SourceRegistryError)
        schema = source.get("schema") or ""
        lifecycle = source.get("lifecycle") or ""
        source_of_truth = source.get("source_of_truth")
        lines.append(
            "| "
            + " | ".join(
                [
                    f"`{source['id']}`",
                    f"`{source['source_layer']}`",
                    f"`{source['source_kind']}`",
                    f"`{lifecycle}`" if lifecycle else "",
                    "" if source_of_truth is None else f"`{source_of_truth}`",
                    f"`{source['owner']}`",
                    f"`{schema}`" if schema else "",
                ]
            )
            + " |"
        )
    return "\n".join(lines) + "\n"


def rendered_artifacts(
    root: Path,
    compiled_path: Path,
    schema_registry_path: Path,
    ledger_catalog_path: Path,
    output_path: Path,
    markdown_path: Path,
) -> list[GeneratedArtifact]:
    """Return rendered SourceRegistry artifacts."""
    document = registry_document(root, compiled_path, schema_registry_path, ledger_catalog_path, output_path, markdown_path)
    return [
        GeneratedArtifact(root / output_path, dump_yaml(document)),
        GeneratedArtifact(root / markdown_path, render_markdown(document)),
    ]


def main() -> int:
    """Generate or validate SourceRegistry artifacts."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--compiled", type=Path, default=DEFAULT_COMPILED)
    parser.add_argument("--schema-registry", type=Path, default=DEFAULT_SCHEMA_REGISTRY)
    parser.add_argument("--ledger-catalog", type=Path, default=DEFAULT_LEDGER_CATALOG)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    artifacts = rendered_artifacts(
        root,
        args.compiled,
        args.schema_registry,
        args.ledger_catalog,
        args.out,
        args.markdown_out,
    )
    write_or_check_generated_artifacts(
        artifacts,
        check=args.check,
        command_hint="scripts/ci/generate_source_registry.py",
        error_cls=SourceRegistryError,
    )
    verb = "validated" if args.check else "generated"
    print(f"{verb} SourceRegistry: {args.out}, {args.markdown_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
