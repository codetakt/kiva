#!/usr/bin/env python3
"""Validate the pinned LRS source, table vectors, and actual assertion bodies.

This is a drift/traceability check, not a theorem proving semantic completeness.
Review procedures remain visible even when every machine check passes.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import yaml

from report_lrs_interpretations import interpretation_bindings


def load_yaml(text):
    return yaml.load(text, Loader=getattr(yaml, "CSafeLoader", yaml.SafeLoader))


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE = Path(
    "/home/claude/Codex/opensource.ieee.org/xapi/xapi-base-standard-documentation/9274.1.1 xAPI Base Standard for LRSs.md"
)
LEDGER = ROOT / "spec/conformance/lrs-source-projection.yaml"
SEAL = ROOT / "spec/conformance/lrs-assertion-manifest.yaml"


def digest(value):
    return hashlib.sha256(
        json.dumps(
            value, sort_keys=True, ensure_ascii=False, separators=(",", ":")
        ).encode()
    ).hexdigest()


def load_cases():
    cases = {}
    for path in sorted((ROOT / "tests/conformance/http").rglob("*.yaml")):
        document = load_yaml(path.read_text())
        for case in document["cases"]:
            identifier = case["id"]
            if identifier in cases:
                raise ValueError(f"duplicate case {identifier}")
            steps = case.get("steps", [case])
            assertions = []
            for index, step in enumerate(steps):
                prefix = f"/steps/{index}/expect" if "steps" in case else "/expect"
                expectation = step["expect"]
                for key in ["status", "status_in", "status_classification", "headers", "body", "json", "checks"]:
                    if expectation.get(key) is not None:
                        assertions.append(
                            dict(
                                pointer=prefix + "/" + key,
                                sha256=digest(expectation[key]),
                            )
                        )
            cases[identifier] = dict(
                file=str(path.relative_to(ROOT)),
                sha256=digest(case),
                assertions=assertions,
            )
    return cases


def audit_table_inventory(lines, models):
    """Compare actual source table properties with the curated models.

    This detects omitted rows; it does not determine semantics from a column.
    """
    for name, model in models.items():
        anchor = next(
            (
                index
                for index, line in enumerate(lines)
                if line.startswith(model["source_anchor"])
            ),
            None,
        )
        if anchor is None:
            raise ValueError(f"missing source anchor for {name}")
        if name == "context-activities":
            # The four keys are specified in prose in the contextActivities row.
            for key in model["fields"]:
                if f"&quot;{key}&quot;" not in lines[anchor]:
                    raise ValueError(
                        f"context activity key lacks its source anchor: {key}"
                    )
            continue
        table_start = next(
            index
            for index in range(anchor, len(lines))
            if lines[index].startswith("| **Property**")
        )
        rows = []
        for line in lines[table_start + 2 :]:
            if not line.startswith("|"):
                break
            rows.append(line.split("|")[1].strip().replace("\\_", "_"))
        fields = set()
        for field in rows:
            if field == "Interaction Activities":
                if "interaction" not in models:
                    raise ValueError(
                        "Activity Definition interaction table has no model"
                    )
                continue
            if field == "choices, scale, source, target, or steps":
                fields.update(["choices", "scale", "source", "target", "steps"])
            else:
                fields.add(field)
        if fields != set(model["fields"]):
            raise ValueError(
                f"source/model property inventory differs for {name}: {fields.symmetric_difference(model['fields'])}"
            )


def audit_duty_review(lines, cases, review_ids, models):
    """Check authored duty coordinates and concrete references, not their meaning."""
    record = load_yaml((ROOT / "spec/conformance/lrs-duty-review.yaml").read_text())
    if record["unmatched_action_selections"]:
        raise ValueError("duty review contains unmatched action selections")

    def check_source(item):
        start, end = item["source_lines"]
        if start < 1 or end < start or (lines is not None and end > len(lines)):
            raise ValueError(f"invalid duty source span: {item['id']}")
        if (
            lines is not None
            and hashlib.sha256("".join(lines[start - 1 : end]).encode()).hexdigest()
            != item["source_sha256"]
        ):
            raise ValueError(f"duty source span changed: {item['id']}")

    def check_witness(witness):
        actual = cases.get(witness["case"])
        if actual is None or actual["file"] != witness["file"]:
            raise ValueError(f"unknown duty witness: {witness['case']}")
        for assertion in witness["assertions"]:
            if assertion not in actual["assertions"]:
                raise ValueError(
                    f"duty assertion changed: {witness['case']} {assertion['pointer']}"
                )

    seen = set()
    for duty in record["prose_duties"]:
        if duty["id"] in seen:
            raise ValueError(f"duplicate source action: {duty['id']}")
        seen.add(duty["id"])
        check_source(duty)
        if not duty["automated_witnesses"] and not duty["semantic_procedure"]:
            raise ValueError(f"source action has no test: {duty['id']}")
        if duty["semantic_procedure"] and duty["semantic_procedure"] not in review_ids:
            raise ValueError(f"unknown duty procedure: {duty['id']}")
        for witness in duty["automated_witnesses"]:
            check_witness(witness)
    expected_fields = {
        f"{name}.{field}" for name, model in models.items() for field in model["fields"]
    }
    if {field["id"] for field in record["table_field_duties"]} != expected_fields:
        raise ValueError("duty record omits a table field or adds an unknown field")
    for field in record["table_field_duties"]:
        check_source(field)
        name, key = field["id"].split(".", 1)
        if field["contract"] != models[name]["fields"][key]:
            raise ValueError(f"reviewed table contract changed: {field['id']}")
        for placement in field["placements"]:
            for witness in placement["acceptance_witnesses"]:
                check_witness(witness)
            for binding in placement["rejection_bindings"]:
                if binding["case"] not in cases or not any(
                    a["pointer"] == binding["assertion"]
                    for a in cases[binding["case"]]["assertions"]
                ):
                    raise ValueError(
                        f"missing reviewed table rejection: {binding['case']}"
                    )
    return len(record["prose_duties"]), len(record["table_field_duties"])


def audit_policy_table_anchors(lines):
    """Verify declared row names/bytes, not the interpretation of cardinality."""
    policy = load_yaml(
        (ROOT / "spec/conformance/lrs-interpretation-policy.yaml").read_text()
    )
    checked = 0
    for judgment in policy.get("additional_judgments", []):
        anchors = judgment.get("table_anchors", [])
        if anchors and {a["line"] for a in anchors} != set(
            judgment["public_source_lines"]
        ):
            raise ValueError(f"policy table anchor inventory differs: {judgment['id']}")
        for anchor in anchors:
            if lines is None:
                continue
            line = anchor["line"]
            if line < 1 or line > len(lines):
                raise ValueError(
                    f"policy table anchor outside source: {judgment['id']}"
                )
            text = lines[line - 1]
            cells = text.split("|")
            if (
                not text.startswith("|")
                or len(cells) < 3
                or cells[1].strip().replace("\\_", "_") != anchor["property"]
            ):
                raise ValueError(
                    f"policy table anchor property differs: {judgment['id']}:{line}"
                )
            if hashlib.sha256(text.encode()).hexdigest() != anchor["source_sha256"]:
                raise ValueError(
                    f"policy table anchor bytes differ: {judgment['id']}:{line}"
                )
            checked += 1
    return checked


def audit(source, update_bindings):
    ledger = load_yaml(LEDGER.read_text())
    data = source.read_bytes() if source else None
    actual = (
        hashlib.sha256(data).hexdigest()
        if data is not None
        else ledger["source"]["sha256"]
    )
    if data is not None and (
        actual != ledger["source"]["sha256"] or len(data) != ledger["source"]["bytes"]
    ):
        raise ValueError(
            "LRS source bytes changed: explicit source re-reading and a new baseline are required"
        )
    lines = data.decode("utf-8").splitlines(keepends=True) if data is not None else None
    policy_table_anchors = audit_policy_table_anchors(lines)
    model = load_yaml((ROOT / "tests/conformance/models/lrs-tables.yaml").read_text())
    if lines is not None:
        audit_table_inventory(lines, model["models"])
    expected_start = 1
    for unit in ledger["source_units"]:
        start, end = unit["lines"]
        if start != expected_start or end < start:
            raise ValueError(f"source unit gap or overlap at {unit['lines']}")
        if (
            lines is not None
            and hashlib.sha256("".join(lines[start - 1 : end]).encode()).hexdigest()
            != unit["sha256"]
        ):
            raise ValueError(f"source unit hash mismatch at {unit['lines']}")
        expected_start = end + 1
    if (
        expected_start
        != (len(lines) if lines is not None else ledger["source"]["lines"]) + 1
    ):
        raise ValueError("source ledger does not account for every source line")
    cases = load_cases()
    reviews = load_yaml((ROOT / "spec/conformance/lrs-review-tests.yaml").read_text())[
        "tests"
    ]
    review_ids = {review["id"] for review in reviews}
    reviewed_actions, reviewed_fields = audit_duty_review(
        lines, cases, review_ids, model["models"]
    )
    rule_bindings = []
    for rule in ledger["rules"]:
        selected = [
            name
            for name in cases
            if any(name.startswith(selector) for selector in rule["case_selectors"])
        ]
        if rule.get("review") and rule["review"] not in review_ids:
            raise ValueError(f"unknown review test in {rule['id']}")
        if not selected and not rule.get("review"):
            raise ValueError(f"source rule has no executable/review test: {rule['id']}")
        rule_bindings.append(
            dict(rule=rule["id"], cases=selected, review=rule.get("review"))
        )
    manifest = dict(
        schema_version=1,
        source_sha256=actual,
        execution_contract_files={
            path: hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
            for path in [
                "Cargo.toml",
                "Cargo.lock",
                "spec/conformance/schemas/conformance-case.schema.json",
                "tests/conformance/fixtures/identifier-grammar.json",
                "tests/conformance/fixtures/identifier-grammar.sources.json",
                "tools/xapi-lrs-conformance/Cargo.toml",
                "tools/xapi-lrs-conformance/src/lib.rs",
                "tools/xapi-lrs-conformance/src/assertions.rs",
                "tools/xapi-lrs-conformance/src/main.rs",
            ]
        },
        table_model_sha256=hashlib.sha256(
            (ROOT / "tests/conformance/models/lrs-tables.yaml").read_bytes()
        ).hexdigest(),
        table_bindings_sha256=hashlib.sha256(
            (ROOT / "spec/conformance/lrs-table-bindings.yaml").read_bytes()
        ).hexdigest(),
        projection_ledger_sha256=hashlib.sha256(LEDGER.read_bytes()).hexdigest(),
        duty_review_sha256=hashlib.sha256(
            (ROOT / "spec/conformance/lrs-duty-review.yaml").read_bytes()
        ).hexdigest(),
        published_source_record_sha256=hashlib.sha256(
            (ROOT / ledger["source"]["published_source_record"]).read_bytes()
        ).hexdigest(),
        interpretation_contract=interpretation_bindings(ROOT),
        review_procedures_sha256=hashlib.sha256(
            (ROOT / "spec/conformance/lrs-review-tests.yaml").read_bytes()
        ).hexdigest(),
        cases=cases,
        rule_bindings=rule_bindings,
    )
    if update_bindings:
        if source is None:
            raise ValueError(
                "updating assertion bindings requires the pinned source bytes"
            )
        SEAL.write_text(
            yaml.dump(
                manifest,
                Dumper=getattr(yaml, "CSafeDumper", yaml.SafeDumper),
                sort_keys=False,
                width=100,
            )
        )
    elif not SEAL.exists() or load_yaml(SEAL.read_text()) != manifest:
        raise ValueError(
            "assertion bodies or source bindings changed; review the diff and explicitly regenerate the assertion manifest"
        )
    table_bindings = load_yaml(
        (ROOT / "spec/conformance/lrs-table-bindings.yaml").read_text()
    )
    for binding in table_bindings["bindings"]:
        name = binding["case"]
        if name not in cases:
            raise ValueError(f"missing table vector case {name}")
        if not any(
            item["pointer"] == binding["assertion"]
            for item in cases[name]["assertions"]
        ):
            raise ValueError(
                f"table binding lacks actual assertion: {name} {binding['assertion']}"
            )
    mapped = {name for rule in rule_bindings for name in rule["cases"]}
    unbound = sorted(set(cases).difference(mapped))
    return dict(
        status="source-and-assertion-drift-check-passed"
        if source
        else "assertion-drift-check-passed-source-not-verified",
        source_bytes_verified=source is not None,
        source_revision=ledger["source"]["revision"],
        source_sha256=actual,
        source_units=len(ledger["source_units"]),
        semantic_rule_groups=len(rule_bindings),
        executable_cases=len(cases),
        source_bound_cases=len(mapped),
        cases_without_source_rule=unbound,
        table_conditions=len(table_bindings["bindings"]),
        review_procedures=len(reviews),
        reviewed_source_actions=reviewed_actions,
        reviewed_table_fields=reviewed_fields,
        reviewed_policy_table_anchors=policy_table_anchors,
        duty_review_meaning="author judgments; audit checks coordinates/references/bytes, not semantic completeness",
        review_execution="not-established-by-this-audit",
        interpretation_profile=manifest["interpretation_contract"]["profile"],
        interpretation_dependent_cases=len(
            manifest["interpretation_contract"]["dependencies"]
        ),
        completeness_claim="not-proven-by-counts-or-hashes",
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument(
        "--skip-source",
        action="store_true",
        help="check assertions only and explicitly report source bytes as unverified",
    )
    parser.add_argument(
        "--update-bindings",
        action="store_true",
        help="write a reviewable assertion manifest after an intentional change",
    )
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    try:
        report = audit(None if args.skip_source else args.source, args.update_bindings)
    except (ValueError, KeyError, FileNotFoundError) as error:
        raise SystemExit(str(error)) from error
    print(
        json.dumps(report, indent=2)
        if args.json
        else f"validated source/expectation bindings: {report}"
    )


if __name__ == "__main__":
    main()
