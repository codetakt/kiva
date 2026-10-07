#!/usr/bin/env python3
"""Validate a Kiva PDS publication evidence bundle."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker


DEFAULT_MANIFEST_SCHEMA = Path("spec/pds/schemas/pds-evidence-manifest.schema.json")


class PdsEvidenceBundleError(RuntimeError):
    """Raised when a PDS evidence bundle is inconsistent."""


def load_json(path: Path) -> Any:
    """Load JSON from disk."""
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def format_validation_error(error: Any) -> str:
    """Format a JSON Schema validation error with a stable JSON path."""
    path = "$"
    for part in error.absolute_path:
        if isinstance(part, int):
            path += f"[{part}]"
        else:
            path += f".{part}"
    return f"- {path}: {error.message}"


def validate_schema(schema_path: Path, instance: Any, label: str) -> None:
    """Validate a JSON instance against a schema."""
    schema = load_json(schema_path)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(validator.iter_errors(instance), key=lambda error: list(error.absolute_path))
    if errors:
        details = "\n".join(format_validation_error(error) for error in errors)
        raise PdsEvidenceBundleError(f"{label} violates {schema_path}:\n{details}")


def safe_relative_path(value: str) -> Path:
    """Return a safe bundle-relative path."""
    path = Path(value)
    if path.is_absolute() or ".." in path.parts:
        raise PdsEvidenceBundleError(f"unsafe bundle-relative path: {value}")
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


def normalize_checksum_path(value: str) -> str:
    """Normalize sha256sum path fields."""
    path = value.strip()
    if path.startswith("*"):
        path = path[1:]
    if path.startswith("./"):
        path = path[2:]
    safe_relative_path(path)
    return path


def read_checksums(path: Path) -> dict[str, str]:
    """Read a sha256sum file."""
    checksums: dict[str, str] = {}
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        parts = line.split(maxsplit=1)
        if len(parts) != 2:
            raise PdsEvidenceBundleError(f"{path}:{line_number}: invalid checksum line")
        digest, raw_relative = parts
        if len(digest) != 64 or any(char not in "0123456789abcdef" for char in digest):
            raise PdsEvidenceBundleError(f"{path}:{line_number}: invalid sha256 digest")
        relative = normalize_checksum_path(raw_relative)
        if relative in checksums:
            raise PdsEvidenceBundleError(f"{path}:{line_number}: duplicate checksum for {relative}")
        checksums[relative] = digest
    return checksums


def file_sha256(path: Path) -> str:
    """Calculate SHA-256 for a file."""
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def validate_checksums(root: Path) -> None:
    """Validate checksum coverage and digest values."""
    checksum_path = root / "metadata/checksums.sha256"
    if not checksum_path.is_file():
        raise PdsEvidenceBundleError("missing metadata/checksums.sha256")
    actual_files = bundle_files(root)
    checksums = read_checksums(checksum_path)
    missing = sorted(actual_files - set(checksums))
    extra = sorted(set(checksums) - actual_files)
    if missing:
        raise PdsEvidenceBundleError(f"missing checksums for: {', '.join(missing)}")
    if extra:
        raise PdsEvidenceBundleError(f"checksums reference missing files: {', '.join(extra)}")
    for relative, expected in sorted(checksums.items()):
        actual = file_sha256(root / relative)
        if actual != expected:
            raise PdsEvidenceBundleError(
                f"checksum mismatch for {relative}: expected {expected}, got {actual}"
            )


def validate_artifact_lists(
    root: Path,
    manifest: dict[str, Any],
    *,
    skip_checksums: bool = False,
) -> None:
    """Validate manifest artifact lists against bundle directories."""
    for group in ["reports", "logs", "examples", "schemas", "docs", "metadata"]:
        listed = set(manifest["artifacts"][group])
        actual = {
            path.relative_to(root).as_posix()
            for path in (root / group).glob("*")
            if path.is_file()
        }
        if skip_checksums and group == "metadata":
            listed.discard("metadata/checksums.sha256")
        if listed != actual:
            missing = sorted(actual - listed)
            extra = sorted(listed - actual)
            details = []
            if missing:
                details.append(f"missing from manifest: {', '.join(missing)}")
            if extra:
                details.append(f"listed but absent: {', '.join(extra)}")
            raise PdsEvidenceBundleError(f"artifact list mismatch for {group}: {'; '.join(details)}")
        for relative in listed:
            path = safe_relative_path(relative)
            if not path.parts or path.parts[0] != group:
                raise PdsEvidenceBundleError(f"{group} artifact is outside {group}/: {relative}")


def require_file(root: Path, relative: str) -> Path:
    """Require a bundle-relative file."""
    path = safe_relative_path(relative)
    full_path = root / path
    if not full_path.is_file():
        raise PdsEvidenceBundleError(f"missing required bundle file: {relative}")
    return full_path


def validate_check_evidence(root: Path, label: str, evidence: dict[str, Any]) -> None:
    """Validate one check summary."""
    if not evidence["enabled"]:
        expected = {
            "status": "skipped",
            "report_path": None,
            "log_path": None,
            "checks": 0,
            "passed": 0,
            "failed": 0,
            "timeout": 0,
        }
        for key, value in expected.items():
            if evidence[key] != value:
                raise PdsEvidenceBundleError(f"skipped {label} has invalid {key}")
        return

    if evidence["report_path"] is None or evidence["log_path"] is None:
        raise PdsEvidenceBundleError(f"enabled {label} is missing report_path or log_path")
    require_file(root, evidence["report_path"])
    require_file(root, evidence["log_path"])
    expected_status = "failed" if evidence["failed"] or evidence["timeout"] else "passed"
    if evidence["status"] != expected_status:
        raise PdsEvidenceBundleError(
            f"{label} status is {evidence['status']}, expected {expected_status}"
        )
    if evidence["checks"] != evidence["passed"] + evidence["failed"] + evidence["timeout"]:
        raise PdsEvidenceBundleError(f"{label} check counts do not add up")


def validate_totals(manifest: dict[str, Any]) -> None:
    """Validate manifest totals across all check summaries."""
    checks = [
        *manifest["tests"].values(),
        *manifest["verification"].values(),
    ]
    expected = {
        "enabled_checks": sum(1 for check in checks if check["enabled"]),
        "skipped_checks": sum(1 for check in checks if not check["enabled"]),
        "checks": sum(check["checks"] for check in checks),
        "passed": sum(check["passed"] for check in checks),
        "failed": sum(check["failed"] for check in checks),
        "timeout": sum(check["timeout"] for check in checks),
    }
    if manifest["totals"] != expected:
        raise PdsEvidenceBundleError(f"manifest totals are {manifest['totals']}, expected {expected}")
    expected_status = "failed" if expected["failed"] or expected["timeout"] else "passed"
    if manifest["status"] != expected_status:
        raise PdsEvidenceBundleError(
            f"manifest status is {manifest['status']}, expected {expected_status}"
        )


def validate_bundle(root: Path, manifest_schema: Path, *, skip_checksums: bool = False) -> None:
    """Validate one PDS evidence bundle."""
    manifest_path = root / "metadata/manifest.json"
    if not manifest_path.is_file():
        raise PdsEvidenceBundleError("missing metadata/manifest.json")
    manifest = load_json(manifest_path)
    if not isinstance(manifest, dict):
        raise PdsEvidenceBundleError("manifest root must be a JSON object")
    validate_schema(manifest_schema, manifest, "PDS evidence manifest")
    require_file(root, "README.md")
    require_file(root, manifest["contract"]["compatibility_note"])
    require_file(root, "reports/pds-openapi-validation.json")
    require_file(root, "reports/pds-publication-validation.json")
    require_file(root, "reports/pds-deployment-template-validation.json")
    require_file(root, "reports/pds-schema-export.json")
    require_file(root, "metadata/deployment-templates.yaml")
    require_file(root, "metadata/pds-deployment-template-catalog.schema.json")
    validate_artifact_lists(root, manifest, skip_checksums=skip_checksums)
    for name, evidence in manifest["tests"].items():
        validate_check_evidence(root, f"tests.{name}", evidence)
    for name, evidence in manifest["verification"].items():
        validate_check_evidence(root, f"verification.{name}", evidence)
    validate_totals(manifest)
    if not skip_checksums:
        validate_checksums(root)


def main() -> int:
    """Run PDS evidence bundle validation."""
    parser = argparse.ArgumentParser()
    parser.add_argument("bundle", type=Path)
    parser.add_argument("--manifest-schema", type=Path, default=DEFAULT_MANIFEST_SCHEMA)
    parser.add_argument(
        "--skip-checksums",
        action="store_true",
        help="validate bundle structure before metadata/checksums.sha256 is generated",
    )
    args = parser.parse_args()

    root = args.bundle.resolve()
    if not root.is_dir():
        raise PdsEvidenceBundleError(f"bundle path is not a directory: {root}")
    validate_bundle(root, args.manifest_schema, skip_checksums=args.skip_checksums)
    print(f"validated PDS evidence bundle: {root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
