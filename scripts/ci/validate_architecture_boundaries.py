#!/usr/bin/env python3
"""Validate Kiva architecture boundary metadata against source layout."""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path
from typing import Any

import yaml

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from application_operation_catalog_codegen import (
    render_operation_catalog,
    render_operation_ids,
    render_workflow_metadata,
)
from surface_catalog import (
    load_surface_catalog,
    load_operation_ownership_source,
    load_pds_resource_family_source,
    load_surface_family_source,
    load_surface_projection_policy,
    project_surface_catalog_bundle,
    render_surface_profile_registry,
    render_yaml,
    surface_component_paths,
    surface_catalog_paths,
    surface_projection_targets,
)
from kiva_ci.evidence import (
    load_evidence_check_manifest,
    validate_evidence_check_manifest_metadata,
    validate_evidence_check_scripts,
)
from kiva_ci.architecture import (
    load_architecture_manifest,
    validate_compatibility_surface_ledgers as validate_compatibility_ledgers,
)
from kiva_ci.openapi import collect_operations_by_id


DEFAULT_MANIFEST = Path("spec/architecture/boundaries.yaml")


class BoundaryError(RuntimeError):
    """Raised when architecture boundary metadata drifts from source."""


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise BoundaryError(f"{path} root must be a mapping")
    return value


def require_list(value: Any, label: str) -> list[Any]:
    """Return a YAML list or raise a validation error."""
    if not isinstance(value, list):
        raise BoundaryError(f"{label} must be a list")
    return value


def require_string_list(value: Any, label: str) -> list[str]:
    """Return a YAML string list or raise a validation error."""
    values = require_list(value, label)
    if not all(isinstance(item, str) and item for item in values):
        raise BoundaryError(f"{label} entries must be non-empty strings")
    return values


