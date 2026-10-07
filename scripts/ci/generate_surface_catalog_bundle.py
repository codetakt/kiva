#!/usr/bin/env python3
"""Generate the surface catalog bundle from split source ledgers."""

from __future__ import annotations

import argparse
from pathlib import Path

from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts
from surface_catalog import (
    load_operation_ownership_source,
    load_pds_resource_family_source,
    load_surface_family_source,
    load_surface_projection_policy,
    project_surface_catalog_bundle,
    render_yaml,
    surface_component_paths,
)


DEFAULT_POLICY = Path("spec/architecture/surface-projections.yaml")


class SurfaceCatalogBundleError(RuntimeError):
    """Raised when the surface catalog bundle cannot be generated."""


def parse_args() -> argparse.Namespace:
    """Parse CLI arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--policy", type=Path, default=DEFAULT_POLICY)
    parser.add_argument("--operation-ownership", type=Path)
    parser.add_argument("--pds-resource-families", type=Path)
    parser.add_argument("--surface-families", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def main() -> int:
    """Generate or validate the surface catalog bundle."""
    args = parse_args()
    policy = load_surface_projection_policy(args.policy, SurfaceCatalogBundleError)
    component_paths = surface_component_paths(policy, args.policy, SurfaceCatalogBundleError)
    operation_ownership_path = args.operation_ownership or component_paths["operation_ownership"]
    pds_family_path = args.pds_resource_families or component_paths["pds_resource_families"]
    surface_family_path = args.surface_families or component_paths["surface_families"]
    output_path = args.out or component_paths["generated_bundle"]

    operation_ownership_source = load_operation_ownership_source(
        operation_ownership_path,
        SurfaceCatalogBundleError,
    )
    pds_family_source = load_pds_resource_family_source(pds_family_path, SurfaceCatalogBundleError)
    surface_family_source = load_surface_family_source(surface_family_path, SurfaceCatalogBundleError)
    bundle = project_surface_catalog_bundle(
        operation_ownership_source,
        policy,
        pds_family_source,
        surface_family_source,
        operation_ownership_path=operation_ownership_path,
        policy_path=args.policy,
        pds_family_path=pds_family_path,
        surface_family_path=surface_family_path,
        error_cls=SurfaceCatalogBundleError,
    )
    rendered = render_yaml(bundle)
    command_hint = "scripts/ci/generate_surface_catalog_bundle.py"
    try:
        write_or_check_generated_artifacts(
            [GeneratedArtifact(output_path, rendered)],
            check=args.check,
            command_hint=command_hint,
            error_cls=SurfaceCatalogBundleError,
        )
    except SurfaceCatalogBundleError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated surface catalog bundle: {output_path}")
        return 0
    print(f"generated surface catalog bundle: {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
