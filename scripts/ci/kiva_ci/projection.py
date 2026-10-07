"""Shared helpers for small generated projection scripts."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any, TypeVar

from .ledger import validate_json_schema
from .manifest import (
    load_json,
    load_yaml,
    require_list,
    require_mapping,
    require_string,
    validation_error,
)


ErrorT = TypeVar("ErrorT", bound=RuntimeError)
NORMAL_FORM_SYNC_POLICY = "normal-form-primary-with-projection-spec-compatibility-view"


@dataclass(frozen=True)
class ProjectionIdentity:
    """Stable identity for a ProjectionRuntime row."""

    source: str
    projection_id: str
    projection_kind: str

    @classmethod
    def from_row(cls, row: dict[str, Any], label: str, error_cls: type[ErrorT]) -> "ProjectionIdentity":
        """Parse and validate projection identity fields."""
        source, projection_id, projection_kind = require_projection_identity(row, label, error_cls)
        return cls(source=source, projection_id=projection_id, projection_kind=projection_kind)

    def as_tuple(self) -> tuple[str, str, str]:
        """Return the identity tuple used by compatibility validators."""
        return (self.source, self.projection_id, self.projection_kind)


@dataclass(frozen=True)
class ProjectionRuntimeRow:
    """Typed ProjectionRuntime row loaded from the primary normal form."""

    identity: ProjectionIdentity
    row: dict[str, Any]

    def compatibility_projection(self, error_cls: type[ErrorT]) -> dict[str, Any]:
        """Project this runtime row into a ProjectionSpec-compatible row."""
        projection: dict[str, Any] = {
            "projection_id": self.identity.projection_id,
            "projection_kind": self.identity.projection_kind,
        }
        add_optional_string(projection, "source", self.row.get("source"))
        add_optional_string(projection, "strategy", self.row.get("strategy"))
        add_optional_list(
            projection,
            "projection_sources",
            self.row.get("projection_sources"),
            f"{self.identity.source}.projection_sources",
            error_cls,
        )
        add_optional_list(
            projection,
            "projection_source_refs",
            self.row.get("projection_source_refs"),
            f"{self.identity.source}.projection_source_refs",
            error_cls,
        )
        add_optional_string(projection, "projection_generator", self.row.get("projection_generator"))
        add_optional_string(projection, "validator", self.row.get("validator"))
        return projection


@dataclass(frozen=True)
class ProjectionArtifactRow:
    """Typed ProjectionArtifact row loaded from the primary normal form."""

    identity: ProjectionIdentity
    row: dict[str, Any]

    def apply_to(self, projection: dict[str, Any], error_cls: type[ErrorT]) -> None:
        """Attach artifact fields to a ProjectionSpec-compatible row."""
        add_optional_list(
            projection,
            "generated_artifacts",
            self.row.get("generated_artifacts"),
            f"{self.identity.source}.generated_artifacts",
            error_cls,
        )
        add_optional_list(
            projection,
            "projection_targets",
            self.row.get("projection_targets"),
            f"{self.identity.source}.projection_targets",
            error_cls,
        )
        add_optional_list(
            projection,
            "projection_exports",
            self.row.get("projection_exports"),
            f"{self.identity.source}.projection_exports",
            error_cls,
        )
        add_optional_string(projection, "bundle", self.row.get("bundle"))
        add_optional_string(projection, "index", self.row.get("index"))


@dataclass(frozen=True)
class ProjectionCheckBindingRow:
    """Typed ProjectionCheckBinding row loaded from the primary normal form."""

    identity: ProjectionIdentity
    group: str
    check_id: str

    @classmethod
    def from_row(cls, row: dict[str, Any], label: str, error_cls: type[ErrorT]) -> "ProjectionCheckBindingRow":
        """Parse and validate one check binding row."""
        identity = ProjectionIdentity.from_row(row, label, error_cls)
        return cls(
            identity=identity,
            group=require_string(row.get("group"), f"{label}.group", error_cls),
            check_id=require_string(row.get("check_id"), f"{label}.check_id", error_cls),
        )


def require_envelope(
    document: dict[str, Any],
    *,
    path: Path,
    kind: str,
    error_cls: type[ErrorT],
    version_field: str = "schema_version",
    version: int = 1,
) -> None:
    """Validate the common schema-version/kind/status source envelope."""
    if document.get(version_field) != version:
        raise validation_error(error_cls, f"{path}.{version_field} must be {version}")
    if document.get("kind") != kind:
        raise validation_error(error_cls, f"{path}.kind must be {kind}")
    if document.get("status") != "active":
        raise validation_error(error_cls, f"{path}.status must be active")


def validate_against_schema(
    document: dict[str, Any],
    *,
    schema_path: Path,
    error_cls: type[ErrorT],
) -> None:
    """Validate one generated document against a JSON schema path."""
    validate_json_schema(
        document,
        load_json(schema_path, error_cls),
        label=schema_path.as_posix(),
        error_cls=error_cls,
    )


def optional_nonempty_list(value: Any, label: str, error_cls: type[ErrorT]) -> list[str]:
    """Return optional non-empty string entries as strings."""
    if value is None:
        return []
    values = require_list(value, label, error_cls)
    if not all(isinstance(item, str) and item for item in values):
        raise validation_error(error_cls, f"{label} entries must be non-empty strings")
    return [str(item) for item in values]


def projection_auxiliary_paths(
    projection_spec: dict[str, Any],
    projection_spec_path: Path,
    error_cls: type[ErrorT],
) -> dict[str, Path]:
    """Return normal-form projection ledger paths declared by ProjectionSpec."""
    auxiliary = require_mapping(
        projection_spec.get("projection_auxiliary_sources"),
        f"{projection_spec_path}.projection_auxiliary_sources",
        error_cls,
    )
    if auxiliary.get("synchronization_policy") != NORMAL_FORM_SYNC_POLICY:
        raise validation_error(
            error_cls,
            f"{projection_spec_path}.projection_auxiliary_sources.synchronization_policy must be {NORMAL_FORM_SYNC_POLICY}",
        )
    return {
        key: Path(require_string(auxiliary.get(key), f"{projection_spec_path}.projection_auxiliary_sources.{key}", error_cls))
        for key in ("artifacts", "runtime", "check_bindings")
    }


def require_projection_identity(
    row: dict[str, Any],
    label: str,
    error_cls: type[ErrorT],
) -> tuple[str, str, str]:
    """Return one normal-form projection row identity."""
    return (
        require_string(row.get("projection_source"), f"{label}.projection_source", error_cls),
        require_string(row.get("projection_id"), f"{label}.projection_id", error_cls),
        require_string(row.get("projection_kind"), f"{label}.projection_kind", error_cls),
    )


def rows_by_projection_source(
    rows: Any,
    label: str,
    error_cls: type[ErrorT],
) -> dict[str, dict[str, Any]]:
    """Return normal-form rows keyed by projection_source."""
    keyed: dict[str, dict[str, Any]] = {}
    for index, value in enumerate(require_list(rows, label, error_cls)):
        row = require_mapping(value, f"{label}[{index}]", error_cls)
        source, _, _ = require_projection_identity(row, f"{label}[{index}]", error_cls)
        if source in keyed:
            raise validation_error(error_cls, f"{label} duplicates projection_source: {source}")
        keyed[source] = row
    return keyed


def typed_rows_by_projection_source(
    rows: Any,
    label: str,
    error_cls: type[ErrorT],
    row_type: type[ProjectionRuntimeRow] | type[ProjectionArtifactRow],
) -> dict[str, ProjectionRuntimeRow] | dict[str, ProjectionArtifactRow]:
    """Return typed normal-form rows keyed by projection_source."""
    keyed: dict[str, ProjectionRuntimeRow] | dict[str, ProjectionArtifactRow] = {}
    for index, value in enumerate(require_list(rows, label, error_cls)):
        row = require_mapping(value, f"{label}[{index}]", error_cls)
        identity = ProjectionIdentity.from_row(row, f"{label}[{index}]", error_cls)
        if identity.source in keyed:
            raise validation_error(error_cls, f"{label} duplicates projection_source: {identity.source}")
        keyed[identity.source] = row_type(identity=identity, row=row)
    return keyed


def validate_normal_form_document(
    document: dict[str, Any],
    *,
    path: Path,
    kind: str,
    projection_spec_path: Path,
    error_cls: type[ErrorT],
) -> None:
    """Validate one projection normal-form ledger envelope."""
    require_envelope(document, path=path, kind=kind, error_cls=error_cls)
    if document.get("projection_spec_source") != projection_spec_path.as_posix():
        raise validation_error(
            error_cls,
            f"{path}.projection_spec_source must be {projection_spec_path.as_posix()}",
        )
    if document.get("synchronization_policy") != NORMAL_FORM_SYNC_POLICY:
        raise validation_error(
            error_cls,
            f"{path}.synchronization_policy must be {NORMAL_FORM_SYNC_POLICY}",
        )


def add_optional_list(
    projection: dict[str, Any],
    key: str,
    value: Any,
    label: str,
    error_cls: type[ErrorT],
) -> None:
    """Attach a list field only when non-empty."""
    values = optional_nonempty_list(value, label, error_cls)
    if values:
        projection[key] = values


def add_optional_string(projection: dict[str, Any], key: str, value: Any) -> None:
    """Attach a string field only when non-empty."""
    if isinstance(value, str) and value:
        projection[key] = value


def projection_specs_from_normal_form(
    root: Path,
    projection_spec: dict[str, Any],
    projection_spec_path: Path,
    error_cls: type[ErrorT],
) -> dict[str, dict[str, Any]]:
    """Return ProjectionSpec-compatible rows reconstructed from normal-form ledgers."""
    paths = projection_auxiliary_paths(projection_spec, projection_spec_path, error_cls)
    return projection_specs_from_normal_form_paths(
        root,
        projection_spec_path,
        paths["artifacts"],
        paths["runtime"],
        paths["check_bindings"],
        error_cls,
    )


def projection_specs_from_normal_form_paths(
    root: Path,
    projection_spec_path: Path,
    artifacts_path: Path,
    runtime_path: Path,
    check_bindings_path: Path,
    error_cls: type[ErrorT],
) -> dict[str, dict[str, Any]]:
    """Return ProjectionSpec-compatible rows from primary normal-form ledger paths."""
    artifacts_doc = load_yaml(root / artifacts_path, error_cls)
    runtime_doc = load_yaml(root / runtime_path, error_cls)
    checks_doc = load_yaml(root / check_bindings_path, error_cls)
    validate_normal_form_document(
        artifacts_doc,
        path=artifacts_path,
        kind="kiva-projection-artifacts-source",
        projection_spec_path=projection_spec_path,
        error_cls=error_cls,
    )
    validate_normal_form_document(
        runtime_doc,
        path=runtime_path,
        kind="kiva-projection-runtime-source",
        projection_spec_path=projection_spec_path,
        error_cls=error_cls,
    )
    validate_normal_form_document(
        checks_doc,
        path=check_bindings_path,
        kind="kiva-projection-check-bindings-source",
        projection_spec_path=projection_spec_path,
        error_cls=error_cls,
    )

    runtime_rows = typed_rows_by_projection_source(
        runtime_doc.get("runtime"),
        f"{runtime_path}.runtime",
        error_cls,
        ProjectionRuntimeRow,
    )
    artifact_rows = typed_rows_by_projection_source(
        artifacts_doc.get("artifacts"),
        f"{artifacts_path}.artifacts",
        error_cls,
        ProjectionArtifactRow,
    )
    projections: dict[str, dict[str, Any]] = {}
    for source_key, runtime in runtime_rows.items():
        projection = runtime.compatibility_projection(error_cls)
        artifact = artifact_rows.get(source_key)
        if artifact is not None:
            if artifact.identity.as_tuple() != runtime.identity.as_tuple():
                raise validation_error(error_cls, f"projection artifact identity drift for {source_key}")
            artifact.apply_to(projection, error_cls)
        projections[source_key] = projection

    check_rows = require_list(checks_doc.get("bindings"), f"{check_bindings_path}.bindings", error_cls)
    for index, value in enumerate(check_rows):
        check = require_mapping(value, f"{check_bindings_path}.bindings[{index}]", error_cls)
        binding = ProjectionCheckBindingRow.from_row(
            check,
            f"{check_bindings_path}.bindings[{index}]",
            error_cls,
        )
        source_key = binding.identity.source
        projection = projections.get(source_key)
        if projection is None:
            raise validation_error(error_cls, f"check binding references missing projection source: {source_key}")
        if projection["projection_id"] != binding.identity.projection_id or projection["projection_kind"] != binding.identity.projection_kind:
            raise validation_error(error_cls, f"check binding identity drift for {source_key}")
        projection.setdefault("projection_checks", []).append(
            {
                "group": binding.group,
                "check_id": binding.check_id,
            }
        )

    missing_artifacts = sorted(set(artifact_rows).difference(runtime_rows))
    if missing_artifacts:
        raise validation_error(error_cls, f"projection artifacts reference missing runtime rows: {missing_artifacts}")
    return projections


def normalized_projection_row(row: dict[str, Any], error_cls: type[ErrorT]) -> dict[str, Any]:
    """Return a ProjectionSpec row with empty compatibility fields removed."""
    normalized: dict[str, Any] = {}
    for key, value in row.items():
        if value in ("", [], None):
            continue
        if key == "projection_checks":
            normalized[key] = [
                {
                    "group": require_string(check.get("group"), f"{key}[].group", error_cls),
                    "check_id": require_string(check.get("check_id"), f"{key}[].check_id", error_cls),
                }
                for check in require_list(value, key, error_cls)
            ]
            continue
        normalized[key] = value
    return normalized


def validate_projection_spec_compatibility_view(
    projection_spec: dict[str, Any],
    projection_spec_path: Path,
    normal_form_rows: dict[str, dict[str, Any]],
    error_cls: type[ErrorT],
) -> None:
    """Validate ProjectionSpec inline rows remain a compatibility view."""
    inline = require_mapping(
        projection_spec.get("projections"),
        f"{projection_spec_path}.projections",
        error_cls,
    )
    inline_keys = set(str(key) for key in inline)
    normal_keys = set(normal_form_rows)
    if inline_keys != normal_keys:
        raise validation_error(
            error_cls,
            f"{projection_spec_path}.projections keys drift from normal-form ledgers: missing={sorted(normal_keys - inline_keys)}, extra={sorted(inline_keys - normal_keys)}",
        )
    for source_key, expected in normal_form_rows.items():
        actual = require_mapping(
            inline.get(source_key),
            f"{projection_spec_path}.projections.{source_key}",
            error_cls,
        )
        if normalized_projection_row(actual, error_cls) != normalized_projection_row(expected, error_cls):
            raise validation_error(
                error_cls,
                f"{projection_spec_path}.projections.{source_key} drifted from normal-form ledgers",
            )
