#!/usr/bin/env python3
"""Generate the HTTP operation catalog from OpenAPI contracts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from kiva_ci.openapi import collect_http_operations
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_CONTRACTS = [
    Path("spec/openapi/kiva-analytics.yaml"),
    Path("spec/openapi/kiva-xapi.yaml"),
    Path("spec/openapi/kiva-pds.yaml"),
    Path("spec/openapi/kiva-confidential.yaml"),
]
DEFAULT_MARKDOWN = Path("docs/architecture/generated/http-operation-catalog.md")
DEFAULT_EVIDENCE = Path("generated/http-operation-catalog.evidence.json")


class HttpOperationCatalogError(RuntimeError):
    """Raised when the generated HTTP operation catalog cannot be built."""


def markdown_cell(value: object) -> str:
    """Render a Markdown table cell without introducing pipe ambiguity."""
    return str(value).replace("|", "\\|").replace("\n", " ")


def build_catalog(contract_paths: list[Path]) -> list[dict[str, object]]:
    """Build and validate the combined HTTP operation catalog."""
    operations: list[dict[str, object]] = []
    seen_operation_ids: set[tuple[str, str]] = set()
    seen_routes: set[tuple[str, str, str]] = set()
    for contract_path in contract_paths:
        contract_operations = collect_http_operations(contract_path, HttpOperationCatalogError)
        for operation in contract_operations:
            operation_id_key = (
                operation["openapi_contract"],
                operation["operation_id"],
            )
            route_key = (
                operation["openapi_contract"],
                operation["path"],
                operation["method"],
            )
            if operation_id_key in seen_operation_ids:
                raise HttpOperationCatalogError(
                    "duplicate operationId in HTTP operation catalog: "
                    f"{operation['openapi_contract']}#{operation['operation_id']}"
                )
            if route_key in seen_routes:
                raise HttpOperationCatalogError(
                    "duplicate route in HTTP operation catalog: "
                    f"{operation['openapi_contract']} {operation['method']} {operation['path']}"
                )
            seen_operation_ids.add(operation_id_key)
            seen_routes.add(route_key)
            operations.append(operation)
    return operations


def render_markdown(contract_paths: list[Path], operations: list[dict[str, Any]]) -> str:
    """Render the HTTP operation catalog as Markdown review evidence."""
    rows = [
        "| OpenAPI Contract | Method | Path | Operation ID | Required Scopes | Tags |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for operation in operations:
        scopes = ", ".join(f"`{scope}`" for scope in operation["required_scopes"])
        tags = ", ".join(f"`{tag}`" for tag in operation["tags"])
        rows.append(
            "| "
            + " | ".join(
                [
                    f"`{markdown_cell(operation['openapi_contract'])}`",
                    f"`{markdown_cell(operation['method'].upper())}`",
                    f"`{markdown_cell(operation['path'])}`",
                    f"`{markdown_cell(operation['operation_id'])}`",
                    scopes,
                    tags,
                ]
            )
            + " |"
        )
    contract_list = ", ".join(f"`{path.as_posix()}`" for path in contract_paths)
    return f"""# Generated HTTP Operation Catalog

This file is generated from {contract_list} by
`scripts/ci/generate_http_operation_catalog.py`.

It is review evidence for the complete HTTP operation projection from OpenAPI.
Application workflow and PDS family ownership are projected from the generated
surface catalog bundle at `generated/architecture/surface-catalog.yaml`.

{chr(10).join(rows)}
"""


def render_evidence(contract_paths: list[Path], operations: list[dict[str, Any]]) -> str:
    """Render the HTTP operation catalog as machine-readable evidence."""
    evidence = {
        "schema_version": 1,
        "kind": "kiva-http-operation-catalog-evidence",
        "generated_from": [path.as_posix() for path in contract_paths],
        "operation_count": len(operations),
        "operations": operations,
    }
    return json.dumps(evidence, indent=2, sort_keys=True) + "\n"


def main() -> int:
    """Generate or validate the HTTP operation catalog artifacts."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--contract",
        action="append",
        type=Path,
        dest="contracts",
        help="OpenAPI contract to include. May be repeated.",
    )
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--evidence-out", type=Path, default=DEFAULT_EVIDENCE)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    contract_paths = args.contracts if args.contracts is not None else DEFAULT_CONTRACTS
    operations = build_catalog(contract_paths)
    markdown = render_markdown(contract_paths, operations)
    evidence = render_evidence(contract_paths, operations)
    command_hint = "scripts/ci/generate_http_operation_catalog.py"
    try:
        write_or_check_generated_artifacts(
            [
                GeneratedArtifact(args.markdown_out, markdown),
                GeneratedArtifact(args.evidence_out, evidence),
            ],
            check=args.check,
            command_hint=command_hint,
            error_cls=HttpOperationCatalogError,
        )
    except HttpOperationCatalogError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(
            "validated generated HTTP operation catalog: "
            f"{args.markdown_out}, {args.evidence_out}"
        )
        return 0

    print(
        "generated HTTP operation catalog: "
        f"{args.markdown_out}, {args.evidence_out}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
