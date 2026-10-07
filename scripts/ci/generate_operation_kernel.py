#!/usr/bin/env python3
"""Generate the first-class OperationKernel IR from canonical surface sources."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.manifest import load_json, load_yaml, require_list, require_mapping, require_string
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts
from surface_catalog import (
    load_operation_ownership_source,
    load_http_operation_catalog,
    load_pds_resource_family_source,
    load_surface_family_source,
    load_surface_projection_policy,
    project_pds_resource_family_ledger,
    project_surface_catalog_bundle,
    project_surface_family_ledger,
    surface_catalog_paths,
    surface_component_paths,
    surface_operation_algebra,
    validate_entries_against_http_catalog,
)
from surface_catalog_model import surface_entries


DEFAULT_SURFACE_POLICY = Path("spec/architecture/surface-projections.yaml")
DEFAULT_PDS_CORE_PLANES = Path("spec/pds/core-planes.source.yaml")
DEFAULT_OUTPUT = Path("generated/architecture/operation-kernel.yaml")
DEFAULT_MARKDOWN = Path("docs/architecture/generated/operation-kernel.md")
DEFAULT_SCHEMA = Path("spec/architecture/schemas/operation-kernel.schema.json")


class OperationKernelError(RuntimeError):
    """Raised when the OperationKernel projection cannot be generated."""


def operation_key(row: dict[str, Any]) -> tuple[str, str]:
    """Return the contract-local OpenAPI operation key for a row."""
    return (
        require_string(row.get("openapi_contract"), "operation.openapi_contract", OperationKernelError),
        require_string(row.get("operation_id"), "operation.operation_id", OperationKernelError),
    )


def surface_family_assignments(
    document: dict[str, Any],
    label: str,
) -> dict[tuple[str, str], dict[str, Any]]:
    """Return exactly-one surface-family assignment keyed by OpenAPI operation."""
    if document.get("kind") != "kiva-surface-family-ledger":
        raise OperationKernelError(f"{label}.kind must be kiva-surface-family-ledger")
    assignments: dict[tuple[str, str], dict[str, Any]] = {}
    for family_index, raw_family in enumerate(
        require_list(document.get("families"), f"{label}.families", OperationKernelError)
    ):
        family = require_mapping(raw_family, f"{label}.families[{family_index}]", OperationKernelError)
        family_id = require_string(family.get("id"), f"{label}.families[{family_index}].id", OperationKernelError)
        surface = require_string(family.get("surface"), f"{label}.families[{family_index}].surface", OperationKernelError)
        for operation_index, raw_operation in enumerate(
            require_list(
                family.get("operations"),
                f"{label}.families[{family_index}].operations",
                OperationKernelError,
            )
        ):
            operation = require_mapping(
                raw_operation,
                f"{label}.families[{family_index}].operations[{operation_index}]",
                OperationKernelError,
            )
            key = operation_key(operation)
            if key in assignments:
                raise OperationKernelError(f"duplicate surface-family assignment for {key}")
            assignments[key] = {
                "surface_family": family_id,
                "surface": surface,
                "description": require_string(
                    family.get("description"),
                    f"{label}.families[{family_index}].description",
                    OperationKernelError,
                ),
            }
    return assignments


def pds_family_assignments(
    document: dict[str, Any],
    label: str,
    http_by_route: dict[tuple[str, str, str], dict[str, Any]],
) -> dict[tuple[str, str], dict[str, Any]]:
    """Return optional PDS resource-family assignments keyed by OpenAPI operation."""
    if document.get("kind") != "kiva-pds-resource-family-ledger":
        raise OperationKernelError(f"{label}.kind must be kiva-pds-resource-family-ledger")
    source_contract = require_string(
        document.get("source_contract"),
        f"{label}.source_contract",
        OperationKernelError,
    )
    assignments: dict[tuple[str, str], dict[str, Any]] = {}
    for family_index, raw_family in enumerate(
        require_list(document.get("resource_families"), f"{label}.resource_families", OperationKernelError)
    ):
        family = require_mapping(raw_family, f"{label}.resource_families[{family_index}]", OperationKernelError)
        family_id = require_string(
            family.get("id"),
            f"{label}.resource_families[{family_index}].id",
            OperationKernelError,
        )
        for operation_index, raw_operation in enumerate(
            require_list(
                family.get("operations"),
                f"{label}.resource_families[{family_index}].operations",
                OperationKernelError,
            )
        ):
            operation = require_mapping(
                raw_operation,
                f"{label}.resource_families[{family_index}].operations[{operation_index}]",
                OperationKernelError,
            )
            route_key = (
                source_contract,
                require_string(
                    operation.get("method"),
                    f"{label}.resource_families[{family_index}].operations[{operation_index}].method",
                    OperationKernelError,
                ).lower(),
                require_string(
                    operation.get("path"),
                    f"{label}.resource_families[{family_index}].operations[{operation_index}].path",
                    OperationKernelError,
                ),
            )
            http_operation = http_by_route.get(route_key)
            if http_operation is None:
                raise OperationKernelError(f"missing HTTP operation for PDS route {route_key}")
            key = operation_key(http_operation)
            if key in assignments:
                raise OperationKernelError(f"duplicate PDS resource-family assignment for {key}")
            assignments[key] = {
                "pds_resource_family": family_id,
                "route_context": require_string(
                    operation.get("route_context"),
                    f"{label}.resource_families[{family_index}].operations[{operation_index}].route_context",
                    OperationKernelError,
                ),
            }
    return assignments


def pds_plane_assignments(path: Path) -> dict[str, list[str]]:
    """Return PDS core-plane ids keyed by PDS resource-family id."""
    document = load_yaml(path, OperationKernelError)
    if document.get("kind") != "kiva-pds-core-planes-source":
        raise OperationKernelError(f"{path}.kind must be kiva-pds-core-planes-source")
    plane_model = require_mapping(document.get("plane_model"), f"{path}.plane_model", OperationKernelError)
    plane_order = [
        require_string(value, f"{path}.plane_model.planes[]", OperationKernelError)
        for value in require_list(plane_model.get("planes"), f"{path}.plane_model.planes", OperationKernelError)
    ]
    plane_rank = {plane_id: index for index, plane_id in enumerate(plane_order)}
    assignments: dict[str, list[str]] = {}
    for plane_index, raw_plane in enumerate(
        require_list(document.get("planes"), f"{path}.planes", OperationKernelError)
    ):
        plane = require_mapping(raw_plane, f"{path}.planes[{plane_index}]", OperationKernelError)
        plane_id = require_string(plane.get("id"), f"{path}.planes[{plane_index}].id", OperationKernelError)
        if plane_id not in plane_rank:
            raise OperationKernelError(f"{path}.planes[{plane_index}].id is not declared in plane_model: {plane_id}")
        for family_id in [
            require_string(value, f"{path}.planes[{plane_index}].resource_family_refs[]", OperationKernelError)
            for value in require_list(
                plane.get("resource_family_refs"),
                f"{path}.planes[{plane_index}].resource_family_refs",
                OperationKernelError,
            )
        ]:
            assignments.setdefault(family_id, []).append(plane_id)
    return {
        family_id: sorted(set(planes), key=lambda plane_id: plane_rank[plane_id])
        for family_id, planes in assignments.items()
    }


def contract_label(openapi_contract: str) -> str:
    """Return a compact label for a contract path."""
    return Path(openapi_contract).stem.removeprefix("kiva-")


def kernel_id(openapi_contract: str, operation_id: str) -> str:
    """Return a stable OperationKernel row id."""
    return f"{contract_label(openapi_contract)}.{operation_id}"


def optional_mapping(values: dict[str, Any]) -> dict[str, Any] | None:
    """Return a mapping only when at least one field is present."""
    compact = {key: value for key, value in values.items() if value is not None}
    return compact or None


def build_surface_catalog_from_sources(
    policy_path: Path,
) -> tuple[dict[str, Any], Path, dict[str, Path]]:
    """Build the in-memory SurfaceOperation normal form from canonical sources."""
    policy = load_surface_projection_policy(policy_path, OperationKernelError)
    component_paths = surface_component_paths(policy, policy_path, OperationKernelError)
    operation_ownership_path = component_paths["operation_ownership"]
    pds_family_path = component_paths["pds_resource_families"]
    surface_family_path = component_paths["surface_families"]
    catalog = project_surface_catalog_bundle(
        load_operation_ownership_source(operation_ownership_path, OperationKernelError),
        policy,
        load_pds_resource_family_source(pds_family_path, OperationKernelError),
        load_surface_family_source(surface_family_path, OperationKernelError),
        operation_ownership_path=operation_ownership_path,
        policy_path=policy_path,
        pds_family_path=pds_family_path,
        surface_family_path=surface_family_path,
        error_cls=OperationKernelError,
    )
    return catalog, component_paths["generated_bundle"], component_paths


def operation_kernel_document(
    policy_path: Path,
    pds_core_planes_path: Path,
    output_path: Path,
    markdown_path: Path,
) -> dict[str, Any]:
    """Project OperationKernel rows from OpenAPI and canonical surface source ledgers."""
    catalog, surface_catalog_path, component_paths = build_surface_catalog_from_sources(policy_path)
    paths = surface_catalog_paths(catalog, surface_catalog_path, OperationKernelError)
    http_operations = load_http_operation_catalog(paths["http_catalog"], OperationKernelError)
    entries = surface_entries(catalog, OperationKernelError)
    validate_entries_against_http_catalog(
        [entry.to_mapping() for entry in entries],
        http_operations,
        OperationKernelError,
    )
    http_by_key = {operation_key(operation): operation for operation in http_operations}
    http_by_route = {
        (
            operation["openapi_contract"],
            operation["method"],
            operation["path"],
        ): operation
        for operation in http_operations
    }
    pds_family_ledger = project_pds_resource_family_ledger(catalog, OperationKernelError)
    surface_family_ledger = project_surface_family_ledger(catalog, OperationKernelError)
    pds_assignments = pds_family_assignments(
        pds_family_ledger,
        "projected_pds_resource_family_ledger",
        http_by_route,
    )
    pds_planes_by_family = pds_plane_assignments(pds_core_planes_path)
    surface_assignments = surface_family_assignments(
        surface_family_ledger,
        "projected_surface_family_ledger",
    )

    rows: list[dict[str, Any]] = []
    for entry in entries:
        key = entry.operation_key
        http_operation = http_by_key.get(key)
        if http_operation is None:
            raise OperationKernelError(f"missing HTTP operation for {key}")
        surface_assignment = surface_assignments.get(key)
        if surface_assignment is None:
            raise OperationKernelError(f"missing surface-family assignment for {key}")
        pds_assignment = pds_assignments.get(key)
        if entry.pds_resource_family is not None and pds_assignment is None:
            raise OperationKernelError(f"missing PDS resource-family assignment for {key}")
        if pds_assignment is not None and pds_assignment["pds_resource_family"] != entry.pds_resource_family:
            raise OperationKernelError(f"PDS resource-family drift for {key}")
        pds_planes: list[str] = []
        if pds_assignment is not None:
            pds_planes = pds_planes_by_family.get(pds_assignment["pds_resource_family"], [])
            if not pds_planes:
                raise OperationKernelError(
                    f"PDS resource family {pds_assignment['pds_resource_family']} has no PDS core-plane assignment"
                )
        rows.append(
            {
                "id": kernel_id(entry.openapi_contract, entry.operation_id),
                "operation_key": {
                    "openapi_contract": entry.openapi_contract,
                    "operation_id": entry.operation_id,
                },
                "route_key": {
                    "openapi_contract": entry.openapi_contract,
                    "method": entry.method,
                    "path": entry.path,
                },
                "http_contract": {
                    "method": entry.method,
                    "path": entry.path,
                    "operation_id": entry.operation_id,
                    "required_scopes": list(entry.required_scopes),
                    "requirements": http_operation.get("requirements", []),
                    "tags": http_operation.get("tags", []),
                    "summary": http_operation.get("summary"),
                },
                "application_operation": optional_mapping(
                    {
                        "id": entry.application_operation_id,
                        "surface": entry.application_surface,
                        "workflow_label": entry.workflow_label,
                        "rust_workflow_kind": entry.rust_workflow_kind,
                        "workflow_family": entry.workflow_family,
                    }
                ),
                "resource_family": {
                    **surface_assignment,
                    "pds_resource_family": None if pds_assignment is None else pds_assignment["pds_resource_family"],
                    "pds_route_context": None if pds_assignment is None else pds_assignment["route_context"],
                    "pds_planes": pds_planes,
                },
                "evidence_refs": {
                    "http_operation_catalog": paths["http_catalog"].as_posix(),
                    "surface_catalog": surface_catalog_path.as_posix(),
                    "surface_family_ledger": paths["surface_families"].as_posix(),
                    "pds_resource_family_ledger": paths["pds_families"].as_posix(),
                },
            }
        )

    application_count = sum(1 for row in rows if row["application_operation"] is not None)
    pds_count = sum(1 for row in rows if row["resource_family"]["pds_resource_family"] is not None)
    return {
        "schema_version": 1,
        "kind": "kiva-operation-kernel-ir",
        "status": "active",
        "description": (
            "First-class OperationKernel IR joining HTTP contract identity, "
            "SurfaceOperation semantics, workflow ownership, resource-family "
            "assignment, and requirement evidence for every public operation."
        ),
        "generated_from": [
            policy_path.as_posix(),
            component_paths["operation_ownership"].as_posix(),
            component_paths["pds_resource_families"].as_posix(),
            component_paths["surface_families"].as_posix(),
            pds_core_planes_path.as_posix(),
            paths["http_catalog"].as_posix(),
        ],
        "generated_artifacts": {
            "operation_kernel": output_path.as_posix(),
            "markdown": markdown_path.as_posix(),
        },
        "projection_basis": {
            "root_type": "OperationKernel",
            "source_root_type": "SurfaceOperation",
            "source_normal_form": "surface_operation_normal_form",
            "source_algebra": surface_operation_algebra(
                catalog,
                policy_path,
                OperationKernelError,
            ).get("root_type"),
            "join_type": (
                "SurfaceOperation + HttpOperationEvidence + "
                "SurfaceFamilyAssignment + Optional<PdsResourceFamilyAssignment>"
            ),
            "rule": (
                "Every public HTTP operation has exactly one OperationKernel row; "
                "workflow, PDS family, PDS core planes, route, scope, and requirement evidence "
                "are projections, not parallel sources of truth."
            ),
        },
        "operation_count": len(rows),
        "application_operation_count": application_count,
        "pds_resource_operation_count": pds_count,
        "operations": rows,
    }


def validate_schema(document: dict[str, Any], schema_path: Path) -> None:
    """Validate the generated OperationKernel document against its JSON Schema."""
    schema = load_json(schema_path, OperationKernelError)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)
    errors = sorted(validator.iter_errors(document), key=lambda error: list(error.path))
    if errors:
        details = "\n".join(
            f"- {'/'.join(str(part) for part in error.path) or '<root>'}: {error.message}"
            for error in errors
        )
        raise OperationKernelError(f"OperationKernel violates {schema_path}:\n{details}")


def markdown_cell(value: object) -> str:
    """Render a Markdown table cell."""
    return str(value).replace("|", "\\|").replace("\n", " ")


def optional_cell(value: str | None) -> str:
    """Render an optional Markdown value."""
    return "" if value is None else f"`{markdown_cell(value)}`"


def render_markdown(document: dict[str, Any], policy_path: Path) -> str:
    """Render a concise OperationKernel review table."""
    rows = [
        "| Contract | Method | Path | Operation ID | Workflow | Surface Family | PDS Family | PDS Planes | Requirements |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for row in require_list(document.get("operations"), "operation_kernel.operations", OperationKernelError):
        operation = require_mapping(row, "operation_kernel.operations[]", OperationKernelError)
        http_contract = require_mapping(operation.get("http_contract"), "operation.http_contract", OperationKernelError)
        resource_family = require_mapping(
            operation.get("resource_family"),
            "operation.resource_family",
            OperationKernelError,
        )
        app = operation.get("application_operation")
        workflow = None
        if app is not None:
            workflow = require_string(
                require_mapping(app, "operation.application_operation", OperationKernelError).get("workflow_label"),
                "operation.application_operation.workflow_label",
                OperationKernelError,
            )
        requirements = http_contract.get("requirements", [])
        requirement_cell = ", ".join(f"`{markdown_cell(value)}`" for value in requirements)
        pds_planes = resource_family.get("pds_planes", [])
        pds_plane_cell = ", ".join(f"`{markdown_cell(value)}`" for value in pds_planes)
        rows.append(
            "| "
            + " | ".join(
                [
                    f"`{markdown_cell(operation['operation_key']['openapi_contract'])}`",
                    f"`{markdown_cell(str(http_contract['method']).upper())}`",
                    f"`{markdown_cell(http_contract['path'])}`",
                    f"`{markdown_cell(http_contract['operation_id'])}`",
                    optional_cell(workflow),
                    f"`{markdown_cell(resource_family['surface_family'])}`",
                    optional_cell(resource_family.get("pds_resource_family")),
                    pds_plane_cell,
                    requirement_cell,
                ]
            )
            + " |"
        )
    return f"""# Generated OperationKernel

