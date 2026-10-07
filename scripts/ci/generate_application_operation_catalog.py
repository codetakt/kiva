#!/usr/bin/env python3
"""Generate the Rust application operation catalog from the YAML ledger."""

from __future__ import annotations

import argparse
from pathlib import Path

import yaml

from application_operation_catalog_codegen import (
    render_operation_catalog_evidence,
    render_operation_catalog_markdown,
    render_operation_catalog,
    render_operation_ids,
    render_workflow_metadata,
)
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts
from surface_catalog import SURFACE_CATALOG_KIND, project_application_operation_catalog


class ApplicationOperationCatalogError(RuntimeError):
    """Raised when application operation catalog generation fails."""


def load_catalog(path: Path) -> dict[str, object]:
    """Load the YAML operation catalog."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise ValueError(f"{path} root must be a mapping")
    if value.get("kind") == SURFACE_CATALOG_KIND:
        return project_application_operation_catalog(value, ValueError)
    return value


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--catalog",
        type=Path,
        default=Path("generated/architecture/surface-catalog.yaml"),
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("crates/kiva-application/src/operation_catalog/generated/registry.rs"),
    )
    parser.add_argument(
        "--operation-ids-out",
        type=Path,
        default=Path("crates/kiva-application/src/operation_catalog/generated/operation_ids.rs"),
    )
    parser.add_argument(
        "--metadata-out",
        type=Path,
        default=Path("crates/kiva-application/src/operation_catalog/generated/workflow_metadata.rs"),
    )
    parser.add_argument(
        "--markdown-out",
        type=Path,
        default=Path("docs/architecture/generated/application-operation-catalog.md"),
    )
    parser.add_argument(
        "--evidence-out",
        type=Path,
        default=Path("generated/application-operation-catalog.evidence.json"),
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify the output file is current without writing it",
    )
    args = parser.parse_args()

    catalog = load_catalog(args.catalog)
    rendered = render_operation_catalog(catalog)
    rendered_operation_ids = render_operation_ids(catalog)
    rendered_metadata = render_workflow_metadata(catalog)
    rendered_markdown = render_operation_catalog_markdown(catalog)
    rendered_evidence = render_operation_catalog_evidence(catalog)
    artifacts = [
        GeneratedArtifact(args.out, rendered),
        GeneratedArtifact(args.operation_ids_out, rendered_operation_ids),
        GeneratedArtifact(args.metadata_out, rendered_metadata),
        GeneratedArtifact(args.markdown_out, rendered_markdown),
        GeneratedArtifact(args.evidence_out, rendered_evidence),
    ]
    command_hint = "scripts/ci/generate_application_operation_catalog.py"
    try:
        write_or_check_generated_artifacts(
            artifacts,
            check=args.check,
            command_hint=command_hint,
            error_cls=ApplicationOperationCatalogError,
        )
    except ApplicationOperationCatalogError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(
            "validated generated application operation catalog: "
            f"{args.out}, {args.operation_ids_out}, {args.metadata_out}, "
            f"{args.markdown_out}, {args.evidence_out}"
        )
        return
    print(
        "generated application operation catalog: "
        f"{args.out}, {args.operation_ids_out}, {args.metadata_out}, "
        f"{args.markdown_out}, {args.evidence_out}"
    )


if __name__ == "__main__":
    main()
