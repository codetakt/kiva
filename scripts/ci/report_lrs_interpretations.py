#!/usr/bin/env python3
"""Bind known source interpretations to assertions without changing run outcomes.

Without a run report this is a dependency inventory. Historical execution needs
a receipt binding the report, case files and execution contracts. Neither mode
issues a whole-standard conformance verdict.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
POLICY = "spec/conformance/lrs-interpretation-policy.yaml"


def load_yaml(path):
    return yaml.load(
        path.read_text(), Loader=getattr(yaml, "CSafeLoader", yaml.SafeLoader)
    )


def file_digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def value_digest(value):
    return hashlib.sha256(
        json.dumps(
            value, sort_keys=True, ensure_ascii=False, separators=(",", ":")
        ).encode()
    ).hexdigest()


def case_documents(root):
    cases = {}
    for path in sorted((root / "tests/conformance/http").rglob("*.yaml")):
        for case in load_yaml(path)["cases"]:
            if case["id"] in cases:
                raise ValueError(f"duplicate case {case['id']}")
            cases[case["id"]] = (str(path.relative_to(root)), case)
    return cases


def request_matches(selector, request, expectation):
    if request.get("method") not in selector["methods"]:
        return False
    if request.get("path") != selector["path"]:
        return False
    statuses = [expectation["status"], *expectation.get("status_in", [])]
    if selector.get("expected_statuses") and not set(statuses).intersection(
        selector["expected_statuses"]
    ):
        return False
    query = request.get("query", {})
    if any(key in query for key in selector.get("query_absent", [])):
        return False
    if any(
        query.get(key) != value
        for key, value in selector.get("query_equals", {}).items()
    ):
        return False
    return selector.get("body_type") != "array" or isinstance(request.get("body"), list)


def interpretation_bindings(root):
    policy = load_yaml(root / POLICY)
    published = load_yaml(root / policy["published_source_record"])
    published_decisions = policy["decisions"]
    decisions = published_decisions + policy.get("additional_judgments", [])
    ids = [decision["id"] for decision in decisions]
    if len(ids) != len(set(ids)):
        raise ValueError("duplicate interpretation decision")
    if {d["finding"] for d in published_decisions} != {
        f["id"] for f in published["findings"]
    }:
        raise ValueError(
            "interpretation decisions do not cover the published source findings"
        )
    for decision in decisions:
        if (
            decision["status"] != "project-adopted"
            or decision["official_adjudication"] != "not-established"
        ):
            raise ValueError(
                "official interpretation status requires a new reviewed contract"
            )
        for field in ["chosen_rule", "rationale", "rejected_reading"]:
            if not decision.get(field):
                raise ValueError(f"missing interpretation {field}: {decision['id']}")
        for selector in decision.get("request_selectors", []):
            if set(selector) - {
                "methods",
                "path",
                "query_absent",
                "query_equals",
                "body_type",
                "expected_statuses",
            }:
                raise ValueError(f"unknown interpretation selector: {decision['id']}")
            if selector.get("body_type") not in [None, "array"]:
                raise ValueError(
                    f"unsupported interpretation body selector: {decision['id']}"
                )
    models = load_yaml(root / "tests/conformance/models/lrs-tables.yaml")["models"]
    table = load_yaml(root / "spec/conformance/lrs-table-bindings.yaml")["bindings"]
    table_by_case = {}
    for binding in table:
        table_by_case.setdefault(binding["case"], []).append(binding)
    cases = case_documents(root)
    bindings = {}
    for decision in decisions:
        for name in decision.get("explicit_cases", []):
            if name not in cases:
                raise ValueError(f"unknown explicit interpretation case {name}")
        for name, (_, case) in cases.items():
            dependencies = []
            if name in decision.get("explicit_cases", []):
                dependencies.append((None, "curated-case"))
            for binding in table_by_case.get(name, []):
                if not decision.get("table_formats"):
                    continue
                field = binding["field"]
                vector = models[binding["model"]]["fields"].get(field, {})
                condition = binding["condition"]
                fmt = vector.get("format")
                if fmt in decision["table_formats"] and re.fullmatch(
                    re.escape(field or "") + r"-value-\d+", condition
                ):
                    if fmt != "iri-array" or condition.endswith("-value-0"):
                        dependencies.append((None, "table-format-value"))
            for index, step in enumerate(case.get("steps", [case])):
                req = step["request"]
                if any(
                    request_matches(s, req, step["expect"])
                    for s in decision.get("request_selectors", [])
                ):
                    dependencies.append((index, "request-context"))
                if any(
                    req.get("query", {}).get(key) in values
                    for key, values in decision.get("query_literals", {}).items()
                ):
                    dependencies.append((index, "curated-query-literal"))
            for index, reason in dependencies:
                # Curated/table vectors are single-step cases; if that changes,
                # binding the complete case avoids guessing which step matters.
                pointer = "" if index is None and "steps" in case else "/expect"
                if index is not None and "steps" in case:
                    pointer = f"/steps/{index}/expect"
                expectation = case
                for token in pointer.lstrip("/").split("/") if pointer else []:
                    expectation = (
                        expectation[int(token)]
                        if isinstance(expectation, list)
                        else expectation[token]
                    )
                item = dict(
                    interpretation=decision["id"],
                    pointer=pointer,
                    assertion_sha256=value_digest(expectation),
                    reason=reason,
                )
                if item not in bindings.setdefault(name, []):
                    bindings[name].append(item)
    return dict(
        profile=policy["profile"],
        policy_sha256=file_digest(root / POLICY),
        analyzer_sha256=file_digest(root / "scripts/ci/report_lrs_interpretations.py"),
        published_source_record_sha256=file_digest(
            root / policy["published_source_record"]
        ),
        report_schema_sha256=file_digest(
            root / "spec/conformance/schemas/conformance-run-report.schema.json"
        ),
        dependencies=bindings,
    )


def classify_report(root, report_path, receipt_path, cases, bindings):
    from jsonschema import Draft202012Validator, ValidationError

    report = json.loads(report_path.read_text())
    schema = json.loads(
        (
            root / "spec/conformance/schemas/conformance-run-report.schema.json"
        ).read_text()
    )
    try:
        Draft202012Validator(schema).validate(report)
    except ValidationError as error:
        raise ValueError(
            f"execution report violates its contract: {error.message}"
        ) from error
    receipt = json.loads(receipt_path.read_text())
    if file_digest(report_path) not in receipt["artifacts_sha256"].values():
        raise ValueError("execution report bytes are not bound by the evidence receipt")
    seal = load_yaml(root / "spec/conformance/lrs-assertion-manifest.yaml")
    for path, digest in seal["execution_contract_files"].items():
        if receipt["input_sha256"].get(path) != digest:
            raise ValueError(f"execution contract not bound to current bytes: {path}")
    outcomes = {}
    case_file_hashes = {
        path: file_digest(root / path) for path in {p for p, _ in cases.values()}
    }
    groups = {
        name: Counter()
        for name in ["no_registered_dependency", "interpretation_dependent"]
    }
    for suite in report["suites"]:
        for case in suite["cases"]:
            name = case["id"]
            if name not in cases or name in outcomes:
                raise ValueError(f"unknown or duplicate execution case {name}")
            path, source = cases[name]
            if (
                suite["path"] != path
                or case["requirements"] != source["requirements"]
                or case["safety"] != source["safety"]
            ):
                raise ValueError(
                    f"execution case identity differs from current case: {name}"
                )
            if receipt["input_sha256"].get(path) != case_file_hashes[path]:
                raise ValueError(
                    f"execution case file not bound to current bytes: {path}"
                )
            if (case["outcome"] == "failed") != bool(case["failures"]):
                raise ValueError(f"outcome/failure mismatch: {name}")
            if (case["outcome"] == "not-run") != bool(case.get("not_run_reasons")):
                raise ValueError(f"not-run/reason mismatch: {name}")
            bucket = (
                "interpretation_dependent"
                if name in bindings["dependencies"]
                else "no_registered_dependency"
            )
            groups[bucket][case["outcome"]] += 1
            outcomes[name] = dict(
                outcome=case["outcome"],
                failures=case["failures"],
                not_run_reasons=case.get("not_run_reasons", []),
                diagnostic_statuses=case.get("diagnostic_statuses", []),
                classification=bucket,
                interpretations=sorted(
                    {
                        b["interpretation"]
                        for b in bindings["dependencies"].get(name, [])
                    }
                ),
            )
    totals = dict(
        total=len(outcomes),
        **{
            s: sum(c[s] for c in groups.values())
            for s in ["passed", "failed", "validated"]
        },
    )
    if report["schema_version"] >= 2:
        totals["not_run"] = sum(c["not-run"] for c in groups.values())
    if totals != report["totals"]:
        raise ValueError("execution totals do not match case outcomes")
    expected_status = (
        "failed"
        if totals["failed"]
        else "validated"
        if report["mode"] == "validate-only"
        else "incomplete"
        if totals.get("not_run", 0)
        else "passed"
    )
    if report["status"] != expected_status or any(
        c["outcome"] == "validated"
        if report["mode"] == "execute-http"
        else c["outcome"] != "validated"
        for c in outcomes.values()
    ):
        raise ValueError("execution mode/status does not match case outcomes")
    return dict(
        report_sha256=file_digest(report_path),
        receipt_sha256=file_digest(receipt_path),
        evidence_kind="qualified-existing-execution"
        if report["mode"] == "execute-http"
        else "validate-only",
        raw_status=report["status"],
        raw_totals=totals,
        groups={key: dict(value) for key, value in groups.items()},
        cases_absent_from_report=sorted(set(cases) - set(outcomes)),
        cases=outcomes,
        conformance_verdict="not-issued",
        interpretation_attribution="Whole-case dependency, including setup/observation; does not prove a particular failure was caused by an interpretation",
    )


def main():
    from audit_lrs_source_projection import DEFAULT_SOURCE, audit

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--skip-source", action="store_true")
    parser.add_argument("--run-report", type=Path)
    parser.add_argument("--evidence-receipt", type=Path)
    parser.add_argument("--require-complete-report", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    if bool(args.run_report) != bool(args.evidence_receipt):
        parser.error("--run-report and --evidence-receipt must be supplied together")
    if args.require_complete_report and not args.run_report:
        parser.error("--require-complete-report requires a run report")
    try:
        audit_result = audit(None if args.skip_source else args.source, False)
        cases = case_documents(ROOT)
        bindings = interpretation_bindings(ROOT)
        policy = load_yaml(ROOT / POLICY)
        counts = Counter(
            b["interpretation"]
            for deps in bindings["dependencies"].values()
            for b in deps
        )
        result = dict(
            schema_version=1,
            status="interpretation-dependencies-checked",
            profile=bindings["profile"],
            policy_sha256=bindings["policy_sha256"],
            source_bytes_verified=audit_result["source_bytes_verified"],
            source_sha256=audit_result["source_sha256"],
            cases=len(cases),
            cases_with_registered_dependency=len(bindings["dependencies"]),
            decisions={
                d["id"]: dict(
                    assertion_dependencies=counts[d["id"]],
                    exercise_status="dependent-assertions-bound"
                    if counts[d["id"]]
                    else "no-dependent-case",
                    official_adjudication=d["official_adjudication"],
                    unresolved_probe=d.get("unresolved_probe"),
                )
                for d in policy["decisions"]
            },
            meaning="No registered dependency is not a certainty/completeness verdict. Raw failures are retained; no case is waived.",
            whole_standard_completeness="not-established",
            dependencies=bindings["dependencies"],
        )
        if args.run_report:
            result["execution"] = classify_report(
                ROOT, args.run_report, args.evidence_receipt, cases, bindings
            )
            if (
                args.require_complete_report
                and result["execution"]["cases_absent_from_report"]
            ):
                raise ValueError(
                    "execution report does not contain the complete current case inventory"
                )
    except (ValueError, KeyError, FileNotFoundError) as error:
        raise SystemExit(str(error)) from error
    if args.json:
        print(json.dumps(result, indent=2))
    else:
        print(f"Profile: {result['profile']}")
        print(
            f"Registered dependencies: {result['cases_with_registered_dependency']}/{result['cases']} cases"
        )
        for name, decision in result["decisions"].items():
            print(
                f"- {name}: {decision['assertion_dependencies']} assertion dependencies"
            )
        if "execution" in result:
            print(
                f"Raw outcome: {result['execution']['raw_status']}; {result['execution']['raw_totals']}"
            )
            print(f"Classified outcomes: {result['execution']['groups']}")
        print(result["meaning"])


if __name__ == "__main__":
    main()
