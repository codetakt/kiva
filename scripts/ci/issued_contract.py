"""Structural traceability checks for the issued IEEE LRS contract.

These checks validate references and recorded source spans, not paraphrase meaning.
The private licensed source is optional for ordinary CI; verification mode requires it.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any

import yaml


STRENGTHS = {
    "shall", "shall-not", "should", "should-not", "may", "permission",
    "data-contract", "context", "scope", "unresolved",
}


def validate_issued_contract(
    catalog: dict[str, Any], issues: list[str], source_text: Path | None = None,
) -> dict[str, Any]:
    """Validate the detailed contract and both directions of its family links."""
    contract = catalog.get("issued_contract")
    if not isinstance(contract, dict):
        issues.append("issued_contract must be present")
        return {"status": "missing"}
    start = len(issues)

    def check(condition: bool, message: str) -> None:
        if not condition:
            issues.append("issued_contract: " + message)

    def records(name: str, id_key: str = "id") -> dict[str, dict[str, Any]]:
        rows = contract.get(name)
        check(isinstance(rows, list) and bool(rows), name + " must be a nonempty list")
        result = {}
        for row in rows if isinstance(rows, list) else []:
            if not isinstance(row, dict) or not isinstance(row.get(id_key), str):
                check(False, name + " row must have a string " + id_key)
                continue
            identifier = row[id_key]
            check(identifier not in result, name + " duplicate " + identifier)
            result[identifier] = row
        return result

    def text_fields(row: dict[str, Any], names: tuple[str, ...], label: str) -> None:
        for name in names:
            check(isinstance(row.get(name), str) and bool(row[name].strip()), label + "." + name + " is required")

    def string_list(row: dict[str, Any], name: str, label: str) -> list[str]:
        values = row.get(name)
        valid = isinstance(values, list) and all(isinstance(v, str) and v for v in values)
        check(valid, label + "." + name + " must be a string list")
        return values if valid else []

    def sha256_field(row: dict[str, Any], name: str, label: str) -> str | None:
        value = row.get(name)
        valid = isinstance(value, str) and re.fullmatch(r"[0-9a-fA-F]{64}", value) is not None
        check(valid, label + "." + name + " must be a SHA-256 hex digest")
        return value.lower() if valid else None

    check(contract.get("schema_version") == 1, "unsupported schema_version")
    check(contract.get("designation") == "IEEE Std 9274.1.1-2023", "unexpected designation")
    check(contract.get("status") in {"candidate", "projection-reviewed", "published"}, "invalid status")
    text_fields(contract, ("scope", "authority"), "contract")
    units = records("source_units")
    rules = records("rules")
    dependencies = records("dependencies")
    questions = records("open_questions")
    links = records("family_projection", "family_id")
    families = catalog.get("requirements")
    check(isinstance(families, list) and bool(families), "requirements must be a nonempty list")
    family_ids: set[str] = set()
    for index, row in enumerate(families if isinstance(families, list) else []):
        identifier = row.get("id") if isinstance(row, dict) else None
        if not isinstance(identifier, str) or not identifier.strip():
            check(False, f"requirements[{index}].id must be a nonempty string")
            continue
        check(identifier not in family_ids, "duplicate requirement id: " + identifier)
        family_ids.add(identifier)
    check(set(links) == family_ids, "family projection must account for every existing family exactly once")
    linked_rules: set[str] = set()
    for identifier, row in links.items():
        targets = string_list(row, "rule_ids", identifier)
        check(bool(targets), identifier + " has no detailed rules")
        check(set(targets) <= rules.keys(), identifier + " refers to unknown detailed rules")
        check(len(targets) == len(set(targets)), identifier + " has duplicate rule links")
        linked_rules.update(targets)
        text_fields(row, ("relation", "evidence_reuse"), identifier)
    new_rules = string_list(contract, "new_rule_ids", "contract")
    check(set(new_rules) == rules.keys() - linked_rules, "new_rule_ids must be the exact detailed-rule remainder")
    check(len(new_rules) == len(set(new_rules)), "new_rule_ids contains duplicates")
    referenced_units: set[str] = set()
    for identifier, row in rules.items():
        text_fields(row, ("actor", "when", "requirement", "interpretation", "locator"), identifier)
        check(row.get("strength") in STRENGTHS, identifier + " has an invalid strength")
        sources = string_list(row, "source_units", identifier)
        check(bool(sources) and set(sources) <= units.keys(), identifier + " has missing/unknown source units")
        referenced_units.update(sources)
        string_list(row, "exceptions", identifier)
        rule_issues = string_list(row, "issues", identifier)
        check(set(rule_issues) <= questions.keys(), identifier + " has unknown open questions")
        check(set(string_list(row, "dependencies", identifier)) <= dependencies.keys(), identifier + " has unknown dependencies")
        check(row.get("interpretation") in {"source-stated", "project-interpretation", "unresolved"}, identifier + " has invalid interpretation")
        check(not rule_issues or row.get("interpretation") == "unresolved", identifier + " hides unresolved questions")
        unresolved = row.get("strength") == "unresolved" or row.get("interpretation") == "unresolved"
        check(not unresolved or bool(rule_issues), identifier + " lacks its unresolved question")
        if "related_rules" in row:
            related = string_list(row, "related_rules", identifier)
            check(set(related) <= rules.keys(), identifier + " has unknown related rules")
            check(identifier not in related, identifier + " relates to itself")
            check(len(related) == len(set(related)), identifier + " has duplicate related rules")
    previous_end = None
    span_count = 0
    source = contract.get("source")
    if not isinstance(source, dict):
        check(False, "source must be a mapping")
        source = {}
    sha256_field(source, "pdf_sha256", "source")
    text_digest = sha256_field(source, "text_sha256", "source")
    data = source_text.read_bytes() if source_text is not None else None
    if data is not None:
        check(hashlib.sha256(data).hexdigest() == text_digest, "issued text digest mismatch")
    for identifier, row in units.items():
        unit_issue_start = len(issues)
        text_fields(row, ("clause", "heading", "reason"), identifier)
        span_digest = sha256_field(row, "sha256", identifier)
        check(row.get("disposition") in {"projected", "context"}, identifier + " is not classified")
        check(row.get("disposition") != "projected" or identifier in referenced_units, identifier + " is projected without a rule")
        if "counterpart" in row:
            check(row["counterpart"] in units, identifier + " has an unknown counterpart")
        pdf_page = row.get("pdf_page")
        check(type(pdf_page) is int and pdf_page > 0, identifier + " has invalid PDF page")
        lf_start, lf_end = row.get("lf_start"), row.get("lf_end")
        valid_lf = type(lf_start) is int and lf_start > 0 and type(lf_end) is int and lf_end >= lf_start
        check(valid_lf, identifier + " has invalid LF coordinates")
        begin, size = row.get("byte_start"), row.get("byte_length")
        valid = type(begin) is int and begin >= 0 and type(size) is int and size > 0
        check(valid, identifier + " has invalid byte coordinates")
        if not valid:
            continue
        if previous_end is not None:
            check(begin == previous_end, identifier + " has a gap/overlap with the preceding source unit")
        previous_end = begin + size
        if data is not None:
            check(begin + size <= len(data), identifier + " extends beyond the source")
            check(hashlib.sha256(data[begin:begin + size]).hexdigest() == span_digest, identifier + " span digest mismatch")
            check(data[:begin].count(b"\n") + 1 == lf_start, identifier + " LF start mismatch")
            check(data[:begin + size - 1].count(b"\n") + 1 == lf_end, identifier + " LF end mismatch")
            check(data[:begin].count(b"\f") + 1 == pdf_page, identifier + " PDF page mismatch")
            if len(issues) == unit_issue_start:
                span_count += 1
    for identifier, row in dependencies.items():
        text_fields(row, ("reference", "edition_rule", "incorporation", "owner", "status", "evidence_required", "original_reading"), identifier)
    return {
        "status": "passed" if len(issues) == start else "failed",
        "rules": len(rules), "source_units": len(units), "families": len(links),
        "new_rules": len(new_rules), "dependencies": len(dependencies),
        "unresolved_rules": sum(r.get("interpretation") == "unresolved" for r in rules.values()),
        "source_spans_verified": span_count,
        "source_verification": "fresh" if data is not None else "not-run-private-source-not-required-for-CI",
        "semantic_completeness": "not-established-by-this-check",
        "implementation_conformance": "not-assessed",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=Path("spec/requirements/ieee-39274-1-1-2025.yaml"))
    parser.add_argument("--source-text", type=Path)
    args = parser.parse_args()
    catalog = yaml.safe_load(args.catalog.read_text())
    issues: list[str] = []
    report = validate_issued_contract(catalog, issues, args.source_text)
    report["issues"] = issues
    print(json.dumps(report, indent=2))
    return bool(issues)


if __name__ == "__main__":
    raise SystemExit(main())
