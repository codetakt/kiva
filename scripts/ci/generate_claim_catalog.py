#!/usr/bin/env python3
"""Generate review evidence for the Kiva implementation claim catalog."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import yaml

from kiva_ci.runtime import GeneratedArtifact, write_or_check_generated_artifacts


DEFAULT_CATALOG = Path("spec/architecture/claim-catalog.yaml")
DEFAULT_MATURITY = Path("spec/architecture/claim-maturity.yaml")
DEFAULT_MARKDOWN = Path("docs/architecture/generated/claim-catalog.md")
DEFAULT_EVIDENCE = Path("generated/claim-catalog.evidence.json")


class ClaimCatalogError(RuntimeError):
    """Raised when the claim catalog is inconsistent."""


def load_yaml(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise ClaimCatalogError(f"{path} root must be a mapping")
    return value


def require_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ClaimCatalogError(f"{label} must be a non-empty string")
    return value


def require_string_list(value: Any, label: str) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(item, str) and item for item in value):
        raise ClaimCatalogError(f"{label} must be a list of non-empty strings")
    return value


def require_maturity_levels(maturity_catalog: dict[str, Any]) -> dict[str, dict[str, Any]]:
    if maturity_catalog.get("schema_version") != 1:
        raise ClaimCatalogError("maturity schema_version must be 1")
    if maturity_catalog.get("kind") != "kiva-claim-maturity-catalog":
        raise ClaimCatalogError("maturity kind must be kiva-claim-maturity-catalog")
    levels = maturity_catalog.get("levels")
    if not isinstance(levels, list) or not levels:
        raise ClaimCatalogError("maturity levels must be a non-empty list")
    normalized: dict[str, dict[str, Any]] = {}
    for index, raw_level in enumerate(levels):
        if not isinstance(raw_level, dict):
            raise ClaimCatalogError(f"maturity.levels[{index}] must be a mapping")
        level_id = require_string(raw_level.get("id"), f"maturity.levels[{index}].id")
        if level_id in normalized:
            raise ClaimCatalogError(f"duplicate claim maturity level: {level_id}")
        normalized[level_id] = {
            "id": level_id,
            "description": require_string(
                raw_level.get("description"),
                f"maturity.levels[{index}].description",
            ),
        }
    return normalized


def require_entries(
    catalog: dict[str, Any],
    maturity_levels: dict[str, dict[str, Any]],
) -> list[dict[str, Any]]:
    if catalog.get("schema_version") != 1:
        raise ClaimCatalogError("schema_version must be 1")
    if catalog.get("kind") != "kiva-architecture-claim-catalog":
        raise ClaimCatalogError("kind must be kiva-architecture-claim-catalog")
    entries = catalog.get("entries")
    if not isinstance(entries, list) or not entries:
        raise ClaimCatalogError("entries must be a non-empty list")
    normalized: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, raw_entry in enumerate(entries):
        if not isinstance(raw_entry, dict):
            raise ClaimCatalogError(f"entries[{index}] must be a mapping")
        entry_id = require_string(raw_entry.get("id"), f"entries[{index}].id")
        if entry_id in seen:
            raise ClaimCatalogError(f"duplicate claim id: {entry_id}")
        seen.add(entry_id)
        maturity = require_string(raw_entry.get("claim_maturity"), f"{entry_id}.claim_maturity")
        if maturity not in maturity_levels:
            raise ClaimCatalogError(f"{entry_id}.claim_maturity is invalid: {maturity}")
        normalized.append(
            {
                "id": entry_id,
                "status": require_string(raw_entry.get("status"), f"{entry_id}.status"),
                "claim_maturity": maturity,
                "claim": require_string(raw_entry.get("claim"), f"{entry_id}.claim"),
                "boundary": require_string(raw_entry.get("boundary"), f"{entry_id}.boundary"),
                "evidence_paths": require_string_list(
                    raw_entry.get("evidence_paths"),
                    f"{entry_id}.evidence_paths",
                ),
            }
        )
    return normalized


def validate_paths(
    root: Path,
    catalog: dict[str, Any],
    maturity_path: Path,
    entries: list[dict[str, Any]],
) -> None:
    require_existing_path(root, maturity_path.as_posix())
    for raw_path in require_string_list(catalog.get("source_paths"), "source_paths"):
        require_existing_path(root, raw_path)
    for entry in entries:
        for raw_path in entry["evidence_paths"]:
            require_existing_path(root, raw_path)


def require_existing_path(root: Path, raw_path: str) -> None:
    path = Path(raw_path)
    if path.is_absolute():
        raise ClaimCatalogError(f"claim catalog path must be repository-relative: {raw_path}")
    if not (root / path).exists():
        raise ClaimCatalogError(f"claim catalog path is missing: {raw_path}")


def markdown_cell(value: object) -> str:
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_markdown(catalog: dict[str, Any], entries: list[dict[str, Any]]) -> str:
    rows = [
        "| Contract ID | Boundary | Referenced Artifacts |",
        "| --- | --- | ---: |",
    ]
    for entry in entries:
        rows.append(
            "| "
            + " | ".join(
                [
                    f"`{markdown_cell(entry['id'])}`",
                    f"`{markdown_cell(entry['boundary'])}`",
                    str(len(entry["evidence_paths"])),
                ]
            )
            + " |"
        )
    return f"""# Generated Claim Catalog

This file is generated from `spec/architecture/claim-catalog.yaml` by
`scripts/ci/generate_claim_catalog.py`.

This index lists contract boundaries and their referenced artifacts.
Execution results and assurance assessments are kept locally under `target/`.

{chr(10).join(rows)}
"""


def render_evidence(catalog: dict[str, Any], entries: list[dict[str, Any]]) -> str:
    evidence = {
        "schema_version": 1,
        "kind": "kiva-architecture-claim-catalog-evidence",
        "generated_from": "spec/architecture/claim-catalog.yaml",
        "status": catalog.get("status"),
        "claim_count": len(entries),
        "claims": entries,
    }
    return json.dumps(evidence, indent=2, sort_keys=True) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=DEFAULT_CATALOG)
    parser.add_argument("--maturity", type=Path, default=DEFAULT_MATURITY)
    parser.add_argument("--markdown-out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--evidence-out", type=Path, default=DEFAULT_EVIDENCE)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    root = Path(".").resolve()
    catalog = load_yaml(args.catalog)
    maturity_catalog = load_yaml(args.maturity)
    maturity_levels = require_maturity_levels(maturity_catalog)
    entries = require_entries(catalog, maturity_levels)
    validate_paths(root, catalog, args.maturity, entries)
    markdown = render_markdown(catalog, entries)
    evidence = render_evidence(catalog, entries)
    command_hint = "scripts/ci/generate_claim_catalog.py"
    try:
        write_or_check_generated_artifacts(
            [
                GeneratedArtifact(args.markdown_out, markdown),
                GeneratedArtifact(args.evidence_out, evidence),
            ],
            check=args.check,
            command_hint=command_hint,
            error_cls=ClaimCatalogError,
        )
    except ClaimCatalogError as error:
        raise SystemExit(str(error)) from error
    if args.check:
        print(f"validated generated claim catalog: {args.markdown_out}, {args.evidence_out}")
        return 0

    print(f"generated claim catalog: {args.markdown_out}, {args.evidence_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
