#!/usr/bin/env python3
"""Run complete cases on isolated backends, retaining every shard's raw evidence.

This is an execution strategy, not an acceptance gate. The existing strict gate
still evaluates the combined report against the entire known-gap baseline.
"""

from __future__ import annotations

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import copy
import json
import os
from pathlib import Path
import subprocess
import xml.etree.ElementTree as ET

from check_conformance_regressions import (
    BACKENDS, ROOT, SCHEMA, STANDARD, definitions, load_json, require,
)
import jsonschema


def partitions(cases, workers):
    require(type(workers) is int and 2 <= workers <= 8, "workers must be between 2 and 8")
    require(len(cases) >= workers, "each worker requires at least one case")
    identifiers = [key[1] for key in cases]
    require(len(set(identifiers)) == len(identifiers), "case IDs must be globally unique")
    require(all(identifier and "\n" not in identifier and "\r" not in identifier
                for identifier in identifiers), "invalid case selection identifier")
    keys = list(cases)
    return [keys[index::workers] for index in range(workers)]


def totals_and_status(cases):
    counts = Counter(case["outcome"].replace("-", "_") for case in cases)
    require(not (set(counts) - {"passed", "failed", "not_run"}), "non-execution case outcome")
    totals = {key: counts[key] for key in ("passed", "failed", "validated", "not_run")}
    totals["total"] = len(cases)
    status = "failed" if counts["failed"] else "incomplete" if counts["not_run"] else "passed"
    return totals, status


def merge_reports(reports, exits, groups, cases, suites):
    """Require exact assignment coverage and retain each original case verbatim."""
    require(len(reports) == len(exits) == len(groups), "missing shard report or exit")
    require([key for group in groups for key in group]
            and Counter(key for group in groups for key in group) == Counter(cases.keys()),
            "shard assignments must cover each case exactly once")
    observed = {}
    for report, exit_code, group in zip(reports, exits, groups, strict=True):
        require(report["schema_version"] == 2 and report["standard"] == STANDARD
                and report["mode"] == "execute-http", "invalid shard report metadata")
        found, found_suites = {}, set()
        for suite in report["suites"]:
            sid = suite["id"]
            require(sid in suites and sid not in found_suites, "unknown or duplicate shard suite")
            require(suite["path"] == suites[sid], "shard suite path drift")
            found_suites.add(sid)
            for case in suite["cases"]:
                key = (sid, case["id"])
                require(key in cases and key not in found and key not in observed,
                        "unknown or duplicate shard case")
                require(case["requirements"] == cases[key]["requirements"]
                        and case["safety"] == cases[key]["safety"], "shard case metadata drift")
                found[key] = case
        require(set(found) == set(group) and found_suites == {key[0] for key in group},
                "shard report differs from its assigned cases")
        totals, status = totals_and_status(list(found.values()))
        require(report["totals"] == totals and report["status"] == status,
                "shard totals/status mismatch")
        require(type(exit_code) is int and exit_code == (0 if status == "passed" else 1),
                "shard exit/report mismatch")
        observed.update(found)
    require(set(observed) == set(cases), "missing aggregate cases")
    totals, status = totals_and_status(list(observed.values()))
    return dict(schema_version=2, standard=STANDARD, mode="execute-http", status=status,
                totals=totals, suites=[dict(id=sid, path=path, cases=[observed[key]
                    for key in cases if key[0] == sid]) for sid, path in suites.items()])


def properties(element):
    items = element.findall("./properties/property")
    result = {item.attrib["name"]: item.attrib["value"] for item in items}
    require(len(result) == len(items), "duplicate JUnit property")
    return result


