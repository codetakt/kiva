#!/usr/bin/env python3
"""Generate the compatibility-retirement review plane."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

from generate_compatibility_deletion_readiness import deletion_readiness_document
from kiva_ci.architecture_compiler import dump_yaml
from kiva_ci.manifest import (
    load_json,
    load_yaml,
    require_list,
    require_mapping,
    require_string,
)
from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_COMPATIBILITY_SURFACES = Path("spec/architecture/compatibility-surfaces.yaml")
DEFAULT_DTO_BOUNDARY = Path("spec/openapi/dto-boundary-ledger.yaml")
DEFAULT_DTO_RETIREMENT = Path("spec/openapi/semantic-compatibility-retirement.yaml")
DEFAULT_OUTPUT = Path("generated/architecture/compatibility-retirement-plane.yaml")
DEFAULT_MARKDOWN = Path("docs/architecture/generated/compatibility-retirement-plane.md")
DEFAULT_SCHEMA = Path("spec/architecture/schemas/compatibility-retirement-plane.schema.json")


class CompatibilityRetirementPlaneError(RuntimeError):
    """Raised when the compatibility-retirement plane cannot be generated."""


def compatibility_surface_rows(root: Path, surface_path: Path) -> list[dict[str, Any]]:
    """Return compatibility surface rows joined to executable deletion readiness."""
    source = load_yaml(root / surface_path, CompatibilityRetirementPlaneError)
    readiness = deletion_readiness_document(root, surface_path)
    readiness_by_id = {
        require_string(entry.get("id"), "readiness.entries[].id", CompatibilityRetirementPlaneError): entry
        for entry in require_list(readiness.get("entries"), "readiness.entries", CompatibilityRetirementPlaneError)
    }
    rows: list[dict[str, Any]] = []
    for index, value in enumerate(
        require_list(source.get("entries"), f"{surface_path}.entries", CompatibilityRetirementPlaneError)
    ):
        entry = require_mapping(value, f"{surface_path}.entries[{index}]", CompatibilityRetirementPlaneError)
        entry_id = require_string(entry.get("id"), f"{surface_path}.entries[{index}].id", CompatibilityRetirementPlaneError)
        readiness_entry = require_mapping(
            readiness_by_id.get(entry_id),
            f"readiness.entries[{entry_id}]",
            CompatibilityRetirementPlaneError,
        )
        readiness_state = require_string(
            readiness_entry.get("readiness"),
            f"readiness.entries[{entry_id}].readiness",
            CompatibilityRetirementPlaneError,
        )
        status = require_string(entry.get("status"), f"{entry_id}.status", CompatibilityRetirementPlaneError)
        next_status = entry.get("next_status")
        if readiness_state == "ready-to-delete" and status == "retained":
            decision = "owner-action-required-before-deletion"
        elif readiness_state == "removed":
            decision = "closed"
        else:
            decision = "blocked"
        rows.append(
            {
                "id": entry_id,
                "owner": require_string(entry.get("owner"), f"{entry_id}.owner", CompatibilityRetirementPlaneError),
                "status": status,
                "next_status": None if next_status is None else str(next_status),
                "readiness": readiness_state,
                "decision": decision,
                "migration_target": require_string(
                    entry.get("migration_target"),
                    f"{entry_id}.migration_target",
                    CompatibilityRetirementPlaneError,
                ),
                "retirement_milestone": require_string(
                    entry.get("retirement_milestone"),
                    f"{entry_id}.retirement_milestone",
                    CompatibilityRetirementPlaneError,
                ),
                "retirement_condition": require_string(
                    entry.get("retirement_condition"),
                    f"{entry_id}.retirement_condition",
                    CompatibilityRetirementPlaneError,
                ),
                "expansion_policy": require_string(
                    entry.get("expansion_policy"),
                    f"{entry_id}.expansion_policy",
                    CompatibilityRetirementPlaneError,
                ),
                "blockers": list(readiness_entry.get("blockers", [])),
                "gate_count": len(
                    require_list(
                        readiness_entry.get("gates"),
                        f"readiness.entries[{entry_id}].gates",
                        CompatibilityRetirementPlaneError,
                    )
                ),
            }
        )
    return rows


def dto_retirement_rows(
    root: Path,
    dto_boundary_path: Path,
    dto_retirement_path: Path,
) -> list[dict[str, Any]]:
    """Return DTO compatibility-retirement rows."""
    boundary = load_yaml(root / dto_boundary_path, CompatibilityRetirementPlaneError)
    retirement = load_yaml(root / dto_retirement_path, CompatibilityRetirementPlaneError)
    rows: list[dict[str, Any]] = []
    active_guards = require_mapping(
        boundary.get("active_guards"),
        f"{dto_boundary_path}.active_guards",
        CompatibilityRetirementPlaneError,
    )
    for guard_name, guard_path in active_guards.items():
        rows.append(
            {
                "id": f"dto-active-guard.{guard_name}",
                "kind": "active-guard",
                "status": "active",
                "path": str(guard_path),
                "decision": "retain-until-semantic-compatibility-retirement",
            }
        )
    retired_archives = require_mapping(
        retirement.get("retired_archives"),
        f"{dto_retirement_path}.retired_archives",
        CompatibilityRetirementPlaneError,
    )
    for archive_name, raw_archive in retired_archives.items():
        archive = require_mapping(
            raw_archive,
            f"{dto_retirement_path}.retired_archives.{archive_name}",
            CompatibilityRetirementPlaneError,
        )
        rows.append(
            {
                "id": f"dto-retired-archive.{archive_name}",
                "kind": "retired-archive",
                "status": require_string(
                    archive.get("status"),
                    f"{dto_retirement_path}.retired_archives.{archive_name}.status",
                    CompatibilityRetirementPlaneError,
                ),
                "path": require_string(
                    archive.get("path"),
                    f"{dto_retirement_path}.retired_archives.{archive_name}.path",
                    CompatibilityRetirementPlaneError,
                ),
                "decision": "closed",
            }
        )
    return rows


def retirement_plane_document(
    root: Path,
    compatibility_surfaces: Path,
    dto_boundary: Path,
    dto_retirement: Path,
    output: Path,
    markdown: Path,
) -> dict[str, Any]:
    """Return the compatibility-retirement plane."""
    surface_rows = compatibility_surface_rows(root, compatibility_surfaces)
    dto_rows = dto_retirement_rows(root, dto_boundary, dto_retirement)
    owner_action = [row for row in surface_rows if row["decision"] == "owner-action-required-before-deletion"]
    blocked = [row for row in surface_rows if row["decision"] == "blocked"]
    return {
        "schema_version": 1,
        "kind": "kiva-compatibility-retirement-plane",
        "status": "active",
        "description": (
            "Unified compatibility-retirement review plane joining removed or "
            "retained compatibility-surface history, DTO semantic-compatibility "
            "retirement, and executable deletion-readiness gates."
        ),
        "generated_from": [
            compatibility_surfaces.as_posix(),
            dto_boundary.as_posix(),
            dto_retirement.as_posix(),
            "generated/architecture/compatibility-deletion-readiness.yaml",
        ],
        "generated_artifacts": {
            "retirement_plane": output.as_posix(),
            "markdown": markdown.as_posix(),
        },
        "policy": {
            "ready_to_delete_is_not_delete_authorization": True,
            "retained_surfaces_require_owner_action_before_deletion": True,
            "dto_semantic_guards_remain_until_contract_and_semantic_maps_are_both_closed": True,
        },
        "summary": {
            "compatibility_surface_count": len(surface_rows),
            "owner_action_required_count": len(owner_action),
            "blocked_count": len(blocked),
            "dto_retirement_row_count": len(dto_rows),
        },
        "compatibility_surfaces": surface_rows,
        "dto_retirement": dto_rows,
    }


def validate_schema(document: dict[str, Any], schema_path: Path) -> None:
    """Validate the generated compatibility-retirement plane."""
    schema = load_json(schema_path, CompatibilityRetirementPlaneError)
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document),
        key=lambda error: list(error.path),
    )
    if errors:
        details = "\n".join(
            f"- {'/'.join(str(part) for part in error.path) or '<root>'}: {error.message}"
            for error in errors
        )
        raise CompatibilityRetirementPlaneError(
            f"CompatibilityRetirementPlane violates {schema_path}:\n{details}"
        )


def markdown_cell(value: object) -> str:
    """Render a Markdown table cell."""
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_markdown(document: dict[str, Any]) -> str:
    """Render the compatibility-retirement review table."""
    rows = [
        "| ID | Owner | Status | Readiness | Decision |",
        "| --- | --- | --- | --- | --- |",
    ]
    for row in require_list(
        document.get("compatibility_surfaces"),
        "compatibility_surfaces",
        CompatibilityRetirementPlaneError,
    ):
        entry = require_mapping(row, "compatibility_surfaces[]", CompatibilityRetirementPlaneError)
        rows.append(
            "| "
            + " | ".join(
                [
                    f"`{markdown_cell(entry['id'])}`",
                    f"`{markdown_cell(entry['owner'])}`",
                    f"`{markdown_cell(entry['status'])}`",
                    f"`{markdown_cell(entry['readiness'])}`",
                    f"`{markdown_cell(entry['decision'])}`",
                ]
            )
            + " |"
        )
    return f"""# Generated Compatibility Retirement Plane

