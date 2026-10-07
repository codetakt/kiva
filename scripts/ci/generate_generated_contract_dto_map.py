#!/usr/bin/env python3
"""Generate the OpenAPI generated-contract DTO map."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.dto_boundary_ledger import load_boundary_ledger
from kiva_ci.manifest import (
    load_yaml,
    require_existing_path,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
)
from kiva_ci.openapi import (
    HTTP_METHODS,
    collect_dto_boundary_schema_closure,
    collect_request_body_dto_schema_closure_names,
    collect_schema_ref_names,
    component_ref_name,
)
from kiva_ci.rust_dto import (
    RustDtoError,
    public_rust_items,
    rust_source_text,
    rust_struct_fields,
)
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_LEDGER = Path("spec/openapi/dto-boundary-ledger.yaml")
DEFAULT_OUTPUT = Path("spec/openapi/generated-contract-dto-map.yaml")


class GeneratedContractDtoMapError(RuntimeError):
    """Raised when the generated-contract DTO map cannot be generated."""


def collect_all_response_dto_schema_closure_names(
    openapi: dict[str, Any],
    openapi_path: str,
) -> set[str]:
    """Collect DTO-bearing schema closure for every OpenAPI response body."""
    components = require_mapping(
        openapi.get("components"),
        f"{openapi_path}.components",
        GeneratedContractDtoMapError,
    )
    response_components = require_mapping(
        components.get("responses", {}),
        f"{openapi_path}.components.responses",
        GeneratedContractDtoMapError,
    )
    paths = require_mapping(
        openapi.get("paths"),
        f"{openapi_path}.paths",
        GeneratedContractDtoMapError,
    )
    seeds: set[str] = set()
    for path_name, path_item_value in paths.items():
        path_item = require_mapping(
            path_item_value,
            f"{openapi_path}.paths.{path_name}",
            GeneratedContractDtoMapError,
        )
        for method, operation_value in path_item.items():
            if str(method).lower() not in HTTP_METHODS:
                continue
            operation = require_mapping(
                operation_value,
                f"{openapi_path}.paths.{path_name}.{method}",
                GeneratedContractDtoMapError,
            )
            responses = require_mapping(
                operation.get("responses"),
                f"{openapi_path}.paths.{path_name}.{method}.responses",
                GeneratedContractDtoMapError,
            )
            for status_code, response_value in responses.items():
                response = require_mapping(
                    response_value,
                    f"{openapi_path}.paths.{path_name}.{method}.responses.{status_code}",
                    GeneratedContractDtoMapError,
                )
                response_ref = component_ref_name(
                    response.get("$ref"),
                    "#/components/responses/",
                    f"{openapi_path}.paths.{path_name}.{method}.responses.{status_code}.$ref",
                    GeneratedContractDtoMapError,
                )
                if response_ref is not None:
                    response = require_mapping(
                        response_components.get(response_ref),
                        f"{openapi_path}.components.responses.{response_ref}",
                        GeneratedContractDtoMapError,
                    )
                content = response.get("content")
                if content is None:
                    continue
                for media_type, media_value in require_mapping(
                    content,
                    f"{openapi_path}.paths.{path_name}.{method}.responses.{status_code}.content",
                    GeneratedContractDtoMapError,
                ).items():
                    media = require_mapping(
                        media_value,
                        f"{openapi_path}.paths.{path_name}.{method}.responses.{status_code}.content.{media_type}",
                        GeneratedContractDtoMapError,
                    )
                    schema = require_mapping(
                        media.get("schema"),
                        f"{openapi_path}.paths.{path_name}.{method}.responses.{status_code}.content.{media_type}.schema",
                        GeneratedContractDtoMapError,
                    )
                    seeds.update(
                        collect_schema_ref_names(
                            schema,
                            f"{openapi_path}.paths.{path_name}.{method}.responses.{status_code}.content.{media_type}.schema",
                            GeneratedContractDtoMapError,
                        )
                    )
    return collect_dto_boundary_schema_closure(
        openapi,
        openapi_path,
        seeds,
        GeneratedContractDtoMapError,
    )


def schema_shape(schema: dict[str, Any], schema_name: str, rust_source: str) -> str:
    """Return the strictest OpenAPI/Rust map shape the generated DTO can satisfy."""
    if isinstance(schema.get("enum"), list):
        return "enum"
    properties = schema.get("properties")
    if schema.get("type") == "object" and isinstance(properties, dict):
        required = schema.get("required", [])
        if not isinstance(required, list) or not all(isinstance(item, str) for item in required):
            raise GeneratedContractDtoMapError(f"{schema_name}.required must be a string list")
        try:
            rust_fields = rust_struct_fields(rust_source, schema_name)
        except RustDtoError:
            return "presence"
        openapi_fields = set(properties)
        openapi_required = set(required)
        rust_field_names = set(rust_fields)
        rust_required = {field for field, is_required in rust_fields.items() if is_required}
        if openapi_fields == rust_field_names and openapi_required == rust_required:
            return "object"
    return "presence"


def schema_map(
    schemas: dict[str, Any],
    schema_names: set[str],
    rust_items: set[str],
    rust_source: str,
    contract_name: str,
) -> dict[str, dict[str, str]]:
    """Return generated map rows for one DTO contract."""
    missing_rust_items = sorted(schema_names.difference(rust_items))
    if missing_rust_items:
        raise GeneratedContractDtoMapError(
            f"{contract_name} generated crate is missing public DTOs: "
            + ", ".join(missing_rust_items)
        )
    return {
        schema_name: {
            "rust": schema_name,
            "shape": schema_shape(
                require_mapping(
                    schemas.get(schema_name),
                    f"{contract_name}.{schema_name}",
                    GeneratedContractDtoMapError,
                ),
                schema_name,
                rust_source,
            ),
        }
        for schema_name in sorted(schema_names)
    }


def contract_schema_names(
    *,
    contract_name: str,
    contract_count: int,
    all_public_schemas: set[str],
    request_schemas: set[str],
    response_schemas: set[str],
) -> set[str]:
    """Return the OpenAPI schema set represented by one generated DTO contract."""
    if contract_count == 1:
        return all_public_schemas
    if "request" in contract_name:
        return request_schemas
    if "response" in contract_name:
        return response_schemas
    raise GeneratedContractDtoMapError(
        f"{contract_name} must be the only generated DTO contract or include request/response in its name"
    )


def generated_contract_rows(root: Path, ledger: dict[str, Any]) -> list[dict[str, Any]]:
    """Return generated-contract DTO map rows from the DTO boundary ledger."""
    rows: list[dict[str, Any]] = []
    for index, raw_contract in enumerate(
        require_list(ledger.get("contracts"), "contracts", GeneratedContractDtoMapError)
    ):
        contract = require_mapping(raw_contract, f"contracts[{index}]", GeneratedContractDtoMapError)
        openapi_path = require_string(
            contract.get("openapi"),
            f"contracts[{index}].openapi",
            GeneratedContractDtoMapError,
        )
        generation_target = require_string(
            contract.get("generation_target"),
            f"{openapi_path}.generation_target",
            GeneratedContractDtoMapError,
        )
        contract_names = require_string_list(
            contract.get("generated_contract_dto_map_contracts"),
            f"{openapi_path}.generated_contract_dto_map_contracts",
            GeneratedContractDtoMapError,
            allow_empty=True,
        )
        if not contract_names:
            continue
        openapi = load_yaml(root / openapi_path, GeneratedContractDtoMapError)
        components = require_mapping(
            openapi.get("components"),
            f"{openapi_path}.components",
            GeneratedContractDtoMapError,
        )
        schemas = require_mapping(
            components.get("schemas"),
            f"{openapi_path}.components.schemas",
            GeneratedContractDtoMapError,
        )
        rust_source_path = f"{generation_target}/src/lib.rs"
        require_existing_path(
            root,
            rust_source_path,
            f"{openapi_path}.generation_target.lib",
            GeneratedContractDtoMapError,
        )
        rust_items = public_rust_items(root / rust_source_path)
        rust_source = rust_source_text(root, [rust_source_path])
        all_public_schemas = {schema_name for schema_name in schemas if schema_name in rust_items}
        request_schemas = collect_request_body_dto_schema_closure_names(
            openapi,
            openapi_path,
            GeneratedContractDtoMapError,
        ).intersection(rust_items)
        response_schemas = collect_all_response_dto_schema_closure_names(
            openapi,
            openapi_path,
        ).intersection(rust_items)
        for contract_name in contract_names:
            selected_schemas = contract_schema_names(
                contract_name=contract_name,
                contract_count=len(contract_names),
                all_public_schemas=all_public_schemas,
                request_schemas=request_schemas,
                response_schemas=response_schemas,
            )
            rows.append(
                {
                    "name": contract_name,
                    "openapi": openapi_path,
                    "rust_sources": [rust_source_path],
                    "schemas": schema_map(
                        schemas,
                        selected_schemas,
                        rust_items,
                        rust_source,
                        contract_name,
                    ),
                }
            )
    return rows


def generated_contract_dto_map_document(root: Path, ledger_path: Path) -> dict[str, Any]:
    """Return the generated-contract DTO map document."""
    ledger = load_boundary_ledger(root, ledger_path)
    if ledger.get("source_of_truth") != "openapi-contracts":
        raise GeneratedContractDtoMapError("source_of_truth must be openapi-contracts")
    return {
        "version": 1,
        "kind": "kiva-openapi-rust-dto-map-section",
        "section": "generated-contract-dtos",
        "source_of_truth": "openapi-contracts",
        "contracts": generated_contract_rows(root, ledger),
    }


def render_generated_contract_dto_map(root: Path, ledger_path: Path) -> str:
    """Render generated-contract DTO map YAML."""
    return dump_yaml(generated_contract_dto_map_document(root, ledger_path))


def main() -> int:
    """Generate or validate the generated-contract DTO map."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--ledger", type=Path, default=DEFAULT_LEDGER)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    rendered = render_generated_contract_dto_map(root, args.ledger)
    output_path = root / args.out
    command_hint = "scripts/ci/generate_generated_contract_dto_map.py"
    try:
        write_or_check_generated_artifacts(
            [GeneratedArtifact(output_path, rendered)],
            check=args.check,
            command_hint=command_hint,
            error_cls=GeneratedContractDtoMapError,
        )
    except GeneratedContractDtoMapError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated-contract DTO map: {args.out}")
        return 0
    print(f"generated generated-contract DTO map: {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
