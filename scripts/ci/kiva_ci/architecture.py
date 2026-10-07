"""Architecture-boundary manifest helpers for Kiva CI scripts."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any, TypeVar

from .compatibility import validate_compatibility_surface_ledgers
from .manifest import (
    generated_target_values,
    load_yaml,
    relative_path,
    require_bool,
    require_existing_path,
    require_list,
    require_mapping,
    require_string,
    require_string_list,
    validation_error,
)


ErrorT = TypeVar("ErrorT", bound=RuntimeError)

SOURCE_LAYER_FIELDS = (
    "canonical_sources",
    "projections",
    "evidence_indexes",
    "compatibility_ledgers",
)
SOURCE_KIND_BY_LAYER_FIELD = {
    "canonical_sources": "canonical",
    "projections": "projection",
    "evidence_indexes": "evidence-index",
    "compatibility_ledgers": "compatibility-ledger",
}
SOURCE_LAYERS = {"contract", "operation", "assurance", "governance"}
SOURCE_KINDS = set(SOURCE_KIND_BY_LAYER_FIELD.values())

ARTIFACT_RETENTION_CLASSES = {
    "committed-generated-code",
    "committed-generated-evidence",
    "committed-generated-ledger",
    "committed-review-doc",
    "committed-schema-artifact",
}
ARTIFACT_CLASSES = {
    "architecture-guard",
    "claim-catalog",
    "confidential-mechanism",
    "dto-boundary",
    "generated-catalog",
    "generated-evidence",
    "generated-ledger",
    "schema-artifact",
}


@dataclass(frozen=True)
class ArtifactCatalogEntry:
    """Typed row in a generated architecture artifact catalog."""

    path: str
    owner_source: str
    artifact_class: str
    retention_class: str
    committed: bool
    publication_surface: str

    @classmethod
    def from_mapping(
        cls,
        value: Any,
        label: str,
        error_cls: type[ErrorT],
    ) -> "ArtifactCatalogEntry":
        """Parse and validate one artifact-catalog row."""
        row = require_mapping(value, label, error_cls)
        artifact_class = require_string(row.get("artifact_class"), f"{label}.artifact_class", error_cls)
        if artifact_class not in ARTIFACT_CLASSES:
            raise validation_error(error_cls, f"{label}.artifact_class is invalid: {artifact_class}")
        retention_class = require_string(row.get("retention_class"), f"{label}.retention_class", error_cls)
        if retention_class not in ARTIFACT_RETENTION_CLASSES:
            raise validation_error(error_cls, f"{label}.retention_class is invalid: {retention_class}")
        return cls(
            path=require_string(row.get("path"), f"{label}.path", error_cls),
            owner_source=require_string(row.get("owner_source"), f"{label}.owner_source", error_cls),
            artifact_class=artifact_class,
            retention_class=retention_class,
            committed=require_bool(row.get("committed"), f"{label}.committed", error_cls),
            publication_surface=require_string(
                row.get("publication_surface"),
                f"{label}.publication_surface",
                error_cls,
            ),
        )

    def as_mapping(self) -> dict[str, Any]:
        """Return the stable YAML/JSON mapping form."""
        return {
            "path": self.path,
            "owner_source": self.owner_source,
            "artifact_class": self.artifact_class,
            "retention_class": self.retention_class,
            "committed": self.committed,
            "publication_surface": self.publication_surface,
        }


def artifact_catalog_entries(
    values: Any,
    label: str,
    error_cls: type[ErrorT],
) -> list[ArtifactCatalogEntry]:
    """Return typed artifact-catalog rows with duplicate-path checking."""
    entries: list[ArtifactCatalogEntry] = []
    seen_paths: set[str] = set()
    for index, value in enumerate(require_list(values, label, error_cls)):
        entry = ArtifactCatalogEntry.from_mapping(value, f"{label}[{index}]", error_cls)
        if entry.path in seen_paths:
            raise validation_error(error_cls, f"duplicate artifact catalog path: {entry.path}")
        seen_paths.add(entry.path)
        entries.append(entry)
    return entries


def validate_artifact_catalog_entries(
    root: Path,
    values: Any,
    label: str,
    error_cls: type[ErrorT],
) -> list[str]:
    """Validate one artifact catalog and return checked repository paths."""
    entries = artifact_catalog_entries(values, label, error_cls)
    checked: list[str] = []
    for entry in entries:
        require_existing_path(root, entry.path, f"{label}.path", error_cls)
        if entry.committed is not True:
            raise validation_error(error_cls, f"{entry.path} must be committed")
        checked.append(entry.path)
    return checked


def source_classification_from_policy(
    policy: dict[str, Any],
    sources: dict[str, Any],
    label: str,
    error_cls: type[ErrorT],
) -> dict[str, dict[str, str]]:
    """Return source-layer and source-kind metadata derived from policy groups."""
    layers = require_mapping(policy.get("source_layers"), f"{label}.source_layers", error_cls)
    classified: dict[str, dict[str, str]] = {}
    for layer_id, raw_layer in layers.items():
        if layer_id not in SOURCE_LAYERS:
            raise validation_error(error_cls, f"{label}.source_layers has invalid layer: {layer_id}")
        layer = require_mapping(raw_layer, f"{label}.source_layers.{layer_id}", error_cls)
        require_string(layer.get("description"), f"{label}.source_layers.{layer_id}.description", error_cls)
        for field in SOURCE_LAYER_FIELDS:
            expected_kind = SOURCE_KIND_BY_LAYER_FIELD[field]
            for value in require_string_list(
                layer.get(field),
                f"{label}.source_layers.{layer_id}.{field}",
                error_cls,
                allow_empty=True,
            ):
                if value not in sources:
                    continue
                previous = classified.setdefault(
                    value,
                    {"source_layer": layer_id, "source_kind": expected_kind},
                )
                if previous["source_layer"] != layer_id or previous["source_kind"] != expected_kind:
                    raise validation_error(
                        error_cls,
                        f"{label}.source_layers classifies {value} more than once",
                    )
    missing = sorted(set(sources).difference(classified))
    if missing:
        raise validation_error(
            error_cls,
            f"{label}.source_layers does not classify sources: {', '.join(missing)}",
        )
    return classified


def classified_source_map(
    sources: dict[str, Any],
    policy: dict[str, Any],
    label: str,
    error_cls: type[ErrorT],
) -> dict[str, dict[str, Any]]:
    """Return source entries with policy-derived classification fields injected."""
    classification = source_classification_from_policy(policy, sources, label, error_cls)
    result: dict[str, dict[str, Any]] = {}
    for source_key, raw_source in sources.items():
        source = require_mapping(raw_source, f"{label}.sources.{source_key}", error_cls)
        if "source_layer" in source or "source_kind" in source:
            raise validation_error(
                error_cls,
                f"{label}.sources.{source_key} must not declare source_layer/source_kind; "
                "classification is owned by architecture policy",
            )
        result[source_key] = {**classification[source_key], **source}
    return result


def artifact_class_policy_from_architecture_policy(
    policy: dict[str, Any],
    label: str,
    error_cls: type[ErrorT],
) -> dict[str, Any]:
    """Return the declarative artifact-class policy."""
    artifact_policy = require_mapping(
        policy.get("artifact_class_policy"),
        f"{label}.artifact_class_policy",
        error_cls,
    )
    default_artifact_class = require_string(
        artifact_policy.get("default_artifact_class"),
        f"{label}.artifact_class_policy.default_artifact_class",
        error_cls,
    )
    if default_artifact_class not in ARTIFACT_CLASSES:
        raise validation_error(
            error_cls,
            f"{label}.artifact_class_policy.default_artifact_class is invalid: {default_artifact_class}",
        )
    source_overrides = require_mapping(
        artifact_policy.get("source_overrides"),
        f"{label}.artifact_class_policy.source_overrides",
        error_cls,
    )
    projection_kind_artifact_classes = require_mapping(
        artifact_policy.get("projection_kind_artifact_classes"),
        f"{label}.artifact_class_policy.projection_kind_artifact_classes",
        error_cls,
    )
    review_policy = require_mapping(
        artifact_policy.get("review_policy"),
        f"{label}.artifact_class_policy.review_policy",
        error_cls,
    )
    for field in (
        "editable_sources",
        "generated_artifacts",
        "human_review_scope",
        "ci_review_scope",
        "release_evidence_scope",
        "rule",
    ):
        require_string(
            review_policy.get(field),
            f"{label}.artifact_class_policy.review_policy.{field}",
            error_cls,
        )
    for mapping_label, mapping in (
        ("source_overrides", source_overrides),
        ("projection_kind_artifact_classes", projection_kind_artifact_classes),
    ):
        for key, value in mapping.items():
            if not isinstance(key, str) or not key:
                raise validation_error(
                    error_cls,
                    f"{label}.artifact_class_policy.{mapping_label} keys must be non-empty strings",
                )
            artifact_class = require_string(
                value,
                f"{label}.artifact_class_policy.{mapping_label}.{key}",
                error_cls,
            )
            if artifact_class not in ARTIFACT_CLASSES:
                raise validation_error(
                    error_cls,
                    f"{label}.artifact_class_policy.{mapping_label}.{key} is invalid: {artifact_class}",
                )
    return {
        "default_artifact_class": default_artifact_class,
        "source_overrides": source_overrides,
        "projection_kind_artifact_classes": projection_kind_artifact_classes,
        "review_policy": review_policy,
    }


def artifact_class_for_source(
    source_key: str,
    source: dict[str, Any],
    artifact_policy: dict[str, Any],
) -> str:
    """Return the artifact class declared by architecture policy."""
    source_overrides = artifact_policy["source_overrides"]
    if source_key in source_overrides:
        return str(source_overrides[source_key])
    projection_kind = source.get("projection_kind")
    projection_kind_classes = artifact_policy["projection_kind_artifact_classes"]
    if projection_kind in projection_kind_classes:
        return str(projection_kind_classes[projection_kind])
    return str(artifact_policy["default_artifact_class"])


def retention_class_for_artifact(path: str, artifact_class: str) -> str:
    """Return the repository retention class for one generated artifact path."""
    if path.endswith(".rs"):
        return "committed-generated-code"
    if path.startswith("docs/"):
        return "committed-review-doc"
    if artifact_class == "schema-artifact" or "schemas" in path:
        return "committed-schema-artifact"
    if path.endswith(".evidence.json") or artifact_class.endswith("evidence"):
        return "committed-generated-evidence"
    return "committed-generated-ledger"


def publication_surface_for_artifact(path: str) -> str:
    """Return the broad publication surface for a generated artifact path."""
    if path.startswith("docs/"):
        return "documentation"
    if path.startswith("crates/"):
        return "rust"
    if path.startswith("spec/"):
        return "specification"
    if path.startswith("generated/"):
        return "evidence"
    return "repository"


def artifact_catalog_from_sources(
    sources: dict[str, Any],
    graph_generated_artifacts: list[str],
    architecture_policy: dict[str, Any],
    error_cls: type[ErrorT],
) -> list[dict[str, Any]]:
    """Return typed generated-artifact metadata derived from source targets."""
    rows: dict[str, dict[str, Any]] = {}
    artifact_policy = artifact_class_policy_from_architecture_policy(
        architecture_policy,
        "architecture_policy",
        error_cls,
    )

    def add_row(path: str, owner_source: str, artifact_class: str) -> None:
        retention_class = retention_class_for_artifact(path, artifact_class)
        if retention_class not in ARTIFACT_RETENTION_CLASSES:
            raise validation_error(error_cls, f"{path} has invalid retention class: {retention_class}")
        existing = rows.get(path)
        row = {
            "path": path,
            "owner_source": owner_source,
            "artifact_class": artifact_class,
            "retention_class": retention_class,
            "committed": True,
            "publication_surface": publication_surface_for_artifact(path),
        }
        if existing is not None and existing != row:
            comparable_existing = {
                key: value for key, value in existing.items() if key != "owner_source"
            }
            comparable_row = {
                key: value for key, value in row.items() if key != "owner_source"
            }
            if comparable_existing != comparable_row:
                raise validation_error(error_cls, f"generated artifact has conflicting owners: {path}")
            owners = sorted(set(existing["owner_source"].split("+")).union({owner_source}))
            existing["owner_source"] = "+".join(owners)
            return
        rows[path] = row

    for path in graph_generated_artifacts:
        add_row(path, "architecture_graph", "architecture-guard")

    for source_key, raw_source in sources.items():
        source = require_mapping(raw_source, f"sources.{source_key}", error_cls)
        artifact_class = artifact_class_for_source(source_key, source, artifact_policy)
        for path in generated_target_values(source_key, source, error_cls):
            add_row(path, source_key, artifact_class)

    return [rows[path] for path in sorted(rows)]


def merge_manifest_section(
    base: dict[str, Any],
    overlay: dict[str, Any],
    source: Path,
    error_cls: type[ErrorT],
) -> None:
    """Merge one included architecture manifest into the base manifest."""
    for key, value in overlay.items():
        if key in {"description", "includes"}:
            continue
        if key not in base:
            base[key] = value
            continue
        current = base[key]
        if isinstance(current, list) and isinstance(value, list):
            current.extend(value)
            continue
        if isinstance(current, dict) and isinstance(value, dict):
            current.update(value)
            continue
        if current != value:
            raise validation_error(
                error_cls,
                f"{source} cannot override architecture manifest key: {key}",
            )


def load_architecture_manifest(
    root: Path,
    manifest_path: Path,
    error_cls: type[ErrorT],
    seen: set[Path] | None = None,
) -> tuple[dict[str, Any], set[Path]]:
    """Load an architecture manifest and any repo-relative included fragments."""
    resolved = manifest_path.resolve()
    if seen is None:
        seen = set()
    if resolved in seen:
        raise validation_error(error_cls, f"architecture manifest include cycle: {resolved}")
    seen.add(resolved)

    document = load_yaml(resolved, error_cls)
    merged = {key: value for key, value in document.items() if key != "includes"}
    manifest_paths = {resolved}
    includes = require_string_list(
        document.get("includes", []),
        f"{resolved}.includes",
        error_cls,
        allow_empty=True,
    )
    for raw_include in includes:
        include_path = root / relative_path(raw_include, f"{resolved}.includes entry", error_cls)
        included, included_paths = load_architecture_manifest(root, include_path, error_cls, seen)
        merge_manifest_section(merged, included, include_path, error_cls)
        manifest_paths.update(included_paths)
    return merged, manifest_paths
