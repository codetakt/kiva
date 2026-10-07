#!/usr/bin/env python3
"""Generate the OpenAPI DTO map projection manifest."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import yaml

from kiva_ci.manifest import (
    load_yaml,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
)
from kiva_ci.dto_boundary_ledger import load_boundary_ledger
from kiva_ci.openapi_dto_map import (
    dto_map_contracts_by_name,
    load_openapi_dto_map,
    rust_source_paths,
)
from kiva_ci.openapi import (
    collect_request_body_dto_schema_closure_names,
    collect_request_body_schema_names,
    collect_success_response_dto_schema_closure_names,
    collect_success_response_schema_names,
)
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_LEDGER = Path("spec/openapi/dto-boundary-ledger.yaml")
DEFAULT_DTO_MAP = Path("spec/openapi/semantic-compatibility-map.yaml")
DEFAULT_OUTPUT = Path("generated/openapi-dto-map-projection.yaml")


class DtoMapProjectionError(RuntimeError):
    """Raised when the OpenAPI DTO map projection cannot be generated."""


def coverage_sets(openapi: dict[str, Any], openapi_path: str) -> dict[str, set[str]]:
    """Return supported generated-slice coverage sets for one OpenAPI contract."""
    return {
        "all-request-body-schemas": collect_request_body_schema_names(
            openapi,
            openapi_path,
            DtoMapProjectionError,
        ),
        "all-request-body-schema-closure": collect_request_body_dto_schema_closure_names(
            openapi,
            openapi_path,
            DtoMapProjectionError,
        ),
        "all-success-response-schemas": collect_success_response_schema_names(
            openapi,
            openapi_path,
            DtoMapProjectionError,
        ),
        "all-success-response-schema-closure": collect_success_response_dto_schema_closure_names(
            openapi,
            openapi_path,
            DtoMapProjectionError,
        ),
    }


def slice_projection(
    *,
    name: str,
    slice_info: dict[str, Any],
    coverage_by_name: dict[str, set[str]],
) -> dict[str, Any]:
    """Return one generated DTO slice membership projection."""
    coverage_name = require_string(
        slice_info.get("coverage"),
        f"{name}.coverage",
        DtoMapProjectionError,
    )
    if coverage_name not in coverage_by_name:
        raise DtoMapProjectionError(f"{name}.coverage is invalid: {coverage_name}")
    raw_schemas = slice_info.get("schemas")
    schemas = (
        set(require_string_list(raw_schemas, f"{name}.schemas", DtoMapProjectionError))
        if raw_schemas is not None
        else set(coverage_by_name[coverage_name])
    )
    return {
        "name": name,
        "crate": require_string(slice_info.get("crate"), f"{name}.crate", DtoMapProjectionError),
        "coverage": coverage_name,
        "schema_count": len(schemas),
        "schemas": sorted(schemas),
    }


def generation_target_schema_names(
    dto_contract_names: list[str],
    dto_contracts: dict[str, dict[str, Any]],
    generation_target: str,
) -> set[str]:
    """Return schema names mapped by DTO contracts under a generation target."""
    schemas: set[str] = set()
    for contract_name in dto_contract_names:
        contract = dto_contracts.get(contract_name)
        if contract is None:
            raise DtoMapProjectionError(f"dto_map_contracts references missing contract: {contract_name}")
        contract_schemas = require_mapping(
            contract.get("schemas"),
            f"{contract_name}.schemas",
            DtoMapProjectionError,
        )
        if any(
            raw_source == generation_target or raw_source.startswith(f"{generation_target}/")
            for raw_source in rust_source_paths(contract, contract_name, DtoMapProjectionError)
        ):
            schemas.update(str(schema_name) for schema_name in contract_schemas)
    return schemas


def contract_projection(
    root: Path,
    contract: dict[str, Any],
    index: int,
    dto_contracts: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    """Return the generated DTO membership projection for one ledger contract."""
    openapi_path = require_string(
        contract.get("openapi"),
        f"contracts[{index}].openapi",
        DtoMapProjectionError,
    )
    openapi = load_yaml(root / openapi_path, DtoMapProjectionError)
    generation_target = require_string(
        contract.get("generation_target"),
        f"{openapi_path}.generation_target",
        DtoMapProjectionError,
    )
    external_contract_boundary = require_string(
        contract.get("external_contract_boundary"),
        f"{openapi_path}.external_contract_boundary",
        DtoMapProjectionError,
    )
    dto_contract_names = require_string_list(
        contract.get("dto_map_contracts"),
        f"{openapi_path}.dto_map_contracts",
        DtoMapProjectionError,
        allow_empty=True,
    )
    generated_contract_names = require_string_list(
        contract.get("generated_contract_dto_map_contracts"),
        f"{openapi_path}.generated_contract_dto_map_contracts",
        DtoMapProjectionError,
        allow_empty=True,
    )
    semantic_compatibility_names = require_string_list(
        contract.get("semantic_compatibility_map_contracts"),
        f"{openapi_path}.semantic_compatibility_map_contracts",
        DtoMapProjectionError,
        allow_empty=True,
    )
    coverage_by_name = coverage_sets(openapi, openapi_path)
    generated_slices: list[dict[str, Any]] = []
    if "active_generated_slice" in contract:
        generated_slices.append(
            slice_projection(
                name="active_generated_slice",
                slice_info=require_mapping(
                    contract.get("active_generated_slice"),
                    f"{openapi_path}.active_generated_slice",
                    DtoMapProjectionError,
                ),
                coverage_by_name=coverage_by_name,
            )
        )
    if "generated_response_slice" in contract:
        generated_slices.append(
            slice_projection(
                name="generated_response_slice",
                slice_info=require_mapping(
                    contract.get("generated_response_slice"),
                    f"{openapi_path}.generated_response_slice",
                    DtoMapProjectionError,
                ),
                coverage_by_name=coverage_by_name,
            )
        )

    generation_target_schemas = generation_target_schema_names(
        dto_contract_names,
        dto_contracts,
        generation_target,
    )
    return {
        "openapi": openapi_path,
        "boundary_status": require_string(
            contract.get("boundary_status"),
            f"{openapi_path}.boundary_status",
            DtoMapProjectionError,
        ),
        "generation_target": generation_target,
        "external_contract_boundary": external_contract_boundary,
        "internal_semantic_boundaries": require_string_list(
            contract.get("internal_semantic_boundaries"),
            f"{openapi_path}.internal_semantic_boundaries",
            DtoMapProjectionError,
        ),
        "dto_map_contracts": dto_contract_names,
        "generated_contract_dto_map_contracts": generated_contract_names,
        "semantic_compatibility_map_contracts": semantic_compatibility_names,
        "generation_target_schema_count": len(generation_target_schemas),
        "generation_target_schemas": sorted(generation_target_schemas),
        "generated_slices": generated_slices,
    }


def projection_document(root: Path, ledger_path: Path, dto_map_path: Path) -> dict[str, Any]:
    """Return the generated OpenAPI DTO map projection document."""
    ledger = load_boundary_ledger(root, ledger_path)
    active_guards = require_mapping(
        ledger.get("active_guards"),
        "active_guards",
        DtoMapProjectionError,
    )
    generated_contract_dto_map = require_string(
        active_guards.get("generated_contract_dto_map"),
        "active_guards.generated_contract_dto_map",
        DtoMapProjectionError,
    )
    semantic_compatibility_map = require_string(
        active_guards.get("semantic_compatibility_map"),
        "active_guards.semantic_compatibility_map",
        DtoMapProjectionError,
    )
    if dto_map_path.as_posix() != semantic_compatibility_map:
        raise DtoMapProjectionError(
            f"--dto-map must be the active semantic compatibility map: {semantic_compatibility_map}"
        )
    dto_contracts: dict[str, dict[str, Any]] = {}
    source_dto_maps: list[dict[str, Any]] = []
    for role, raw_path in (
        ("generated-contract-dtos", generated_contract_dto_map),
        ("semantic-compatibility", semantic_compatibility_map),
    ):
        dto_map = load_openapi_dto_map(root, Path(raw_path), DtoMapProjectionError)
        contracts = dto_map_contracts_by_name(
            dto_map,
            raw_path,
            DtoMapProjectionError,
        )
        for name, contract in contracts.items():
            if name in dto_contracts:
                raise DtoMapProjectionError(f"duplicate DTO map contract name: {name}")
            dto_contracts[name] = contract
        source_dto_maps.append(
            {
                "path": raw_path,
                "role": role,
                "contract_count": len(contracts),
            }
        )
    contracts = [
        contract_projection(
            root,
            require_mapping(value, f"contracts[{index}]", DtoMapProjectionError),
            index,
            dto_contracts,
        )
        for index, value in enumerate(
            require_list(ledger.get("contracts"), "contracts", DtoMapProjectionError)
        )
    ]
    return {
        "schema_version": 1,
        "kind": "kiva-openapi-dto-map-projection",
        "source_ledger": ledger_path.as_posix(),
        "source_dto_maps": source_dto_maps,
        "source_of_truth": require_string(
            ledger.get("source_of_truth"),
            "source_of_truth",
            DtoMapProjectionError,
        ),
        "contracts": contracts,
    }


def render_projection(root: Path, ledger_path: Path, dto_map_path: Path) -> str:
    """Render the expected projection YAML."""
    return yaml.safe_dump(
        projection_document(root, ledger_path, dto_map_path),
        sort_keys=False,
        allow_unicode=False,
    )


def main() -> int:
    """Generate or validate the OpenAPI DTO map projection manifest."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--ledger", type=Path, default=DEFAULT_LEDGER)
    parser.add_argument("--dto-map", type=Path, default=DEFAULT_DTO_MAP)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    rendered = render_projection(root, args.ledger, args.dto_map)
    output_path = root / args.out
    command_hint = "scripts/ci/generate_openapi_dto_map_projection.py"
    try:
        write_or_check_generated_artifacts(
            [GeneratedArtifact(output_path, rendered)],
            check=args.check,
            command_hint=command_hint,
            error_cls=DtoMapProjectionError,
        )
    except DtoMapProjectionError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated OpenAPI DTO map projection: {args.out}")
        return 0
    print(f"generated OpenAPI DTO map projection: {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
