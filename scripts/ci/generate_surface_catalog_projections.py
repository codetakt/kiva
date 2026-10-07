#!/usr/bin/env python3
"""Generate surface-catalog projection ledgers."""

from __future__ import annotations

import argparse
from pathlib import Path

from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts
from surface_catalog import (
    load_surface_catalog,
    project_application_operation_catalog,
    project_pds_resource_family_ledger,
    project_surface_family_ledger,
    render_surface_profile_registry,
    render_yaml,
    surface_catalog_paths,
)


DEFAULT_SURFACE_CATALOG = Path("generated/architecture/surface-catalog.yaml")


class SurfaceCatalogProjectionError(RuntimeError):
    """Raised when surface-catalog projections cannot be generated."""


def parse_args() -> argparse.Namespace:
    """Parse CLI arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--surface-catalog", type=Path, default=DEFAULT_SURFACE_CATALOG)
    parser.add_argument("--application-catalog-out", type=Path)
    parser.add_argument("--pds-families-out", type=Path)
    parser.add_argument("--surface-families-out", type=Path)
    parser.add_argument("--surface-profile-registry-out", type=Path)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def main() -> int:
    """Generate or validate projection ledgers."""
    args = parse_args()
    document = load_surface_catalog(args.surface_catalog, SurfaceCatalogProjectionError)
    paths = surface_catalog_paths(document, args.surface_catalog, SurfaceCatalogProjectionError)
    application_catalog = args.application_catalog_out or paths["application_catalog"]
    pds_families = args.pds_families_out or paths["pds_families"]
    surface_families = args.surface_families_out or paths["surface_families"]
    surface_profile_registry = (
        args.surface_profile_registry_out or paths["rust_surface_profile_registry"]
    )

    rendered_application_catalog = render_yaml(
        project_application_operation_catalog(document, SurfaceCatalogProjectionError),
    )
    rendered_pds_families = render_yaml(
        project_pds_resource_family_ledger(document, SurfaceCatalogProjectionError),
    )
    rendered_surface_families = render_yaml(
        project_surface_family_ledger(document, SurfaceCatalogProjectionError),
    )
    rendered_surface_profile_registry = render_surface_profile_registry(
        document,
        SurfaceCatalogProjectionError,
    )
    artifacts = [
        GeneratedArtifact(application_catalog, rendered_application_catalog),
        GeneratedArtifact(pds_families, rendered_pds_families),
        GeneratedArtifact(surface_families, rendered_surface_families),
        GeneratedArtifact(surface_profile_registry, rendered_surface_profile_registry),
    ]
    command_hint = "scripts/ci/generate_surface_catalog_projections.py"
    try:
        write_or_check_generated_artifacts(
            artifacts,
            check=args.check,
            command_hint=command_hint,
            error_cls=SurfaceCatalogProjectionError,
        )
    except SurfaceCatalogProjectionError as error:
        raise SystemExit(str(error)) from error

    if args.check:
        print(
            "validated surface catalog projections: "
            f"{application_catalog}, {pds_families}, {surface_families}, {surface_profile_registry}"
        )
        return 0

    print(
        "generated surface catalog projections: "
        f"{application_catalog}, {pds_families}, {surface_families}, {surface_profile_registry}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
