#!/usr/bin/env python3
"""Validate the generated cross-surface family ledger."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from kiva_ci.manifest import (
    load_yaml,
    relative_path,
    require_existing_path,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
)
from surface_catalog import (
    load_surface_catalog,
    project_surface_family_ledger,
    render_yaml,
    surface_catalog_paths,
)


DEFAULT_SURFACE_CATALOG = Path("generated/architecture/surface-catalog.yaml")


class SurfaceFamilyError(RuntimeError):
    """Raised when the cross-surface family ledger is inconsistent."""


def validate_surface_operation_projection_basis(
    ledger: dict[str, Any],
    expected_role: str,
) -> int:
    """Validate common SurfaceOperation projection provenance."""
    basis = require_mapping(ledger.get("projection_basis"), "projection_basis", SurfaceFamilyError)
    if basis.get("root_type") != "SurfaceOperation":
        raise SurfaceFamilyError("projection_basis.root_type must be SurfaceOperation")
    if basis.get("projection_role") != expected_role:
        raise SurfaceFamilyError(f"projection_basis.projection_role must be {expected_role}")
    if require_string_list(
        basis.get("operation_key_fields"),
        "projection_basis.operation_key_fields",
        SurfaceFamilyError,
    ) != ["openapi_contract", "operation_id"]:
        raise SurfaceFamilyError("projection_basis.operation_key_fields drift")
    if require_string_list(
        basis.get("route_key_fields"),
        "projection_basis.route_key_fields",
        SurfaceFamilyError,
    ) != ["openapi_contract", "method", "path"]:
        raise SurfaceFamilyError("projection_basis.route_key_fields drift")
    operation_count = basis.get("operation_count")
    if not isinstance(operation_count, int) or operation_count <= 0:
        raise SurfaceFamilyError("projection_basis.operation_count must be a positive integer")
    return operation_count


def validate_surface_family_ledger(root: Path, catalog_path: Path) -> dict[str, Any]:
    """Validate the generated surface-family ledger and return a report."""
    catalog = load_surface_catalog(root / catalog_path, SurfaceFamilyError)
    paths = surface_catalog_paths(catalog, catalog_path, SurfaceFamilyError, root=root)
    ledger_path = paths["surface_families"]
    require_existing_path(root, ledger_path.as_posix(), "surface_family_ledger", SurfaceFamilyError)
    expected = render_yaml(project_surface_family_ledger(catalog, SurfaceFamilyError))
    actual = (root / ledger_path).read_text(encoding="utf-8")
    if actual != expected:
        raise SurfaceFamilyError(
            f"{ledger_path} is not current; run scripts/ci/generate_surface_catalog_projections.py"
        )

    ledger = load_yaml(root / ledger_path, SurfaceFamilyError)
    if ledger.get("kind") != "kiva-surface-family-ledger":
        raise SurfaceFamilyError(f"{ledger_path}.kind must be kiva-surface-family-ledger")
    if ledger.get("generated_from") != catalog_path.as_posix():
        raise SurfaceFamilyError(f"{ledger_path}.generated_from must be {catalog_path.as_posix()}")
    surface_operation_count = validate_surface_operation_projection_basis(
        ledger,
        "cross-surface-family-ownership-projection",
    )
    families = require_list(ledger.get("families"), f"{ledger_path}.families", SurfaceFamilyError)
    seen_operations: set[tuple[str, str, str]] = set()
    operation_count = 0
    for index, value in enumerate(families):
        family = require_mapping(value, f"{ledger_path}.families[{index}]", SurfaceFamilyError)
        family_id = require_string(family.get("id"), f"{ledger_path}.families[{index}].id", SurfaceFamilyError)
        operations = require_list(
            family.get("operations"),
            f"{ledger_path}.families[{index}].operations",
            SurfaceFamilyError,
        )
        declared_count = family.get("operation_count")
        if declared_count != len(operations):
            raise SurfaceFamilyError(f"{family_id}.operation_count must equal operations length")
        for operation_index, raw_operation in enumerate(operations):
            operation = require_mapping(
                raw_operation,
                f"{ledger_path}.families[{index}].operations[{operation_index}]",
                SurfaceFamilyError,
            )
            operation_key = (
                require_string(operation.get("openapi_contract"), f"{family_id}.openapi_contract", SurfaceFamilyError),
                require_string(operation.get("method"), f"{family_id}.method", SurfaceFamilyError),
                require_string(operation.get("path"), f"{family_id}.path", SurfaceFamilyError),
            )
            if operation_key in seen_operations:
                raise SurfaceFamilyError(f"duplicate surface-family operation owner: {operation_key}")
            seen_operations.add(operation_key)
        operation_count += len(operations)
    catalog_operation_count = len(require_list(catalog.get("entries"), f"{catalog_path}.entries", SurfaceFamilyError))
    if operation_count != catalog_operation_count:
        raise SurfaceFamilyError(
            f"{ledger_path} operation count drift: ledger={operation_count} catalog={catalog_operation_count}"
        )
    return {
        "catalog": catalog_path.as_posix(),
        "ledger": ledger_path.as_posix(),
        "family_count": len(families),
        "operation_count": operation_count,
        "surface_operation_count": surface_operation_count,
    }


def parse_args() -> argparse.Namespace:
    """Parse CLI arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--surface-catalog", type=Path, default=DEFAULT_SURFACE_CATALOG)
    parser.add_argument("--json-report", type=Path)
    return parser.parse_args()


def main() -> int:
    """CLI entrypoint."""
    args = parse_args()
    root = args.root.resolve()
    try:
        catalog_path = relative_path(str(args.surface_catalog), "--surface-catalog", SurfaceFamilyError)
        report = validate_surface_family_ledger(root, catalog_path)
    except SurfaceFamilyError as error:
        print(f"surface-family validation failed: {error}", file=sys.stderr)
        return 1

    if args.json_report is not None:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    else:
        print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
