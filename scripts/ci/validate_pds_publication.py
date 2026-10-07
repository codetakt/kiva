#!/usr/bin/env python3
"""Validate Kiva PDS publication examples and generated artifacts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import yaml
from jsonschema import Draft202012Validator, FormatChecker, RefResolver

from export_pds_openapi_schemas import check_export
from validate_pds_deployment_templates import validate_catalog as validate_deployment_catalog
from validate_pds_openapi import FORBIDDEN_AUDIT_FIELDS, validate_contract


class PdsPublicationError(RuntimeError):
    """Raised when PDS publication artifacts are incomplete or inconsistent."""


def load_json(path: Path) -> Any:
    """Load JSON from disk."""
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML mapping from disk."""
    with path.open("r", encoding="utf-8") as handle:
        value = yaml.safe_load(handle)
    if not isinstance(value, dict):
        raise PdsPublicationError(f"{path} root must be a mapping")
    return value


def format_validation_error(error: Any) -> str:
    """Format a JSON Schema validation error with a stable JSON path."""
    path = "$"
    for part in error.absolute_path:
        if isinstance(part, int):
            path += f"[{part}]"
        else:
            path += f".{part}"
    return f"- {path}: {error.message}"


def json_contains_key(value: Any, key: str) -> bool:
    """Return whether a JSON value recursively contains a key."""
    if isinstance(value, dict):
        return key in value or any(json_contains_key(child, key) for child in value.values())
    if isinstance(value, list):
        return any(json_contains_key(child, key) for child in value)
    return False


def validate_example(
    document: dict[str, Any],
    examples_dir: Path,
    example: dict[str, Any],
) -> None:
    """Validate one indexed example against the PDS OpenAPI contract."""
    for key in ["id", "file", "schema", "kind", "operation"]:
        if not isinstance(example.get(key), str) or not example[key]:
            raise PdsPublicationError(f"example entry is missing non-empty {key}")
    path = examples_dir / example["file"]
    if not path.is_file():
        raise PdsPublicationError(f"missing PDS example file: {path}")
    instance = load_json(path)
    schema_name = example["schema"]
    schemas = document.get("components", {}).get("schemas", {})
    if schema_name not in schemas:
        raise PdsPublicationError(f"{path} references missing schema {schema_name}")

    schema = {"$ref": f"#/components/schemas/{schema_name}"}
    validator = Draft202012Validator(
        schema,
        resolver=RefResolver.from_schema(document),
        format_checker=FormatChecker(),
    )
    errors = sorted(validator.iter_errors(instance), key=lambda error: list(error.absolute_path))
    if errors:
        details = "\n".join(format_validation_error(error) for error in errors)
        raise PdsPublicationError(f"{path} violates {schema_name}:\n{details}")

    forbidden = sorted(field for field in FORBIDDEN_AUDIT_FIELDS if json_contains_key(instance, field))
    if forbidden:
        raise PdsPublicationError(f"{path} exposes forbidden raw payload fields: {', '.join(forbidden)}")


def validate_examples(contract: Path, examples_dir: Path) -> dict[str, Any]:
    """Validate all indexed PDS examples."""
    document = load_yaml(contract)
    validate_contract(document)
    index_path = examples_dir / "index.yaml"
    index = load_yaml(index_path)
    examples = index.get("examples")
    if not isinstance(examples, list) or not examples:
        raise PdsPublicationError(f"{index_path} must define a non-empty examples list")
    seen_ids: set[str] = set()
    seen_files: set[str] = set()
    for example in examples:
        if not isinstance(example, dict):
            raise PdsPublicationError(f"{index_path} example entries must be mappings")
        example_id = example.get("id")
        example_file = example.get("file")
        if example_id in seen_ids:
            raise PdsPublicationError(f"duplicate PDS example id: {example_id}")
        if example_file in seen_files:
            raise PdsPublicationError(f"duplicate PDS example file: {example_file}")
        seen_ids.add(example_id)
        seen_files.add(example_file)
        validate_example(document, examples_dir, example)
    return {
        "schema_version": 1,
        "kind": "pds-publication-examples-validation",
        "status": "passed",
        "contract": contract.as_posix(),
        "examples_dir": examples_dir.as_posix(),
        "example_count": len(examples),
        "examples": sorted(seen_ids),
    }


