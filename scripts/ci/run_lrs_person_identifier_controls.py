#!/usr/bin/env python3
"""Qualify the returned-Person URI defect using the existing full Memory Suite.

Common fixture preparation omits absent optional fields in both branches.
The sole branch difference is a valid or unterminated IP-literal URI.
The exact failed case/check sets are declared before execution. The bounded HTTP(S) locator provider resolves the valid Account branch;
malformed OpenIDs still require their own exact failure witnesses. Full-suite runs retain the memory gap baseline.
"""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import time

from check_conformance_regressions import compare, expectations, require

ROOT = Path(__file__).resolve().parents[2]
CASES = {'person-response-all-identifiers-' + field for field in ['mbox', 'mbox_sha1sum', 'openid', 'account']}
CHECKS = {'account-openid-0', 'mbox-openid-0', 'mbox_sha1sum-openid-0', 'openid-openid-1'}
ACCOUNT = 'person-response-all-identifiers-account'
PROCEDURE_GAPS = {
    'person-association-inventory': 'Partial executor checks presence and syntax only. Independently controlled no-additional-information and Agent association inventories require the manual person-semantic-identifiers procedure; random identifiers do not establish fallback applicability.',
}


def validate_control_results(report, exit_code, cases, suites, gaps, mode, focused):
    """Use exact outcomes/reasons; a known provider gap is not a general waiver."""
    require(mode in ('valid', 'invalid'), 'unknown control mode')
    targets = {key for key in cases if key[1] in CASES}
    require(len(targets) == len(CASES) and {key[1] for key in targets} == CASES,
            'missing or duplicate Person control definition')
    require(not targets & gaps.keys(), 'resolved Person provider cases must not remain baseline gaps')
    selected = {key: value for key, value in cases.items() if not focused or key in targets}
    selected_suites = {sid: path for sid, path in suites.items() if any(key[0] == sid for key in selected)}
    expected = {key: copy.deepcopy(value) for key, value in gaps.items() if key in selected}
    if mode == 'invalid':
        for key in targets:
            index = 1 if key[1].endswith('-openid') else 0
            expected[key] = {'outcome': 'failed', 'not_run_reasons': [],
                             'failures': [f'/openid/{index} is not valid uri direct syntax']}
    compare(report, exit_code, selected, selected_suites, expected)
    # The integration baseline deliberately does not bind unresolved checks for
    # known gaps. These finite Person controls require a narrower observation set.
    for suite in report['suites']:
        for case in suite['cases']:
            if (suite['id'], case['id']) in targets:
                unresolved = []
                require(case.get('unresolved_checks', []) == unresolved,
                        f'{case["id"]}: unexpected unresolved Person observation')
    return {case['id']: case for suite in report['suites'] for case in suite['cases']}


def validate_procedure_checks(checks, exit_code, mode):
    """Require all four malformed-OpenID witnesses and retain independent procedure gaps."""
    require(mode in ('valid', 'invalid'), 'unknown control mode')
    by_id = {check['id']: check for check in checks}
    require(len(by_id) == len(checks), 'duplicate procedure check')
    require(all(c['outcome'] in ('passed', 'failed', 'not-run') for c in checks),
            'unexpected procedure outcome')
    failed = {c['id'] for c in checks if c['outcome'] == 'failed'}
    require(failed == (CHECKS if mode == 'invalid' else set()), 'unexpected procedure failure set')
    require({c['id']: c.get('reason') for c in checks if c['outcome'] == 'not-run'} == PROCEDURE_GAPS,
            'unexpected unexecuted Person condition')
    require(CHECKS <= by_id.keys() and all(
        by_id[name]['outcome'] == ('failed' if mode == 'invalid' else 'passed') for name in CHECKS),
        'missing OpenID control witness')
    require(exit_code == (1 if mode == 'invalid' else 2), 'wrong procedure exit code')
    return failed


def port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


