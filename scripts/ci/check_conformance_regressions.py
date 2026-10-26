#!/usr/bin/env python3
"""Compare complete HTTP results with explicit, temporary integration gaps.

This gate never changes the runner's outcomes or establishes LRS conformance.
The strict run_*_conformance.sh entry points retain their original exit policy.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

import jsonschema
import yaml

ROOT = Path(__file__).resolve().parents[2]
BASELINE = "spec/conformance/ci-execution-baseline.json"
SCHEMA = "spec/conformance/schemas/conformance-run-report.schema.json"
BACKENDS = {"memory", "postgres"}
STANDARD = "IEEE/ISO/IEC 39274-1-1-2025"  # Existing report protocol label.
INITIAL_VISIBILITY_DELAY = "Statement not yet retrievable within the configured observation window; no normative availability deadline is specified"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def digest(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True,
                                    separators=(",", ":")).encode()).hexdigest()


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path):
    return json.loads(path.read_text(), object_pairs_hook=unique_object)


def definitions(root):
    """Enumerate the entire current suite, including normally passing cases."""
    cases, suites, files = {}, {}, {}
    for path in sorted((root / "tests/conformance/http").rglob("*.yaml")):
        relative = path.relative_to(root).as_posix()
        files[relative] = sha(path)
        document = yaml.load(path.read_text(), Loader=yaml.CSafeLoader)
        suite = document["suite"]["id"]
        require(suite not in suites, f"duplicate definition suite: {suite}")
        suites[suite] = relative
        for case in document["cases"]:
            key = (suite, case["id"])
            require(key not in cases, f"duplicate definition case: {key}")
            cases[key] = case
    require(cases, "empty case inventory")
    return cases, suites, files


def expectations(root, backend):
    require(backend in BACKENDS, "unknown backend")
    baseline = load_json(root / BASELINE)
    require(baseline["schema_version"] == 1 and
            baseline["claim"] == "known-gap-regression-only", "unsupported baseline")
    cases, suites, files = definitions(root)
    require(files == baseline["case_files"], "baseline case-file drift")
    require(len(cases) == baseline["case_count"], "baseline case inventory drift")
    profiles = ["spec/conformance/endpoint-profile.example.yaml",
                "spec/conformance/auth-profile.example.yaml"]
    require(baseline["profiles"] == {p: sha(root / p) for p in profiles},
            "baseline profile drift")
    expected, seen = {}, set()
    for entry in baseline["gaps"]:
        require(set(entry) == {"suite", "case", "case_sha256", "backends", "outcome",
                               "failures", "not_run_reasons", "finding"},
                "unknown or missing gap field")
        key = (entry["suite"], entry["case"])
        require(key in cases and key not in seen, f"unknown/duplicate baseline case: {key}")
        seen.add(key)
        require(entry["case_sha256"] == digest(cases[key]), f"gap definition drift: {key}")
        require(isinstance(entry["finding"], str) and entry["finding"].strip(),
                f"missing gap disposition identifier: {key}")
        backends = entry["backends"]
        require(isinstance(backends, list) and backends and
                len(backends) == len(set(backends)) and set(backends) <= BACKENDS,
                f"invalid gap backends: {key}")
        outcome = entry["outcome"]
        failures, reasons = entry["failures"], entry["not_run_reasons"]
        require(isinstance(failures, list) and isinstance(reasons, list) and
                all(isinstance(x, str) and x for x in failures + reasons),
                f"invalid gap messages: {key}")
        require((outcome == "failed" and failures and not reasons) or
                (outcome == "not-run" and reasons and not failures),
                f"invalid gap outcome/reasons: {key}")
        if backend in backends:
            expected[key] = entry
    return baseline, cases, suites, expected


def attempt_reasons(attempt):
    """Use the runner's case-level spelling, including the exact step ID."""
    return [f"step {attempt['step']}: {failure}" if attempt['step'] is not None else failure
            for failure in attempt['failures']]


def resolved_initial_delay(attempts, index, definition, allowed_reasons):
    """Exempt only a schema-eligible initial 404 resolved by the same read step."""
    attempt = attempts[index]
    if (attempt.get('pending_visibility') is not True or attempt.get('status') != 404
            or attempt.get('failures') != [INITIAL_VISIBILITY_DELAY]):
        return False
    step = attempt['step']
    matching = [s for s in definition.get('steps', []) if s.get('id') == step]
    operation = definition if step is None else matching[0] if len(matching) == 1 else {}
    request, expect = operation.get('request', {}), operation.get('expect', {})
    if not (request.get('method') == 'GET' and request.get('path') == '/statements'
            and type(request.get('await_retrieval_ms')) is int and request['await_retrieval_ms'] > 0
            and {'statementId', 'voidedStatementId'} & set(request.get('query', {}))
            and expect.get('status') == 200):
        return False
    if any(a['step'] == step and not a['pending_visibility']
           for a in attempts[:index]):
        return False
    for later in attempts[index + 1:]:
        if later['step'] == step and not later['pending_visibility']:
            # Visibility can be established even when a separately reviewed
            # oracle/product gap remains. Trust only this case's baseline;
            # compare() also requires its full outcome and reasons to match.
            return later['status'] == 200 and all(
                reason in allowed_reasons for reason in attempt_reasons(later))
    return False