def verify_aggregate(directory, workers, report, junit_path, cases, suites, files, validator):
    """Independently reassemble bound raw shards when enforcing the gate."""
    groups = partitions(cases, workers)
    expected = dict(schema_version=1, strategy="ordered-round-robin-v1", workers=workers,
                    case_files=files, assignments=[[list(key) for key in group] for group in groups])
    require(load_json(directory / "plan.json") == expected, "shard execution plan drift")
    reports, exits, documents = [], [], []
    for index, group in enumerate(groups):
        child = directory / f"{index:02}"
        require((child / "case-ids.txt").read_text() == "".join(key[1] + "\n" for key in group),
                "shard case selection drift")
        result = load_json(child / "result.json")
        require(result.get("index") == index and set(result) == {"index", "runner_exit"},
                "shard execution result mismatch")
        raw = load_json(child / "http.json")
        validator.validate(raw)
        reports.append(raw)
        exits.append(result["runner_exit"])
        try:
            documents.append(ET.parse(child / "http.xml").getroot())
        except ET.ParseError as error:
            raise ValueError(f"invalid shard JUnit: {index}") from error
    require(merge_reports(reports, exits, groups, cases, suites) == report,
            "aggregate JSON differs from raw shard observations")
    combined = merge_junit(documents, reports, report)
    try:
        actual = ET.parse(junit_path).getroot()
    except ET.ParseError as error:
        raise ValueError("invalid aggregate JUnit") from error
    ET.indent(combined, space="  ")
    ET.indent(actual, space="  ")
    require(ET.tostring(actual) == ET.tostring(combined),
            "aggregate JUnit differs from raw shard observations")


def merge_junit(documents, reports, merged):
    """Copy raw testcase elements, checking their identities and verdicts."""
    require(len(documents) == len(reports), "missing shard JUnit")
    elements = {}
    for document, report in zip(documents, reports, strict=True):
        require(document.tag == "testsuites" and document.get("name") == STANDARD,
                "invalid shard JUnit root")
        require(document.get("tests") == str(report["totals"]["total"])
                and document.get("failures") == str(report["totals"]["failed"]),
                "shard JUnit totals mismatch")
        require(properties(document) == {"schema_version": "2", "standard": STANDARD,
                "mode": "execute-http", "status": report["status"]}, "shard JUnit metadata drift")
        expected = {(suite["id"], case["id"]): case for suite in report["suites"]
                    for case in suite["cases"]}
        suites = {suite["id"]: suite for suite in report["suites"]}
        seen, seen_suites = set(), set()
        for suite in document.findall("testsuite"):
            sid = suite.get("name")
            require(sid in suites and sid not in seen_suites, "unknown/duplicate JUnit suite")
            seen_suites.add(sid)
            rows = suites[sid]["cases"]
            require(properties(suite) == {"path": suites[sid]["path"]}
                    and suite.get("tests") == str(len(rows))
                    and suite.get("failures") == str(sum(c["outcome"] == "failed" for c in rows)),
                    "shard JUnit suite mismatch")
            for element in suite.findall("testcase"):
                key = (element.get("classname"), element.get("name"))
                require(key[0] == sid and key in expected and key not in seen and key not in elements,
                        "unknown/duplicate JUnit case")
                case = expected[key]
                require(properties(element) == {"outcome": case["outcome"],
                        "requirements": ",".join(case["requirements"])}, "JUnit case metadata drift")
                require([(failure.get("message"), failure.text) for failure in element.findall("failure")]
                        == [(message, message) for message in case["failures"]]
                        and [skip.get("message") for skip in element.findall("skipped")]
                        == case.get("not_run_reasons", []), "JUnit diagnostics mismatch")
                diagnostics = element.findall("property")
                require(len(diagnostics) == 1 and diagnostics[0].get("name") == "diagnostic_statuses"
                        and json.loads(diagnostics[0].attrib["value"]) == case.get("diagnostic_statuses", []),
                        "JUnit diagnostic statuses mismatch")
                seen.add(key)
                elements[key] = copy.deepcopy(element)
        require(seen == set(expected) and seen_suites == set(suites), "missing JUnit cases/suites")
    result = ET.Element("testsuites", name=STANDARD, tests=str(merged["totals"]["total"]),
                        failures=str(merged["totals"]["failed"]))
    props = ET.SubElement(result, "properties")
    for name, value in {"schema_version": "2", "standard": STANDARD, "mode": "execute-http",
                        "status": merged["status"]}.items():
        ET.SubElement(props, "property", name=name, value=value)
    for row in merged["suites"]:
        suite = ET.SubElement(result, "testsuite", name=row["id"], tests=str(len(row["cases"])),
                              failures=str(sum(c["outcome"] == "failed" for c in row["cases"])))
        ET.SubElement(ET.SubElement(suite, "properties"), "property", name="path", value=row["path"])
        for case in row["cases"]:
            suite.append(elements[(row["id"], case["id"])])
    return result


