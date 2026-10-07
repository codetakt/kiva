"""Route coverage projection helpers for Kiva HTTP surfaces."""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any, TypeVar

from .manifest import (
    ManifestError,
    require_existing_path,
    require_list,
    require_mapping,
    require_string,
    validation_error,
)


PATH_CONSTANT = re.compile(
    r"pub\(crate\)\s+const\s+(?P<name>[A-Z0-9_]+):\s*&str\s*=\s*\"(?P<path>[^\"]+)\";",
    re.MULTILINE,
)

ErrorT = TypeVar("ErrorT", bound=RuntimeError)


@dataclass(frozen=True)
class RouteCoverageTarget:
    """Route-file target for one OpenAPI contract."""

    openapi_contract: str
    route_file: Path
    path_binding: str
    path_constants_file: Path | None = None


@dataclass(frozen=True)
class RouteBlock:
    """One `.route(...)` registration block."""

    source: str

    def contains_path_anchor(self, anchor: str) -> bool:
        """Return whether this route block contains a path literal or constant."""
        return anchor in self.source

    def contains_method(self, method: str) -> bool:
        """Return whether this route block registers a method function."""
        return f"{method}(" in self.source or f".{method}(" in self.source


def _matching_paren(source: str, open_paren: int) -> int:
    """Return the matching parenthesis index."""
    depth = 0
    for index in range(open_paren, len(source)):
        character = source[index]
        if character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
            if depth == 0:
                return index
    raise ManifestError("route block has unbalanced parentheses")


def route_blocks(source: str) -> list[RouteBlock]:
    """Return all Axum `.route(...)` registration blocks in a source file."""
    blocks: list[RouteBlock] = []
    search_from = 0
    while True:
        start = source.find(".route(", search_from)
        if start < 0:
            return blocks
        open_paren = source.find("(", start)
        close_paren = _matching_paren(source, open_paren)
        blocks.append(RouteBlock(source[start : close_paren + 1]))
        search_from = close_paren + 1


def path_constants(source: str) -> dict[str, str]:
    """Return path constants keyed by path value."""
    constants: dict[str, str] = {}
    for match in PATH_CONSTANT.finditer(source):
        constants[match.group("path")] = match.group("name")
    return constants


def route_coverage_targets(
    root: Path,
    surface_catalog: dict[str, Any],
    manifest_path: Path,
    error_cls: type[ErrorT] = ManifestError,
) -> dict[str, RouteCoverageTarget]:
    """Return route coverage targets keyed by OpenAPI contract path."""
    projection = require_mapping(
        surface_catalog.get("route_coverage_projection"),
        f"{manifest_path}.route_coverage_projection",
        error_cls,
    )
    targets = require_list(
        projection.get("targets"),
        f"{manifest_path}.route_coverage_projection.targets",
        error_cls,
    )
    by_contract: dict[str, RouteCoverageTarget] = {}
    for index, value in enumerate(targets):
        target = require_mapping(
            value,
            f"{manifest_path}.route_coverage_projection.targets[{index}]",
            error_cls,
        )
        contract = require_string(
            target.get("openapi_contract"),
            f"{manifest_path}.route_coverage_projection.targets[{index}].openapi_contract",
            error_cls,
        )
        route_file = require_existing_path(
            root,
            require_string(
                target.get("route_file"),
                f"{manifest_path}.route_coverage_projection.targets[{index}].route_file",
                error_cls,
            ),
            f"{manifest_path}.route_coverage_projection.targets[{index}].route_file",
            error_cls,
        )
        path_binding = require_string(
            target.get("path_binding"),
            f"{manifest_path}.route_coverage_projection.targets[{index}].path_binding",
            error_cls,
        )
        if path_binding not in {"literal", "path-constant"}:
            raise validation_error(error_cls, f"{contract}.path_binding is invalid: {path_binding}")
        path_constants_file = None
        if path_binding == "path-constant":
            path_constants_file = require_existing_path(
                root,
                require_string(
                    target.get("path_constants_file"),
                    f"{manifest_path}.route_coverage_projection.targets[{index}].path_constants_file",
                    error_cls,
                ),
                f"{manifest_path}.route_coverage_projection.targets[{index}].path_constants_file",
                error_cls,
            )
        if contract in by_contract:
            raise validation_error(error_cls, f"duplicate route coverage target: {contract}")
        by_contract[contract] = RouteCoverageTarget(
            openapi_contract=contract,
            route_file=route_file,
            path_binding=path_binding,
            path_constants_file=path_constants_file,
        )
    return by_contract


def method_anchor(method: str) -> tuple[str, str]:
    """Return the route-file method anchor and binding label."""
    if method == "head":
        return "get", "implicit-head-via-get"
    return method, "explicit-method-router"


def path_anchor(
    entry: dict[str, Any],
    target: RouteCoverageTarget,
    constants_by_path: dict[str, str],
    error_cls: type[ErrorT],
) -> tuple[str, str | None]:
    """Return the route-file path anchor and optional path constant name."""
    path = require_string(entry.get("path"), "surface_entry.path", error_cls)
    if target.path_binding == "literal":
        return f'"{path}"', None
    constant = constants_by_path.get(path)
    if constant is None:
        raise validation_error(error_cls, f"{target.openapi_contract} has no path constant for {path}")
    return constant, constant


def build_route_coverage_rows(
    root: Path,
    surface_catalog: dict[str, Any],
    manifest_path: Path,
    entries: list[dict[str, Any]],
    error_cls: type[ErrorT] = ManifestError,
) -> list[dict[str, Any]]:
    """Build and validate route coverage rows for surface entries."""
    targets = route_coverage_targets(root, surface_catalog, manifest_path, error_cls)
    route_files = {
        contract: target.route_file.read_text(encoding="utf-8")
        for contract, target in targets.items()
    }
    route_file_blocks = {
        contract: route_blocks(source)
        for contract, source in route_files.items()
    }
    constants = {
        contract: (
            path_constants(target.path_constants_file.read_text(encoding="utf-8"))
            if target.path_constants_file is not None
            else {}
        )
        for contract, target in targets.items()
    }

    rows: list[dict[str, Any]] = []
    for entry in entries:
        contract = require_string(entry.get("openapi_contract"), "surface_entry.openapi_contract", error_cls)
        target = targets.get(contract)
        if target is None:
            raise validation_error(error_cls, f"missing route coverage target for {contract}")
        anchor, constant = path_anchor(entry, target, constants[contract], error_cls)
        method = require_string(entry.get("method"), "surface_entry.method", error_cls)
        method_function, method_binding = method_anchor(method)
        matched_block = next(
            (
                block
                for block in route_file_blocks[contract]
                if block.contains_path_anchor(anchor) and block.contains_method(method_function)
            ),
            None,
        )
        if matched_block is None:
            raise validation_error(
                error_cls,
                f"{contract} {method} {entry['path']} is not covered by {target.route_file}",
            )
        rows.append(
            {
                "openapi_contract": contract,
                "method": method,
                "path": entry["path"],
                "operation_id": require_string(
                    entry.get("operation_id"),
                    "surface_entry.operation_id",
                    error_cls,
                ),
                "route_file": target.route_file.as_posix(),
                "route_anchor": anchor,
                "path_constant": constant,
                "method_anchor": method_function,
                "method_binding": method_binding,
            }
        )
    return rows
