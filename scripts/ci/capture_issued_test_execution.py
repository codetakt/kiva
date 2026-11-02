#!/usr/bin/env python3
"""Capture a local backend run and its partial/manual procedure boundary.

Build the server/runner and generate locked definitions before invocation. This
producer surrounds the actual wrapper execution; it cannot renew old results.
Successful capture is evidence collection, not test-specification conformity.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

from report_issued_test_results import ROOT, file_sha256, implementation_inputs, assess_capture
from collect_lrs_procedure_evidence import collect_capture, combined_wrapper_exit


def capture(backend, output, fixture, server, runner, root=ROOT, workers=1):
    if backend not in ('memory', 'postgres'):
        raise ValueError('unknown backend')
    if os.environ.get('KIVA_CONFORMANCE_CASE_IDS_FILE'):
        raise ValueError('full capture cannot use a case selection')
    if type(workers) is not int or not 1 <= workers <= 8:
        raise ValueError('workers must be between 1 and 8')
    if workers > 1 and any(name.startswith(('KIVA_MEMORY_CONFORMANCE_', 'KIVA_POSTGRES_CONFORMANCE_'))
                           for name in os.environ):
        raise ValueError('parallel capture requires automatic fixture isolation')
    projection_path = root / 'spec/conformance/issued-test-projection.json'
    projection = json.loads(projection_path.read_text())
    names = set(projection['execution_contract']) | set(projection['definition_files'])
    inputs = {name: file_sha256(root / name) for name in sorted(names)}
    for mapping in ('execution_contract', 'definition_files'):
        if any(inputs[name] != digest for name, digest in projection[mapping].items()):
            raise ValueError('stale projection: ' + mapping)
    implementation = {'backend': backend, 'input_sha256': implementation_inputs(root),
                      'server_binary_sha256': file_sha256(server),
                      'runner_binary_sha256': file_sha256(runner)}
    projection_hash = file_sha256(projection_path)
    fixture_hash = file_sha256(fixture)
    output.mkdir(parents=True, exist_ok=False)
    (output / 'fixture.json').write_bytes(fixture.read_bytes())
    (output / 'case-receipt.json').write_text(json.dumps({'cases': projection['cases']}) + '\n')
    env = dict(os.environ, KIVA_CONFORMANCE_ARTIFACTS_READY='1',
               KIVA_CONFORMANCE_JSON_REPORT=str(output / 'http.json'),
               KIVA_CONFORMANCE_JUNIT_REPORT=str(output / 'http.xml'),
               KIVA_CONFORMANCE_PROCEDURE_REPORT_ROOT=str(output / 'procedures'),
               KIVA_CONFORMANCE_NATIVE_REPORT_ROOT=str(output / 'native'),
               KIVA_CONFORMANCE_PROCEDURE_FIXTURE=str(fixture))
    errors = []
    status = None
    with (output / 'execution.log').open('w') as log:
        try:
            def run(command, environment):
                return subprocess.run(command, cwd=root, env=environment,
                    stdout=log, stderr=subprocess.STDOUT, check=False).returncode
            wrapper = ['bash', f'scripts/ci/run_{backend}_conformance.sh']
            if workers == 1:
                status = run(wrapper, env)
            else:
                http_env = {key: value for key, value in env.items()
                            if key not in ('KIVA_CONFORMANCE_PROCEDURE_REPORT_ROOT',
                                           'KIVA_CONFORMANCE_NATIVE_REPORT_ROOT',
                                           'KIVA_CONFORMANCE_PROCEDURE_FIXTURE')}
                status = run([sys.executable, '-B', 'scripts/ci/run_parallel_conformance.py',
                              '--backend', backend, '--workers', str(workers),
                              '--out', str(output / 'shards')], http_env)
                # Procedures use one additional isolated backend. Preserve its
                # one-case setup run separately; it never replaces the full HTTP
                # report or supplies coverage for missing procedure obligations.
                selection = output / 'procedure-setup-case-ids.txt'
                selection.write_text(next(iter(projection['cases'])) + '\n')
                env.update(KIVA_CONFORMANCE_CASE_IDS_FILE=str(selection),
                           KIVA_CONFORMANCE_JSON_REPORT=str(output / 'procedure-setup-http.json'),
                           KIVA_CONFORMANCE_JUNIT_REPORT=str(output / 'procedure-setup-http.xml'))
                procedure_status = run(wrapper, env)
                status = status or procedure_status
        except OSError as error:
            errors.append(str(error))
    partial_status=status
    try:
        assembled=collect_capture(output,backend,root)
        (output/'procedure-evidence.json').write_text(json.dumps(assembled,ensure_ascii=False,indent=2)+'\n')
        status=combined_wrapper_exit(status,assembled,json.loads((output/'http.json').read_text()),root)
    except (OSError,ValueError,KeyError,TypeError) as error:
        errors.append('procedure evidence unavailable: '+str(error))
    try:
        if inputs != {name: file_sha256(root / name) for name in inputs}:
            errors.append('definition/oracle input drift during execution')
        if implementation['input_sha256'] != implementation_inputs(root):
            errors.append('implementation input drift during execution')
        if (file_sha256(projection_path) != projection_hash or file_sha256(fixture) != fixture_hash
                or file_sha256(server) != implementation['server_binary_sha256']
                or file_sha256(runner) != implementation['runner_binary_sha256']):
            errors.append('projection, fixture or binary changed during execution')
    except OSError as error:
        errors.append('execution input became unavailable: ' + str(error))
    for name in ('http.json', 'http.xml', 'procedures/summary.json', 'native/receipt.json', 'procedure-evidence.json'):
        if not (output / name).is_file():
            errors.append('missing execution artifact: ' + name)
    receipt = {'schema_version': 1, 'kind': 'issued-test-execution-receipt',
               'claim': 'local author execution capture; not independent acceptance',
               'projection_sha256': projection_hash, 'input_sha256': inputs,
               'implementation': implementation, 'fixture_sha256': fixture_hash,
               'wrapper_exit': status, 'partial_wrapper_exit':partial_status, 'workers': workers, 'errors': errors,
               'artifacts_sha256': {str(path.relative_to(output)): file_sha256(path)
                   for path in sorted(output.rglob('*')) if path.is_file()}}
    (output / 'execution.json').write_text(json.dumps(receipt, indent=2) + '\n')
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--backend', required=True, choices=['memory', 'postgres'])
    parser.add_argument('--out', required=True, type=Path)
    parser.add_argument('--fixture', required=True, type=Path)
    parser.add_argument('--workers', type=int, choices=range(1, 9), default=1)
    args = parser.parse_args()
    target = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target')).resolve() / 'release'
    output = args.out.resolve()
    receipt = capture(args.backend, output, args.fixture.resolve(),
                      target / 'kiva-server', target / 'xapi-lrs-conformance', workers=args.workers)
    print(json.dumps({'backend': args.backend, 'wrapper_exit': receipt['wrapper_exit'],
                      'errors': receipt['errors'], 'claim': receipt['claim']}, indent=2))
    if receipt['errors']:
        return 2
    # Reconstruct semantic links from original observations as the release gate
    # does; a previously written summary cannot replace this verification.
    report=assess_capture(output)
    (output/'issued-results.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(report['summary'],ensure_ascii=False,indent=2))
    return 0 if report['summary']['test_specification_conformant'] else 2


if __name__ == '__main__':
    raise SystemExit(main())
