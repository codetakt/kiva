"""Shared runtime helpers for Kiva generator and validator entrypoints."""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .manifest import require_mapping, require_string, validation_error


@dataclass(frozen=True)
class GeneratedArtifact:
    """One generated artifact rendered by a CI generator."""

    path: Path
    text: str


@dataclass(frozen=True)
class LedgerRuntime:
    """Loaded source-ledger context for one generator or validator."""

    root: Path
    path: Path
    document: dict[str, Any]

    @classmethod
    def load(
        cls,
        root: Path,
        path: Path,
        error_cls: type[RuntimeError],
    ) -> "LedgerRuntime":
        """Load one YAML source ledger from a repository root."""
        import yaml

        full_path = root / path
        if not full_path.exists():
            raise error_cls(f"{path} does not exist")
        with full_path.open("r", encoding="utf-8") as handle:
            value = yaml.safe_load(handle)
        if not isinstance(value, dict):
            raise error_cls(f"{path} must be a mapping")
        return cls(root=root, path=path, document=value)

    def require_envelope(self, *, kind: str, error_cls: type[RuntimeError]) -> None:
        """Validate the common schema_version/kind/status envelope."""
        if self.document.get("schema_version") != 1:
            raise error_cls(f"{self.path}.schema_version must be 1")
        if self.document.get("kind") != kind:
            raise error_cls(f"{self.path}.kind must be {kind}")
        if self.document.get("status") != "active":
            raise error_cls(f"{self.path}.status must be active")


@dataclass(frozen=True)
class ProjectionRuntime:
    """Rendered projection context with a stable command hint."""

    command_hint: str
    artifacts: tuple[GeneratedArtifact, ...]

    def write_or_check(self, *, check: bool, error_cls: type[RuntimeError]) -> None:
        """Write or drift-check all projection artifacts."""
        write_or_check_generated_artifacts(
            list(self.artifacts),
            check=check,
            command_hint=self.command_hint,
            error_cls=error_cls,
        )


@dataclass(frozen=True)
class ValidatorRuntime:
    """Small report accumulator for validators that check repository paths."""

    checked_paths: tuple[str, ...] = ()

    def with_checked(self, *paths: str) -> "ValidatorRuntime":
        """Return a validator runtime with additional checked paths."""
        return ValidatorRuntime(checked_paths=(*self.checked_paths, *paths))

    def report(self, *, kind: str, status: str = "passed") -> dict[str, Any]:
        """Return a stable validation report mapping."""
        return {
            "kind": kind,
            "status": status,
            "checked_paths": list(self.checked_paths),
        }


@dataclass(frozen=True)
class RegistryCommand:
    """One registry-driven CI command invocation."""

    entrypoint: Path
    family: str
    name: str

    @classmethod
    def from_check(
        cls,
        root: Path,
        check: dict[str, Any],
        check_id: str,
        error_cls: type[RuntimeError],
    ) -> "RegistryCommand":
        """Parse the ci_command block from one evidence-check row."""
        ci_command = require_mapping(check.get("ci_command"), f"{check_id}.ci_command", error_cls)
        entrypoint = root / require_string(
            ci_command.get("entrypoint"),
            f"{check_id}.ci_command.entrypoint",
            error_cls,
        )
        family = require_string(
            ci_command.get("family"),
            f"{check_id}.ci_command.family",
            error_cls,
        )
        name = require_string(
            ci_command.get("name"),
            f"{check_id}.ci_command.name",
            error_cls,
        )
        if family not in {"generate", "validate"}:
            raise validation_error(error_cls, f"{check_id}.ci_command.family must be generate or validate")
        if name != check_id:
            raise validation_error(error_cls, f"{check_id}.ci_command.name must match check id")
        if not entrypoint.exists():
            raise validation_error(error_cls, f"{check_id}.ci_command.entrypoint does not exist: {entrypoint}")
        return cls(entrypoint=entrypoint, family=family, name=name)

    def argv(self, forwarded_runtime_args: list[str]) -> list[str]:
        """Return the argv used to invoke this registry command."""
        command = [sys.executable, self.entrypoint.as_posix(), self.family, self.name]
        if self.family == "generate":
            command.append("--check")
        if forwarded_runtime_args:
            command.extend(["--", *forwarded_runtime_args])
        return command

    def report(self, root: Path) -> dict[str, str]:
        """Return the report form embedded in evidence-check run output."""
        return {
            "entrypoint": self.entrypoint.relative_to(root).as_posix(),
            "family": self.family,
            "name": self.name,
        }


def read_required_text(path: Path, error_cls: type[RuntimeError]) -> str:
    """Read a generated artifact that must already exist."""
    if not path.exists():
        raise error_cls(f"{path} is missing; run the owning generator")
    return path.read_text(encoding="utf-8")


def write_generated_artifact(artifact: GeneratedArtifact) -> None:
    """Write one generated artifact, creating parent directories."""
    artifact.path.parent.mkdir(parents=True, exist_ok=True)
    artifact.path.write_text(artifact.text, encoding="utf-8")


def check_generated_artifact(
    artifact: GeneratedArtifact,
    command_hint: str,
    error_cls: type[RuntimeError],
) -> None:
    """Validate one generated artifact is current."""
    if read_required_text(artifact.path, error_cls) != artifact.text:
        raise error_cls(f"{artifact.path} is not current; run {command_hint}")


def write_or_check_generated_artifacts(
    artifacts: list[GeneratedArtifact],
    *,
    check: bool,
    command_hint: str,
    error_cls: type[RuntimeError],
) -> None:
    """Write generated artifacts or validate them in check mode."""
    for artifact in artifacts:
        if check:
            check_generated_artifact(artifact, command_hint, error_cls)
        else:
            write_generated_artifact(artifact)