This file is generated by `scripts/ci/generate_compatibility_retirement_plane.py`.

`ready-to-delete` is executable evidence that a retained compatibility surface
has no known internal call-site blockers. It is not deletion authorization.
Removed surfaces stay listed as retirement history.

{chr(10).join(rows)}
"""


def main() -> int:
    """Generate or validate the compatibility-retirement plane."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--compatibility-surfaces", type=Path, default=DEFAULT_COMPATIBILITY_SURFACES)
    parser.add_argument("--dto-boundary", type=Path, default=DEFAULT_DTO_BOUNDARY)
    parser.add_argument("--dto-retirement", type=Path, default=DEFAULT_DTO_RETIREMENT)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    document = retirement_plane_document(
        root,
        args.compatibility_surfaces,
        args.dto_boundary,
        args.dto_retirement,
        args.out,
        args.markdown_out,
    )
    validate_schema(document, root / args.schema)
    rendered_yaml = dump_yaml(document)
    rendered_markdown = render_markdown(document)
    try:
        write_or_check_generated_artifacts(
            [
                GeneratedArtifact(root / args.out, rendered_yaml),
                GeneratedArtifact(root / args.markdown_out, rendered_markdown),
            ],
            check=args.check,
            command_hint="scripts/ci/generate_compatibility_retirement_plane.py",
            error_cls=CompatibilityRetirementPlaneError,
        )
    except CompatibilityRetirementPlaneError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated compatibility-retirement plane: {args.out}, {args.markdown_out}")
        return 0
    print(f"generated compatibility-retirement plane: {args.out}, {args.markdown_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
