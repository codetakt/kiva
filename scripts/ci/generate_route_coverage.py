#!/usr/bin/env python3
"""Generate route coverage evidence from the canonical surface catalog."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts
from kiva_ci.route_coverage import build_route_coverage_rows
from surface_catalog import (
    load_http_operation_catalog,
    load_surface_catalog,
    require_surface_entries,
    surface_catalog_paths,
    validate_entries_against_http_catalog,
)
from surface_catalog_model import surface_operation_projection_basis


DEFAULT_SURFACE_CATALOG = Path("generated/architecture/surface-catalog.yaml")


class RouteCoverageError(RuntimeError):
    """Raised when route coverage evidence cannot be generated."""


def markdown_cell(value: object) -> str:
    """Render a Markdown table cell without introducing pipe ambiguity."""
    return str(value).replace("|", "\\|").replace("\n", " ")


def optional_cell(value: str | None) -> str:
    """Render an optional Markdown value."""
    return "" if value is None else f"`{markdown_cell(value)}`"


def render_markdown(
    rows: list[dict[str, Any]],
    surface_catalog: Path,
    http_catalog: Path,
) -> str:
    """Render human-readable route coverage."""
    table = [
        "| OpenAPI Contract | Method | Path | Operation ID | Route File | Route Anchor | Method Binding |",
        "| --- | --- | --- | --- | --- | --- | --- |",
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
                    f"`{markdown_cell(row['route_file'])}`",
                    optional_cell(row["path_constant"]) or f"`{markdown_cell(row['route_anchor'])}`",
                    f"`{markdown_cell(row['method_binding'])}`",
                ]
            )
            + " |"
        )
    return f"""# Generated Route Coverage

This file is generated from `{surface_catalog.as_posix()}` by
`scripts/ci/generate_route_coverage.py`.

The route coverage projection checks that every OpenAPI operation in the
surface catalog has a matching Axum route registration. `HEAD` operations are
covered through the corresponding `GET` route registration.

The surface catalog is validated against `{http_catalog.as_posix()}` before
route coverage is rendered.

{chr(10).join(table)}
"""


def render_evidence(
    rows: list[dict[str, Any]],
    surface_catalog: Path,
    http_catalog: Path,
    catalog: dict[str, Any],
) -> str:
    """Render machine-readable route coverage evidence."""
    evidence = {
        "schema_version": 1,
        "kind": "kiva-route-coverage-evidence",
        "surface_catalog": surface_catalog.as_posix(),
        "projection_basis": surface_operation_projection_basis(
            catalog,
            "axum-route-coverage-evidence-projection",
            RouteCoverageError,
        ),
        "generated_from": [
            surface_catalog.as_posix(),
            http_catalog.as_posix(),
        ],
        "operation_count": len(rows),
        "implicit_head_operation_count": sum(
            1 for row in rows if row["method_binding"] == "implicit-head-via-get"
        ),
        "routes": rows,
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
    """Generate or validate route coverage evidence."""
    args = parse_args()
    root = Path(".").resolve()
    catalog = load_surface_catalog(args.surface_catalog, RouteCoverageError)
    catalog_paths = surface_catalog_paths(catalog, args.surface_catalog, RouteCoverageError)
    http_catalog = args.http_catalog or catalog_paths["http_catalog"]
    markdown_out = args.markdown_out or catalog_paths["route_coverage_markdown"]
    evidence_out = args.evidence_out or catalog_paths["route_coverage_evidence"]

    entries = require_surface_entries(catalog, RouteCoverageError)
    http_operations = load_http_operation_catalog(http_catalog, RouteCoverageError)
    validate_entries_against_http_catalog(entries, http_operations, RouteCoverageError)
    rows = build_route_coverage_rows(
        root,
        catalog,
        args.surface_catalog,
        entries,
        RouteCoverageError,
    )
    markdown = render_markdown(rows, args.surface_catalog, http_catalog)
    evidence = render_evidence(rows, args.surface_catalog, http_catalog, catalog)
    command_hint = "scripts/ci/generate_route_coverage.py"
    try:
        write_or_check_generated_artifacts(
            [
                GeneratedArtifact(markdown_out, markdown),
                GeneratedArtifact(evidence_out, evidence),
            ],
            check=args.check,
            command_hint=command_hint,
            error_cls=RouteCoverageError,
        )
    except RouteCoverageError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated route coverage: {markdown_out}, {evidence_out}")
        return 0

    print(f"generated route coverage: {markdown_out}, {evidence_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