def ready(number, process):
    for _ in range(300):
        if process.poll() is not None:
            raise RuntimeError('Control server exited before readiness')
        try:
            with socket.create_connection(('127.0.0.1', number), timeout=.1):
                return
        except OSError:
            time.sleep(.02)
    raise RuntimeError('Control server not ready')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runner', required=True, type=Path)
    parser.add_argument('--server', required=True, type=Path)
    parser.add_argument('--out', required=True, type=Path)
    parser.add_argument('--focused', action='store_true', help='Only the four existing all-identifier cases')
    args = parser.parse_args()
    _, definitions, suites, gaps = expectations(ROOT, 'memory')
    args.out.mkdir(parents=True, exist_ok=False)
    env = os.environ.copy()
    env.pop('DATABASE_URL', None)
    env.update(KIVA_MAX_STATEMENT_BODY_BYTES='4096', KIVA_RATE_LIMIT_PATH='/__kiva/conformance/rate-limit',
               KIVA_RATE_LIMIT_REQUEST_COUNT='3', KIVA_RATE_LIMIT_WINDOW_SECONDS='1',
               KIVA_CONFORMANCE_WRITER_A_TOKEN='statement-writer-a',
               KIVA_CONFORMANCE_READER_TOKEN='statement-reader', KIVA_CONFORMANCE_INVALID_TOKEN='invalid-credential')
    results = {}
    reports = {}
    for mode in ['valid', 'invalid']:
        out = args.out / mode
        out.mkdir()
        upstream, front = port(), port()
        env['KIVA_LISTEN_ADDR'] = '127.0.0.1:' + str(upstream)
        processes = []
        with (out / 'server.log').open('w') as log:
            try:
                processes.append(subprocess.Popen([str(args.server.resolve())], env=env, cwd=ROOT, stdout=log, stderr=log))
                ready(upstream, processes[-1])
                proxy = ROOT / 'tests/conformance/controls/person_identifier_proxy.py'
                processes.append(subprocess.Popen([sys.executable, str(proxy), '--listen', str(front), '--upstream', str(upstream),
                                                   '--mode', mode, '--trace', str((out / 'mutation.trace.jsonl').resolve())], stdout=log, stderr=log))
                ready(front, processes[-1])
                endpoint = 'http://127.0.0.1:' + str(front)
                command = [str(args.runner.resolve()), '--endpoint', endpoint, '--allow-destructive', '--json',
                           '--endpoint-profile', 'spec/conformance/endpoint-profile.example.yaml',
                           '--auth-profile-file', 'spec/conformance/auth-profile.example.yaml']
                if args.focused:
                    for name in sorted(CASES):
                        command += ['--case-id', name]
                run = subprocess.run(command, capture_output=True, text=True, cwd=ROOT, env=env)
                (out / 'suite.json').write_text(run.stdout)
                (out / 'suite.stderr').write_text(run.stderr)
                report = json.loads(run.stdout)
                reports[mode] = validate_control_results(
                    report, run.returncode, definitions, suites, gaps, mode, args.focused)
                procedure = [sys.executable, str(ROOT / 'scripts/ci/run_lrs_procedures.py'), '--endpoint', endpoint,
                             '--runner', str(args.runner.resolve()), '--out', str((out / 'procedure').resolve()),
                             '--fixture', str(ROOT / 'tests/conformance/controls/person_identifier_fixture.json'),
                             '--procedure', 'person-semantic-identifiers']
                proc = subprocess.run(procedure, capture_output=True, text=True, cwd=ROOT, env=env)
                (out / 'procedure.log').write_text(proc.stdout + proc.stderr)
                checks = json.loads((out / 'procedure/summary.json').read_text())['procedures']['person-semantic-identifiers']['checks']
                failed = validate_procedure_checks(checks, proc.returncode, mode)
                traces = [json.loads(line) for line in (out / 'mutation.trace.jsonl').read_text().splitlines()]
                inserted = 'https://[' if mode == 'invalid' else 'https://[2001:db8::1]/person'
                if not traces or any(t['added_value'] != inserted for t in traces):
                    raise RuntimeError('Response mutation not verified')
                results[mode] = {'totals': report['totals'], 'suite_exit': run.returncode,
                                 'procedure_exit': proc.returncode, 'failed_checks': sorted(failed),
                                 'passed_checks': sorted(c['id'] for c in checks if c['outcome'] == 'passed'),
                                 'mutated_responses': len(traces), 'command': command}
                print(mode, report['totals'], 'failed Person checks', len(failed), flush=True)
            finally:
                for process in reversed(processes):
                    process.terminate()
                    process.wait(timeout=10)
    if reports['valid'].keys() != reports['invalid'].keys():
        raise RuntimeError('Different case selection')
    changes = {name for name in reports['valid'] if reports['valid'][name]['outcome'] != reports['invalid'][name]['outcome']}
    if changes != CASES:
        raise RuntimeError('Unexpected case outcome delta: ' + str(changes ^ CASES))
    for name in CASES:
        expected = 'passed'
        if reports['valid'][name]['outcome'] != expected or reports['invalid'][name]['outcome'] != 'failed':
            raise RuntimeError('Missing expected baseline/single-defect FAIL pair: ' + name)
    if not CHECKS <= set(results['valid']['passed_checks']):
        raise RuntimeError('Missing conforming procedure counterparts')
    result = {'axis': 'Suite design', 'scope': 'Bounded returned-Person URI contrast only', 'results': results,
              'account_locator_evidence': 'HTTP(S) access mechanism after RFC 3987 mapping; other schemes and full grammar qualification remain outside this control',
              'expected_changed_cases': sorted(CASES), 'changed_cases': sorted(changes), 'exact_sets_match': True,
              'input_sha256': {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in [Path(__file__), args.runner, args.server,
                               ROOT / 'tests/conformance/controls/person_identifier_proxy.py', ROOT / 'scripts/ci/run_lrs_procedures.py',
                               ROOT / 'tests/conformance/controls/person_identifier_fixture.json',
                               ROOT / 'scripts/ci/check_conformance_regressions.py', ROOT / 'spec/conformance/ci-execution-baseline.json']}}
    (args.out / 'summary.json').write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    main()
