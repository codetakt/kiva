#!/usr/bin/env python3
"""Qualify finite procedure defect models with exact failed check sets."""
import argparse
import hashlib
import json
import socket
import subprocess
import sys
import time
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]


def port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1',0))
        return sock.getsockname()[1]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runner',type=Path,required=True)
    parser.add_argument('--out',type=Path,required=True)
    parser.add_argument('--expectations',type=Path,default=ROOT/'tests/conformance/controls/procedure_expectations.json')
    args=parser.parse_args()
    contract=json.loads(args.expectations.read_text())
    args.out.mkdir(parents=True,exist_ok=False)
    results={}
    for model,declared in contract['models'].items():
        transport = declared.get('transport', 'reference' if model == 'none' else 'proxy')
        if transport not in ['reference', 'proxy']:
            raise ValueError('Unknown control transport')
        upstream,front=port(),port()
        processes=[]
        with (args.out/(model+'.server.log')).open('w') as log:
            try:
                mutation = 'person-valid-enrichment' if model == 'none' else model if transport == 'reference' else 'none'
                processes.append(subprocess.Popen([sys.executable,str(ROOT/'tests/conformance/controls/evaluation_reference.py'),str(upstream),'--trace',str(args.out/(model+'.trace.jsonl')),'--mutation',mutation],stdout=log,stderr=log))
                endpoint=upstream
                if transport == 'proxy':
                    processes.append(subprocess.Popen([sys.executable,str(ROOT/'tests/conformance/controls/evaluation_proxy.py'),str(front),str(upstream),model],stdout=log,stderr=log));endpoint=front
                for _ in range(100):
                    try:
                        with socket.create_connection(('127.0.0.1',endpoint),timeout=.1):break
                    except OSError:time.sleep(.02)
                command=[sys.executable,str(ROOT/'scripts/ci/run_lrs_procedures.py'),'--endpoint',f'http://127.0.0.1:{endpoint}','--runner',str(args.runner),'--fixture',str(ROOT/'tests/conformance/controls/reference_procedure_fixture.json'),'--out',str(args.out/model)]
                for name in contract['procedures']:command+=['--procedure',name]
                run=subprocess.run(command,capture_output=True,text=True)
                (args.out/(model+'.log')).write_text(run.stdout+run.stderr)
                report=json.loads((args.out/model/'summary.json').read_text())
                checks={name+'/'+c['id']:c for name,p in report['procedures'].items() for c in p['checks']}
                if any(c['id'] == 'execution-error' for c in checks.values()):
                    raise RuntimeError('Procedure execution error is not a discriminating FAIL')
                failed={name for name,c in checks.items() if c['outcome']=='failed'}
                expected=set(declared['expected_failed_checks'])
                if len(expected)!=len(declared['expected_failed_checks']):raise RuntimeError('duplicate declaration')
                matched=failed==expected
                unresolved={name for name,c in checks.items() if c['outcome']=='not-run'}
                if unresolved!=set(contract['unexecuted_checks']):raise RuntimeError('unexpected unexecuted subconditions: '+str(unresolved))
                if not matched:raise RuntimeError(f'{model}: missing={sorted(expected-failed)}, unexpected={sorted(failed-expected)}')
                if run.returncode!=(1 if expected else 2):raise RuntimeError('wrong procedure exit status')
                results[model]={'exit_code':run.returncode,'expected_failed_checks':sorted(expected),'failed_checks':sorted(failed),'exact_failure_set_match':matched,'passed_checks':sum(c['outcome']=='passed' for c in checks.values()),'unexecuted_checks':sorted(unresolved),'report_sha256':hashlib.sha256((args.out/model/'summary.json').read_bytes()).hexdigest()}
                print(model,'exact match',len(failed),'failures',flush=True)
            finally:
                for process in reversed(processes):process.terminate();process.wait(timeout=5)
    positive=json.loads((args.out/'none/summary.json').read_text())
    passed={n+'/'+c['id'] for n,p in positive['procedures'].items() for c in p['checks'] if c['outcome']=='passed'}
    for model,declaration in contract['models'].items():
        if not set(declaration['expected_failed_checks'])<=passed:raise RuntimeError('Missing conforming PASS contrast for '+model)
    summary={'scope':'Bounded detection models only; every defect check has executed positive PASS and single-defect FAIL. Unexecuted full-grammar conditions do not close.','axis':'Suite design','models':results,'input_sha256':{str(p.resolve().relative_to(ROOT)) if p.resolve().is_relative_to(ROOT) else str(p.resolve()):hashlib.sha256(p.read_bytes()).hexdigest() for p in [Path(__file__),args.expectations,args.runner,ROOT/'tests/conformance/controls/evaluation_reference.py',ROOT/'tests/conformance/controls/evaluation_proxy.py',ROOT/'scripts/ci/run_lrs_procedures.py',ROOT/'tests/conformance/controls/reference_procedure_fixture.json']}}
    (args.out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')


if __name__=='__main__':main()
