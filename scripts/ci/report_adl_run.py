#!/usr/bin/env python3
"""Normalize an ADL LRS conformance run and compare it with pinned expectations."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from collections import Counter, defaultdict
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import yaml
from jsonschema import Draft202012Validator, FormatChecker


DEFAULT_EXPECTATIONS = Path("spec/conformance/adl-run-expectations.yaml")
DEFAULT_EXPECTATIONS_SCHEMA = Path(
    "spec/conformance/schemas/adl-run-expectations.schema.json"
)
DEFAULT_REPORT_SCHEMA = Path("spec/conformance/schemas/adl-run-report.schema.json")
DEFAULT_REQUIREMENTS = Path("spec/requirements/ieee-39274-1-1-2025.yaml")
DEFAULT_COMPATIBILITY_MAP = Path("spec/conformance/adl-compatibility-map.yaml")
ID_SCHEME = "sha256-title-path-occurrence-v1"


class AdlRunError(RuntimeError):
    """Raised when ADL run inputs or expectations are invalid."""


def load_json(path: Path) -> Any:
    """Load JSON from a file."""
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def load_yaml(path: Path) -> Any:
    """Load YAML from a file."""
    with path.open("r", encoding="utf-8") as handle:
        return yaml.safe_load(handle)


def validate_schema(instance: Any, schema_path: Path, label: str) -> None:
    """Validate an instance against a JSON Schema."""
    schema = load_json(schema_path)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(validator.iter_errors(instance), key=lambda error: list(error.absolute_path))
    if errors:
        lines = []
        for error in errors:
            location = "$"
            for part in error.absolute_path:
                location += f"[{part}]" if isinstance(part, int) else f".{part}"
            lines.append(f"- {location}: {error.message}")
        raise AdlRunError(f"{label} violates {schema_path}:\n" + "\n".join(lines))


def git_state(root: Path) -> tuple[str, bool]:
    """Return the exact revision and dirty state of one Git checkout."""
    revision = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    dirty = bool(
        subprocess.run(
            ["git", "-C", str(root), "status", "--porcelain"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    )
    return revision, dirty


def stable_test_id(title_path: list[str], occurrence: int) -> str:
    """Derive a stable local identity from an ADL title path and occurrence."""
    material = "\x1f".join([*title_path, str(occurrence)]).encode()
    return f"adl-v2-{hashlib.sha256(material).hexdigest()[:20]}"


def extract_tests(raw: dict[str, Any]) -> list[dict[str, Any]]:
    """Flatten leaf tests from the hierarchical ADL JSON log."""
    root_log = raw.get("log")
    if not isinstance(root_log, dict):
        raise AdlRunError("ADL raw report does not contain a log object")
    leaves: list[dict[str, Any]] = []
    occurrences: defaultdict[tuple[str, ...], int] = defaultdict(int)

    def visit(node: dict[str, Any], parents: list[str]) -> None:
        title = node.get("title")
        children = node.get("tests")
        if not isinstance(title, str) or not isinstance(children, list):
            raise AdlRunError("ADL test tree contains an invalid node")
        path = [*parents, title] if title else parents
        if children:
            for child in children:
                if not isinstance(child, dict):
                    raise AdlRunError("ADL test tree contains a non-object child")
                visit(child, path)
            return
        if not path:
            raise AdlRunError("ADL leaf test has no title path")
        key = tuple(path)
        occurrences[key] += 1
        occurrence = occurrences[key]
        status = node.get("status")
        actual = {"passed": "pass", "failed": "fail", "skipped": "skip"}.get(status)
        if actual is None:
            raise AdlRunError(f"ADL leaf test has unsupported status {status!r}: {' / '.join(path)}")
        error = node.get("error")
        leaves.append(
            {
                "id": stable_test_id(path, occurrence),
                "title_path": path,
                "occurrence": occurrence,
                "actual": actual,
                "error": error if isinstance(error, str) else None,
            }
        )

    visit(root_log, [])
    return leaves


def collect_ids(value: Any) -> set[str]:
    """Collect id fields recursively from a YAML data structure."""
    found: set[str] = set()
    if isinstance(value, dict):
        identifier = value.get("id")
        if isinstance(identifier, str):
            found.add(identifier)
        for child in value.values():
            found.update(collect_ids(child))
    elif isinstance(value, list):
        for child in value:
            found.update(collect_ids(child))
    return found


def compatibility_relationships(value: Any) -> dict[str, str]:
    """Collect compatibility entry relationships by id."""
    found: dict[str, str] = {}
    if isinstance(value, dict):
        identifier = value.get("id")
        relationship = value.get("relationship")
        if isinstance(identifier, str) and isinstance(relationship, str):
            found[identifier] = relationship
        for child in value.values():
            found.update(compatibility_relationships(child))
    elif isinstance(value, list):
        for child in value:
            found.update(compatibility_relationships(child))
    return found


def validate_references(
    expectations: dict[str, Any], requirements: Any, compatibility_map: Any
) -> None:
    """Validate requirement and compatibility references in expectation rows."""
    requirement_ids = collect_ids(requirements)
    relationships = compatibility_relationships(compatibility_map)
    for row in expectations["tests"]:
        for reference in row.get("requirement_refs", []):
            if reference not in requirement_ids:
                raise AdlRunError(f"unknown IEEE requirement reference {reference} in {row['id']}")
        for reference in row.get("compatibility_refs", []):
            relationship = relationships.get(reference)
            if relationship is None:
                raise AdlRunError(f"unknown ADL compatibility reference {reference} in {row['id']}")
            if row["expected"] in {"divergence", "skip"} and relationship not in {
                "obsolete",
                "absent",
            }:
                raise AdlRunError(
                    f"{row['id']} references {reference} with relationship {relationship}; "
                    "divergence/skip references must be obsolete or absent"
                )


def expected_matches_actual(expected: str, actual: str) -> bool:
    """Evaluate the freshness-sensitive expectation rule."""
    return {
        "pass": actual == "pass",
        "divergence": actual in {"fail", "skip"},
        "skip": actual == "skip",
        "open-defect": actual in {"fail", "skip"},
    }[expected]


def normalize(args: argparse.Namespace) -> tuple[dict[str, Any], list[str]]:
    """Build the normalized report and return report-level mismatch reasons."""
    raw = load_json(args.raw_report)
    if not isinstance(raw, dict):
        raise AdlRunError("ADL raw report root must be an object")
    expectations = load_yaml(args.expectations)
    if not isinstance(expectations, dict):
        raise AdlRunError("ADL expectations root must be an object")
    validate_schema(expectations, args.expectations_schema, "ADL run expectations")
    validate_references(
        expectations,
        load_yaml(args.requirements),
        load_yaml(args.compatibility_map),
    )

    actual_rows = extract_tests(raw)
    expected_rows = expectations["tests"]
    actual_by_id = {row["id"]: row for row in actual_rows}
    expected_by_id = {row["id"]: row for row in expected_rows}
    if len(actual_by_id) != len(actual_rows):
        raise AdlRunError("derived ADL test ids are not unique")
    if len(expected_by_id) != len(expected_rows):
        raise AdlRunError("expectation ledger contains duplicate test ids")

    suite_revision, suite_dirty = git_state(args.adl_root)
    kiva_revision, kiva_dirty = git_state(args.kiva_root)
    suite = expectations["suite"]
    summary = raw.get("summary", {})
    advertised = summary.get("total")
    if not isinstance(advertised, int):
        raise AdlRunError("ADL raw report summary.total must be an integer")

    reasons: list[str] = []
    if suite_revision != suite["git_revision"]:
        reasons.append("ADL suite revision differs from the expectation ledger")
    if suite_dirty:
        reasons.append("ADL suite checkout is dirty")
    if summary.get("version") != suite["target_version"]:
        reasons.append("ADL target version differs from the expectation ledger")
    if len(actual_rows) != suite["expected_observed_tests"]:
        reasons.append("observed ADL leaf count differs from the expectation ledger")
    if advertised != suite["upstream_advertised_tests"]:
        reasons.append("ADL advertised test count differs from the expectation ledger")

    unknown_ids = sorted(set(actual_by_id) - set(expected_by_id))
    missing_ids = sorted(set(expected_by_id) - set(actual_by_id))
    results: list[dict[str, Any]] = []
    mismatched = 0
    for identifier in sorted(set(actual_by_id) | set(expected_by_id)):
        actual = actual_by_id.get(identifier)
        expected = expected_by_id.get(identifier)
        if actual is None:
            row = {
                "id": identifier,
                "title_path": expected["title_path"],
                "occurrence": expected["occurrence"],
                "actual": "not-run",
                "expected": expected["expected"],
                "verdict": "mismatch",
                "requirement_refs": expected.get("requirement_refs", []),
                "compatibility_refs": expected.get("compatibility_refs", []),
                "rationale": expected.get("rationale"),
                "error": None,
            }
        elif expected is None:
            row = {
                **actual,
                "expected": "unknown",
                "verdict": "mismatch",
                "requirement_refs": [],
                "compatibility_refs": [],
                "rationale": None,
            }
        else:
            identity_matches = (
                actual["title_path"] == expected["title_path"]
                and actual["occurrence"] == expected["occurrence"]
            )
            matched = identity_matches and expected_matches_actual(
                expected["expected"], actual["actual"]
            )
            row = {
                **actual,
                "expected": expected["expected"],
                "verdict": "matched" if matched else "mismatch",
                "requirement_refs": expected.get("requirement_refs", []),
                "compatibility_refs": expected.get("compatibility_refs", []),
                "rationale": expected.get("rationale"),
            }
        mismatched += row["verdict"] == "mismatch"
        results.append(row)

    expected_counts = Counter(row["expected"] for row in expected_rows)
    actual_counts = Counter(row["actual"] for row in actual_rows)
    if unknown_ids:
        reasons.append(f"{len(unknown_ids)} ADL test ids are absent from the expectation ledger")
    if missing_ids:
        reasons.append(f"{len(missing_ids)} expectation ids were not executed")
    if mismatched:
        reasons.append(f"{mismatched} test outcomes do not match expectations")

    report = {
        "schema_version": 1,
        "standard": "IEEE/ISO/IEC 39274-1-1-2025",
        "kind": "adl-lrs-compatibility-run",
        "status": "failed" if reasons else "passed",
        "role": "external-compatibility-evidence-not-normative-source",
        "generated_at_utc": datetime.now(UTC).replace(microsecond=0).isoformat(),
        "kiva": {"git_revision": kiva_revision, "git_dirty": kiva_dirty},
        "suite": {
            "id": suite["id"],
            "upstream": suite["upstream"],
            "target_version": suite["target_version"],
            "git_revision": suite_revision,
            "git_dirty": suite_dirty,
            "test_id_scheme": ID_SCHEME,
            "advertised_tests": advertised,
            "observed_tests": len(actual_rows),
            "advertised_observed_delta": abs(advertised - len(actual_rows)),
        },
        "expectations": {
            "path": str(args.expectations),
            "suite_revision_matches": suite_revision == suite["git_revision"],
            "observed_count_matches": len(actual_rows) == suite["expected_observed_tests"],
        },
        "totals": {
            "expected": len(expected_rows),
            "observed": len(actual_rows),
            "actual_pass": actual_counts["pass"],
            "actual_fail": actual_counts["fail"],
            "actual_skip": actual_counts["skip"],
            "expected_pass": expected_counts["pass"],
            "expected_divergence": expected_counts["divergence"],
            "expected_skip": expected_counts["skip"],
            "expected_open_defect": expected_counts["open-defect"],
            "matched": len(results) - mismatched,
            "mismatched": mismatched,
            "unknown": len(unknown_ids),
            "missing": len(missing_ids),
        },
        "results": results,
    }
    validate_schema(report, args.report_schema, "normalized ADL run report")
    return report, reasons


def main() -> int:
    """Normalize one ADL run and enforce its expectation ledger."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw-report", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--adl-root", required=True, type=Path)
    parser.add_argument("--kiva-root", type=Path, default=Path("."))
    parser.add_argument("--expectations", type=Path, default=DEFAULT_EXPECTATIONS)
    parser.add_argument("--expectations-schema", type=Path, default=DEFAULT_EXPECTATIONS_SCHEMA)
    parser.add_argument("--report-schema", type=Path, default=DEFAULT_REPORT_SCHEMA)
    parser.add_argument("--requirements", type=Path, default=DEFAULT_REQUIREMENTS)
    parser.add_argument("--compatibility-map", type=Path, default=DEFAULT_COMPATIBILITY_MAP)
    args = parser.parse_args()

    report, reasons = normalize(args)
    payload = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output is None:
        sys.stdout.write(payload)
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(payload, encoding="utf-8")
    if reasons:
        for reason in reasons:
            print(f"ADL expectation mismatch: {reason}", file=sys.stderr)
        return 1
    print(
        f"validated ADL run: {report['totals']['matched']}/{report['totals']['expected']} "
        "outcomes matched expectations",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