def compare(report, runner_exit, cases, suites, expected):
    """Fail closed on missing cases, new failures, changed reasons or stale gaps."""
    require(report["schema_version"] == 2 and report["standard"] == STANDARD and
            report["mode"] == "execute-http", "expected current HTTP execution report")
    require(type(runner_exit) is int and runner_exit in (0, 1), "runner infrastructure exit")
    found, found_suites, totals = set(), set(), Counter()
    for suite in report["suites"]:
        sid = suite["id"]
        require(sid in suites and sid not in found_suites, f"unknown/duplicate report suite: {sid}")
        require(Path(suite["path"]).as_posix() == suites[sid], f"report suite path mismatch: {sid}")
        found_suites.add(sid)
        for case in suite["cases"]:
            key = (sid, case["id"])
            require(key in cases and key not in found, f"unknown/duplicate report case: {key}")
            found.add(key)
            definition = cases[key]
            require(case["requirements"] == definition["requirements"] and
                    case["safety"] == definition["safety"], f"report case metadata drift: {key}")
            gap = expected.get(key, {})
            outcome = gap.get("outcome", "passed")
            require(case["outcome"] == outcome,
                    f"unexpected outcome or stale baseline: {key}: {case['outcome']} != {outcome}")
            require(key in expected or not case.get("unresolved_checks"),
                    f"unresolved checks outside known gaps: {key}")
            require(Counter(case["failures"]) == Counter(gap.get("failures", [])) and
                    Counter(case.get("not_run_reasons", [])) == Counter(gap.get("not_run_reasons", [])),
                    f"changed failure/unavailability reasons: {key}")
            # An excluded status or hidden retrieval failure cannot count as a
            # clean passing result merely because the top-level label says PASS.
            require(not case.get("diagnostic_statuses"), f"unreviewed diagnostic status: {key}")
            attempts = case.get("retrieval_attempts", [])
            allowed_reasons = gap.get("failures", []) + gap.get("not_run_reasons", [])
            for index, attempt in enumerate(attempts):
                if resolved_initial_delay(attempts, index, definition, allowed_reasons):
                    continue
                for message in attempt_reasons(attempt):
                    require(message in allowed_reasons,
                            f"retrieval-attempt violation: {key}")
            totals[outcome.replace("-", "_")] += 1
    require(found == set(cases) and found_suites == set(suites), "missing report cases or suites")
    calculated = {name: totals[name] for name in ("passed", "failed", "validated", "not_run")}
    calculated["total"] = len(found)
    require(report["totals"] == calculated, "inconsistent report totals")
    status = "failed" if totals["failed"] else "incomplete" if totals["not_run"] else "passed"
    require(report["status"] == status, "inconsistent report status")
    require(runner_exit == (0 if status == "passed" else 1), "runner exit/report mismatch")
    return {"schema_version": 1, "claim": "known-gap-regression-only",
            "regression_gate": "accepted", "conformance_complete": False,
            "runner_exit": runner_exit, "runner_status": status, "totals": calculated,
            "known_gaps": [list(key) for key in sorted(expected)]}


def check(root, backend, report_path, runner_exit):
    _, cases, suites, expected = expectations(root, backend)
    report = load_json(report_path)
    schema = load_json(root / SCHEMA)
    jsonschema.Draft202012Validator(schema).validate(report)
    result = compare(report, runner_exit, cases, suites, expected)
    result.update(backend=backend, baseline_sha256=sha(root / BASELINE),
                  report_sha256=sha(report_path), artifact_lock_sha256=sha(
                      root / "spec/conformance/lrs-generated-artifacts.lock.json"))
    return result


def capture_inputs(root, backend):
    paths = [BASELINE, SCHEMA, 'spec/conformance/lrs-generated-artifacts.lock.json',
             'scripts/ci/check_conformance_regressions.py', 'scripts/ci/conformance_lib.sh',
             f'scripts/ci/run_{backend}_conformance.sh']
    return {p: sha(root / p) for p in paths}


