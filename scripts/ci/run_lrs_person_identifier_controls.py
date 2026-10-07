#!/usr/bin/env python3
"""Qualify the returned-Person URI defect using the existing full Memory Suite.

Common fixture preparation omits absent optional fields in both branches.
The sole branch difference is a valid or unterminated IP-literal URI.
The exact failed case/check sets are declared before execution.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
CASES = {'person-response-all-identifiers-' + field for field in ['mbox', 'mbox_sha1sum', 'openid', 'account']}
CHECKS = {'account-openid-0', 'mbox-openid-0', 'mbox_sha1sum-openid-0', 'openid-openid-1'}


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
                reports[mode] = {c['id']: c for suite in report['suites'] for c in suite['cases']}
                if any(c['outcome'] not in ['passed', 'failed'] for c in reports[mode].values()):
                    raise RuntimeError('Not every selected case executed')
                if any('HTTP request attempt' in failure for c in reports[mode].values() for failure in c['failures']):
                    raise RuntimeError('Transport errors are not defect-detection evidence')
                procedure = [sys.executable, str(ROOT / 'scripts/ci/run_lrs_procedures.py'), '--endpoint', endpoint,
                             '--runner', str(args.runner.resolve()), '--out', str((out / 'procedure').resolve()),
                             '--fixture', str(ROOT / 'tests/conformance/controls/kiva_procedure_fixture.json'),
                             '--procedure', 'person-semantic-identifiers']
                proc = subprocess.run(procedure, capture_output=True, text=True, cwd=ROOT, env=env)
                (out / 'procedure.log').write_text(proc.stdout + proc.stderr)
                checks = json.loads((out / 'procedure/summary.json').read_text())['procedures']['person-semantic-identifiers']['checks']
                if any(c['id'] == 'execution-error' for c in checks):
                    raise RuntimeError('Procedure execution error is not a discriminating FAIL')
                failed = {c['id'] for c in checks if c['outcome'] == 'failed'}
                expected = CHECKS if mode == 'invalid' else set()
                if failed != expected:
                    raise RuntimeError('Unexpected procedure failure set: ' + str(failed ^ expected))
                if {c['id'] for c in checks if c['outcome'] == 'not-run'} != {'full-external-grammar'}:
                    raise RuntimeError('Unexpected unexecuted Person condition')
                if proc.returncode != (1 if mode == 'invalid' else 2):
                    raise RuntimeError('Wrong procedure exit code')
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
        if reports['valid'][name]['outcome'] != 'passed' or reports['invalid'][name]['outcome'] != 'failed':
            raise RuntimeError('Missing conforming PASS/single-defect FAIL pair: ' + name)
    if not CHECKS <= set(results['valid']['passed_checks']):
        raise RuntimeError('Missing conforming procedure counterparts')
    result = {'axis': 'Suite design', 'scope': 'Bounded returned-Person URI contrast only', 'results': results,
              'expected_changed_cases': sorted(CASES), 'changed_cases': sorted(changes), 'exact_sets_match': True,
              'input_sha256': {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in [Path(__file__), args.runner, args.server,
                               ROOT / 'tests/conformance/controls/person_identifier_proxy.py', ROOT / 'scripts/ci/run_lrs_procedures.py']}}
    (args.out / 'summary.json').write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    main()
