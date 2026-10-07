#!/usr/bin/env python3
"""Validate a Kiva xAPI LRS conformance evidence bundle as a whole."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any
from xml.etree import ElementTree

from jsonschema import Draft202012Validator, FormatChecker


DEFAULT_MANIFEST_SCHEMA = Path(
    "spec/conformance/schemas/conformance-evidence-manifest.schema.json"
)
DEFAULT_RUN_SCHEMA = Path("spec/conformance/schemas/conformance-run-report.schema.json")
DEFAULT_CASE_MATRIX_SCHEMA = Path(
    "spec/conformance/schemas/conformance-case-matrix-report.schema.json"
)
DEFAULT_COVERAGE_SCHEMA = Path("spec/conformance/schemas/conformance-coverage-report.schema.json")
DEFAULT_ADL_SCHEMA = Path("spec/conformance/schemas/adl-compatibility-report.schema.json")
DEFAULT_ADL_RUN_SCHEMA = Path("spec/conformance/schemas/adl-run-report.schema.json")
DEFAULT_REQUIREMENT_AUDIT_SCHEMA = Path(
    "spec/conformance/schemas/requirement-catalog-audit-report.schema.json"
)
DEFAULT_POSTGRES_VERIFICATION_SCHEMA = Path(
    "spec/conformance/schemas/postgres-verification-report.schema.json"
)

REQUIRED_STATIC_FILES = [
    "README.md",
    "reports/requirement-catalog-audit.json",
    "reports/requirement-catalog-audit.md",
    "reports/xapi-validate-only.json",
    "reports/conformance-case-matrix.json",
    "reports/conformance-case-matrix.md",
    "reports/conformance-coverage.json",
    "reports/conformance-coverage.md",
    "reports/adl-compatibility.json",
    "reports/adl-compatibility.md",
    "junit/xapi-validate-only.xml",
    "logs/tool-versions.log",
    "logs/openapi-validation.log",
    "logs/conformance-plan-validation.log",
    "logs/conformance-fixture-validation.log",
    "logs/requirement-catalog-audit-json.log",
    "logs/requirement-catalog-audit-md.log",
    "logs/xapi-validate-only-json.log",
    "logs/conformance-case-matrix-json.log",
    "logs/conformance-case-matrix-md.log",
    "logs/conformance-coverage-json.log",
    "logs/conformance-coverage-md.log",
    "logs/adl-compatibility-json.log",
    "logs/adl-compatibility-md.log",
    "logs/static-report-schema-validation.log",
]


class ConformanceEvidenceBundleError(RuntimeError):
    """Raised when a conformance evidence bundle is inconsistent."""


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
        raise ConformanceEvidenceBundleError(f"{label} violates {schema_path}:\n{details}")


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
        raise ConformanceEvidenceBundleError(f"unsafe bundle-relative path: {value}")
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
            raise ConformanceEvidenceBundleError(f"{path}:{line_number}: invalid checksum line")
        digest, raw_relative = parts
        if len(digest) != 64 or any(char not in "0123456789abcdef" for char in digest):
            raise ConformanceEvidenceBundleError(f"{path}:{line_number}: invalid sha256 digest")
        relative = normalize_checksum_path(raw_relative)
        if relative in checksums:
            raise ConformanceEvidenceBundleError(
                f"{path}:{line_number}: duplicate checksum for {relative}"
            )
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
        raise ConformanceEvidenceBundleError("missing metadata/checksums.sha256")
    actual_files = bundle_files(root)
    checksums = read_checksums(checksum_path)
    checksum_files = set(checksums)
    missing = sorted(actual_files - checksum_files)
    extra = sorted(checksum_files - actual_files)
    if missing:
        raise ConformanceEvidenceBundleError(f"missing checksums for: {', '.join(missing)}")
    if extra:
        raise ConformanceEvidenceBundleError(
            f"checksums reference missing files: {', '.join(extra)}"
        )
    for relative, expected in sorted(checksums.items()):
        actual = file_sha256(root / relative)
        if actual != expected:
            raise ConformanceEvidenceBundleError(
                f"checksum mismatch for {relative}: expected {expected}, got {actual}"
            )


def validate_artifact_lists(root: Path, manifest: dict[str, Any]) -> None:
    """Validate manifest artifact lists against the bundle directory tree."""
    for group in ["reports", "junit", "logs", "metadata"]:
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
            raise ConformanceEvidenceBundleError(
                f"artifact list mismatch for {group}: {'; '.join(details)}"
            )
        for relative in listed:
            path = safe_relative_path(relative)
            if not path.parts or path.parts[0] != group:
                raise ConformanceEvidenceBundleError(
                    f"{group} artifact is outside {group}/: {relative}"
                )


def require_file(root: Path, relative: str) -> Path:
    """Return a required bundle file path."""
    path = safe_relative_path(relative)
    full_path = root / path
    if not full_path.is_file():
        raise ConformanceEvidenceBundleError(f"missing required bundle file: {relative}")
    return full_path


def require_absent(root: Path, relative: str) -> None:
    """Require a bundle file to be absent."""
    path = safe_relative_path(relative)
    if (root / path).exists():
        raise ConformanceEvidenceBundleError(f"unexpected bundle file: {relative}")


def validate_report(root: Path, relative: str, schema_path: Path, label: str) -> dict[str, Any]:
    """Validate and return one JSON report."""
    path = require_file(root, relative)
    report = load_json(path)
    validate_schema(schema_path, report, label)
    if not isinstance(report, dict):
        raise ConformanceEvidenceBundleError(f"{relative} root must be a JSON object")
    return report


def validate_junit_xml(path: Path) -> None:
    """Validate that a JUnit artifact is well-formed XML."""
    try:
        ElementTree.parse(path)
    except ElementTree.ParseError as error:
        raise ConformanceEvidenceBundleError(f"invalid JUnit XML {path}: {error}") from error


def validate_run_report(report: dict[str, Any], relative: str, expected_mode: str) -> None:
    """Validate conformance-run report consistency beyond JSON Schema."""
    if report["mode"] != expected_mode:
        raise ConformanceEvidenceBundleError(
            f"{relative} mode is {report['mode']}, expected {expected_mode}"
        )
    totals = report["totals"]
    if expected_mode == "validate-only":
        expected_status = "validated"
        expected_validated = totals["total"]
        if totals["validated"] != expected_validated:
            raise ConformanceEvidenceBundleError(
                f"{relative} validated count is {totals['validated']}, "
                f"expected {expected_validated}"
            )
    else:
        expected_status = "failed" if totals["failed"] else "passed"
        if totals["passed"] + totals["failed"] != totals["total"]:
            raise ConformanceEvidenceBundleError(
                f"{relative} passed+failed does not equal total"
            )
    if report["status"] != expected_status:
        raise ConformanceEvidenceBundleError(
            f"{relative} status is {report['status']}, expected {expected_status}"
        )


def validate_required_reports(root: Path) -> None:
    """Validate static conformance reports that every bundle must contain."""
    for relative in REQUIRED_STATIC_FILES:
        require_file(root, relative)
    validate_junit_xml(require_file(root, "junit/xapi-validate-only.xml"))
    validate_report(
        root,
        "reports/requirement-catalog-audit.json",
        DEFAULT_REQUIREMENT_AUDIT_SCHEMA,
        "requirement catalog audit",
    )
    validate_report(
        root,
        "reports/conformance-case-matrix.json",
        DEFAULT_CASE_MATRIX_SCHEMA,
        "conformance case matrix",
    )
    validate_report(
        root,
        "reports/conformance-coverage.json",
        DEFAULT_COVERAGE_SCHEMA,
        "conformance coverage",
    )
    validate_report(
        root,
        "reports/adl-compatibility.json",
        DEFAULT_ADL_SCHEMA,
        "ADL compatibility",
    )
    validate_run_report(
        validate_report(
            root,
            "reports/xapi-validate-only.json",
            DEFAULT_RUN_SCHEMA,
            "validate-only conformance run",
        ),
        "reports/xapi-validate-only.json",
        "validate-only",
    )


def validate_live_http(root: Path, manifest: dict[str, Any]) -> None:
    """Validate live HTTP conformance report presence and status."""
    live_specs = [
        ("memory", "xapi-memory-http"),
        ("postgres", "xapi-postgres-http"),
    ]
    for flag_name, stem in live_specs:
        report_relative = f"reports/{stem}.json"
        junit_relative = f"junit/{stem}.xml"
        log_relative = f"logs/{stem}.log"
        validation_log_relative = f"logs/{stem}-report-validation.log"
        if manifest["live_http"][flag_name]:
            report = validate_report(
                root,
                report_relative,
                DEFAULT_RUN_SCHEMA,
                f"{flag_name} HTTP conformance run",
            )
            validate_run_report(report, report_relative, "execute-http")
            validate_junit_xml(require_file(root, junit_relative))
            require_file(root, log_relative)
            require_file(root, validation_log_relative)
        else:
            require_absent(root, report_relative)
            require_absent(root, junit_relative)
            require_absent(root, log_relative)
            require_absent(root, validation_log_relative)


def validate_postgres_verification(root: Path, manifest: dict[str, Any]) -> None:
    """Validate optional PostgreSQL execution verification evidence."""
    report_relative = "reports/postgres-verification.json"
    log_relative = "logs/postgres-verification.log"
    validation_log_relative = "logs/postgres-verification-report-validation.log"
    if manifest["verification"]["postgres"]:
        report = validate_report(
            root,
            report_relative,
            DEFAULT_POSTGRES_VERIFICATION_SCHEMA,
            "PostgreSQL verification",
        )
        if report["status"] != "passed":
            raise ConformanceEvidenceBundleError(
                f"{report_relative} status is {report['status']}, expected passed"
            )
        require_file(root, log_relative)
        require_file(root, validation_log_relative)
    else:
        require_absent(root, report_relative)
        require_absent(root, log_relative)
        require_absent(root, validation_log_relative)


def validate_adl_run(root: Path, manifest: dict[str, Any]) -> None:
    """Validate optional full ADL execution compatibility evidence."""
    report_path = root / "reports/adl-run.json"
    if not report_path.exists():
        return
    report = validate_report(
        root,
        "reports/adl-run.json",
        DEFAULT_ADL_RUN_SCHEMA,
        "ADL execution compatibility",
    )
    totals = report["totals"]
    results = report["results"]
    if report["status"] != "passed":
        raise ConformanceEvidenceBundleError("reports/adl-run.json status is not passed")
    if not report["expectations"]["suite_revision_matches"]:
        raise ConformanceEvidenceBundleError("ADL suite revision does not match expectations")
    if not report["expectations"]["observed_count_matches"]:
        raise ConformanceEvidenceBundleError("ADL observed count does not match expectations")
    if totals["mismatched"] or totals["unknown"] or totals["missing"]:
        raise ConformanceEvidenceBundleError(
            "ADL execution report contains mismatched, unknown, or missing tests"
        )
    if totals["expected"] != totals["observed"] or totals["expected"] != len(results):
        raise ConformanceEvidenceBundleError("ADL execution result counts are inconsistent")
    if totals["matched"] != len(results):
        raise ConformanceEvidenceBundleError("not every ADL execution result is matched")
    for result in results:
        if result["verdict"] != "matched":
            raise ConformanceEvidenceBundleError(
                f"ADL execution result is not matched: {result['id']}"
            )
        expected = result["expected"]
        actual = result["actual"]
        valid = (
            (expected == "pass" and actual == "pass")
            or (expected == "divergence" and actual in {"fail", "skip"})
            or (expected == "skip" and actual == "skip")
            or (expected == "open-defect" and actual in {"fail", "skip"})
        )
        if not valid:
            raise ConformanceEvidenceBundleError(
                f"ADL execution expectation is stale or inconsistent: {result['id']}"
            )
    if report["kiva"]["git_revision"] != manifest["git"]["commit"]:
        raise ConformanceEvidenceBundleError(
            "ADL execution report and bundle name different Kiva revisions"
        )
    if report["kiva"]["git_dirty"] != manifest["git"]["dirty"]:
        raise ConformanceEvidenceBundleError(
            "ADL execution report and bundle disagree on Kiva dirty state"
        )
    for relative in [
        "logs/adl-run-raw.json",
        "logs/adl-run-console.log",
        "logs/adl-run-server.log",
        "logs/adl-run-runner.log",
        "logs/adl-run-report-validation.log",
    ]:
        require_file(root, relative)


def validate_bundle(root: Path, manifest_schema: Path) -> None:
    """Validate one conformance evidence bundle."""
    manifest_path = root / "metadata/manifest.json"
    if not manifest_path.is_file():
        raise ConformanceEvidenceBundleError("missing metadata/manifest.json")
    manifest = load_json(manifest_path)
    if not isinstance(manifest, dict):
        raise ConformanceEvidenceBundleError("manifest root must be a JSON object")
    validate_schema(manifest_schema, manifest, "conformance evidence manifest")
    validate_artifact_lists(root, manifest)
    validate_required_reports(root)
    validate_live_http(root, manifest)
    validate_postgres_verification(root, manifest)
    validate_adl_run(root, manifest)
    validate_checksums(root)


def main() -> int:
    """Run conformance evidence bundle validation."""
    parser = argparse.ArgumentParser()
    parser.add_argument("bundle", type=Path)
    parser.add_argument("--manifest-schema", type=Path, default=DEFAULT_MANIFEST_SCHEMA)
    args = parser.parse_args()

    root = args.bundle.resolve()
    if not root.is_dir():
        raise ConformanceEvidenceBundleError(f"bundle path is not a directory: {root}")
    validate_bundle(root, args.manifest_schema)
    print(f"validated conformance evidence bundle: {root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
