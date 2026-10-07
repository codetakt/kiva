#!/usr/bin/env python3
"""Validate a formal verification evidence bundle as a whole."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker


DEFAULT_REPORT_SCHEMA = Path("spec/verification/schemas/formal-verification-report.schema.json")
DEFAULT_MANIFEST_SCHEMA = Path(
    "spec/verification/schemas/formal-verification-evidence-manifest.schema.json"
)


class FormalEvidenceBundleError(RuntimeError):
    """Raised when a formal verification evidence bundle is inconsistent."""


def load_json(path: Path) -> Any:
    """Load JSON from a path."""
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def validate_schema(schema_path: Path, instance: Any, label: str) -> None:
    """Validate an instance against one JSON Schema."""
    schema = load_json(schema_path)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(
        validator.iter_errors(instance),
        key=lambda error: list(error.absolute_path),
    )
    if errors:
        details = "\n".join(format_validation_error(error) for error in errors)
        raise FormalEvidenceBundleError(f"{label} violates {schema_path}:\n{details}")


def format_validation_error(error: Any) -> str:
    """Format a JSON Schema validation error with a stable JSON path."""
    path = "$"
    for part in error.absolute_path:
        if isinstance(part, int):
            path += f"[{part}]"
        else:
            path += f".{part}"
    return f"- {path}: {error.message}"


def safe_relative_path(value: str) -> Path:
    """Return a safe relative path from a manifest or checksum entry."""
    path = Path(value)
    if path.is_absolute() or ".." in path.parts:
        raise FormalEvidenceBundleError(f"unsafe bundle-relative path: {value}")
    return path


def normalize_checksum_path(value: str) -> str:
    """Normalize a sha256sum path field into manifest-relative form."""
    path = value.strip()
    if path.startswith("*"):
        path = path[1:]
    if path.startswith("./"):
        path = path[2:]
    safe_relative_path(path)
    return path


def bundle_files(root: Path) -> set[str]:
    """Return all bundle files except the checksum file itself."""
    files = set()
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        relative = path.relative_to(root).as_posix()
        if relative == "metadata/checksums.sha256":
            continue
        files.add(relative)
    return files


def read_checksums(path: Path) -> dict[str, str]:
    """Read a sha256sum file keyed by normalized bundle-relative path."""
    checksums: dict[str, str] = {}
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        parts = line.split(maxsplit=1)
        if len(parts) != 2:
            raise FormalEvidenceBundleError(f"{path}:{line_number}: invalid checksum line")
        digest, raw_relative = parts
        if len(digest) != 64 or any(char not in "0123456789abcdef" for char in digest):
            raise FormalEvidenceBundleError(f"{path}:{line_number}: invalid sha256 digest")
        relative = normalize_checksum_path(raw_relative)
        if relative in checksums:
            raise FormalEvidenceBundleError(f"{path}:{line_number}: duplicate checksum for {relative}")
        checksums[relative] = digest
    return checksums


def file_sha256(path: Path) -> str:
    """Calculate the SHA-256 digest for a file."""
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def validate_checksums(root: Path) -> None:
    """Validate checksum coverage and digest values."""
    checksum_path = root / "metadata/checksums.sha256"
    if not checksum_path.is_file():
        raise FormalEvidenceBundleError("missing metadata/checksums.sha256")
    actual_files = bundle_files(root)
    checksums = read_checksums(checksum_path)
    checksum_files = set(checksums)
    missing = sorted(actual_files - checksum_files)
    extra = sorted(checksum_files - actual_files)
    if missing:
        raise FormalEvidenceBundleError(f"missing checksums for: {', '.join(missing)}")
    if extra:
        raise FormalEvidenceBundleError(f"checksums reference missing files: {', '.join(extra)}")
    for relative, expected in sorted(checksums.items()):
        actual = file_sha256(root / relative)
        if actual != expected:
            raise FormalEvidenceBundleError(
                f"checksum mismatch for {relative}: expected {expected}, got {actual}"
            )


def validate_artifact_lists(root: Path, manifest: dict[str, Any]) -> None:
    """Validate manifest artifact lists against the bundle directory tree."""
    for group in ["reports", "logs", "metadata"]:
        listed = set(manifest["artifacts"][group])
        actual = {
            path.relative_to(root).as_posix()
            for path in (root / group).glob("*")
            if path.is_file()
        }
        if listed != actual:
            missing = sorted(actual - listed)
            extra = sorted(listed - actual)
            details = []
            if missing:
                details.append(f"missing from manifest: {', '.join(missing)}")
            if extra:
                details.append(f"listed but absent: {', '.join(extra)}")
            raise FormalEvidenceBundleError(f"artifact list mismatch for {group}: {'; '.join(details)}")
        for relative in listed:
            safe_relative_path(relative)


def validate_report(
    root: Path,
    report_schema: Path,
    tool_name: str,
    tool_evidence: dict[str, Any],
) -> None:
    """Validate one enabled tool report and compare it to manifest totals."""
    if not tool_evidence["enabled"]:
        expected = {
            "status": "skipped",
            "report_path": None,
            "log_path": None,
            "tool_version": None,
            "suite": None,
            "checks": 0,
            "passed": 0,
            "failed": 0,
            "timeout": 0,
        }
        for key, value in expected.items():
            if tool_evidence[key] != value:
                raise FormalEvidenceBundleError(f"skipped {tool_name} has invalid {key}")
        return

    for key in ["report_path", "log_path", "tool_version"]:
        if tool_evidence[key] is None:
            raise FormalEvidenceBundleError(f"enabled {tool_name} is missing {key}")

    report_path = safe_relative_path(tool_evidence["report_path"])
    log_path = safe_relative_path(tool_evidence["log_path"])
    if not (root / log_path).is_file():
        raise FormalEvidenceBundleError(f"missing {tool_name} log: {log_path}")
    report = load_json(root / report_path)
    validate_schema(report_schema, report, f"{tool_name} report")

    expected_kind = f"{tool_name}-verification"
    if report["kind"] != expected_kind:
        raise FormalEvidenceBundleError(
            f"{tool_name} report kind is {report['kind']}, expected {expected_kind}"
        )
    if report["tool_version"] != tool_evidence["tool_version"]:
        raise FormalEvidenceBundleError(f"{tool_name} tool_version does not match manifest")
    if report.get("suite") != tool_evidence["suite"]:
        raise FormalEvidenceBundleError(f"{tool_name} suite does not match manifest")

    checks = report["checks"]
    counts = {
        "checks": len(checks),
        "passed": sum(1 for check in checks if check["status"] == "passed"),
        "failed": sum(1 for check in checks if check["status"] == "failed"),
        "timeout": sum(1 for check in checks if check["status"] == "timeout"),
    }
    for key, value in counts.items():
        if tool_evidence[key] != value:
            raise FormalEvidenceBundleError(
                f"{tool_name} {key} is {tool_evidence[key]}, expected {value}"
            )
    expected_status = "failed" if counts["failed"] or counts["timeout"] else report["status"]
    if tool_evidence["status"] != expected_status:
        raise FormalEvidenceBundleError(
            f"{tool_name} status is {tool_evidence['status']}, expected {expected_status}"
        )


def validate_totals(manifest: dict[str, Any]) -> None:
    """Validate manifest totals against per-tool evidence."""
    tools = list(manifest["verification"].values())
    expected = {
        "enabled_tools": sum(1 for tool in tools if tool["enabled"]),
        "skipped_tools": sum(1 for tool in tools if not tool["enabled"]),
        "checks": sum(tool["checks"] for tool in tools),
        "passed": sum(tool["passed"] for tool in tools),
        "failed": sum(tool["failed"] for tool in tools),
        "timeout": sum(tool["timeout"] for tool in tools),
    }
    if manifest["totals"] != expected:
        raise FormalEvidenceBundleError(
            f"manifest totals are {manifest['totals']}, expected {expected}"
        )
    expected_status = "failed" if expected["failed"] or expected["timeout"] else "passed"
    if manifest["status"] != expected_status:
        raise FormalEvidenceBundleError(
            f"manifest status is {manifest['status']}, expected {expected_status}"
        )


def validate_bundle(root: Path, manifest_schema: Path, report_schema: Path) -> None:
    """Validate one formal evidence bundle."""
    manifest_path = root / "metadata/manifest.json"
    if not manifest_path.is_file():
        raise FormalEvidenceBundleError("missing metadata/manifest.json")
    manifest = load_json(manifest_path)
    if not isinstance(manifest, dict):
        raise FormalEvidenceBundleError("manifest root must be a JSON object")
    validate_schema(manifest_schema, manifest, "formal verification evidence manifest")
    validate_artifact_lists(root, manifest)
    for tool_name, tool_evidence in manifest["verification"].items():
        validate_report(root, report_schema, tool_name, tool_evidence)
    validate_totals(manifest)
    validate_checksums(root)


def main() -> int:
    """Run bundle validation."""
    parser = argparse.ArgumentParser()
    parser.add_argument("bundle", type=Path)
    parser.add_argument("--manifest-schema", type=Path, default=DEFAULT_MANIFEST_SCHEMA)
    parser.add_argument("--report-schema", type=Path, default=DEFAULT_REPORT_SCHEMA)
    args = parser.parse_args()

    root = args.bundle.resolve()
    if not root.is_dir():
        raise FormalEvidenceBundleError(f"bundle path is not a directory: {root}")
    validate_bundle(root, args.manifest_schema, args.report_schema)
    print(f"validated formal verification evidence bundle: {root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