def require_string(value: Any, label: str) -> str:
    """Return a non-empty string or raise a validation error."""
    if not isinstance(value, str) or not value:
        raise BoundaryError(f"{label} must be a non-empty string")
    return value


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a validation error."""
    if not isinstance(value, dict):
        raise BoundaryError(f"{label} must be a mapping")
    return value


def relative_path(raw_path: str, label: str) -> Path:
    """Parse a manifest path and reject absolute paths."""
    path = Path(raw_path)
    if path.is_absolute():
        raise BoundaryError(f"{label} must be repository-relative: {raw_path}")
    return path


def validate_openapi_evidence(root: Path, manifest: dict[str, Any]) -> list[str]:
    """Validate that OpenAPI contract evidence paths exist."""
    checked: list[str] = []
    contracts = require_string_list(
        manifest.get("openapi_contracts"),
        "openapi_contracts",
    )
    for raw_contract in contracts:
        contract = relative_path(raw_contract, "openapi_contracts entry")
        document = load_yaml(root / contract)
        metadata = document.get("x-kiva-contract")
        if not isinstance(metadata, dict):
            raise BoundaryError(f"{contract} missing x-kiva-contract mapping")
        evidence = require_list(
            metadata.get("evidence"),
            f"{contract}.x-kiva-contract.evidence",
        )
        for raw_path in evidence:
            if not isinstance(raw_path, str) or not raw_path:
                raise BoundaryError(
                    f"{contract}.x-kiva-contract.evidence entries must be non-empty strings"
                )
            path = root / raw_path
            if not path.exists():
                raise BoundaryError(f"{contract} evidence path is missing: {raw_path}")
            checked.append(raw_path)
    return checked


OPERATION_SPEC_REGEX = re.compile(
    r'operation_spec!\(\s*"(?P<id>[^"]+)",\s*'
    r"ApplicationOperationSurface::(?P<surface_variant>[A-Za-z]+),\s*"
    r"(?P<rust_workflow_kind>(?:AnalyticsWorkflowKind|PdsWorkflowKind|ConfidentialWorkflowKind)::[A-Za-z]+)"
    r"\.workflow_spec\(\),\s*"
    r'"(?P<rust_workflow_kind_literal>[^"]+)",\s*'
    r'"(?P<openapi_contract>[^"]+)",\s*'
    r'"(?P<operation_id>[^"]+)",\s*'
    r"&\[(?P<scopes>[^\]]*)\]\s*,?\s*\)",
    re.DOTALL,
)


def rust_operation_surface(surface_variant: str) -> str:
    """Return the catalog surface label for a Rust registry variant."""
    match surface_variant:
        case "Analytics":
            return "analytics"
        case "Pds":
            return "pds"
        case "Confidential":
            return "confidential"
        case _:
            raise BoundaryError(f"unknown Rust operation surface: {surface_variant}")


def parse_rust_operation_registry(path: Path) -> dict[str, dict[str, Any]]:
    """Parse the Rust application operation registry macro invocations."""
    text = path.read_text(encoding="utf-8")
    if "APPLICATION_OPERATION_SPECS" not in text:
        raise BoundaryError(f"{path} missing APPLICATION_OPERATION_SPECS")
    entries: dict[str, dict[str, Any]] = {}
    for match in OPERATION_SPEC_REGEX.finditer(text):
        scopes = re.findall(r'"([^"]+)"', match.group("scopes"))
        entry = {
            "id": match.group("id"),
            "surface": rust_operation_surface(match.group("surface_variant")),
            "rust_workflow_kind": match.group("rust_workflow_kind"),
            "openapi_contract": match.group("openapi_contract"),
            "operation_id": match.group("operation_id"),
            "required_scopes": scopes,
        }
        if entry["rust_workflow_kind"] != match.group("rust_workflow_kind_literal"):
            raise BoundaryError(
                f"{path} operation spec rust kind literal drift: {entry['id']}"
            )
        entry_id = entry["id"]
        if entry_id in entries:
            raise BoundaryError(f"{path} duplicate Rust operation spec id: {entry_id}")
        entries[entry_id] = entry
    if not entries:
        raise BoundaryError(f"{path} contains no operation_spec! entries")
    return entries


def catalog_entry_projection(entry: dict[str, Any], entry_id: str) -> dict[str, Any]:
    """Return the catalog fields mirrored by the Rust registry."""
    return {
        "id": entry_id,
        "surface": require_string(entry.get("surface"), f"{entry_id}.surface"),
        "workflow_label": require_string(
            entry.get("workflow_label"),
            f"{entry_id}.workflow_label",
        ),
        "rust_workflow_kind": require_string(
            entry.get("rust_workflow_kind"),
            f"{entry_id}.rust_workflow_kind",
        ),
        "openapi_contract": require_string(
            entry.get("openapi_contract"),
            f"{entry_id}.openapi_contract",
        ),
        "operation_id": require_string(entry.get("operation_id"), f"{entry_id}.operation_id"),
        "required_scopes": require_string_list(
            entry.get("required_scopes"),
            f"{entry_id}.required_scopes",
        ),
    }


def catalog_registry_projection(entry: dict[str, Any], entry_id: str) -> dict[str, Any]:
    """Return catalog fields that must be mirrored directly by the Rust registry."""
    projected = catalog_entry_projection(entry, entry_id)
    return {
        key: value
        for key, value in projected.items()
        if key != "workflow_label"
    }


def validate_operation_catalogs(root: Path, manifest: dict[str, Any]) -> list[str]:
    """Validate operation catalog links between OpenAPI, scopes, Rust labels, and evidence."""
    checked: list[str] = []
    catalogs = require_string_list(
        manifest.get("operation_catalogs", []),
        "operation_catalogs",
    )
    for raw_catalog in catalogs:
        catalog_path = relative_path(raw_catalog, "operation_catalogs entry")
        catalog = load_yaml(root / catalog_path)
        source_text = ""
        source_paths = require_string_list(
            catalog.get("source_paths", []),
            f"{raw_catalog}.source_paths",
        )
        if not source_paths:
            raise BoundaryError(f"{raw_catalog}.source_paths must not be empty")
        for raw_source in source_paths:
            source = root / relative_path(raw_source, f"{raw_catalog}.source_paths entry")
            if not source.exists():
                raise BoundaryError(f"{raw_catalog} source path is missing: {raw_source}")
            source_text += source.read_text(encoding="utf-8")
            checked.append(raw_source)
        raw_registry = require_string(
            catalog.get("rust_registry_path"),
            f"{raw_catalog}.rust_registry_path",
        )
        registry_path = root / relative_path(raw_registry, f"{raw_catalog}.rust_registry_path")
        if not registry_path.exists():
            raise BoundaryError(f"{raw_catalog} Rust registry path is missing: {raw_registry}")
        expected_registry = render_operation_catalog(catalog)
        actual_registry = registry_path.read_text(encoding="utf-8")
        if actual_registry != expected_registry:
            raise BoundaryError(
                f"{raw_catalog} generated Rust operation registry is stale: {raw_registry}"
            )
        raw_operation_ids = require_string(
            catalog.get("rust_operation_ids_path"),
            f"{raw_catalog}.rust_operation_ids_path",
        )
        operation_ids_path = root / relative_path(
            raw_operation_ids,
            f"{raw_catalog}.rust_operation_ids_path",
        )
        if not operation_ids_path.exists():
            raise BoundaryError(
                f"{raw_catalog} Rust operation ids path is missing: {raw_operation_ids}"
            )
        expected_operation_ids = render_operation_ids(catalog)
        actual_operation_ids = operation_ids_path.read_text(encoding="utf-8")
        if actual_operation_ids != expected_operation_ids:
            raise BoundaryError(
                f"{raw_catalog} generated Rust operation ids are stale: {raw_operation_ids}"
            )
        checked.append(raw_operation_ids)
        raw_metadata = require_string(
            catalog.get("rust_workflow_metadata_path"),
            f"{raw_catalog}.rust_workflow_metadata_path",
        )
        metadata_path = root / relative_path(
            raw_metadata,
            f"{raw_catalog}.rust_workflow_metadata_path",
        )
        if not metadata_path.exists():
            raise BoundaryError(
                f"{raw_catalog} Rust workflow metadata path is missing: {raw_metadata}"
            )
        expected_metadata = render_workflow_metadata(catalog)
        actual_metadata = metadata_path.read_text(encoding="utf-8")
        if actual_metadata != expected_metadata:
            raise BoundaryError(
                f"{raw_catalog} generated Rust workflow metadata is stale: {raw_metadata}"
            )
        checked.append(raw_metadata)
        rust_registry = parse_rust_operation_registry(registry_path)
        checked.append(raw_registry)

        openapi_cache: dict[str, dict[str, dict[str, Any]]] = {}
        entry_ids: set[str] = set()
        workflow_labels: set[str] = set()
        contract_operations: set[tuple[str, str]] = set()
        for index, entry in enumerate(require_list(catalog.get("entries"), f"{raw_catalog}.entries")):
            if not isinstance(entry, dict):
                raise BoundaryError(f"{raw_catalog}.entries[{index}] must be a mapping")
            entry_id = require_string(entry.get("id"), f"{raw_catalog}.entries[{index}].id")
            if entry_id in entry_ids:
                raise BoundaryError(f"{raw_catalog} duplicate entry id: {entry_id}")
            entry_ids.add(entry_id)
            projected_entry = catalog_entry_projection(entry, entry_id)
            registry_projection = catalog_registry_projection(entry, entry_id)
            registry_entry = rust_registry.get(entry_id)
            if registry_entry is None:
                raise BoundaryError(f"{entry_id} missing from Rust operation registry")
            if registry_entry != registry_projection:
                raise BoundaryError(
                    f"{entry_id} Rust operation registry drift: "
                    f"catalog={registry_projection} rust={registry_entry}"
                )
            surface = projected_entry["surface"]
            workflow_label = projected_entry["workflow_label"]
            if workflow_label in workflow_labels:
                raise BoundaryError(f"{raw_catalog} duplicate workflow label: {workflow_label}")
            workflow_labels.add(workflow_label)
            rust_workflow_kind = projected_entry["rust_workflow_kind"]
            openapi_contract = projected_entry["openapi_contract"]
            operation_id = projected_entry["operation_id"]
            operation_key = (openapi_contract, operation_id)
            if operation_key in contract_operations:
                raise BoundaryError(
                    f"{raw_catalog} duplicate operationId for {openapi_contract}: {operation_id}"
                )
            contract_operations.add(operation_key)
            required_scopes = projected_entry["required_scopes"]
            evidence_paths = require_string_list(
                entry.get("evidence_paths", []),
                f"{entry_id}.evidence_paths",
            )
            if workflow_label not in source_text:
                raise BoundaryError(f"{entry_id} workflow label missing from Rust sources")
            if rust_workflow_kind not in source_text:
                raise BoundaryError(f"{entry_id} workflow kind missing from Rust sources")

            if openapi_contract not in openapi_cache:
                contract_path = root / relative_path(openapi_contract, f"{entry_id}.openapi_contract")
                openapi_cache[openapi_contract] = collect_operations_by_id(
                    load_yaml(contract_path),
                    openapi_contract,
                    BoundaryError,
                )
            operation = openapi_cache[openapi_contract].get(operation_id)
            if operation is None:
                raise BoundaryError(f"{entry_id} operationId missing from {openapi_contract}: {operation_id}")
            operation_scopes = require_string_list(
                operation.get("x-kiva-required-scopes", []),
                f"{entry_id}.{operation_id}.x-kiva-required-scopes",
            )
            if sorted(operation_scopes) != sorted(required_scopes):
                raise BoundaryError(
                    f"{entry_id} scope drift: catalog={required_scopes} openapi={operation_scopes}"
                )
            for raw_evidence in evidence_paths:
                evidence = root / relative_path(raw_evidence, f"{entry_id}.evidence_paths entry")
                if not evidence.exists():
                    raise BoundaryError(f"{entry_id} evidence path is missing: {raw_evidence}")
                checked.append(raw_evidence)
            checked.append(f"{entry_id}:surface:{surface}")
            checked.append(f"{entry_id}:{operation_id}:{workflow_label}")
        missing_registry_entries = sorted(set(rust_registry).difference(entry_ids))
        if missing_registry_entries:
            raise BoundaryError(
                f"{raw_catalog} Rust registry has entries missing from catalog: "
                + ", ".join(missing_registry_entries)
            )
    return checked


def validate_source_groups(root: Path, manifest: dict[str, Any]) -> list[str]:
    """Validate that manifest source paths exist and contain required anchors."""
    checked: list[str] = []
    for index, group in enumerate(require_list(manifest.get("source_groups"), "source_groups")):
        if not isinstance(group, dict):
            raise BoundaryError(f"source_groups[{index}] must be a mapping")
        group_id = group.get("id", f"source_groups[{index}]")
        paths = require_string_list(group.get("paths"), f"{group_id}.paths")
        required_text = require_string_list(
            group.get("required_text", []),
            f"{group_id}.required_text",
        )
        group_text = ""
        for raw_path in paths:
            path = root / relative_path(raw_path, f"{group_id}.paths entry")
            if not path.exists():
                raise BoundaryError(f"{group_id} source path is missing: {raw_path}")
            if path.is_file():
                group_text += path.read_text(encoding="utf-8")
            checked.append(raw_path)
        for anchor in required_text:
            if anchor not in group_text:
                raise BoundaryError(f"{group_id} missing source anchor: {anchor}")
            checked.append(f"{group_id}:{anchor}")
    return checked


def validate_surface_source_model(root: Path) -> list[str]:
    """Validate split surface source ledgers and generated bundle freshness."""
    checked: list[str] = []
    policy_path = Path("spec/architecture/surface-projections.yaml")
    policy = load_surface_projection_policy(root / policy_path, BoundaryError)
    component_paths = surface_component_paths(policy, policy_path, BoundaryError)
    operation_ownership_path = component_paths["operation_ownership"]
    pds_family_path = component_paths["pds_resource_families"]
    surface_family_path = component_paths["surface_families"]
    bundle_path = component_paths["generated_bundle"]
    split_policy_sources = require_mapping(
        policy.get("split_policy_sources"),
        f"{policy_path}.split_policy_sources",
    )
    split_paths = {
        source_id: relative_path(
            require_string(
                split_policy_sources.get(source_id),
                f"{policy_path}.split_policy_sources.{source_id}",
            ),
            f"{policy_path}.split_policy_sources.{source_id}",
        )
        for source_id in ("model", "generation", "implementation")
    }
    for path in (
        policy_path,
        *split_paths.values(),
        operation_ownership_path,
        pds_family_path,
        surface_family_path,
        bundle_path,
    ):
        if not (root / path).exists():
            raise BoundaryError(f"surface source model path is missing: {path}")
        checked.append(path.as_posix())

    edit_policy = require_mapping(
        policy.get("edit_policy"),
        f"{policy_path}.edit_policy",
    )
    review_entrypoint = require_string(
        edit_policy.get("review_entrypoint"),
        f"{policy_path}.edit_policy.review_entrypoint",
    )
    if review_entrypoint != policy_path.as_posix():
        raise BoundaryError(
            f"{policy_path}.edit_policy.review_entrypoint must be {policy_path}"
        )
    operation_overlay_entrypoint = require_string(
        edit_policy.get("operation_overlay_entrypoint"),
        f"{policy_path}.edit_policy.operation_overlay_entrypoint",
    )
    if operation_overlay_entrypoint != operation_ownership_path.as_posix():
        raise BoundaryError(
            f"{policy_path}.edit_policy.operation_overlay_entrypoint must be {operation_ownership_path}"
        )
    human_sources = set(
        require_string_list(
            edit_policy.get("human_editable_sources"),
            f"{policy_path}.edit_policy.human_editable_sources",
        )
    )
    expected_human_sources = {
        policy_path.as_posix(),
        *(path.as_posix() for path in split_paths.values()),
        operation_ownership_path.as_posix(),
        pds_family_path.as_posix(),
        surface_family_path.as_posix(),
    }
    if human_sources != expected_human_sources:
        raise BoundaryError(
            f"{policy_path}.edit_policy.human_editable_sources drift; "
            f"expected={sorted(expected_human_sources)}, actual={sorted(human_sources)}"
        )
    checked.append("surface-source-model:edit-policy-human-sources")

    expected_generated_outputs = {bundle_path.as_posix()}
    expected_generated_outputs.update(
        target["target"]
        for target in surface_projection_targets(policy, policy_path, BoundaryError).values()
    )
    if "generated_outputs" in edit_policy:
        raise BoundaryError(
            f"{policy_path}.edit_policy.generated_outputs is derived; "
            "declare generated_bundle or surface_operation_algebra.projections instead"
        )
    for raw_path in sorted(expected_generated_outputs):
        if not (root / relative_path(raw_path, f"{policy_path}.derived_generated_outputs")).exists():
            raise BoundaryError(f"surface generated output path is missing: {raw_path}")
    checked.append("surface-source-model:derived-generated-outputs")

    operation_ownership = load_operation_ownership_source(root / operation_ownership_path, BoundaryError)
    pds_families = load_pds_resource_family_source(root / pds_family_path, BoundaryError)
    surface_families = load_surface_family_source(root / surface_family_path, BoundaryError)
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
            error_cls=BoundaryError,
        )
    )
    actual = (root / bundle_path).read_text(encoding="utf-8")
    if actual != projected:
        raise BoundaryError(
            f"{bundle_path} is not current with split surface source ledgers",
        )
    checked.append("surface-source-model:bundle-current")
    catalog = load_surface_catalog(root / bundle_path, BoundaryError)
    catalog_paths = surface_catalog_paths(catalog, bundle_path, BoundaryError, root=root)
    registry_path = catalog_paths["rust_surface_profile_registry"]
    expected_registry = render_surface_profile_registry(catalog, BoundaryError)
    actual_registry = (root / registry_path).read_text(encoding="utf-8")
    if actual_registry != expected_registry:
        raise BoundaryError(
            f"{registry_path} is not current with the generated surface catalog",
        )
    checked.append("surface-source-model:profile-registry-current")
    return checked


def validate_required_doc_anchors(root: Path, manifest: dict[str, Any]) -> list[str]:
    """Validate that design docs mention required current boundary anchors."""
    checked: list[str] = []
    for index, entry in enumerate(require_list(manifest.get("doc_anchors"), "doc_anchors")):
        if not isinstance(entry, dict):
            raise BoundaryError(f"doc_anchors[{index}] must be a mapping")
        raw_path = entry.get("path")
        if not isinstance(raw_path, str) or not raw_path:
            raise BoundaryError(f"doc_anchors[{index}].path must be a non-empty string")
        path = relative_path(raw_path, f"doc_anchors[{index}].path")
        anchors = require_string_list(entry.get("anchors"), f"{raw_path}.anchors")
        text = (root / path).read_text(encoding="utf-8")
        for anchor in anchors:
            if anchor not in text:
                raise BoundaryError(f"{path} missing architecture anchor: {anchor}")
            checked.append(f"{path}:{anchor}")
    return checked


def validate_forbidden_text(
    root: Path,
    manifest: dict[str, Any],
    manifest_paths: set[Path],
) -> list[str]:
    """Validate that retired boundary names are absent from docs and specs."""
    checked: list[str] = []
    retired_tokens = require_string_list(
        manifest.get("retired_tokens", []),
        "retired_tokens",
    )
    docs = (root / "docs").rglob("*.md")
    specs = (root / "spec").rglob("*.yaml")
    for path in sorted([*docs, *specs]):
        relative = path.relative_to(root).as_posix()
        if path.resolve() in manifest_paths:
            continue
        text = path.read_text(encoding="utf-8")
        for token in retired_tokens:
            if token in text:
                raise BoundaryError(f"{relative} contains retired boundary token: {token}")
        checked.append(relative)
    return checked


def iter_rust_files(root: Path, relative: str) -> list[Path]:
    """Return one Rust file or all Rust files under a directory."""
    path = root / relative
    if path.is_file():
        return [path]
    if path.is_dir():
        return sorted(path.rglob("*.rs"))
    raise BoundaryError(f"boundary source path is missing: {relative}")


def validate_server_patterns(root: Path, manifest: dict[str, Any]) -> list[str]:
    """Validate that HTTP surfaces no longer directly extract AppState."""
    checked: list[str] = []
    entries = require_list(
        manifest.get("forbidden_source_patterns", []),
        "forbidden_source_patterns",
    )
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            raise BoundaryError(f"forbidden_source_patterns[{index}] must be a mapping")
        paths = require_string_list(
            entry.get("paths"),
            f"forbidden_source_patterns[{index}].paths",
        )
        patterns = require_string_list(
            entry.get("patterns", []),
            f"forbidden_source_patterns[{index}].patterns",
        )
        regexes = require_string_list(
            entry.get("regexes", []),
            f"forbidden_source_patterns[{index}].regexes",
        )
        if not patterns and not regexes:
            raise BoundaryError(
                f"forbidden_source_patterns[{index}] must define patterns or regexes"
            )
        for relative in paths:
            for path in iter_rust_files(root, relative):
                text = path.read_text(encoding="utf-8")
                for forbidden in patterns:
                    if forbidden in text:
                        raise BoundaryError(
                            f"{path.relative_to(root)} contains forbidden pattern: {forbidden}"
                        )
                for forbidden_regex in regexes:
                    if re.search(forbidden_regex, text):
                        raise BoundaryError(
                            f"{path.relative_to(root)} contains forbidden regex: {forbidden_regex}"
                        )
                checked.append(path.relative_to(root).as_posix())
    return checked


def validate_dependency_rules(root: Path, manifest: dict[str, Any]) -> list[str]:
    """Validate manifest-defined dependency-direction rules."""
    checked: list[str] = []
    rules = require_list(manifest.get("dependency_rules", []), "dependency_rules")
    for index, rule in enumerate(rules):
        if not isinstance(rule, dict):
            raise BoundaryError(f"dependency_rules[{index}] must be a mapping")
        rule_id = rule.get("id", f"dependency_rules[{index}]")
        paths = require_string_list(rule.get("paths"), f"{rule_id}.paths")
        forbidden_patterns = require_string_list(
            rule.get("forbidden_patterns", []),
            f"{rule_id}.forbidden_patterns",
        )
        forbidden_regexes = require_string_list(
            rule.get("forbidden_regexes", []),
            f"{rule_id}.forbidden_regexes",
        )
        if not forbidden_patterns and not forbidden_regexes:
            raise BoundaryError(f"{rule_id} must define forbidden patterns or regexes")
        for relative in paths:
            for path in iter_rust_files(root, relative):
                text = path.read_text(encoding="utf-8")
                for forbidden in forbidden_patterns:
                    if forbidden in text:
                        raise BoundaryError(
                            f"{path.relative_to(root)} violates {rule_id}: {forbidden}"
                        )
                for forbidden_regex in forbidden_regexes:
                    if re.search(forbidden_regex, text):
                        raise BoundaryError(
                            f"{path.relative_to(root)} violates {rule_id}: {forbidden_regex}"
                        )
                checked.append(f"{rule_id}:{path.relative_to(root).as_posix()}")
    return checked


def dependency_names(cargo_toml: Path) -> set[str]:
    """Return dependency names declared in a Cargo manifest."""
    with cargo_toml.open("rb") as handle:
        document = tomllib.load(handle)
    names: set[str] = set()
    for section in ["dependencies", "dev-dependencies", "build-dependencies"]:
        value = document.get(section, {})
        if not isinstance(value, dict):
            raise BoundaryError(f"{cargo_toml}.{section} must be a table")
        names.update(value)
    return names


def validate_cargo_dependency_rules(root: Path, manifest: dict[str, Any]) -> list[str]:
    """Validate manifest-defined Cargo dependency rules."""
    checked: list[str] = []
    rules = require_list(manifest.get("cargo_dependency_rules", []), "cargo_dependency_rules")
    for index, rule in enumerate(rules):
        if not isinstance(rule, dict):
            raise BoundaryError(f"cargo_dependency_rules[{index}] must be a mapping")
        rule_id = rule.get("id", f"cargo_dependency_rules[{index}]")
        manifests = require_string_list(rule.get("manifests"), f"{rule_id}.manifests")
        forbidden_dependencies = set(
            require_string_list(
                rule.get("forbidden_dependencies", []),
                f"{rule_id}.forbidden_dependencies",
            )
        )
        required_dependencies = set(
            require_string_list(
                rule.get("required_dependencies", []),
                f"{rule_id}.required_dependencies",
            )
        )
        if not forbidden_dependencies and not required_dependencies:
            raise BoundaryError(
                f"{rule_id} must define forbidden_dependencies or required_dependencies"
            )
        for raw_manifest in manifests:
            cargo_toml = root / relative_path(raw_manifest, f"{rule_id}.manifests entry")
            if not cargo_toml.exists():
                raise BoundaryError(f"{rule_id} Cargo manifest is missing: {raw_manifest}")
            names = dependency_names(cargo_toml)
            forbidden = sorted(forbidden_dependencies.intersection(names))
            if forbidden:
                raise BoundaryError(
                    f"{raw_manifest} violates {rule_id}: forbidden dependencies "
                    + ", ".join(forbidden)
                )
            missing = sorted(required_dependencies.difference(names))
            if missing:
                raise BoundaryError(
                    f"{raw_manifest} violates {rule_id}: missing dependencies "
                    + ", ".join(missing)
                )
            checked.append(f"{rule_id}:{raw_manifest}")
    return checked


MODULE_DECLARATION_REGEX = re.compile(
    r"(?m)^\s*(?P<visibility>pub(?:\([^)]*\))?)?\s+mod\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*;"
    r"|^\s*mod\s+(?P<private_name>[A-Za-z_][A-Za-z0-9_]*)\s*;"
)


def module_declarations(path: Path) -> dict[str, str]:
    """Return module declarations and their visibility labels."""
    declarations: dict[str, str] = {}
    text = path.read_text(encoding="utf-8")
    for match in MODULE_DECLARATION_REGEX.finditer(text):
        private_name = match.group("private_name")
        if private_name is not None:
            declarations[private_name] = "private"
            continue
        name = match.group("name")
        visibility = match.group("visibility") or "private"
        declarations[name] = visibility
    return declarations


def validate_module_structure_rules(root: Path, manifest: dict[str, Any]) -> list[str]:
    """Validate module declarations without relying on arbitrary text anchors."""
    checked: list[str] = []
    rules = require_list(manifest.get("module_structure_rules", []), "module_structure_rules")
    for index, rule in enumerate(rules):
        if not isinstance(rule, dict):
            raise BoundaryError(f"module_structure_rules[{index}] must be a mapping")
        rule_id = rule.get("id", f"module_structure_rules[{index}]")
        raw_path = require_string(rule.get("path"), f"{rule_id}.path")
        path = root / relative_path(raw_path, f"{rule_id}.path")
        if not path.exists():
            raise BoundaryError(f"{rule_id} module owner is missing: {raw_path}")
        declarations = module_declarations(path)
        for module_index, module in enumerate(
            require_list(rule.get("modules"), f"{rule_id}.modules")
        ):
            if not isinstance(module, dict):
                raise BoundaryError(f"{rule_id}.modules[{module_index}] must be a mapping")
            name = require_string(module.get("name"), f"{rule_id}.modules[{module_index}].name")
            visibility = require_string(
                module.get("visibility", "private"),
                f"{rule_id}.{name}.visibility",
            )
            actual = declarations.get(name)
            if actual != visibility:
                raise BoundaryError(
                    f"{raw_path} violates {rule_id}: module {name} visibility "
                    f"must be {visibility}, got {actual}"
                )
            checked.append(f"{rule_id}:{name}:{visibility}")
    return checked


def validate_route_ownership_rules(root: Path, manifest: dict[str, Any]) -> list[str]:
    """Validate manifest-defined route ownership blocks."""
    checked: list[str] = []
    rules = require_list(
        manifest.get("route_ownership_rules", []),
        "route_ownership_rules",
    )
    for index, rule in enumerate(rules):
        if not isinstance(rule, dict):
            raise BoundaryError(f"route_ownership_rules[{index}] must be a mapping")
        rule_id = rule.get("id", f"route_ownership_rules[{index}]")
        raw_path = rule.get("path")
        if not isinstance(raw_path, str) or not raw_path:
            raise BoundaryError(f"{rule_id}.path must be a non-empty string")
        path = root / relative_path(raw_path, f"{rule_id}.path")
        if not path.exists():
            raise BoundaryError(f"{rule_id} route file is missing: {raw_path}")
        text = path.read_text(encoding="utf-8")
        owners = require_list(rule.get("owners"), f"{rule_id}.owners")
        for owner_index, owner in enumerate(owners):
            if not isinstance(owner, dict):
                raise BoundaryError(f"{rule_id}.owners[{owner_index}] must be a mapping")
            owner_id = owner.get("id", f"owners[{owner_index}]")
            start_anchor = owner.get("start_anchor")
            end_anchor = owner.get("end_anchor")
            if not isinstance(start_anchor, str) or not start_anchor:
                raise BoundaryError(f"{rule_id}.{owner_id}.start_anchor must be a string")
            if not isinstance(end_anchor, str) or not end_anchor:
                raise BoundaryError(f"{rule_id}.{owner_id}.end_anchor must be a string")
            start_index = text.find(start_anchor)
            if start_index < 0:
                raise BoundaryError(f"{rule_id}.{owner_id} missing start anchor: {start_anchor}")
            end_index = text.find(end_anchor, start_index)
            if end_index < 0:
                raise BoundaryError(f"{rule_id}.{owner_id} missing end anchor: {end_anchor}")
            block = text[start_index : end_index + len(end_anchor)]
            required_text = require_string_list(
                owner.get("required_text", []),
                f"{rule_id}.{owner_id}.required_text",
            )
            forbidden_text = require_string_list(
                owner.get("forbidden_text", []),
                f"{rule_id}.{owner_id}.forbidden_text",
            )
            if not required_text and not forbidden_text:
                raise BoundaryError(
                    f"{rule_id}.{owner_id} must define required_text or forbidden_text"
                )
            for required in required_text:
                if required not in block:
                    raise BoundaryError(
                        f"{rule_id}.{owner_id} route block missing: {required}"
                    )
                checked.append(f"{rule_id}:{owner_id}:{required}")
            for forbidden in forbidden_text:
                if forbidden in block:
                    raise BoundaryError(
                        f"{rule_id}.{owner_id} route block contains forbidden text: "
                        f"{forbidden}"
                    )
                checked.append(f"{rule_id}:{owner_id}:forbidden:{forbidden}")
    return checked


def require_positive_int(value: Any, label: str) -> int:
    """Return a positive integer or raise a validation error."""
    if not isinstance(value, int) or value < 1:
        raise BoundaryError(f"{label} must be a positive integer")
    return value


def validate_source_size_rules(root: Path, manifest: dict[str, Any]) -> list[str]:
    """Validate manifest-defined review-size rules for source owners."""
    checked: list[str] = []
    rules = require_list(manifest.get("source_size_rules", []), "source_size_rules")
    for index, rule in enumerate(rules):
        if not isinstance(rule, dict):
            raise BoundaryError(f"source_size_rules[{index}] must be a mapping")
        rule_id = rule.get("id", f"source_size_rules[{index}]")
        paths = require_string_list(rule.get("paths"), f"{rule_id}.paths")
        max_lines = require_positive_int(rule.get("max_lines"), f"{rule_id}.max_lines")
        for relative in paths:
            for path in iter_rust_files(root, relative):
                line_count = len(path.read_text(encoding="utf-8").splitlines())
                if line_count > max_lines:
                    raise BoundaryError(
                        f"{path.relative_to(root)} violates {rule_id}: "
                        f"{line_count} lines > {max_lines}"
                    )
                checked.append(f"{rule_id}:{path.relative_to(root).as_posix()}")
    return checked


WILDCARD_IMPORT_REGEX = re.compile(
    r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?(?:use|pub\s+use)\s+[^;\n]*::\*\s*;"
)


def validate_wildcard_import_rules(root: Path, manifest: dict[str, Any]) -> list[str]:
    """Validate manifest-defined wildcard import rules."""
    checked: list[str] = []
    rules = require_list(
        manifest.get("wildcard_import_rules", []),
        "wildcard_import_rules",
    )
    for index, rule in enumerate(rules):
        if not isinstance(rule, dict):
            raise BoundaryError(f"wildcard_import_rules[{index}] must be a mapping")
        rule_id = rule.get("id", f"wildcard_import_rules[{index}]")
        paths = require_string_list(rule.get("paths"), f"{rule_id}.paths")
        allowed_paths = set(
            require_string_list(rule.get("allowed_paths", []), f"{rule_id}.allowed_paths")
        )
        forbid_allow_attribute = bool(rule.get("forbid_allow_attribute", True))
        for relative in paths:
            for path in iter_rust_files(root, relative):
                relative_path = path.relative_to(root).as_posix()
                if relative_path in allowed_paths:
                    checked.append(f"{rule_id}:{relative_path}:allowed")
                    continue
                text = path.read_text(encoding="utf-8")
                if WILDCARD_IMPORT_REGEX.search(text):
                    raise BoundaryError(f"{relative_path} violates {rule_id}: wildcard import")
                if (
                    forbid_allow_attribute
                    and "#![allow(clippy::wildcard_imports)]" in text
                ):
                    raise BoundaryError(
                        f"{relative_path} violates {rule_id}: wildcard import allowance"
                    )
                checked.append(f"{rule_id}:{relative_path}")
    return checked


def validate_ci_check_manifest(root: Path) -> list[str]:
    """Validate CI check groups used by Nix gates."""
    manifest_path = Path("spec/ci/evidence-checks.yaml")
    manifest = load_evidence_check_manifest(root / manifest_path, BoundaryError)
    checked = [manifest_path.as_posix()]
    groups = validate_evidence_check_manifest_metadata(
        manifest,
        manifest_path.as_posix(),
        BoundaryError,
    )
    for group_id, group in groups.items():
        checked.extend(validate_evidence_check_scripts(root, group_id, group, BoundaryError))
    return checked


def validate(root: Path, manifest_path: Path) -> dict[str, Any]:
    """Validate all architecture boundary drift checks."""
    manifest, manifest_paths = load_architecture_manifest(root, manifest_path, BoundaryError)
    openapi_evidence = validate_openapi_evidence(root, manifest)
    operation_catalogs = validate_operation_catalogs(root, manifest)
    surface_source_model = validate_surface_source_model(root)
    ci_check_manifest = validate_ci_check_manifest(root)
    compatibility_ledgers = validate_compatibility_ledgers(root, manifest, BoundaryError)
    source_anchors = validate_source_groups(root, manifest)
    doc_anchors = validate_required_doc_anchors(root, manifest)
    scanned_docs = validate_forbidden_text(root, manifest, manifest_paths)
    server_sources = validate_server_patterns(root, manifest)
    dependency_rules = validate_dependency_rules(root, manifest)
    cargo_dependency_rules = validate_cargo_dependency_rules(root, manifest)
    module_structure_rules = validate_module_structure_rules(root, manifest)
    route_ownership_rules = validate_route_ownership_rules(root, manifest)
    source_size_rules = validate_source_size_rules(root, manifest)
    wildcard_import_rules = validate_wildcard_import_rules(root, manifest)
    return {
        "kind": "architecture-boundary-validation",
        "status": "passed",
        "manifest": manifest_path.relative_to(root).as_posix(),
        "openapi_evidence_paths": len(openapi_evidence),
        "operation_catalog_checks": len(operation_catalogs),
        "surface_source_model_checks": len(surface_source_model),
        "ci_check_manifest_checks": len(ci_check_manifest),
        "compatibility_ledger_checks": len(compatibility_ledgers),
        "source_anchors": len(source_anchors),
        "doc_anchors": len(doc_anchors),
        "dependency_rule_sources": len(dependency_rules),
        "cargo_dependency_rule_manifests": len(cargo_dependency_rules),
        "module_structure_checks": len(module_structure_rules),
        "route_ownership_checks": len(route_ownership_rules),
        "source_size_rule_sources": len(source_size_rules),
        "wildcard_import_rule_sources": len(wildcard_import_rules),
        "scanned_docs_and_specs": len(scanned_docs),
        "server_sources": len(server_sources),
    }


def main() -> None:
    """Run architecture boundary validation."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    root = args.root.resolve()
    manifest_path = (root / args.manifest).resolve()
    report = validate(root, manifest_path)
    if args.json_report is not None:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    print(
        "validated architecture boundaries: "
        f"{report['openapi_evidence_paths']} evidence paths, "
        f"{report['operation_catalog_checks']} operation-catalog checks, "
        f"{report['surface_source_model_checks']} surface-source checks, "
        f"{report['ci_check_manifest_checks']} ci-check checks, "
        f"{report['compatibility_ledger_checks']} compatibility-ledger checks, "
        f"{report['doc_anchors']} doc anchors, "
        f"{report['dependency_rule_sources']} dependency-rule sources, "
        f"{report['cargo_dependency_rule_manifests']} cargo manifests, "
        f"{report['module_structure_checks']} module checks, "
        f"{report['route_ownership_checks']} route-ownership checks, "
        f"{report['source_size_rule_sources']} source-size sources, "
        f"{report['wildcard_import_rule_sources']} wildcard-rule sources"
    )


if __name__ == "__main__":
    main()