def source_milestones(source: Path) -> list[dict[str, str]]:
    """Extract milestone ids and titles from the integrated PDS ledger table."""
    milestones: list[dict[str, str]] = []
    for line in source.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped.startswith("| M"):
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if not cells:
            continue
        first = cells[0]
        parts = first.split(maxsplit=1)
        if len(parts) != 2:
            continue
        milestone_id, title = parts
        if not milestone_id.startswith("M") or not milestone_id[1:].isdigit():
            continue
        milestones.append({"id": milestone_id, "title": title})
    if not milestones:
        raise PdsPublicationError(f"{source} does not contain a milestone summary table")
    return milestones


def validate_milestone_mirror(mirror_path: Path) -> dict[str, Any]:
    """Validate that the publication milestone mirror tracks the source ledger."""
    mirror = load_yaml(mirror_path)
    source_value = mirror.get("source")
    if not isinstance(source_value, str) or not source_value:
        raise PdsPublicationError(f"{mirror_path} is missing source")
    source_path = Path(source_value)
    if not source_path.is_file():
        raise PdsPublicationError(f"{mirror_path} source does not exist: {source_path}")
    expected = source_milestones(source_path)
    milestones = mirror.get("milestones")
    if not isinstance(milestones, list) or not milestones:
        raise PdsPublicationError(f"{mirror_path} must define a non-empty milestones list")
    actual: list[dict[str, str]] = []
    seen_ids: set[str] = set()
    for milestone in milestones:
        if not isinstance(milestone, dict):
            raise PdsPublicationError(f"{mirror_path} milestone entries must be mappings")
        for key in ["id", "title", "status", "release_label", "claim_boundary"]:
            if not isinstance(milestone.get(key), str) or not milestone[key]:
                raise PdsPublicationError(f"{mirror_path} milestone entry is missing non-empty {key}")
        milestone_id = milestone["id"]
        if milestone_id in seen_ids:
            raise PdsPublicationError(f"{mirror_path} contains duplicate milestone id: {milestone_id}")
        seen_ids.add(milestone_id)
        actual.append({"id": milestone_id, "title": milestone["title"]})
    if actual != expected:
        raise PdsPublicationError(
            f"{mirror_path} does not match {source_path}: expected {expected}, got {actual}"
        )
    return {
        "source": source_path.as_posix(),
        "milestone_count": len(actual),
        "milestones": [milestone["id"] for milestone in actual],
    }


def main() -> int:
    """Run PDS publication validation."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--contract", type=Path, default=Path("spec/openapi/kiva-pds.yaml"))
    parser.add_argument("--examples", type=Path, default=Path("examples/pds"))
    parser.add_argument("--generated-schemas", type=Path, default=Path("generated/pds-openapi-schemas"))
    parser.add_argument(
        "--deployment-templates",
        type=Path,
        default=Path("spec/pds/deployment-templates.yaml"),
    )
    parser.add_argument(
        "--deployment-template-schema",
        type=Path,
        default=Path("spec/pds/schemas/pds-deployment-template-catalog.schema.json"),
    )
    parser.add_argument(
        "--milestone-mirror",
        type=Path,
        default=Path("spec/pds/publication-milestones.yaml"),
    )
    parser.add_argument("--json-report", type=Path)
    args = parser.parse_args()

    check_export(args.contract, args.generated_schemas)
    report = validate_examples(args.contract, args.examples)
    deployment_report = validate_deployment_catalog(
        load_yaml(args.deployment_templates),
        schema=args.deployment_template_schema,
        contract=args.contract,
    )
    report["deployment_templates"] = deployment_report["templates"]
    report["deployment_template_count"] = deployment_report["template_count"]
    report["deployment_kinds"] = deployment_report["deployment_kinds"]
    milestone_report = validate_milestone_mirror(args.milestone_mirror)
    report["milestone_count"] = milestone_report["milestone_count"]
    report["milestones"] = milestone_report["milestones"]
    if args.json_report is not None:
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    print(
        "validated PDS publication artifacts: "
        f"{report['example_count']} examples, "
        f"{report['deployment_template_count']} deployment templates, "
        f"{report['milestone_count']} milestones, "
        f"schemas={args.generated_schemas}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
