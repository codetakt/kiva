#!/usr/bin/env python3
"""Execute bounded conforming/one-defect controls for the evaluation repairs."""

import argparse
import hashlib
import json
import socket
import subprocess
import sys
import time
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]


def port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runner", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--expectations", type=Path, default=ROOT / "tests/conformance/controls/evaluation_expectations.json")
    parser.add_argument("--model", action="append", help="Select named controls; omit to run every declared model")
    args = parser.parse_args()
    contract = json.loads(args.expectations.read_text())
    if contract["schema_version"] != 1:
        raise SystemExit("unsupported control contract version")
    models = contract["models"]
    selected = args.model or list(models)
    if set(selected) - models.keys():
        raise SystemExit("unknown control selection")
    args.out.mkdir(parents=True, exist_ok=True)
    if (args.out / "summary.json").exists():
        raise SystemExit("use a fresh output directory")
    from lrs_projection_evaluation_repairs import evaluation_repair_cases
    from lrs_projection_followup import followup_cases
    new_ids = {c["id"] for c in [*evaluation_repair_cases(), *followup_cases()]}
    cases = {}
    for path in (ROOT / "tests/conformance/http").rglob("*.yaml"):
        for case in yaml.load(path.read_text(), Loader=yaml.CSafeLoader)["cases"]:
            name = case["id"]
            if name in new_ids or name in {"statement-member-order-is-unrestricted", "statement-root-id-omitted-accepted", "statement-unknown-local-offset-policy", "statement-accepts-semantic-version-outside-legacy-adl-range", "attachment-attachments-0-fileUrl-omitted-accepted", "attachment-object-attachments-0-fileUrl-omitted-accepted", "attachment-put-reject-json-without-file-url"} or (name.startswith("interaction-") and any("-" + field + "-" in name for field in ["scale", "source", "target", "steps"]) and name.endswith(("-accepted", "-omitted-accepted", "-empty-array-accepted"))) or (name.startswith(("context-agent-", "context-group-")) and "relevantTypes" in name and name.endswith(("-value-0", "-accepted", "-omitted-accepted"))):
                cases[name] = case
    suite = args.out / "witnesses.yaml"
    suite.write_text(yaml.safe_dump({"suite": {"id": "evaluation-repair-controls", "standard": "IEEE/ISO/IEC 39274-1-1-2025", "description": "Bounded evaluation repair witnesses"}, "cases": list(cases.values())}, sort_keys=False))
    proxy = ROOT / "tests/conformance/controls/evaluation_proxy.py"
    reference = ROOT / "tests/conformance/controls/evaluation_reference.py"
    for name, model in models.items():
        expected = model["expected_failed_cases"]
        if len(expected) != len(set(expected)) or set(expected) - cases.keys():
            raise SystemExit("invalid expected case set for " + name)
        if model["transport"] not in ["reference", "proxy"]:
            raise SystemExit("invalid model transport for " + name)
    results = {}
    for mutation in selected:
        model = models[mutation]
        upstream_port, proxy_port = port(), port()
        trace = args.out / (mutation + ".trace.jsonl")
        log = (args.out / (mutation + ".server.log")).open("w")
        processes = []
        try:
            processes.append(subprocess.Popen([sys.executable, str(reference), str(upstream_port), "--trace", str(trace), "--mutation", mutation if model["transport"] == "reference" else "none"], stdout=log, stderr=log))
            endpoint_port = upstream_port
            if model["transport"] == "proxy":
                processes.append(subprocess.Popen([sys.executable, str(proxy), str(proxy_port), str(upstream_port), mutation], stdout=log, stderr=log))
                endpoint_port = proxy_port
            for _ in range(100):
                try:
                    with socket.create_connection(("127.0.0.1", endpoint_port), timeout=0.1):
                        break
                except OSError:
                    time.sleep(0.02)
            run = subprocess.run([str(args.runner.resolve()), "--case", str(suite), "--endpoint", f"http://127.0.0.1:{endpoint_port}", "--allow-destructive", "--json"], capture_output=True, text=True)
            (args.out / (mutation + ".json")).write_text(run.stdout)
            (args.out / (mutation + ".stderr")).write_text(run.stderr)
            report = json.loads(run.stdout)
            outcomes = {c["id"]: c["outcome"] for s in report["suites"] for c in s["cases"]}
            results[mutation] = {"exit_code": run.returncode, "totals": report["totals"], "failed_cases": [name for name, outcome in outcomes.items() if outcome == "failed"], "report_sha256": hashlib.sha256(run.stdout.encode()).hexdigest()}
            print(mutation, report["totals"], flush=True)
            expected = set(model["expected_failed_cases"])
            actual = set(results[mutation]["failed_cases"])
            results[mutation]["expected_failed_cases"] = sorted(expected)
            results[mutation]["exact_failure_set_match"] = actual == expected
            if actual != expected:
                raise RuntimeError(f"{mutation}: failure set mismatch; missing={sorted(expected - actual)}; unexpected={sorted(actual - expected)}")
            if set(outcomes.values()) - {"passed", "failed"} or len(outcomes) != len(cases):
                raise RuntimeError("control did not execute every witness: " + mutation)
            if run.returncode != (1 if expected else 0) or report["totals"]["not_run"] or report["totals"]["validated"]:
                raise RuntimeError("invalid reference outcome/exit status: " + mutation)
        finally:
            for process in reversed(processes):
                process.terminate()
                process.wait(timeout=5)
            log.close()
    property_order_transport = "not checked: conforming baseline not selected"
    if "none" in selected:
        traces = [json.loads(line) for line in (args.out / "none.trace.jsonl").read_text().splitlines()]
        order_ids = {s["request"]["query"]["statementId"] for s in cases["statement-member-order-is-unrestricted"]["steps"][:2]}
        transmitted = [bytes.fromhex(t["request_body_hex"]) for t in traces if t["method"] == "PUT" and t["query"].get("statementId") in order_ids]
        normalized = [{key: value for key, value in json.loads(raw).items() if key != "id"} for raw in transmitted]
        assert len(order_ids) == 2 and len(transmitted) == 2 and transmitted[0] != transmitted[1] and list(json.loads(transmitted[0])) != list(json.loads(transmitted[1])) and normalized[0] == normalized[1], "property-order transport must differ without a duplicate-id confound"
        property_order_transport = "two unequal raw bodies, distinct ids, equal parsed JSON except id; both accepted"
    for mutation in ["none", "reject-unknown-local-offset", "unknown-local-offset-not-normalized"]:
        if mutation not in selected:
            continue
        traces = [json.loads(line) for line in (args.out / (mutation + ".trace.jsonl")).read_text().splitlines()]
        identifier = cases["statement-unknown-local-offset-policy"]["steps"][0]["request"]["body"]["id"]
        reads = [t for t in traces if t["method"] == "GET" and t["query"].get("statementId") == identifier]
        if (mutation == "reject-unknown-local-offset" and reads) or (mutation != "reject-unknown-local-offset" and not reads):
            raise RuntimeError("unknown-offset readback must occur only after acceptance: " + mutation)
    result = {"scope": "bounded Suite design controls; not a target LRS verdict or whole-standard closure", "cases": len(cases), "property_order_transport": property_order_transport, "controls": results, "input_sha256": {str(p.relative_to(ROOT)) if p.is_relative_to(ROOT) else str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in [reference, proxy, args.expectations, args.runner, suite]}}
    (args.out / "summary.json").write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