def execute(root, backend, workers, output):
    output.mkdir(parents=True, exist_ok=False)
    # Generate once before workers share the read-only definitions. Cargo's own
    # lock protects reuse of the same release artifacts by the child wrappers.
    subprocess.run(["bash", "-euc", "source scripts/ci/conformance_lib.sh; kiva_build_conformance_binaries"],
                   cwd=root, check=True)
    cases, suites, files = definitions(root)
    groups = partitions(cases, workers)
    plan = dict(schema_version=1, strategy="ordered-round-robin-v1", workers=workers,
                case_files=files, assignments=[[list(key) for key in group] for group in groups])
    (output / "plan.json").write_text(json.dumps(plan, indent=2) + "\n")

    def run(index):
        directory = output / f"{index:02}"
        directory.mkdir()
        selected = directory / "case-ids.txt"
        selected.write_text("".join(key[1] + "\n" for key in groups[index]))
        env = dict(os.environ, KIVA_CONFORMANCE_ARTIFACTS_READY="1",
                   KIVA_CONFORMANCE_CASE_IDS_FILE=str(selected),
                   KIVA_CONFORMANCE_JSON_REPORT=str(directory / "http.json"),
                   KIVA_CONFORMANCE_JUNIT_REPORT=str(directory / "http.xml"))
        with (directory / "execution.log").open("w") as log:
            try:
                code = subprocess.run(["bash", f"scripts/ci/run_{backend}_conformance.sh"],
                                      cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT,
                                      check=False).returncode
                result = dict(index=index, runner_exit=code)
            except OSError as error:
                result = dict(index=index, runner_exit=None, error=str(error))
        (directory / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        return result

    with ThreadPoolExecutor(max_workers=workers) as pool:
        results = list(pool.map(run, range(workers)))
    # Do not stop other workers when a peer fails. All raw logs/reports survive
    # even when no complete aggregate can be produced.
    reports = [load_json(output / f"{index:02}/http.json") for index in range(workers)]
    validator = jsonschema.Draft202012Validator(load_json(root / SCHEMA))
    for report in reports:
        validator.validate(report)
    merged = merge_reports(reports, [r["runner_exit"] for r in results], groups, cases, suites)
    documents = [ET.parse(output / f"{index:02}/http.xml").getroot() for index in range(workers)]
    junit = merge_junit(documents, reports, merged)
    validator.validate(merged)
    Path(os.environ["KIVA_CONFORMANCE_JSON_REPORT"]).write_text(
        json.dumps(merged, ensure_ascii=False, indent=2) + "\n")
    ET.indent(junit, space="  ")
    ET.ElementTree(junit).write(os.environ["KIVA_CONFORMANCE_JUNIT_REPORT"],
                               encoding="utf-8", xml_declaration=True)
    return 0 if merged["status"] == "passed" else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--backend", required=True, choices=sorted(BACKENDS))
    parser.add_argument("--workers", required=True, type=int, choices=range(2, 9))
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    return execute(ROOT, args.backend, args.workers, args.out.resolve())


if __name__ == "__main__":
    raise SystemExit(main())
