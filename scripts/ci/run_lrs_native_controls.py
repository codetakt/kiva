#!/usr/bin/env python3
"""Compile and execute actual Rust controls with input, binary and result binding.

A named test is an observation, not a claim that it covers an entire contract.
The procedure collector must separately curate the semantic links to each test.
"""
import argparse
import datetime as dt
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

from report_issued_test_results import ROOT, implementation_inputs, file_sha256
from run_lrs_python_controls import validate as validate_python

PACKAGES = ('kiva-xapi', 'kiva-server', 'xapi-lrs-conformance')


def parse_results(text, inventory):
    results = {}
    for name, outcome in re.findall(r'^test (\S+) \.\.\. (ok|FAILED|ignored)(?:,.*)?$', text, re.M):
        if name in results or name not in inventory:
            raise ValueError('duplicate or unknown native control: ' + name)
        results[name] = {'ok': 'passed', 'FAILED': 'failed', 'ignored': 'not-run'}[outcome]
    if set(results) != set(inventory):
        raise ValueError('native result inventory mismatch: ' + repr(sorted(set(inventory) ^ set(results))))
    return results


def capture(output, backend):
    if backend == 'postgres' and not os.environ.get('KIVA_POSTGRES_TEST_DATABASE_URL'):
        raise ValueError('PostgreSQL native controls require a migrated disposable database')
    output.mkdir(parents=True, exist_ok=False)
    inputs = implementation_inputs(ROOT)
    started = dt.datetime.now(dt.UTC).isoformat()
    report = dict(schema_version=1, kind='lrs-native-execution', backend=backend,
                  started=started, input_sha256=inputs, binaries={}, tests={}, errors=[])
    toolchain={}
    for name in ('cargo','rustc'):
        executable=Path(shutil.which(name) or name).resolve(strict=True)
        version=subprocess.run([str(executable),'--version','--verbose'],cwd=ROOT,
                               capture_output=True,text=True,check=True)
        toolchain[name]={'path':str(executable),'sha256':file_sha256(executable),
                         'version':version.stdout,'stderr':version.stderr}
    (output/'toolchain.json').write_text(json.dumps(toolchain,indent=2)+'\n')
    command = ['cargo', 'test', '--locked', '--lib', '--no-run', '--message-format=json']
    for package in PACKAGES:
        command += ['-p', package]
    with (output/'build.jsonl').open('w') as log, (output/'build.stderr').open('w') as err:
        build = subprocess.run(command, cwd=ROOT, stdout=log, stderr=err, check=False)
    if build.returncode:
        report['errors'].append('native compilation failed')
    else:
        executables = {}
        for line in (output/'build.jsonl').read_text().splitlines():
            item = json.loads(line)
            if item.get('reason') != 'compiler-artifact' or not item.get('executable'):
                continue
            package = item['target']['name'].replace('_', '-')
            if package in PACKAGES and item.get('profile', {}).get('test'):
                if package in executables:
                    raise ValueError('duplicate test artifact: ' + package)
                executables[package] = Path(item['executable'])
        if set(executables) != set(PACKAGES):
            raise ValueError('missing native test executable')
        for package, executable in executables.items():
            digest = file_sha256(executable)
            options = ['--test-threads=1'] if package == 'kiva-server' else []
            if package == 'kiva-server':
                options += ['--include-ignored'] if backend == 'postgres' else ['--skip', 'postgres_']
            listed = subprocess.run([str(executable), *options, '--list'], cwd=ROOT,
                                    capture_output=True, text=True, check=True)
            (output/(package+'.inventory')).write_text(listed.stdout)
            inventory = re.findall(r'^(\S+): test$', listed.stdout, re.M)
            if not inventory or len(set(inventory)) != len(inventory):
                raise ValueError('empty or duplicate native inventory')
            log_path = output/(package+'.log')
            with log_path.open('w') as log:
                run = subprocess.run([str(executable), *options], cwd=ROOT,
                                     env=dict(os.environ, KIVA_NATIVE_HTTP_TRACE=str(output/'http')),
                                     stdout=log, stderr=subprocess.STDOUT, check=False)
            if file_sha256(executable) != digest:
                report['errors'].append('native executable changed: '+package)
            results = parse_results(log_path.read_text(), inventory)
            report['binaries'][package] = {'path': str(executable.relative_to(ROOT)), 'sha256':digest,
                'command': options, 'exit_code': run.returncode, 'inventory_sha256': file_sha256(output/(package+'.inventory')),
                'log_sha256':file_sha256(log_path)}
            for name, outcome in results.items():
                report['tests'][package+'::'+name] = {'outcome':outcome, 'log':log_path.name}
            if run.returncode or set(results.values()) != {'passed'}:
                report['errors'].append('native controls did not all pass: '+package)
    with (output/'python.log').open('w') as log:
        python=subprocess.run([sys.executable,'-B',str(ROOT/'scripts/ci/run_lrs_python_controls.py'),
                               '--out',str(output/'python.json')],cwd=ROOT,stdout=log,
                              stderr=subprocess.STDOUT,check=False)
    if python.returncode or not (output/'python.json').is_file():
        report['errors'].append('Python oracle controls did not all pass')
    else:
        for name,outcome in validate_python(json.loads((output/'python.json').read_text())).items():
            report['tests']['python::'+name]={'outcome':outcome,'log':'python.log'}
    if implementation_inputs(ROOT) != inputs:
        report['errors'].append('implementation or fixture changed during native execution')
    report['completed'] = dt.datetime.now(dt.UTC).isoformat()
    report['artifacts_sha256'] = {str(path.relative_to(output)):file_sha256(path) for path in sorted(output.rglob('*')) if path.is_file()}
    report['claim'] = 'actual local controls; author evidence, not independent acceptance or whole-contract closure'
    (output/'receipt.json').write_text(json.dumps(report,indent=2)+'\n')
    return report


