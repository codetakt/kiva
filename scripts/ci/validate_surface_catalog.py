#!/usr/bin/env python3
"""Validate the canonical Kiva surface operation catalog."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from kiva_ci.manifest import (
    load_json,
    relative_path,
    require_existing_path,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
)
from surface_catalog import (
    load_operation_ownership_source,
    load_pds_resource_family_source,
    load_surface_family_source,
    load_http_operation_catalog,
    load_surface_catalog,
    load_surface_projection_policy,
    project_application_operation_catalog,
    project_surface_catalog_bundle,
    project_pds_resource_family_ledger,
    project_surface_family_ledger,
    render_yaml,
    require_surface_entries,
    surface_projection_targets,
    surface_catalog_paths,
    surface_index_rows,
    surface_operation_normal_form,
    validate_entries_against_http_catalog,
)


DEFAULT_MANIFEST = Path("generated/architecture/surface-catalog.yaml")


class SurfaceCatalogError(RuntimeError):
    """Raised when the surface catalog is inconsistent."""


def validate_source_ledgers(root: Path, manifest: Path, document: dict[str, Any]) -> list[str]:
    """Validate canonical input references."""
    checked: list[str] = []
    source_ledgers = require_mapping(
        document.get("source_ledgers"),
        f"{manifest}.source_ledgers",
        SurfaceCatalogError,
    )
    ledger = require_mapping(
        source_ledgers.get("http_operation_catalog"),
        f"{manifest}.source_ledgers.http_operation_catalog",
        SurfaceCatalogError,
    )
    role = require_string(
        ledger.get("role"),
        f"{manifest}.source_ledgers.http_operation_catalog.role",
        SurfaceCatalogError,
    )
    if role != "complete-openapi-http-operation-projection":
        raise SurfaceCatalogError(
            f"{manifest}.source_ledgers.http_operation_catalog.role must describe complete HTTP projection",
        )
    checked.append(
        require_existing_path(
            root,
            require_string(
                ledger.get("path"),
                f"{manifest}.source_ledgers.http_operation_catalog.path",
                SurfaceCatalogError,
            ),
            f"{manifest}.source_ledgers.http_operation_catalog.path",
            SurfaceCatalogError,
        ).as_posix()
    )
    return checked


def validate_surface_operation_algebra(
    manifest: Path,
    document: dict[str, Any],
    paths: dict[str, Path],
) -> None:
    """Validate the single SurfaceOperation algebra declared by the catalog."""
    algebra = require_mapping(
        document.get("surface_operation_algebra"),
        f"{manifest}.surface_operation_algebra",
        SurfaceCatalogError,
    )
    root_type = require_string(
        algebra.get("root_type"),
        f"{manifest}.surface_operation_algebra.root_type",
        SurfaceCatalogError,
    )
    if root_type != "SurfaceOperation":
        raise SurfaceCatalogError(
            f"{manifest}.surface_operation_algebra.root_type must be SurfaceOperation"
        )
    join_type = require_string(
        algebra.get("join_type"),
        f"{manifest}.surface_operation_algebra.join_type",
        SurfaceCatalogError,
    )
    if join_type != "HttpOperation + Optional<SemanticOperationOverlay>":
        raise SurfaceCatalogError(
            f"{manifest}.surface_operation_algebra.join_type must be "
            "HttpOperation + Optional<SemanticOperationOverlay>"
        )
    if algebra.get("sole_operation_algebra") is not True:
        raise SurfaceCatalogError(
            f"{manifest}.surface_operation_algebra.sole_operation_algebra must be true"
        )
    component_sources = require_mapping(
        document.get("component_sources"),
        f"{manifest}.component_sources",
        SurfaceCatalogError,
    )
    expected_paths = {
        "canonical_source": require_string(
            component_sources.get("operation_ownership"),
            f"{manifest}.component_sources.operation_ownership",
            SurfaceCatalogError,
        ),
        "policy_source": require_string(
            component_sources.get("surface_projections"),
            f"{manifest}.component_sources.surface_projections",
            SurfaceCatalogError,
        ),
        "generated_review_surface": manifest.as_posix(),
    }
    for key, expected in expected_paths.items():
        actual = require_string(
            algebra.get(key),
            f"{manifest}.surface_operation_algebra.{key}",
            SurfaceCatalogError,
        )
        if actual != expected:
            raise SurfaceCatalogError(
                f"{manifest}.surface_operation_algebra.{key} must be {expected}"
            )
    required_facets = {
        "openapi_contract",
        "method",
        "path",
        "operation_id",
        "required_scopes",
        "application_operation_id",
        "application_surface",
        "workflow_label",
        "rust_workflow_kind",
        "pds_resource_family",
        "pds_route_context",
    }
    expected_openapi_derived_facets = {
        "openapi_contract",
        "method",
        "path",
        "operation_id",
        "required_scopes",
    }
    openapi_derived = require_mapping(
        algebra.get("openapi_derived_facets"),
        f"{manifest}.surface_operation_algebra.openapi_derived_facets",
        SurfaceCatalogError,
    )
    source_of_truth = require_string(
        openapi_derived.get("source_of_truth"),
        f"{manifest}.surface_operation_algebra.openapi_derived_facets.source_of_truth",
        SurfaceCatalogError,
    )
    if source_of_truth != "openapi-contracts":
        raise SurfaceCatalogError(
            f"{manifest}.surface_operation_algebra.openapi_derived_facets.source_of_truth "
            "must be openapi-contracts"
        )
    derived_facets = set(
        require_string_list(
            openapi_derived.get("compatibility_fields"),
            f"{manifest}.surface_operation_algebra.openapi_derived_facets.compatibility_fields",
            SurfaceCatalogError,
        )
    )
    if derived_facets != expected_openapi_derived_facets:
        raise SurfaceCatalogError(
            f"{manifest}.surface_operation_algebra.openapi_derived_facets.compatibility_fields "
            f"drift; expected={sorted(expected_openapi_derived_facets)}, actual={sorted(derived_facets)}"
        )
    semantic_owned_facets = set(
        require_string_list(
            algebra.get("semantic_owned_facets"),
            f"{manifest}.surface_operation_algebra.semantic_owned_facets",
            SurfaceCatalogError,
        )
    )
    overlap = sorted(derived_facets.intersection(semantic_owned_facets))
    if overlap:
        raise SurfaceCatalogError(
            f"{manifest}.surface_operation_algebra derived and semantic facets overlap: {overlap}"
        )
    if derived_facets.union(semantic_owned_facets) != required_facets.union({"workflow_family"}):
        raise SurfaceCatalogError(
            f"{manifest}.surface_operation_algebra facet partition drift"
        )
    joined_facets = set(
        require_string_list(
            algebra.get("joined_facets"),
            f"{manifest}.surface_operation_algebra.joined_facets",
            SurfaceCatalogError,
        )
    )
    missing_facets = sorted(required_facets.difference(joined_facets))
    if missing_facets:
        raise SurfaceCatalogError(
            f"{manifest}.surface_operation_algebra.joined_facets missing {missing_facets}"
        )
    expected_projection_targets = {
        "operation_kernel": paths["operation_kernel"].as_posix(),
        "operation_kernel_markdown": paths["operation_kernel_markdown"].as_posix(),
        "application_operation_catalog": paths["application_catalog"].as_posix(),
        "pds_resource_family_ledger": paths["pds_families"].as_posix(),
        "surface_family_ledger": paths["surface_families"].as_posix(),
        "surface_operation_index": paths["markdown"].as_posix(),
        "surface_operation_index_evidence": paths["evidence"].as_posix(),
        "route_coverage": paths["route_coverage_markdown"].as_posix(),
        "route_coverage_evidence": paths["route_coverage_evidence"].as_posix(),
        "server_surface_profile_registry": paths["rust_surface_profile_registry"].as_posix(),
    }
    projections = {
        projection_id: projection["target"]
        for projection_id, projection in surface_projection_targets(
            document,
            manifest,
            SurfaceCatalogError,
        ).items()
    }
    if projections != expected_projection_targets:
        raise SurfaceCatalogError(
            f"{manifest}.surface_operation_algebra.projections drift; "
            f"expected={expected_projection_targets}, actual={projections}"
        )
    expected_normal_form = surface_operation_normal_form(document, SurfaceCatalogError)
    actual_normal_form = require_mapping(
        document.get("surface_operation_normal_form"),
        f"{manifest}.surface_operation_normal_form",
        SurfaceCatalogError,
    )
    if actual_normal_form != expected_normal_form:
        raise SurfaceCatalogError(
            f"{manifest}.surface_operation_normal_form drifts from entries"
        )


def validate_projection_outputs(root: Path, manifest: Path, paths: dict[str, Path]) -> list[str]:
    """Validate all generated projection output paths declared by the algebra."""
    checked: list[str] = []
    for key in (
        "operation_kernel",
        "operation_kernel_markdown",
        "application_catalog",
        "pds_families",
        "surface_families",
        "markdown",
        "evidence",
        "route_coverage_markdown",
        "route_coverage_evidence",
        "rust_surface_profile_registry",
    ):
        checked.append(
            require_existing_path(
                root,
                paths[key].as_posix(),
                f"{manifest}.surface_operation_algebra.projections.{key}",
                SurfaceCatalogError,
            ).as_posix()
        )
    return checked


def validate_component_sources(root: Path, manifest: Path, document: dict[str, Any]) -> list[str]:
    """Validate the generated bundle is current with split source ledgers."""
    checked: list[str] = []
    component_sources = require_mapping(
        document.get("component_sources"),
        f"{manifest}.component_sources",
        SurfaceCatalogError,
    )
    operation_ownership_path = relative_path(
        require_string(
            component_sources.get("operation_ownership"),
            f"{manifest}.component_sources.operation_ownership",
            SurfaceCatalogError,
        ),
        f"{manifest}.component_sources.operation_ownership",
        SurfaceCatalogError,
    )
    policy_path = relative_path(
        require_string(
            component_sources.get("surface_projections"),
            f"{manifest}.component_sources.surface_projections",
            SurfaceCatalogError,
        ),
        f"{manifest}.component_sources.surface_projections",
        SurfaceCatalogError,
    )
    pds_family_path = relative_path(
        require_string(
            component_sources.get("pds_resource_families"),
            f"{manifest}.component_sources.pds_resource_families",
            SurfaceCatalogError,
        ),
        f"{manifest}.component_sources.pds_resource_families",
        SurfaceCatalogError,
    )
    surface_family_path = relative_path(
        require_string(
            component_sources.get("surface_families"),
            f"{manifest}.component_sources.surface_families",
            SurfaceCatalogError,
        ),
        f"{manifest}.component_sources.surface_families",
        SurfaceCatalogError,
    )
    for path in (operation_ownership_path, policy_path, pds_family_path, surface_family_path):
        require_existing_path(root, path.as_posix(), f"{manifest}.component_sources", SurfaceCatalogError)
        checked.append(path.as_posix())
    split_policy_sources = require_mapping(
        document.get("split_policy_sources"),
        f"{manifest}.split_policy_sources",
        SurfaceCatalogError,
    )
    for source_id in ("model", "generation", "implementation"):
        raw_path = require_string(
            split_policy_sources.get(source_id),
            f"{manifest}.split_policy_sources.{source_id}",
            SurfaceCatalogError,
        )
        path = relative_path(
            raw_path,
            f"{manifest}.split_policy_sources.{source_id}",
            SurfaceCatalogError,
        )
        require_existing_path(root, path.as_posix(), f"{manifest}.split_policy_sources", SurfaceCatalogError)
        checked.append(path.as_posix())
    operation_ownership = load_operation_ownership_source(root / operation_ownership_path, SurfaceCatalogError)
    policy = load_surface_projection_policy(root / policy_path, SurfaceCatalogError)
    pds_families = load_pds_resource_family_source(root / pds_family_path, SurfaceCatalogError)
    surface_families = load_surface_family_source(root / surface_family_path, SurfaceCatalogError)
    projected = render_yaml(
        project_surface_catalog_bundle(
            operation_ownership,
            policy,
            pds_families,
            surface_families,
            operation_ownership_path=operation_ownership_path,
            policy_path=policy_path,
            pds_family_path=pds_family_path,
            surface_family_path=surface_family_path,
            root=root,
            error_cls=SurfaceCatalogError,
        )
    )
    actual = (root / manifest).read_text(encoding="utf-8")
    if actual != projected:
        raise SurfaceCatalogError(
            f"{manifest} is not current; run scripts/ci/generate_surface_catalog_bundle.py",
        )
    return checked


def validate_join_policy(
    rows: list[dict[str, Any]],
    manifest: Path,
    document: dict[str, Any],
) -> dict[str, int]:
    """Validate canonical surface entries against catalog policy."""
    join_policy = require_mapping(
        document.get("join_policy"),
        f"{manifest}.join_policy",
        SurfaceCatalogError,
    )
    required_http_fields = require_string_list(
        join_policy.get("required_http_fields"),
        f"{manifest}.join_policy.required_http_fields",
        SurfaceCatalogError,
    )
    required_application_fields = require_string_list(
        join_policy.get("required_application_fields"),
        f"{manifest}.join_policy.required_application_fields",
        SurfaceCatalogError,
    )
    required_pds_fields = require_string_list(
        join_policy.get("required_pds_fields"),
        f"{manifest}.join_policy.required_pds_fields",
        SurfaceCatalogError,
    )
    pds_contracts = set(
        require_string_list(
            join_policy.get("require_pds_family_for_contracts"),
            f"{manifest}.join_policy.require_pds_family_for_contracts",
            SurfaceCatalogError,
        )
    )
    application_contexts = set(
        require_string_list(
            join_policy.get("require_application_operations_for_route_contexts"),
            f"{manifest}.join_policy.require_application_operations_for_route_contexts",
            SurfaceCatalogError,
        )
    )

    seen_routes: set[tuple[str, str, str]] = set()
    seen_operation_ids: set[tuple[str, str]] = set()
    pds_operation_count = 0
    application_operation_count = 0
    for index, row in enumerate(rows):
        label = f"surface.entries[{index}]"
        for field in required_http_fields:
            if row.get(field) in (None, ""):
                raise SurfaceCatalogError(f"{label}.{field} is required")
        route_key = (row["openapi_contract"], row["method"], row["path"])
        operation_key = (row["openapi_contract"], row["operation_id"])
        if route_key in seen_routes:
            raise SurfaceCatalogError(f"duplicate surface route entry: {route_key}")
        if operation_key in seen_operation_ids:
            raise SurfaceCatalogError(f"duplicate surface operation entry: {operation_key}")
        seen_routes.add(route_key)
        seen_operation_ids.add(operation_key)
        if row["openapi_contract"] in pds_contracts:
            pds_operation_count += 1
            for field in required_pds_fields:
                if row.get(field) in (None, ""):
                    raise SurfaceCatalogError(f"{label}.{field} is required for PDS contract")
        if row.get("pds_route_context") in application_contexts:
            application_operation_count += 1
            for field in required_application_fields:
                if row.get(field) in (None, ""):
                    raise SurfaceCatalogError(
                        f"{label}.{field} is required for route context {row['pds_route_context']}",
                    )
    return {
        "operation_count": len(rows),
        "pds_operation_count": pds_operation_count,
        "application_operation_count": application_operation_count,
    }


def validate_projection_policy(manifest: Path, document: dict[str, Any], paths: dict[str, Path]) -> None:
    """Validate projection policy fields point at the active targets."""
    projection_policy = require_mapping(
        document.get("projection_policy"),
        f"{manifest}.projection_policy",
        SurfaceCatalogError,
    )
    if projection_policy.get("surface_operation_index_is_generated_from_this_catalog") is not True:
        raise SurfaceCatalogError(
            f"{manifest}.projection_policy.surface_operation_index_is_generated_from_this_catalog must be true",
        )
    if projection_policy.get("generated_projections_are_declared_in_surface_operation_algebra") is not True:
        raise SurfaceCatalogError(
            f"{manifest}.projection_policy.generated_projections_are_declared_in_surface_operation_algebra must be true",
        )


def validate_projection_files(root: Path, document: dict[str, Any], paths: dict[str, Path]) -> None:
    """Validate projection ledgers are current with the surface catalog."""
    expected_application = render_yaml(
        project_application_operation_catalog(document, SurfaceCatalogError),
    )
    expected_pds = render_yaml(project_pds_resource_family_ledger(document, SurfaceCatalogError))
    expected_surface_families = render_yaml(project_surface_family_ledger(document, SurfaceCatalogError))
    actual_application = (root / paths["application_catalog"]).read_text(encoding="utf-8")
    actual_pds = (root / paths["pds_families"]).read_text(encoding="utf-8")
    actual_surface_families = (root / paths["surface_families"]).read_text(encoding="utf-8")
    if actual_application != expected_application:
        raise SurfaceCatalogError(
            f"{paths['application_catalog']} is not current; "
            "run scripts/ci/generate_surface_catalog_projections.py",
        )
    if actual_pds != expected_pds:
        raise SurfaceCatalogError(
            f"{paths['pds_families']} is not current; "
            "run scripts/ci/generate_surface_catalog_projections.py",
        )
    if actual_surface_families != expected_surface_families:
        raise SurfaceCatalogError(
            f"{paths['surface_families']} is not current; "
            "run scripts/ci/generate_surface_catalog_projections.py",
        )


def validate_surface_index_evidence(
    root: Path,
    manifest: Path,
    evidence_path: Path,
    http_catalog: Path,
    rows: list[dict[str, Any]],
) -> None:
    """Validate the generated evidence index points back to the surface catalog."""
    evidence = load_json(root / evidence_path, SurfaceCatalogError)
    if evidence.get("kind") != "kiva-surface-operation-index-evidence":
        raise SurfaceCatalogError(
            f"{evidence_path}.kind must be kiva-surface-operation-index-evidence",
        )
    if evidence.get("surface_catalog") != manifest.as_posix():
        raise SurfaceCatalogError(
            f"{evidence_path}.surface_catalog must be {manifest.as_posix()}",
        )
    generated_from = evidence.get("generated_from")
    expected_generated_from = [manifest.as_posix(), http_catalog.as_posix()]
    if generated_from != expected_generated_from:
        raise SurfaceCatalogError(
            f"{evidence_path}.generated_from must be {expected_generated_from}",
        )
    if evidence.get("http_operation_count") != len(rows):
        raise SurfaceCatalogError(f"{evidence_path}.http_operation_count must be {len(rows)}")
    if evidence.get("operations") != rows:
        raise SurfaceCatalogError(f"{evidence_path}.operations must match rendered surface rows")


def validate_surface_catalog(root: Path, manifest: Path) -> dict[str, Any]:
    """Validate a surface catalog and return a summary."""
    catalog_path = root / manifest
    document = load_surface_catalog(catalog_path, SurfaceCatalogError)
    require_string(document.get("description"), f"{manifest}.description", SurfaceCatalogError)
    validator = require_string(document.get("validator"), f"{manifest}.validator", SurfaceCatalogError)
    if validator != "scripts/ci/validate_surface_catalog.py":
        raise SurfaceCatalogError(f"{manifest}.validator must point at this validator")

    paths = surface_catalog_paths(document, manifest, SurfaceCatalogError, root=root)
    checked_paths = validate_source_ledgers(root, manifest, document)
    checked_paths.extend(validate_component_sources(root, manifest, document))
    checked_paths.extend(validate_projection_outputs(root, manifest, paths))
    validate_surface_operation_algebra(manifest, document, paths)

    entries = require_surface_entries(document, SurfaceCatalogError)
    http_operations = load_http_operation_catalog(root / paths["http_catalog"], SurfaceCatalogError)
    validate_entries_against_http_catalog(entries, http_operations, SurfaceCatalogError)
    rows = surface_index_rows(document, SurfaceCatalogError)
    join_summary = validate_join_policy(rows, manifest, document)
    validate_projection_policy(manifest, document, paths)
    validate_projection_files(root, document, paths)
    validate_surface_index_evidence(root, manifest, paths["evidence"], paths["http_catalog"], rows)

    return {
        "manifest": manifest.as_posix(),
        "kind": document["kind"],
        "checked_paths": sorted(set(checked_paths)),
        **join_summary,
    }


def parse_args() -> argparse.Namespace:
    """Parse CLI arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--json-report", type=Path)
    return parser.parse_args()


def main() -> int:
    """CLI entrypoint."""
    args = parse_args()
    root = args.root.resolve()
    try:
        manifest = relative_path(str(args.manifest), "--manifest", SurfaceCatalogError)
        report = validate_surface_catalog(root, manifest)
    except SurfaceCatalogError as error:
        print(f"surface-catalog validation failed: {error}", file=sys.stderr)
        return 1

    if args.json_report is not None:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    else:
        print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
