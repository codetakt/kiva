#!/usr/bin/env python3
"""Validate OperationKernel is the complete operation review IR."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from kiva_ci.manifest import load_yaml, require_list, require_mapping, require_string


DEFAULT_KERNEL = Path("generated/architecture/operation-kernel.yaml")
DEFAULT_HTTP = Path("generated/http-operation-catalog.evidence.json")
DEFAULT_ROUTE_COVERAGE = Path("generated/route-coverage.evidence.json")
DEFAULT_PDS_FAMILIES = Path("generated/architecture/pds-resource-families.yaml")
DEFAULT_PDS_CORE_PLANES = Path("spec/pds/core-planes.source.yaml")


class OperationKernelAuthorityError(RuntimeError):
    """Raised when OperationKernel is not an authoritative complete join."""


def load_json_mapping(path: Path) -> dict[str, Any]:
    """Load a JSON mapping."""
    with path.open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise OperationKernelAuthorityError(f"{path} root must be a mapping")
    return value


def operation_key(row: dict[str, Any], label: str) -> tuple[str, str]:
    """Return an OpenAPI contract-local operation key."""
    return (
        require_string(row.get("openapi_contract"), f"{label}.openapi_contract", OperationKernelAuthorityError),
        require_string(row.get("operation_id"), f"{label}.operation_id", OperationKernelAuthorityError),
    )


def kernel_operation_key(row: dict[str, Any], label: str) -> tuple[str, str]:
    """Return an OperationKernel operation key."""
    key = require_mapping(row.get("operation_key"), f"{label}.operation_key", OperationKernelAuthorityError)
    return operation_key(key, f"{label}.operation_key")


def kernel_rows(kernel: dict[str, Any], path: Path) -> dict[tuple[str, str], dict[str, Any]]:
    """Return OperationKernel rows keyed by contract/operation_id."""
    if kernel.get("kind") != "kiva-operation-kernel-ir":
        raise OperationKernelAuthorityError(f"{path}.kind must be kiva-operation-kernel-ir")
    rows: dict[tuple[str, str], dict[str, Any]] = {}
    for index, raw_row in enumerate(require_list(kernel.get("operations"), f"{path}.operations", OperationKernelAuthorityError)):
        row = require_mapping(raw_row, f"{path}.operations[{index}]", OperationKernelAuthorityError)
        key = kernel_operation_key(row, f"{path}.operations[{index}]")
        if key in rows:
            raise OperationKernelAuthorityError(f"duplicate OperationKernel operation key: {key}")
        rows[key] = row
    declared_count = kernel.get("operation_count")
    if declared_count != len(rows):
        raise OperationKernelAuthorityError(f"{path}.operation_count must be {len(rows)}")
    return rows


def kernel_rows_by_route(kernel_by_key: dict[tuple[str, str], dict[str, Any]]) -> dict[tuple[str, str, str], dict[str, Any]]:
    """Return OperationKernel rows keyed by contract/method/path."""
    rows: dict[tuple[str, str, str], dict[str, Any]] = {}
    for operation_key_value, row in kernel_by_key.items():
        route = require_mapping(row.get("route_key"), f"kernel[{operation_key_value}].route_key", OperationKernelAuthorityError)
        key = (
            require_string(route.get("openapi_contract"), f"kernel[{operation_key_value}].route_key.openapi_contract", OperationKernelAuthorityError),
            require_string(route.get("method"), f"kernel[{operation_key_value}].route_key.method", OperationKernelAuthorityError),
            require_string(route.get("path"), f"kernel[{operation_key_value}].route_key.path", OperationKernelAuthorityError),
        )
        if key in rows:
            raise OperationKernelAuthorityError(f"duplicate OperationKernel route key: {key}")
        rows[key] = row
    return rows


def require_key_set(label: str, actual: set[tuple[str, str]], expected: set[tuple[str, str]]) -> None:
    """Validate two operation-key sets are equal."""
    missing = sorted(expected.difference(actual))
    extra = sorted(actual.difference(expected))
    if missing or extra:
        raise OperationKernelAuthorityError(f"{label} operation keys drift: missing={missing}, extra={extra}")


def validate_http_contract(kernel_by_key: dict[tuple[str, str], dict[str, Any]], http_catalog: dict[str, Any]) -> None:
    """Validate every HTTP operation is represented by exactly one kernel row."""
    http_rows = [
        require_mapping(row, "http.operations[]", OperationKernelAuthorityError)
        for row in require_list(http_catalog.get("operations"), "http.operations", OperationKernelAuthorityError)
    ]
    http_by_key = {operation_key(row, "http.operations[]"): row for row in http_rows}
    require_key_set("OperationKernel vs HTTP catalog", set(kernel_by_key), set(http_by_key))
    for key, http_row in http_by_key.items():
        kernel_http = require_mapping(
            kernel_by_key[key].get("http_contract"),
            f"kernel[{key}].http_contract",
            OperationKernelAuthorityError,
        )
        for field in ("method", "path", "operation_id"):
            if kernel_http.get(field) != http_row.get(field):
                raise OperationKernelAuthorityError(f"kernel[{key}].http_contract.{field} drifted from HTTP catalog")
        if kernel_http.get("required_scopes") != http_row.get("required_scopes", []):
            raise OperationKernelAuthorityError(f"kernel[{key}].http_contract.required_scopes drifted from HTTP catalog")


def validate_route_coverage(kernel_by_key: dict[tuple[str, str], dict[str, Any]], route_coverage: dict[str, Any]) -> None:
    """Validate every route-coverage row resolves through OperationKernel."""
    route_rows = [
        require_mapping(row, "route_coverage.routes[]", OperationKernelAuthorityError)
        for row in require_list(route_coverage.get("routes"), "route_coverage.routes", OperationKernelAuthorityError)
    ]
    route_keys = {operation_key(row, "route_coverage.routes[]") for row in route_rows}
    require_key_set("RouteCoverage vs OperationKernel", route_keys, set(kernel_by_key))


def validate_pds_assignments(kernel_by_key: dict[tuple[str, str], dict[str, Any]], pds_families: dict[str, Any]) -> int:
    """Validate PDS resource-family assignments are visible through OperationKernel."""
    kernel_by_route = kernel_rows_by_route(kernel_by_key)
    operations = []
    for family_index, raw_family in enumerate(
        require_list(pds_families.get("resource_families"), "pds.resource_families", OperationKernelAuthorityError)
    ):
        family = require_mapping(raw_family, f"pds.resource_families[{family_index}]", OperationKernelAuthorityError)
        family_id = require_string(family.get("id"), f"pds.resource_families[{family_index}].id", OperationKernelAuthorityError)
        for operation_index, raw_operation in enumerate(
            require_list(family.get("operations"), f"pds.resource_families[{family_index}].operations", OperationKernelAuthorityError)
        ):
            operation = require_mapping(
                raw_operation,
                f"pds.resource_families[{family_index}].operations[{operation_index}]",
                OperationKernelAuthorityError,
            )
            route_key = (
                require_string(pds_families.get("source_contract"), "pds.source_contract", OperationKernelAuthorityError),
                require_string(operation.get("method"), "pds.operation.method", OperationKernelAuthorityError),
                require_string(operation.get("path"), "pds.operation.path", OperationKernelAuthorityError),
            )
            operations.append((family_id, route_key))
    for family_id, key in operations:
        kernel_row = kernel_by_route.get(key)
        if kernel_row is None:
            raise OperationKernelAuthorityError(f"PDS operation missing from OperationKernel: {key}")
        resource_family = require_mapping(kernel_row.get("resource_family"), f"kernel[{key}].resource_family", OperationKernelAuthorityError)
        if resource_family.get("pds_resource_family") != family_id:
            raise OperationKernelAuthorityError(f"kernel[{key}].resource_family.pds_resource_family must be {family_id}")
    return len(operations)


def pds_planes_by_family(source: dict[str, Any], source_path: Path) -> dict[str, set[str]]:
    """Return PDS core-plane assignments keyed by resource family."""
    if source.get("kind") != "kiva-pds-core-planes-source":
        raise OperationKernelAuthorityError(f"{source_path}.kind must be kiva-pds-core-planes-source")
    assignments: dict[str, set[str]] = {}
    for plane_index, raw_plane in enumerate(
        require_list(source.get("planes"), f"{source_path}.planes", OperationKernelAuthorityError)
    ):
        plane = require_mapping(raw_plane, f"{source_path}.planes[{plane_index}]", OperationKernelAuthorityError)
        plane_id = require_string(plane.get("id"), f"{source_path}.planes[{plane_index}].id", OperationKernelAuthorityError)
        for family_id in [
            require_string(value, f"{source_path}.planes[{plane_index}].resource_family_refs[]", OperationKernelAuthorityError)
            for value in require_list(
                plane.get("resource_family_refs"),
                f"{source_path}.planes[{plane_index}].resource_family_refs",
                OperationKernelAuthorityError,
            )
        ]:
            assignments.setdefault(family_id, set()).add(plane_id)
    return assignments


def validate_pds_planes(
    kernel_by_key: dict[tuple[str, str], dict[str, Any]],
    pds_core_planes: dict[str, Any],
    pds_core_planes_path: Path,
) -> int:
    """Validate OperationKernel projects PDS core-plane assignments."""
    expected_by_family = pds_planes_by_family(pds_core_planes, pds_core_planes_path)
    count = 0
    for key, row in kernel_by_key.items():
        resource_family = require_mapping(row.get("resource_family"), f"kernel[{key}].resource_family", OperationKernelAuthorityError)
        family_id = resource_family.get("pds_resource_family")
        pds_planes = resource_family.get("pds_planes")
        if family_id is None:
            if pds_planes != []:
                raise OperationKernelAuthorityError(f"kernel[{key}].resource_family.pds_planes must be empty for non-PDS operation")
            continue
        expected = expected_by_family.get(str(family_id))
        if not expected:
            raise OperationKernelAuthorityError(f"PDS resource family has no PDS core-plane assignment: {family_id}")
        actual = set(
            require_string(value, f"kernel[{key}].resource_family.pds_planes[]", OperationKernelAuthorityError)
            for value in require_list(pds_planes, f"kernel[{key}].resource_family.pds_planes", OperationKernelAuthorityError)
        )
        if actual != expected:
            raise OperationKernelAuthorityError(f"kernel[{key}].resource_family.pds_planes must be {sorted(expected)}")
        count += 1
    return count


def validate(root: Path, kernel_path: Path, http_path: Path, route_path: Path, pds_path: Path, pds_core_planes_path: Path) -> dict[str, Any]:
    """Validate the OperationKernel authority contract."""
    kernel = load_yaml(root / kernel_path, OperationKernelAuthorityError)
    http_catalog = load_json_mapping(root / http_path)
    route_coverage = load_json_mapping(root / route_path)
    pds_families = load_yaml(root / pds_path, OperationKernelAuthorityError)
    pds_core_planes = load_yaml(root / pds_core_planes_path, OperationKernelAuthorityError)
    kernel_by_key = kernel_rows(kernel, kernel_path)
    validate_http_contract(kernel_by_key, http_catalog)
    validate_route_coverage(kernel_by_key, route_coverage)
    pds_count = validate_pds_assignments(kernel_by_key, pds_families)
    pds_plane_count = validate_pds_planes(kernel_by_key, pds_core_planes, pds_core_planes_path)
    if pds_plane_count != pds_count:
        raise OperationKernelAuthorityError("PDS plane assignment count must match PDS operation count")
    return {
        "operation_count": len(kernel_by_key),
        "pds_resource_operation_count": pds_count,
    }


def main() -> int:
    """Run the OperationKernel authority validator."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--kernel", type=Path, default=DEFAULT_KERNEL)
    parser.add_argument("--http-catalog", type=Path, default=DEFAULT_HTTP)
    parser.add_argument("--route-coverage", type=Path, default=DEFAULT_ROUTE_COVERAGE)
    parser.add_argument("--pds-families", type=Path, default=DEFAULT_PDS_FAMILIES)
    parser.add_argument("--pds-core-planes", type=Path, default=DEFAULT_PDS_CORE_PLANES)
    args = parser.parse_args()

    report = validate(args.root.resolve(), args.kernel, args.http_catalog, args.route_coverage, args.pds_families, args.pds_core_planes)
    print(
        "validated OperationKernel authority: "
        f"{report['operation_count']} operations, {report['pds_resource_operation_count']} PDS operations"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
