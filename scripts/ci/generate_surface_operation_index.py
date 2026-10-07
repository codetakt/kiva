#!/usr/bin/env python3
"""Generate a surface operation index from the canonical surface catalog."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts
from surface_catalog import (
    load_http_operation_catalog,
    load_surface_catalog,
    require_surface_entries,
    surface_catalog_paths,
    surface_index_rows,
    validate_entries_against_http_catalog,
)
from surface_catalog_model import surface_operation_projection_basis


DEFAULT_SURFACE_CATALOG = Path("generated/architecture/surface-catalog.yaml")


class SurfaceOperationIndexError(RuntimeError):
    """Raised when the surface operation index cannot be built."""


def markdown_cell(value: object) -> str:
    """Render a Markdown table cell without introducing pipe ambiguity."""
    return str(value).replace("|", "\\|").replace("\n", " ")


def scopes_cell(scopes: list[str]) -> str:
    """Render scopes as a compact Markdown cell."""
    return ", ".join(f"`{scope}`" for scope in scopes)


def optional_cell(value: str | None) -> str:
    """Render an optional Markdown value."""
    return "" if value is None else f"`{markdown_cell(value)}`"


def render_markdown(
    rows: list[dict[str, Any]],
    surface_catalog: Path,
    http_catalog: Path,
) -> str:
    """Render a human-readable surface operation table."""
    table = [
        (
            "| OpenAPI Contract | Method | Path | Operation ID | Application Operation | "
            "Workflow | PDS Family | Route Context | Required Scopes |"
        ),
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for row in rows:
        table.append(
            "| "
            + " | ".join(
                [
                    f"`{markdown_cell(row['openapi_contract'])}`",
                    f"`{markdown_cell(str(row['method']).upper())}`",
                    f"`{markdown_cell(row['path'])}`",
                    f"`{markdown_cell(row['operation_id'])}`",
                    optional_cell(row["application_operation_id"]),
                    optional_cell(row["workflow_label"]),
                    optional_cell(row["pds_resource_family"]),
                    optional_cell(row["pds_route_context"]),
                    scopes_cell(row["required_scopes"]),
                ]
            )
            + " |"
        )
    return f"""# Generated Surface Operation Index

This file is generated from `{surface_catalog.as_posix()}` by
`scripts/ci/generate_surface_operation_index.py`.

The surface catalog is the canonical review surface for public HTTP operations,
application workflow ownership, OAuth/OIDC scopes, PDS route contexts, and PDS
resource-family ownership. It is validated against
`{http_catalog.as_posix()}` before this index is rendered.

{chr(10).join(table)}
"""


def render_evidence(
    rows: list[dict[str, Any]],
    surface_catalog: Path,
    http_catalog: Path,
    catalog: dict[str, Any],
) -> str:
    """Render machine-readable surface operation evidence."""
    application_count = sum(1 for row in rows if row["application_operation_id"] is not None)
    pds_family_count = sum(1 for row in rows if row["pds_resource_family"] is not None)
    evidence = {
        "schema_version": 1,
        "kind": "kiva-surface-operation-index-evidence",
        "surface_catalog": surface_catalog.as_posix(),
        "projection_basis": surface_operation_projection_basis(
            catalog,
            "review-index-evidence-projection",
            SurfaceOperationIndexError,
        ),
        "generated_from": [
            surface_catalog.as_posix(),
            http_catalog.as_posix(),
        ],
        "http_operation_count": len(rows),
        "application_operation_count": application_count,
        "pds_family_operation_count": pds_family_count,
        "operations": rows,
    }
    return json.dumps(evidence, indent=2, sort_keys=True) + "\n"


def parse_args() -> argparse.Namespace:
    """Parse CLI arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--surface-catalog", type=Path, default=DEFAULT_SURFACE_CATALOG)
    parser.add_argument("--http-catalog", type=Path)
    parser.add_argument("--markdown-out", type=Path)
    parser.add_argument("--evidence-out", type=Path)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def main() -> int:
    """Generate or validate the surface operation index."""
    args = parse_args()
    catalog = load_surface_catalog(args.surface_catalog, SurfaceOperationIndexError)
    catalog_paths = surface_catalog_paths(catalog, args.surface_catalog, SurfaceOperationIndexError)
    http_catalog = args.http_catalog or catalog_paths["http_catalog"]
    markdown_out = args.markdown_out or catalog_paths["markdown"]
    evidence_out = args.evidence_out or catalog_paths["evidence"]

    entries = require_surface_entries(catalog, SurfaceOperationIndexError)
    http_operations = load_http_operation_catalog(http_catalog, SurfaceOperationIndexError)
    validate_entries_against_http_catalog(entries, http_operations, SurfaceOperationIndexError)
    rows = surface_index_rows(catalog, SurfaceOperationIndexError)

    markdown = render_markdown(rows, args.surface_catalog, http_catalog)
    evidence = render_evidence(rows, args.surface_catalog, http_catalog, catalog)
    command_hint = "scripts/ci/generate_surface_operation_index.py"
    try:
        write_or_check_generated_artifacts(
            [
                GeneratedArtifact(markdown_out, markdown),
                GeneratedArtifact(evidence_out, evidence),
            ],
            check=args.check,
            command_hint=command_hint,
            error_cls=SurfaceOperationIndexError,
        )
    except SurfaceOperationIndexError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(
            "validated generated surface operation index: "
            f"{markdown_out}, {evidence_out}"
        )
        return 0
    print(
        "generated surface operation index: "
        f"{markdown_out}, {evidence_out}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
