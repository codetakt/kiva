#!/usr/bin/env python3
"""Generate ArchitectureGraph, DesignPackage, and LedgerCatalog projections from DesignUniverse."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.manifest import require_list, require_mapping, require_string
from kiva_ci.runtime import GeneratedArtifact, LedgerRuntime, ProjectionRuntime


DEFAULT_SOURCE = Path("spec/architecture/design-universe.source.yaml")
DEFAULT_ARCHITECTURE_GRAPH = Path("spec/architecture/architecture-graph.source.yaml")
DEFAULT_DESIGN_PACKAGES = Path("spec/architecture/design-packages.source.yaml")
DEFAULT_LEDGER_CATALOG = Path("spec/architecture/ledger-catalog.source.yaml")
DEFAULT_SOURCE_NODES = Path("spec/architecture/design-source-nodes.source.yaml")
DEFAULT_PACKAGE_ROWS = Path("spec/architecture/design-package-rows.source.yaml")
DEFAULT_LEDGER_ROWS = Path("spec/architecture/design-ledger-rows.source.yaml")
FRAGMENT_CONTRACTS = {
    "source_nodes": (DEFAULT_SOURCE_NODES, "kiva-design-source-nodes-source", "architecture_graph_projection"),
    "package_rows": (DEFAULT_PACKAGE_ROWS, "kiva-design-package-rows-source", "design_package_projection"),
    "ledger_rows": (DEFAULT_LEDGER_ROWS, "kiva-design-ledger-rows-source", "ledger_catalog_projection"),
}


class DesignUniverseProjectionError(RuntimeError):
    """Raised when DesignUniverse projections cannot be generated."""


def validate_universe_envelope(source: dict[str, Any], source_path: Path) -> dict[str, Any]:
    """Validate the DesignUniverse envelope and return its model row."""
    if source.get("schema_version") != 1:
        raise DesignUniverseProjectionError(f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-design-universe-source":
        raise DesignUniverseProjectionError(f"{source_path}.kind must be kiva-design-universe-source")
    if source.get("status") != "active":
        raise DesignUniverseProjectionError(f"{source_path}.status must be active")
    model = require_mapping(
        source.get("universe_model"),
        f"{source_path}.universe_model",
        DesignUniverseProjectionError,
    )
    if model.get("root_type") != "DesignUniverse":
        raise DesignUniverseProjectionError(f"{source_path}.universe_model.root_type must be DesignUniverse")
    if model.get("primary_edit_entrypoint") is not True:
        raise DesignUniverseProjectionError(f"{source_path}.universe_model.primary_edit_entrypoint must be true")
    expected = {
        "architecture_graph_projection": DEFAULT_ARCHITECTURE_GRAPH.as_posix(),
        "design_package_projection": DEFAULT_DESIGN_PACKAGES.as_posix(),
        "ledger_catalog_projection": DEFAULT_LEDGER_CATALOG.as_posix(),
    }
    for key, expected_path in expected.items():
        actual = require_string(model.get(key), f"{source_path}.universe_model.{key}", DesignUniverseProjectionError)
        if actual != expected_path:
            raise DesignUniverseProjectionError(f"{source_path}.universe_model.{key} must be {expected_path}")
    return model


def validate_projection_rows(source: dict[str, Any], source_path: Path) -> None:
    """Validate declared generated source projection rows."""
    rows = require_list(
        source.get("generated_source_projections"),
        f"{source_path}.generated_source_projections",
        DesignUniverseProjectionError,
    )
    by_id: dict[str, dict[str, Any]] = {}
    for index, raw_row in enumerate(rows):
        row = require_mapping(raw_row, f"{source_path}.generated_source_projections[{index}]", DesignUniverseProjectionError)
        row_id = require_string(row.get("id"), f"{source_path}.generated_source_projections[{index}].id", DesignUniverseProjectionError)
        if row_id in by_id:
            raise DesignUniverseProjectionError(f"duplicate DesignUniverse projection id: {row_id}")
        by_id[row_id] = row
    expected = {
        "architecture-graph": ("ArchitectureGraph", DEFAULT_ARCHITECTURE_GRAPH.as_posix()),
        "design-packages": ("DesignPackage", DEFAULT_DESIGN_PACKAGES.as_posix()),
        "ledger-catalog": ("LedgerCatalog", DEFAULT_LEDGER_CATALOG.as_posix()),
    }
    if set(by_id) != set(expected):
        raise DesignUniverseProjectionError(f"DesignUniverse projections must be {sorted(expected)}")
    for row_id, (root_type, output) in expected.items():
        row = by_id[row_id]
        if row.get("root_type") != root_type:
            raise DesignUniverseProjectionError(f"{row_id}.root_type must be {root_type}")
        if row.get("output") != output:
            raise DesignUniverseProjectionError(f"{row_id}.output must be {output}")
        generator = require_string(row.get("generated_by"), f"{row_id}.generated_by", DesignUniverseProjectionError)
        if generator != "scripts/ci/generate_design_universe_projections.py":
            raise DesignUniverseProjectionError(f"{row_id}.generated_by must be scripts/ci/generate_design_universe_projections.py")


def load_fragments(root: Path, source: dict[str, Any], source_path: Path) -> dict[str, tuple[Path, dict[str, Any]]]:
    """Load and validate normalized DesignUniverse fragments."""
    declarations = require_mapping(
        source.get("source_fragments"),
        f"{source_path}.source_fragments",
        DesignUniverseProjectionError,
    )
    if set(declarations) != set(FRAGMENT_CONTRACTS):
        raise DesignUniverseProjectionError(
            f"{source_path}.source_fragments must be {sorted(FRAGMENT_CONTRACTS)}"
        )
    fragments: dict[str, tuple[Path, dict[str, Any]]] = {}
    for fragment_id, (expected_path, expected_kind, projection_key) in FRAGMENT_CONTRACTS.items():
        declared_path = Path(
            require_string(
                declarations.get(fragment_id),
                f"{source_path}.source_fragments.{fragment_id}",
                DesignUniverseProjectionError,
            )
        )
        if declared_path != expected_path:
            raise DesignUniverseProjectionError(
                f"{source_path}.source_fragments.{fragment_id} must be {expected_path}"
            )
        ledger = LedgerRuntime.load(root, declared_path, DesignUniverseProjectionError)
        ledger.require_envelope(kind=expected_kind, error_cls=DesignUniverseProjectionError)
        fragment = require_mapping(
            ledger.document.get("fragment"),
            f"{declared_path}.fragment",
            DesignUniverseProjectionError,
        )
        if fragment.get("id") != expected_path.stem.removesuffix(".source"):
            raise DesignUniverseProjectionError(f"{declared_path}.fragment.id is invalid")
        if fragment.get("parent_root_type") != "DesignUniverse":
            raise DesignUniverseProjectionError(f"{declared_path}.fragment.parent_root_type must be DesignUniverse")
        if fragment.get("authoritative_rows") is not True:
            raise DesignUniverseProjectionError(f"{declared_path}.fragment.authoritative_rows must be true")
        if ledger.document.get("parent_universe") != source_path.as_posix():
            raise DesignUniverseProjectionError(f"{declared_path}.parent_universe must be {source_path}")
        require_mapping(
            ledger.document.get(projection_key),
            f"{declared_path}.{projection_key}",
            DesignUniverseProjectionError,
        )
        fragments[fragment_id] = (declared_path, ledger.document)
    return fragments


def architecture_graph_document(source: dict[str, Any], source_path: Path, fragment_path: Path) -> dict[str, Any]:
    """Return the generated ArchitectureGraph source projection document."""
    projection = require_mapping(
        source.get("architecture_graph_projection"),
        f"{source_path}.architecture_graph_projection",
        DesignUniverseProjectionError,
    )
    return {
        "schema_version": 1,
        "kind": "kiva-architecture-graph-source",
        "status": "active",
        "description": require_string(
            projection.get("description"),
            f"{source_path}.architecture_graph_projection.description",
            DesignUniverseProjectionError,
        ),
        "generated_from": [source_path.as_posix(), fragment_path.as_posix()],
        "generated_by": "scripts/ci/generate_design_universe_projections.py",
        "generated_artifacts": require_list(
            projection.get("generated_artifacts"),
            f"{source_path}.architecture_graph_projection.generated_artifacts",
            DesignUniverseProjectionError,
        ),
        "projection_policy": require_mapping(
            projection.get("projection_policy"),
            f"{source_path}.architecture_graph_projection.projection_policy",
            DesignUniverseProjectionError,
        ),
        "sources": require_mapping(
            projection.get("sources"),
            f"{source_path}.architecture_graph_projection.sources",
            DesignUniverseProjectionError,
        ),
    }


def design_package_document(source: dict[str, Any], source_path: Path, fragment_path: Path) -> dict[str, Any]:
    """Return the generated DesignPackage source projection document."""
    projection = require_mapping(
        source.get("design_package_projection"),
        f"{source_path}.design_package_projection",
        DesignUniverseProjectionError,
    )
    return {
        "schema_version": 1,
        "kind": "kiva-design-packages-source",
        "status": "active",
        "description": (
            "Generated DesignPackage source projection from DesignUniverse. "
            f"Start at {source_path.as_posix()}; edit {fragment_path.as_posix()}, not this file."
        ),
        "generated_from": [source_path.as_posix(), fragment_path.as_posix()],
        "generated_by": "scripts/ci/generate_design_universe_projections.py",
        "package_model": require_mapping(
            projection.get("package_model"),
            f"{source_path}.design_package_projection.package_model",
            DesignUniverseProjectionError,
        ),
        "packages": require_list(
            projection.get("packages"),
            f"{source_path}.design_package_projection.packages",
            DesignUniverseProjectionError,
        ),
    }


def ledger_catalog_document(source: dict[str, Any], source_path: Path, fragment_path: Path) -> dict[str, Any]:
    """Return the generated LedgerCatalog source projection document."""
    projection = require_mapping(
        source.get("ledger_catalog_projection"),
        f"{source_path}.ledger_catalog_projection",
        DesignUniverseProjectionError,
    )
    return {
        "schema_version": 1,
        "kind": "kiva-ledger-catalog-source",
        "status": "active",
        "description": (
            "Generated LedgerCatalog source projection from DesignUniverse. "
            f"Start at {source_path.as_posix()}; edit {fragment_path.as_posix()}, not this file."
        ),
        "generated_from": [source_path.as_posix(), fragment_path.as_posix()],
        "generated_by": "scripts/ci/generate_design_universe_projections.py",
        "catalog_model": require_mapping(
            projection.get("catalog_model"),
            f"{source_path}.ledger_catalog_projection.catalog_model",
            DesignUniverseProjectionError,
        ),
        "required_generated_compatibility_artifacts": require_list(
            projection.get("required_generated_compatibility_artifacts"),
            f"{source_path}.ledger_catalog_projection.required_generated_compatibility_artifacts",
            DesignUniverseProjectionError,
        ),
        "ledgers": require_list(
            projection.get("ledgers"),
            f"{source_path}.ledger_catalog_projection.ledgers",
            DesignUniverseProjectionError,
        ),
    }


def rendered_artifacts(root: Path, source_path: Path) -> list[GeneratedArtifact]:
    """Return generated source-projection artifacts."""
    ledger = LedgerRuntime.load(root, source_path, DesignUniverseProjectionError)
    ledger.require_envelope(kind="kiva-design-universe-source", error_cls=DesignUniverseProjectionError)
    source = ledger.document
    validate_universe_envelope(source, source_path)
    validate_projection_rows(source, source_path)
    fragments = load_fragments(root, source, source_path)
    source_nodes_path, source_nodes = fragments["source_nodes"]
    package_rows_path, package_rows = fragments["package_rows"]
    ledger_rows_path, ledger_rows = fragments["ledger_rows"]
    return [
        GeneratedArtifact(
            root / DEFAULT_ARCHITECTURE_GRAPH,
            dump_yaml(architecture_graph_document(source_nodes, source_path, source_nodes_path)),
        ),
        GeneratedArtifact(
            root / DEFAULT_DESIGN_PACKAGES,
            dump_yaml(design_package_document(package_rows, source_path, package_rows_path)),
        ),
        GeneratedArtifact(
            root / DEFAULT_LEDGER_CATALOG,
            dump_yaml(ledger_catalog_document(ledger_rows, source_path, ledger_rows_path)),
        ),
    ]


def main() -> int:
    """Generate or validate DesignUniverse source projections."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    projection = ProjectionRuntime(
        command_hint="scripts/ci/generate_design_universe_projections.py",
        artifacts=tuple(rendered_artifacts(root, args.source)),
    )
    try:
        projection.write_or_check(check=args.check, error_cls=DesignUniverseProjectionError)
    except DesignUniverseProjectionError as error:
        raise SystemExit(str(error)) from error
    verb = "validated" if args.check else "generated"
    print(
        f"{verb} DesignUniverse projections: "
        f"{DEFAULT_ARCHITECTURE_GRAPH}, {DEFAULT_DESIGN_PACKAGES}, {DEFAULT_LEDGER_CATALOG}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