This file is generated from `{policy_path.as_posix()}` and its split source
ledgers by
`scripts/ci/generate_operation_kernel.py`.

`OperationKernel` is the first-class operation IR for review. It joins HTTP
contract identity, `SurfaceOperation` semantics, workflow ownership,
resource-family assignment, and requirement evidence. Downstream catalogs remain
specialized projections of this kernel.

{chr(10).join(rows)}
"""


def parse_args() -> argparse.Namespace:
    """Parse CLI arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--surface-policy", type=Path, default=DEFAULT_SURFACE_POLICY)
    parser.add_argument("--pds-core-planes", type=Path, default=DEFAULT_PDS_CORE_PLANES)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def main() -> int:
    """Generate or validate the OperationKernel IR."""
    args = parse_args()
    document = operation_kernel_document(args.surface_policy, args.pds_core_planes, args.out, args.markdown_out)
    validate_schema(document, args.schema)
    rendered_yaml = dump_yaml(document)
    rendered_markdown = render_markdown(document, args.surface_policy)
    command_hint = "scripts/ci/generate_operation_kernel.py"
    try:
        write_or_check_generated_artifacts(
            [
                GeneratedArtifact(args.out, rendered_yaml),
                GeneratedArtifact(args.markdown_out, rendered_markdown),
            ],
            check=args.check,
            command_hint=command_hint,
            error_cls=OperationKernelError,
        )
    except OperationKernelError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated OperationKernel IR: {args.out}, {args.markdown_out}")
        return 0
    print(f"generated OperationKernel IR: {args.out}, {args.markdown_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