def capture(root, backend, output):
    """Persist an execution regardless of its verdict; this is not a gate."""
    require(backend in BACKENDS, "unknown backend")
    output.mkdir(parents=True, exist_ok=True)
    require(not any((output / f"xapi-{backend}-{name}").exists()
                    for name in ("http.json", "http.xml", "regression.json", "execution.json", "execution.log")),
            "fresh run requires unused output paths")
    before = capture_inputs(root, backend)
    env = dict(os.environ)
    require(not env.get("KIVA_CONFORMANCE_PROCEDURE_REPORT_ROOT"),
            "procedure execution is outside the HTTP integration baseline")
    errors = []
    status = None
    with tempfile.TemporaryDirectory(prefix="conformance-run-", dir=output) as temporary:
        raw = Path(temporary) / "http.json"
        junit = Path(temporary) / "http.xml"
        env.update(KIVA_CONFORMANCE_JSON_REPORT=str(raw), KIVA_CONFORMANCE_JUNIT_REPORT=str(junit))
        with (output / f'xapi-{backend}-execution.log').open('w') as log:
            try:
                status = subprocess.run(['bash', f'scripts/ci/run_{backend}_conformance.sh'],
                                        cwd=root, env=env, check=False, stdout=log,
                                        stderr=subprocess.STDOUT).returncode
            except OSError as error:
                errors.append(f'runner launch failed: {error}')
        for source, suffix in [(raw, 'http.json'), (junit, 'http.xml')]:
            if source.is_file():
                source.replace(output / f'xapi-{backend}-{suffix}')
            else:
                errors.append(f'runner did not emit complete report artifacts: {suffix}')
    if before != capture_inputs(root, backend):
        errors.append('execution input drift')
    artifacts = {f'xapi-{backend}-{suffix}': sha(output / f'xapi-{backend}-{suffix}')
                 for suffix in ['http.json', 'http.xml', 'execution.log']
                 if (output / f'xapi-{backend}-{suffix}').is_file()}
    receipt = dict(schema_version=1, claim='execution-capture-only', backend=backend,
                   runner_exit=status, inputs=before, artifacts=artifacts, errors=errors)
    (output / f'xapi-{backend}-execution.json').write_text(json.dumps(receipt, indent=2) + '\n')
    return receipt


def gate(root, backend, captured):
    receipt = load_json(captured / f'xapi-{backend}-execution.json')
    require(receipt['schema_version'] == 1 and receipt['claim'] == 'execution-capture-only'
            and receipt['backend'] == backend, 'invalid capture receipt')
    require(not receipt['errors'], f"incomplete execution capture: {receipt['errors']}")
    require(receipt['inputs'] == capture_inputs(root, backend), 'capture input drift')
    artifacts = {f'xapi-{backend}-{suffix}' for suffix in ['http.json', 'http.xml', 'execution.log']}
    require(set(receipt['artifacts']) == artifacts, 'incomplete capture artifact inventory')
    require(receipt['artifacts'] == {name: sha(captured / name) for name in artifacts},
            'captured report drift')
    result = check(root, backend, captured / f'xapi-{backend}-http.json', receipt['runner_exit'])
    result.update(evidence_kind='captured-wrapper-execution', inputs=receipt['inputs'],
                  capture_receipt_sha256=sha(captured / f'xapi-{backend}-execution.json'),
                  junit_sha256=receipt['artifacts'][f'xapi-{backend}-http.xml'])
    return result


def run(root, backend, output):
    """Strict convenience entry point; capture remains available on rejection."""
    capture(root, backend, output)
    return gate(root, backend, output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "capture", "gate", "check", "verify"])
    parser.add_argument("--backend", required=True, choices=sorted(BACKENDS))
    parser.add_argument("--report", type=Path)
    parser.add_argument("--runner-exit", type=int)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--capture-dir", type=Path)
    args = parser.parse_args()
    if args.action == "verify":
        _, cases, _, expected = expectations(ROOT, args.backend)
        print(json.dumps({"cases": len(cases), "known_gaps": len(expected),
                          "execution": "not-run", "conformance_complete": False}))
        return
    require(args.output_dir is not None, "output directory is required")
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    require(not (output / f"xapi-{args.backend}-regression.json").exists(),
            "regression decision already exists; use a fresh output directory")
    if args.action in {'run', 'capture'}:
        require(args.report is None and args.runner_exit is None and args.capture_dir is None,
                "fresh run cannot accept a supplied report or exit status")
    if args.action == 'capture':
        result = capture(ROOT, args.backend, output)
        print(json.dumps({'claim': result['claim'], 'runner_exit': result['runner_exit'],
                          'errors': result['errors']}))
        print('Execution artifacts saved; no regression or conformance acceptance.')
        return
    try:
        result = evaluate(args, output)
    except (ValueError, KeyError, TypeError, OSError, jsonschema.ValidationError) as error:
        (output / f'xapi-{args.backend}-regression.json').write_text(json.dumps({
            'schema_version': 1, 'claim': 'known-gap-regression-only',
            'regression_gate': 'rejected', 'conformance_complete': False,
            'error': str(error)}, indent=2) + '\n')
        raise
    (output / f"xapi-{args.backend}-regression.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result["totals"], sort_keys=True))
    print("Integration regression gate accepted; LRS conformance is not established.")


def evaluate(args, output):
    if args.action == "run":
        result = run(ROOT, args.backend, output)
    elif args.action == 'gate':
        require(args.capture_dir is not None and args.report is None and args.runner_exit is None,
                'gate requires a capture directory; no supplied report or exit status')
        result = gate(ROOT, args.backend, args.capture_dir.resolve())
    else:
        require(args.report is not None and args.runner_exit is not None,
                "comparison requires a report and original runner exit")
        result = check(ROOT, args.backend, args.report, args.runner_exit)
        result["evidence_kind"] = "report-comparison-only"
    return result


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError, jsonschema.ValidationError) as error:
        raise SystemExit(str(error)) from error
