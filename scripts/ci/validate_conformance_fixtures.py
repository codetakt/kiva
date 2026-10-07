#!/usr/bin/env python3
"""Validate conformance fixture YAML documents against schemas and invariants."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import yaml
from jsonschema import Draft202012Validator


DEFAULT_AUTH_PROFILE = Path("spec/conformance/auth-profile.example.yaml")
DEFAULT_AUTH_SCHEMA = Path("spec/conformance/schemas/auth-profile.schema.json")
DEFAULT_ENDPOINT_PROFILE = Path("spec/conformance/endpoint-profile.example.yaml")
DEFAULT_ENDPOINT_SCHEMA = Path("spec/conformance/schemas/endpoint-profile.schema.json")
TOO_LARGE_PROFILE = "deterministic-too-large-threshold"


class FixtureError(RuntimeError):
    """Raised when a conformance fixture document is invalid."""


def load_json(path: Path) -> Any:
    """Load a JSON document."""
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def load_yaml(path: Path) -> Any:
    """Load a YAML document."""
    with path.open("r", encoding="utf-8") as handle:
        return yaml.safe_load(handle)


def validate_schema(schema_path: Path) -> dict[str, Any]:
    """Validate a JSON Schema and return it."""
    schema = load_json(schema_path)
    if not isinstance(schema, dict):
        raise FixtureError(f"{schema_path} root must be a JSON object")
    Draft202012Validator.check_schema(schema)
    return schema


def validate_instance(schema_path: Path, instance_path: Path, label: str) -> dict[str, Any]:
    """Validate one YAML fixture instance against one JSON Schema."""
    schema = validate_schema(schema_path)
    instance = load_yaml(instance_path)
    validator = Draft202012Validator(schema)
    errors = sorted(
        validator.iter_errors(instance),
        key=lambda error: list(error.absolute_path),
    )
    if errors:
        details = "\n".join(format_validation_error(error) for error in errors)
        raise FixtureError(f"{label} fixture violates {schema_path}:\n{details}")
    if not isinstance(instance, dict):
        raise FixtureError(f"{instance_path} root must be a mapping")
    print(f"validated {label} fixture: {instance_path}")
    return instance


def format_validation_error(error: Any) -> str:
    """Format a jsonschema validation error with a stable JSON path."""
    path = "$"
    for part in error.absolute_path:
        if isinstance(part, int):
            path += f"[{part}]"
        else:
            path += f".{part}"
    return f"- {path}: {error.message}"


def validate_auth_semantics(path: Path, document: dict[str, Any]) -> None:
    """Validate auth profile invariants that are awkward in JSON Schema."""
    profiles = require_mapping(document.get("profiles"), f"{path}.profiles")
    for profile_name, profile_value in profiles.items():
        profile = require_mapping(profile_value, f"{path}.profiles.{profile_name}")
        headers = profile.get("headers", {})
        if not isinstance(headers, dict):
            continue

        seen_headers: set[str] = set()
        if "bearer_token_env" in profile:
            seen_headers.add("authorization")
        for header_name in headers:
            lower_name = str(header_name).lower()
            if lower_name in seen_headers:
                raise FixtureError(
                    f"{path}.profiles.{profile_name} repeats header case-insensitively: {header_name}"
                )
            seen_headers.add(lower_name)


def validate_endpoint_semantics(path: Path, document: dict[str, Any]) -> None:
    """Validate endpoint profile invariants that are awkward in JSON Schema."""
    profiles = require_mapping(document.get("profiles"), f"{path}.profiles")
    too_large = profiles.get(TOO_LARGE_PROFILE)
    if too_large is None:
        return
    profile = require_mapping(too_large, f"{path}.profiles.{TOO_LARGE_PROFILE}")
    max_request_body_bytes = profile["max_request_body_bytes"]
    probe_body_bytes = profile["probe_body_bytes"]
    if probe_body_bytes <= max_request_body_bytes:
        raise FixtureError(
            f"{path}.profiles.{TOO_LARGE_PROFILE}.probe_body_bytes must exceed "
            "max_request_body_bytes"
        )


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    """Return a mapping or raise a fixture error."""
    if not isinstance(value, dict):
        raise FixtureError(f"{label} must be a mapping")
    return value


def main() -> int:
    """Run fixture validation."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--auth-profile", type=Path, default=DEFAULT_AUTH_PROFILE)
    parser.add_argument("--auth-schema", type=Path, default=DEFAULT_AUTH_SCHEMA)
    parser.add_argument("--endpoint-profile", type=Path, default=DEFAULT_ENDPOINT_PROFILE)
    parser.add_argument("--endpoint-schema", type=Path, default=DEFAULT_ENDPOINT_SCHEMA)
    parser.add_argument(
        "--check-schemas",
        action="store_true",
        help="validate fixture JSON Schemas before checking fixture instances",
    )
    args = parser.parse_args()

    if args.check_schemas:
        for schema_path in [args.auth_schema, args.endpoint_schema]:
            validate_schema(schema_path)
            print(f"validated JSON Schema: {schema_path}")

    auth_profile = validate_instance(args.auth_schema, args.auth_profile, "auth profile")
    validate_auth_semantics(args.auth_profile, auth_profile)

    endpoint_profile = validate_instance(
        args.endpoint_schema,
        args.endpoint_profile,
        "endpoint profile",
    )
    validate_endpoint_semantics(args.endpoint_profile, endpoint_profile)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