def verify(directory, backend, inputs=None):
    report = json.loads((directory/'receipt.json').read_text())
    if (report.get('kind') != 'lrs-native-execution' or report.get('schema_version') != 1
            or report.get('backend') != backend or report.get('errors')):
        raise ValueError('invalid or incomplete native execution')
    if report.get('input_sha256') != (inputs if inputs is not None else implementation_inputs(ROOT)):
        raise ValueError('native execution input drift')
    if set(report.get('binaries', {})) != set(PACKAGES):
        raise ValueError('incomplete native binary inventory')
    toolchain=json.loads((directory/'toolchain.json').read_text())
    if (set(toolchain)!= {'cargo','rustc'} or any(not v.get('path')
            or len(v.get('sha256',''))!=64 or not v.get('version','').startswith(name+' ')
            for name,v in toolchain.items())):
        raise ValueError('missing observed toolchain identity')
    observed = {}
    for package, record in report['binaries'].items():
        inventory_path = directory/(package+'.inventory')
        log_path = directory/(package+'.log')
        if (file_sha256(inventory_path) != record['inventory_sha256']
                or file_sha256(log_path) != record['log_sha256'] or record['exit_code'] != 0
                or len(record.get('sha256','')) != 64):
            raise ValueError('native execution artifact drift')
        inventory = re.findall(r'^(\S+): test$', inventory_path.read_text(), re.M)
        for name, outcome in parse_results(log_path.read_text(), inventory).items():
            observed[package+'::'+name] = {'outcome':outcome,'log':log_path.name}
    for name,outcome in validate_python(json.loads((directory/'python.json').read_text())).items():
        observed['python::'+name]={'outcome':outcome,'log':'python.log'}
    if observed != report.get('tests') or any(row['outcome'] != 'passed' for row in observed.values()):
        raise ValueError('native result summary mismatch')
    actual_files = {str(path.relative_to(directory)) for path in directory.rglob('*')
                    if path.is_file() and path != directory/'receipt.json'}
    if actual_files != set(report.get('artifacts_sha256', {})):
        raise ValueError('native artifact inventory mismatch')
    for name, digest in report.get('artifacts_sha256', {}).items():
        if (Path(name).is_absolute() or '..' in Path(name).parts
                or not (directory/name).resolve().is_relative_to(directory.resolve())
                or file_sha256(directory/name) != digest):
            raise ValueError('native artifact mismatch: '+name)
    for path in (directory/'http').glob('*.json'):
        record = json.loads(path.read_text())
        if ('kiva-server::'+record.get('test','') not in observed
                or not isinstance(record.get('request'),dict) or not isinstance(record.get('response'),dict)
                or not isinstance(record.get('received_response'),dict)
                or 'body_hex' not in record['received_response']):
            raise ValueError('incomplete or unbound native HTTP observation')
    for path in (directory/'http/controls').glob('*.json'):
        record=json.loads(path.read_text())
        if ('kiva-server::'+str(record.get('test','')) not in observed
                or not isinstance(record.get('kind'),str) or 'observation' not in record):
            raise ValueError('unbound native runtime observation')
    return report


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--backend', choices=('memory','postgres'), required=True)
    parser.add_argument('--out', type=Path, required=True)
    args=parser.parse_args()
    report=capture(args.out.resolve(),args.backend)
    print(json.dumps({'tests':len(report['tests']),'errors':report['errors']}))
    return bool(report['errors'])

if __name__ == '__main__':
    raise SystemExit(main())
