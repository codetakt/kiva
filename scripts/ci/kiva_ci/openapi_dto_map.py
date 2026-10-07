"""OpenAPI-to-Rust DTO map loading helpers."""

from __future__ import annotations

from pathlib import Path
from typing import Any, TypeVar

from .manifest import (
    load_yaml,
    relative_path,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
    validation_error,
)


ErrorT = TypeVar("ErrorT", bound=RuntimeError)


def dto_map_contracts_by_name(
    document: dict[str, Any],
    label: str,
    error_cls: type[ErrorT],
) -> dict[str, dict[str, Any]]:
    """Return DTO map contracts keyed by stable contract name."""
    contracts_by_name: dict[str, dict[str, Any]] = {}
    for index, value in enumerate(
        require_list(document.get("contracts"), f"{label}.contracts", error_cls)
    ):
        contract = require_mapping(value, f"{label}.contracts[{index}]", error_cls)
        name = require_string(
            contract.get("name"),
            f"{label}.contracts[{index}].name",
            error_cls,
        )
        if name in contracts_by_name:
            raise validation_error(error_cls, f"duplicate DTO map contract name: {name}")
        contracts_by_name[name] = contract
    return contracts_by_name


def rust_source_paths(
    contract: dict[str, Any],
    contract_name: str,
    error_cls: type[ErrorT],
) -> list[str]:
    """Return Rust source paths declared by one DTO map contract."""
    rust_sources = contract.get("rust_sources")
    if rust_sources is not None:
        return require_string_list(
            rust_sources,
            f"{contract_name}.rust_sources",
            error_cls,
        )
    return [
        require_string(
            contract.get("rust"),
            f"{contract_name}.rust",
            error_cls,
        )
    ]


def load_openapi_dto_map(
    root: Path,
    map_path: Path,
    error_cls: type[ErrorT],
) -> dict[str, Any]:
    """Load an OpenAPI DTO map, expanding split section maps when declared."""
    document = load_yaml(root / map_path, error_cls)
    if document.get("version") != 1:
        raise validation_error(error_cls, f"{map_path}.version must be 1")

    source_maps = document.get("source_maps")
    if source_maps is None:
        dto_map_contracts_by_name(document, map_path.as_posix(), error_cls)
        return document

    merged_contracts: list[dict[str, Any]] = []
    seen_names: set[str] = set()
    sections: list[dict[str, Any]] = []
    for index, value in enumerate(
        require_list(source_maps, f"{map_path}.source_maps", error_cls)
    ):
        source_map = require_mapping(value, f"{map_path}.source_maps[{index}]", error_cls)
        source_path = relative_path(
            require_string(
                source_map.get("path"),
                f"{map_path}.source_maps[{index}].path",
                error_cls,
            ),
            f"{map_path}.source_maps[{index}].path",
            error_cls,
        )
        role = require_string(
            source_map.get("role"),
            f"{map_path}.source_maps[{index}].role",
            error_cls,
        )
        section = load_yaml(root / source_path, error_cls)
        if section.get("version") != 1:
            raise validation_error(error_cls, f"{source_path}.version must be 1")
        if section.get("kind") != "kiva-openapi-rust-dto-map-section":
            raise validation_error(
                error_cls,
                f"{source_path}.kind must be kiva-openapi-rust-dto-map-section",
            )
        section_role = require_string(
            section.get("section"),
            f"{source_path}.section",
            error_cls,
        )
        if section_role != role:
            raise validation_error(
                error_cls,
                f"{source_path}.section must match source map role {role}",
            )
        contracts = dto_map_contracts_by_name(section, source_path.as_posix(), error_cls)
        for name, contract in contracts.items():
            if name in seen_names:
                raise validation_error(error_cls, f"duplicate DTO map contract name: {name}")
            seen_names.add(name)
            merged_contracts.append(contract)
        sections.append(
            {
                "path": source_path.as_posix(),
                "role": role,
                "contract_count": len(contracts),
            }
        )

    if "contracts" in document:
        declared = dto_map_contracts_by_name(document, map_path.as_posix(), error_cls)
        merged = {str(contract["name"]): contract for contract in merged_contracts}
        if declared != merged:
            raise validation_error(
                error_cls,
                f"{map_path}.contracts drift from source_maps",
            )

    return {
        **document,
        "contracts": merged_contracts,
        "expanded_source_maps": sections,
    }
