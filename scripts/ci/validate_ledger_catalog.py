#!/usr/bin/env python3
"""Validate LedgerCatalog and DesignPackage source ledgers."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.evidence import load_evidence_check_manifest
from kiva_ci.manifest import (
    load_yaml,
    mapping_by_unique_string_key,
    require_bool,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
    validation_error,
)


DEFAULT_LEDGER_CATALOG = Path("spec/architecture/ledger-catalog.source.yaml")
DEFAULT_DESIGN_PACKAGES = Path("spec/architecture/design-packages.source.yaml")
DEFAULT_DESIGN_UNIVERSE = Path("spec/architecture/design-universe.source.yaml")
DEFAULT_CORE_MODEL = Path("spec/architecture/core-model.source.yaml")
DEFAULT_ARCHITECTURE_GRAPH = Path("spec/architecture/architecture-graph.source.yaml")
DEFAULT_SCHEMA_REGISTRY = Path("spec/architecture/source-ledger-schemas.yaml")
DEFAULT_EVIDENCE_MANIFEST = Path("spec/ci/evidence-checks.yaml")
DESIGN_UNIVERSE_FRAGMENTS = {
    "source_nodes": (Path("spec/architecture/design-source-nodes.source.yaml"), "kiva-design-source-nodes-source"),
    "package_rows": (Path("spec/architecture/design-package-rows.source.yaml"), "kiva-design-package-rows-source"),
    "ledger_rows": (Path("spec/architecture/design-ledger-rows.source.yaml"), "kiva-design-ledger-rows-source"),
}
GENERATED_COMPATIBILITY_ARTIFACTS = {
    "spec/architecture/projection-specs.source.yaml",
    "spec/ci/evidence-checks.source.yaml",
}
LEDGER_LIFECYCLES = {
    "canonical-source",
    "normal-form-source",
    "supporting-source",
    "generated-source-projection",
    "generated-compatibility-artifact",
}
PACKAGE_LIFECYCLES = {
    "canonical-source-package",
    "normal-form-source-package",
    "supporting-source-package",
}


class LedgerCatalogError(RuntimeError):
    """Raised when LedgerCatalog or DesignPackage rows are inconsistent."""


def require_path(root: Path, path_value: str, label: str) -> None:
    """Validate a repository-relative path exists."""
    path = Path(path_value.split("#", maxsplit=1)[0])
    if not (root / path).exists():
        raise validation_error(LedgerCatalogError, f"{label} references missing path: {path_value}")


def schema_by_source(registry: dict[str, Any], registry_path: Path) -> dict[str, str]:
    """Return schema paths keyed by registered source path."""
    rows: dict[str, str] = {}
    for index, raw_row in enumerate(require_list(registry.get("schemas"), f"{registry_path}.schemas", LedgerCatalogError)):
        row = require_mapping(raw_row, f"{registry_path}.schemas[{index}]", LedgerCatalogError)
        source = require_string(row.get("source"), f"{registry_path}.schemas[{index}].source", LedgerCatalogError)
        schema = require_string(row.get("schema"), f"{registry_path}.schemas[{index}].schema", LedgerCatalogError)
        if source in rows:
            raise validation_error(LedgerCatalogError, f"duplicate schema registry source: {source}")
        rows[source] = schema
    return rows


def evidence_check_ids(manifest: dict[str, Any]) -> set[str]:
    """Return all evidence check ids from the generated manifest."""
    ids: set[str] = set()
    for group_index, raw_group in enumerate(require_list(manifest.get("groups"), "groups", LedgerCatalogError)):
        group = require_mapping(raw_group, f"groups[{group_index}]", LedgerCatalogError)
        group_id = require_string(group.get("id"), f"groups[{group_index}].id", LedgerCatalogError)
        for check_index, raw_check in enumerate(
            require_list(group.get("checks"), f"groups.{group_id}.checks", LedgerCatalogError)
        ):
            check = require_mapping(raw_check, f"groups.{group_id}.checks[{check_index}]", LedgerCatalogError)
            ids.add(require_string(check.get("id"), f"groups.{group_id}.checks[{check_index}].id", LedgerCatalogError))
    return ids


def core_model_rows(core_model: dict[str, Any]) -> dict[str, dict[str, Any]]:
    """Return primary and supporting core model rows keyed by id."""
    rows: dict[str, dict[str, Any]] = {}
    for field in ("primary_models", "supporting_models"):
        for index, raw_row in enumerate(require_list(core_model.get(field), f"core_model.{field}", LedgerCatalogError)):
            row = require_mapping(raw_row, f"core_model.{field}[{index}]", LedgerCatalogError)
            row_id = require_string(row.get("id"), f"core_model.{field}[{index}].id", LedgerCatalogError)
            if row_id in rows:
                raise validation_error(LedgerCatalogError, f"duplicate core model id: {row_id}")
            rows[row_id] = row
    return rows


def validate_design_universe(
    root: Path,
    source_path: Path,
    architecture_graph_path: Path,
    design_packages_path: Path,
    ledger_catalog_path: Path,
) -> int:
    """Validate DesignUniverse owns generated architecture/package/catalog projections."""
    source = load_yaml(root / source_path, LedgerCatalogError)
    if source.get("schema_version") != 1:
        raise validation_error(LedgerCatalogError, f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-design-universe-source":
        raise validation_error(LedgerCatalogError, f"{source_path}.kind must be kiva-design-universe-source")
    if source.get("status") != "active":
        raise validation_error(LedgerCatalogError, f"{source_path}.status must be active")
    model = require_mapping(source.get("universe_model"), f"{source_path}.universe_model", LedgerCatalogError)
    if model.get("root_type") != "DesignUniverse":
        raise validation_error(LedgerCatalogError, f"{source_path}.universe_model.root_type must be DesignUniverse")
    if model.get("primary_edit_entrypoint") is not True:
        raise validation_error(LedgerCatalogError, f"{source_path}.universe_model.primary_edit_entrypoint must be true")
    expected_outputs = {
        "architecture_graph_projection": architecture_graph_path.as_posix(),
        "design_package_projection": design_packages_path.as_posix(),
        "ledger_catalog_projection": ledger_catalog_path.as_posix(),
    }
    for key, expected_path in expected_outputs.items():
        actual = require_string(model.get(key), f"{source_path}.universe_model.{key}", LedgerCatalogError)
        if actual != expected_path:
            raise validation_error(LedgerCatalogError, f"{source_path}.universe_model.{key} must be {expected_path}")
        require_path(root, actual, f"{source_path}.universe_model.{key}")
    rows = mapping_by_unique_string_key(
        source.get("generated_source_projections"),
        key="id",
        label=f"{source_path}.generated_source_projections",
        error_cls=LedgerCatalogError,
    )
    expected_rows = {
        "architecture-graph": architecture_graph_path.as_posix(),
        "design-packages": design_packages_path.as_posix(),
        "ledger-catalog": ledger_catalog_path.as_posix(),
    }
    if set(rows) != set(expected_rows):
        raise validation_error(LedgerCatalogError, f"{source_path}.generated_source_projections must be {sorted(expected_rows)}")
    for row_id, expected_output in expected_rows.items():
        row = rows[row_id]
        if require_string(row.get("output"), f"{row_id}.output", LedgerCatalogError) != expected_output:
            raise validation_error(LedgerCatalogError, f"{row_id}.output must be {expected_output}")
        if require_string(row.get("generated_by"), f"{row_id}.generated_by", LedgerCatalogError) != "scripts/ci/generate_design_universe_projections.py":
            raise validation_error(LedgerCatalogError, f"{row_id}.generated_by must be scripts/ci/generate_design_universe_projections.py")
    fragment_rows = require_mapping(
        source.get("source_fragments"),
        f"{source_path}.source_fragments",
        LedgerCatalogError,
    )
    if set(fragment_rows) != set(DESIGN_UNIVERSE_FRAGMENTS):
        raise validation_error(
            LedgerCatalogError,
            f"{source_path}.source_fragments must be {sorted(DESIGN_UNIVERSE_FRAGMENTS)}",
        )
    fragment_paths: dict[str, Path] = {}
    for fragment_id, (expected_path, expected_kind) in DESIGN_UNIVERSE_FRAGMENTS.items():
        fragment_path = Path(
            require_string(
                fragment_rows.get(fragment_id),
                f"{source_path}.source_fragments.{fragment_id}",
                LedgerCatalogError,
            )
        )
        if fragment_path != expected_path:
            raise validation_error(
                LedgerCatalogError,
                f"{source_path}.source_fragments.{fragment_id} must be {expected_path}",
            )
        fragment = load_yaml(root / fragment_path, LedgerCatalogError)
        if fragment.get("kind") != expected_kind:
            raise validation_error(LedgerCatalogError, f"{fragment_path}.kind must be {expected_kind}")
        if fragment.get("parent_universe") != source_path.as_posix():
            raise validation_error(LedgerCatalogError, f"{fragment_path}.parent_universe must be {source_path}")
        fragment_model = require_mapping(fragment.get("fragment"), f"{fragment_path}.fragment", LedgerCatalogError)
        if fragment_model.get("parent_root_type") != "DesignUniverse":
            raise validation_error(LedgerCatalogError, f"{fragment_path}.fragment.parent_root_type must be DesignUniverse")
        if fragment_model.get("authoritative_rows") is not True:
            raise validation_error(LedgerCatalogError, f"{fragment_path}.fragment.authoritative_rows must be true")
        fragment_paths[fragment_id] = fragment_path
    projection_fragments = {
        architecture_graph_path: fragment_paths["source_nodes"],
        design_packages_path: fragment_paths["package_rows"],
        ledger_catalog_path: fragment_paths["ledger_rows"],
    }
    for projection_path, fragment_path in projection_fragments.items():
        projection = load_yaml(root / projection_path, LedgerCatalogError)
        generated_from = require_string_list(
            projection.get("generated_from"),
            f"{projection_path}.generated_from",
            LedgerCatalogError,
        )
        expected_generated_from = [source_path.as_posix(), fragment_path.as_posix()]
        if generated_from != expected_generated_from:
            raise validation_error(
                LedgerCatalogError,
                f"{projection_path}.generated_from must be {expected_generated_from}",
            )
    return len(rows)


def validate_design_packages(
    root: Path,
    source_path: Path,
    core_model_path: Path,
    manifest_path: Path,
) -> int:
    """Validate DesignPackage rows cover every core model row."""
    source = load_yaml(root / source_path, LedgerCatalogError)
    if source.get("schema_version") != 1:
        raise validation_error(LedgerCatalogError, f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-design-packages-source":
        raise validation_error(LedgerCatalogError, f"{source_path}.kind must be kiva-design-packages-source")
    if source.get("status") != "active":
        raise validation_error(LedgerCatalogError, f"{source_path}.status must be active")
    model = require_mapping(source.get("package_model"), f"{source_path}.package_model", LedgerCatalogError)
    if model.get("root_type") != "DesignPackage":
        raise validation_error(LedgerCatalogError, f"{source_path}.package_model.root_type must be DesignPackage")
    if model.get("parent_source") != core_model_path.as_posix():
        raise validation_error(LedgerCatalogError, f"{source_path}.package_model.parent_source must be {core_model_path}")
    require_string(model.get("rule"), f"{source_path}.package_model.rule", LedgerCatalogError)

    core_model = load_yaml(root / core_model_path, LedgerCatalogError)
    core_rows = core_model_rows(core_model)
    manifest = load_evidence_check_manifest(root / manifest_path, LedgerCatalogError)
    known_checks = evidence_check_ids(manifest)
    packages = mapping_by_unique_string_key(
        source.get("packages"),
        key="id",
        label=f"{source_path}.packages",
        error_cls=LedgerCatalogError,
    )
    package_core_ids: set[str] = set()
    for package_id, package in packages.items():
        core_model_id = require_string(package.get("core_model_id"), f"{package_id}.core_model_id", LedgerCatalogError)
        core_row = core_rows.get(core_model_id)
        if core_row is None:
            raise validation_error(LedgerCatalogError, f"{package_id} references unknown core model id: {core_model_id}")
        package_core_ids.add(core_model_id)
        expected_root = require_string(core_row.get("root_type"), f"core_model.{core_model_id}.root_type", LedgerCatalogError)
        if package.get("root_type") != expected_root:
            raise validation_error(LedgerCatalogError, f"{package_id}.root_type must be {expected_root}")
        lifecycle = require_string(package.get("lifecycle"), f"{package_id}.lifecycle", LedgerCatalogError)
        if lifecycle not in PACKAGE_LIFECYCLES:
            raise validation_error(LedgerCatalogError, f"{package_id}.lifecycle is invalid: {lifecycle}")
        required_check = package.get("required_check")
        if required_check is not None and require_string(required_check, f"{package_id}.required_check", LedgerCatalogError) not in known_checks:
            raise validation_error(LedgerCatalogError, f"{package_id} references missing evidence check: {required_check}")
        for field in ("entrypoint_refs", "supporting_refs", "generated_compatibility_refs", "review_refs"):
            for path_value in require_string_list(
                package.get(field, []),
                f"{package_id}.{field}",
                LedgerCatalogError,
                allow_empty=True,
            ):
                require_path(root, path_value, f"{package_id}.{field}")
    missing = sorted(set(core_rows).difference(package_core_ids))
    if missing:
        raise validation_error(LedgerCatalogError, f"DesignPackage rows missing core model ids: {missing}")
    return len(packages)


def validate_ledger_catalog(
    root: Path,
    source_path: Path,
    graph_path: Path,
    schema_registry_path: Path,
    design_packages_path: Path,
) -> int:
    """Validate LedgerCatalog rows against ArchitectureGraph and schema registry."""
    source = load_yaml(root / source_path, LedgerCatalogError)
    if source.get("schema_version") != 1:
        raise validation_error(LedgerCatalogError, f"{source_path}.schema_version must be 1")
    if source.get("kind") != "kiva-ledger-catalog-source":
        raise validation_error(LedgerCatalogError, f"{source_path}.kind must be kiva-ledger-catalog-source")
    if source.get("status") != "active":
        raise validation_error(LedgerCatalogError, f"{source_path}.status must be active")
    model = require_mapping(source.get("catalog_model"), f"{source_path}.catalog_model", LedgerCatalogError)
    if model.get("root_type") != "LedgerCatalog":
        raise validation_error(LedgerCatalogError, f"{source_path}.catalog_model.root_type must be LedgerCatalog")
    expected_model_paths = {
        "architecture_graph": graph_path.as_posix(),
        "schema_registry": schema_registry_path.as_posix(),
        "design_packages": design_packages_path.as_posix(),
        "source_registry": "generated/architecture/source-registry.yaml",
    }
    for key, expected_path in expected_model_paths.items():
        actual = require_string(model.get(key), f"{source_path}.catalog_model.{key}", LedgerCatalogError)
        if actual != expected_path:
            raise validation_error(LedgerCatalogError, f"{source_path}.catalog_model.{key} must be {expected_path}")
        require_path(root, actual, f"{source_path}.catalog_model.{key}")
    require_string(model.get("lifecycle_rule"), f"{source_path}.catalog_model.lifecycle_rule", LedgerCatalogError)
    required_generated = set(
        require_string_list(
            source.get("required_generated_compatibility_artifacts"),
            f"{source_path}.required_generated_compatibility_artifacts",
            LedgerCatalogError,
        )
    )
    if required_generated != GENERATED_COMPATIBILITY_ARTIFACTS:
        raise validation_error(
            LedgerCatalogError,
            f"{source_path}.required_generated_compatibility_artifacts must be {sorted(GENERATED_COMPATIBILITY_ARTIFACTS)}",
        )

    graph = load_yaml(root / graph_path, LedgerCatalogError)
    graph_sources = require_mapping(graph.get("sources"), f"{graph_path}.sources", LedgerCatalogError)
    schemas = schema_by_source(load_yaml(root / schema_registry_path, LedgerCatalogError), schema_registry_path)
    ledger_rows = mapping_by_unique_string_key(
        source.get("ledgers"),
        key="id",
        label=f"{source_path}.ledgers",
        error_cls=LedgerCatalogError,
    )
    generated_rows: set[str] = set()
    for row_id, row in ledger_rows.items():
        path_value = require_string(row.get("path"), f"{row_id}.path", LedgerCatalogError)
        require_path(root, path_value, f"{row_id}.path")
        lifecycle = require_string(row.get("lifecycle"), f"{row_id}.lifecycle", LedgerCatalogError)
        if lifecycle not in LEDGER_LIFECYCLES:
            raise validation_error(LedgerCatalogError, f"{row_id}.lifecycle is invalid: {lifecycle}")
        source_of_truth = require_bool(row.get("source_of_truth"), f"{row_id}.source_of_truth", LedgerCatalogError)
        if lifecycle == "generated-compatibility-artifact":
            generated_rows.add(path_value)
            if source_of_truth:
                raise validation_error(LedgerCatalogError, f"{row_id} generated compatibility artifact must not be source_of_truth")
            require_path(root, require_string(row.get("generated_by"), f"{row_id}.generated_by", LedgerCatalogError), f"{row_id}.generated_by")
        elif lifecycle == "generated-source-projection":
            if source_of_truth:
                raise validation_error(LedgerCatalogError, f"{row_id} generated source projection must not be source_of_truth")
            require_path(root, require_string(row.get("generated_by"), f"{row_id}.generated_by", LedgerCatalogError), f"{row_id}.generated_by")
        elif not source_of_truth:
            raise validation_error(LedgerCatalogError, f"{row_id} {lifecycle} must be source_of_truth")
        source_id = row.get("architecture_graph_source_id")
        if source_id is not None:
            source_id = require_string(source_id, f"{row_id}.architecture_graph_source_id", LedgerCatalogError)
            if source_id not in graph_sources:
                raise validation_error(LedgerCatalogError, f"{row_id} references unknown ArchitectureGraph source id: {source_id}")
            artifacts = require_string_list(
                require_mapping(graph_sources[source_id], f"{graph_path}.sources.{source_id}", LedgerCatalogError).get("artifacts", []),
                f"{graph_path}.sources.{source_id}.artifacts",
                LedgerCatalogError,
                allow_empty=True,
            )
            if path_value not in artifacts:
                raise validation_error(LedgerCatalogError, f"{row_id}.path must appear in ArchitectureGraph artifacts for {source_id}")
        schema = require_string(row.get("schema"), f"{row_id}.schema", LedgerCatalogError)
        require_path(root, schema, f"{row_id}.schema")
        registered_schema = schemas.get(path_value)
        if registered_schema != schema:
            raise validation_error(LedgerCatalogError, f"{row_id}.schema must match schema registry for {path_value}")
        require_path(root, require_string(row.get("validates_with"), f"{row_id}.validates_with", LedgerCatalogError), f"{row_id}.validates_with")
        require_path(root, require_string(row.get("review_surface"), f"{row_id}.review_surface", LedgerCatalogError), f"{row_id}.review_surface")
    if generated_rows != GENERATED_COMPATIBILITY_ARTIFACTS:
        raise validation_error(
            LedgerCatalogError,
            f"generated compatibility LedgerCatalog rows must be {sorted(GENERATED_COMPATIBILITY_ARTIFACTS)}",
        )
    return len(ledger_rows)


def main() -> int:
    """Validate LedgerCatalog and DesignPackage ledgers."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--ledger-catalog", type=Path, default=DEFAULT_LEDGER_CATALOG)
    parser.add_argument("--design-packages", type=Path, default=DEFAULT_DESIGN_PACKAGES)
    parser.add_argument("--design-universe", type=Path, default=DEFAULT_DESIGN_UNIVERSE)
    parser.add_argument("--core-model", type=Path, default=DEFAULT_CORE_MODEL)
    parser.add_argument("--architecture-graph", type=Path, default=DEFAULT_ARCHITECTURE_GRAPH)
    parser.add_argument("--schema-registry", type=Path, default=DEFAULT_SCHEMA_REGISTRY)
    parser.add_argument("--evidence-manifest", type=Path, default=DEFAULT_EVIDENCE_MANIFEST)
    args = parser.parse_args()

    root = args.root.resolve()
    projection_count = validate_design_universe(
        root,
        args.design_universe,
        args.architecture_graph,
        args.design_packages,
        args.ledger_catalog,
    )
    package_count = validate_design_packages(root, args.design_packages, args.core_model, args.evidence_manifest)
    ledger_count = validate_ledger_catalog(root, args.ledger_catalog, args.architecture_graph, args.schema_registry, args.design_packages)
    print(f"validated LedgerCatalog: {ledger_count} ledgers, {package_count} design packages, {projection_count} DesignUniverse projections")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
