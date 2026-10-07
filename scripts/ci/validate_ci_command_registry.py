#!/usr/bin/env python3
"""Validate that `kiva-ci` is derived from the evidence-check registry."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from kiva_ci.cli import CommandFamily, load_commands
from kiva_ci.evidence import load_evidence_check_manifest
from kiva_ci.manifest import require_list, require_mapping, require_string, validation_error


DEFAULT_MANIFEST = Path("spec/ci/evidence-checks.yaml")
DEFAULT_ENTRYPOINT = Path("scripts/ci/kiva-ci")
DEFAULT_REGISTRY = Path("scripts/ci/kiva_ci/cli.py")


class CiCommandRegistryError(RuntimeError):
    """Raised when the typed CI command registry is inconsistent."""


def expected_command_family(check_kind: str) -> CommandFamily:
    """Return the command family expected for one evidence-check kind."""
    return "generate" if check_kind == "generator-check" else "validate"


def validate_ci_execution_model(
    manifest: dict[str, Any],
    manifest_path: Path,
    entrypoint: Path,
) -> None:
    """Validate the manifest declares `kiva-ci` as the primary execution model."""
    model = require_mapping(
        manifest.get("ci_execution_model"),
        f"{manifest_path}.ci_execution_model",
        CiCommandRegistryError,
    )
    expected = {
        "root_type": "RegistryDrivenCommand",
        "primary_entrypoint": entrypoint.as_posix(),
        "command_registry": DEFAULT_REGISTRY.as_posix(),
        "command_source": manifest_path.as_posix(),
    }
    for key, value in expected.items():
        actual = require_string(
            model.get(key),
            f"{manifest_path}.ci_execution_model.{key}",
            CiCommandRegistryError,
        )
        if actual != value:
            raise validation_error(
                CiCommandRegistryError,
                f"{manifest_path}.ci_execution_model.{key} must be {value}",
            )
    require_string(
        model.get("command_rule"),
        f"{manifest_path}.ci_execution_model.command_rule",
        CiCommandRegistryError,
    )


def validate_check_ci_command(
    check: dict[str, Any],
    check_id: str,
    expected_family: CommandFamily,
    entrypoint: Path,
    command_by_key: dict[tuple[CommandFamily, str], Any],
) -> None:
    """Validate a manifest check row points at its canonical `kiva-ci` command."""
    ci_command = require_mapping(
        check.get("ci_command"),
        f"{check_id}.ci_command",
        CiCommandRegistryError,
    )
    family = require_string(ci_command.get("family"), f"{check_id}.ci_command.family", CiCommandRegistryError)
    if family != expected_family:
        raise validation_error(
            CiCommandRegistryError,
            f"{check_id}.ci_command.family must be {expected_family}",
        )
    name = require_string(ci_command.get("name"), f"{check_id}.ci_command.name", CiCommandRegistryError)
    if name != check_id:
        raise validation_error(CiCommandRegistryError, f"{check_id}.ci_command.name must be {check_id}")
    declared_entrypoint = require_string(
        ci_command.get("entrypoint"),
        f"{check_id}.ci_command.entrypoint",
        CiCommandRegistryError,
    )
    if declared_entrypoint != entrypoint.as_posix():
        raise validation_error(
            CiCommandRegistryError,
            f"{check_id}.ci_command.entrypoint must be {entrypoint.as_posix()}",
        )
    command = command_by_key.get((expected_family, check_id))
    if command is None:
        raise validation_error(
            CiCommandRegistryError,
            f"{check_id}.ci_command references missing kiva-ci command {expected_family} {check_id}",
        )
    script = require_string(check.get("script"), f"{check_id}.script", CiCommandRegistryError)
    if command.script.as_posix() != script:
        raise validation_error(
            CiCommandRegistryError,
            f"{check_id}.ci_command script resolves to {command.script}, expected {script}",
        )


def expected_check_commands(manifest: dict[str, Any]) -> set[tuple[CommandFamily, str]]:
    """Return expected generate/validate command keys for manifest checks."""
    expected: set[tuple[CommandFamily, str]] = set()
    for group_index, raw_group in enumerate(
        require_list(manifest.get("groups"), "groups", CiCommandRegistryError)
    ):
        group = require_mapping(raw_group, f"groups[{group_index}]", CiCommandRegistryError)
        group_id = require_string(group.get("id"), f"groups[{group_index}].id", CiCommandRegistryError)
        for check_index, raw_check in enumerate(
            require_list(group.get("checks"), f"groups.{group_id}.checks", CiCommandRegistryError)
        ):
            check = require_mapping(
                raw_check,
                f"groups.{group_id}.checks[{check_index}]",
                CiCommandRegistryError,
            )
            check_id = require_string(check.get("id"), f"groups.{group_id}.checks[{check_index}].id", CiCommandRegistryError)
            check_kind = require_string(check.get("check_kind"), f"groups.{group_id}.{check_id}.check_kind", CiCommandRegistryError)
            expected.add((expected_command_family(check_kind), check_id))
    return expected


def validate_check_command_rows(
    manifest: dict[str, Any],
    entrypoint: Path,
    command_by_key: dict[tuple[CommandFamily, str], Any],
) -> int:
    """Validate every manifest check declares an executable `kiva-ci` command."""
    count = 0
    for group_index, raw_group in enumerate(
        require_list(manifest.get("groups"), "groups", CiCommandRegistryError)
    ):
        group = require_mapping(raw_group, f"groups[{group_index}]", CiCommandRegistryError)
        group_id = require_string(group.get("id"), f"groups[{group_index}].id", CiCommandRegistryError)
        for check_index, raw_check in enumerate(
            require_list(group.get("checks"), f"groups.{group_id}.checks", CiCommandRegistryError)
        ):
            check = require_mapping(
                raw_check,
                f"groups.{group_id}.checks[{check_index}]",
                CiCommandRegistryError,
            )
            check_id = require_string(check.get("id"), f"groups.{group_id}.checks[{check_index}].id", CiCommandRegistryError)
            check_kind = require_string(check.get("check_kind"), f"groups.{group_id}.{check_id}.check_kind", CiCommandRegistryError)
            validate_check_ci_command(
                check,
                check_id,
                expected_command_family(check_kind),
                entrypoint,
                command_by_key,
            )
            count += 1
    return count


def expected_run_commands(manifest: dict[str, Any]) -> set[tuple[CommandFamily, str]]:
    """Return expected run command keys for required evidence groups."""
    expected: set[tuple[CommandFamily, str]] = {
        ("run", "evidence-checks"),
        ("run", "design-review-suite"),
    }
    for index, raw_group in enumerate(
        require_list(manifest.get("required_groups"), "required_groups", CiCommandRegistryError)
    ):
        group = require_mapping(raw_group, f"required_groups[{index}]", CiCommandRegistryError)
        expected.add(
            (
                "run",
                require_string(group.get("id"), f"required_groups[{index}].id", CiCommandRegistryError),
            )
        )
    return expected


def validate_ci_command_registry(root: Path, manifest_path: Path, entrypoint: Path) -> tuple[int, int]:
    """Validate command registry coverage and script existence."""
    manifest = load_evidence_check_manifest(root / manifest_path, CiCommandRegistryError)
    commands = load_commands(root)
    command_by_key = {command.key: command for command in commands}
    validate_ci_execution_model(manifest, manifest_path, entrypoint)
    expected = expected_check_commands(manifest) | expected_run_commands(manifest)
    missing = sorted(expected.difference(command_by_key))
    if missing:
        raise validation_error(CiCommandRegistryError, f"missing kiva-ci commands: {missing}")

    if not (root / entrypoint).is_file():
        raise validation_error(CiCommandRegistryError, f"missing CI entrypoint: {entrypoint}")
    for command in commands:
        script = root / command.script
        if not script.is_file():
            raise validation_error(
                CiCommandRegistryError,
                f"registered {command.family} command {command.name} points to missing script: {command.script}",
            )
        if command.alias_for is None and not command.description:
            raise validation_error(
                CiCommandRegistryError,
                f"registered {command.family} command {command.name} must have a description",
            )
    check_count = validate_check_command_rows(manifest, entrypoint, command_by_key)
    return len(commands), max(len(expected), check_count)


def main() -> int:
    """Validate the typed CI command registry."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--entrypoint", type=Path, default=DEFAULT_ENTRYPOINT)
    args = parser.parse_args()

    command_count, expected_count = validate_ci_command_registry(
        args.root.resolve(),
        args.manifest,
        args.entrypoint,
    )
    print(f"validated CI command registry: {command_count} commands, {expected_count} manifest-backed keys")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
