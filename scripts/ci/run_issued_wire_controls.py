#!/usr/bin/env python3
"""Check actual JSON wire types with a bounded independent strict/coercing peer."""
import argparse
import hashlib
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import subprocess
import threading

import yaml

ROOT=Path(__file__).resolve().parents[2]
CASES=('issued-completion-root-coerced-scaled-3-put','issued-completion-sub-coerced-scaled-3-put')
COERCION_FAILURE='step reject-invalid-child: expected status 400, received 204'
UNVISITED_SUFFIX='step not-stored: not evaluated after step reject-invalid-child stopped execution'


def validate_control_results(report, exit_code, mutation):
    """An unrelated defect or unavailable observation cannot witness coercion."""
    if mutation not in ('strict','coerce-string'):
        raise ValueError('unknown wire control mode')
    failed=mutation=='coerce-string'
    cases=[c for suite in report['suites'] for c in suite['cases']]
    outcomes={c['id']:c['outcome'] for c in cases}
    expected='failed' if failed else 'passed'
    if len(cases)!=len(outcomes) or outcomes!={name:expected for name in CASES}:
        raise ValueError('missing, duplicate or unexpected wire control outcome')
    for case in cases:
        if case['failures']!=([COERCION_FAILURE] if failed else []):
            raise ValueError(f'{case["id"]}: unexpected coercion diagnostic')
        if case.get('unresolved_checks',[])!=([UNVISITED_SUFFIX] if failed else []):
            raise ValueError(f'{case["id"]}: unexpected unresolved observation or unvisited suffix')
        if case.get('not_run_reasons',[])!=[] or case.get('diagnostic_statuses',[])!=[]:
            raise ValueError(f'{case["id"]}: unexpected unavailable or diagnostic observation')
    totals={'total':2,'passed':0 if failed else 2,'failed':2 if failed else 0,'validated':0,'not_run':0}
    if report['totals']!=totals or report['status']!=expected or exit_code!=int(failed):
        raise ValueError('wire control totals/status/exit disagree with outcomes')
    return outcomes


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runner',type=Path,required=True)
    parser.add_argument('--out',type=Path,required=True)
    args=parser.parse_args()
    args.out.mkdir(parents=True,exist_ok=True)
    if (args.out/'summary.json').exists(): raise SystemExit('use a fresh output directory')
    source=ROOT/'tests/conformance/http/generated/issued-completion.yaml'
    doc=yaml.load(source.read_text(),Loader=yaml.CSafeLoader)
    names=CASES
    definitions=[c for c in doc['cases'] if c['id'] in names]
    if len(definitions)!=len(names) or {c['id'] for c in definitions}!=set(names):
        raise ValueError('missing or duplicate wire control definition')
    for case in definitions:
        operations=[(s['id'],s['request']['method'],s['expect']['status']) for s in case['steps']]
        if operations!=[('reject-invalid-child','PUT',400),('not-stored','GET',404)]:
            raise ValueError('wire mutation witness definition changed')
    # Keep the generated wire serialization unchanged: extract case blocks as
    # text instead of passing requests through a different YAML emitter.
    text=source.read_text()
    prefix=text.split('cases:\n',1)[0]+'cases:\n'
    blocks=text.split('\n- id: ')[1:]
    selected=[b for b in blocks if b.splitlines()[0] in names]
    if len(selected)!=len(names): raise SystemExit('missing control cases')
    suite=args.out/'cases.yaml'
    suite.write_text(prefix+''.join('- id: '+b+'\n' for b in selected))
    results={}
    for mutation in ['strict','coerce-string']:
        observed=[]
        class Peer(BaseHTTPRequestHandler):
            def log_message(self,*_): pass
            def do_PUT(self):
                raw=self.rfile.read(int(self.headers.get('Content-Length','0')))
                value=json.loads(raw)
                item=value['object'] if value.get('object',{}).get('objectType')=='SubStatement' else value
                score=item['result']['score']['scaled']
                observed.append(dict(raw_body_hex=raw.hex(),json_type=type(score).__name__,value=score))
                # Independent target fragment: numeric-looking strings are
                # still strings. The mutant explicitly violates that rule.
                status=400 if isinstance(score,str) and mutation=='strict' else 204
                self.send_response(status)
                self.send_header('X-Experience-API-Version','2.0.0')
                self.send_header('X-Experience-API-Consistent-Through','2020-01-01T00:00:00Z')
                self.send_header('Content-Length','0');self.end_headers()
            def do_GET(self):
                self.send_response(404)
                self.send_header('X-Experience-API-Version','2.0.0')
                self.send_header('X-Experience-API-Consistent-Through','2020-01-01T00:00:00Z')
                self.send_header('Content-Length','0');self.end_headers()
        server=ThreadingHTTPServer(('127.0.0.1',0),Peer)
        worker=threading.Thread(target=server.serve_forever,daemon=True);worker.start()
        try:
            run=subprocess.run([str(args.runner.resolve()),'--case',str(suite),'--endpoint',f'http://127.0.0.1:{server.server_port}','--allow-destructive','--json'],capture_output=True,text=True)
        finally:
            server.shutdown();worker.join();server.server_close()
        (args.out/(mutation+'.json')).write_text(run.stdout)
        (args.out/(mutation+'.stderr')).write_text(run.stderr)
        (args.out/(mutation+'.wire.json')).write_text(json.dumps(observed,indent=2)+'\n')
        report=json.loads(run.stdout)
        outcomes=validate_control_results(report,run.returncode,mutation)
        assert len(observed)==len(names) and all(o['json_type']=='str' and o['value']=='1e0' for o in observed),observed
        results[mutation]=dict(outcomes=outcomes,wire_string_preserved=True,exit_code=run.returncode,
            expected_failures=[COERCION_FAILURE] if mutation=='coerce-string' else [],
            expected_unresolved_checks=[UNVISITED_SUFFIX] if mutation=='coerce-string' else [])
    (args.out/'summary.json').write_text(json.dumps(dict(scope='Two raw-wire scalar-type controls; not an LRS reference implementation',results=results,input_sha256={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in [source,args.runner,Path(__file__),suite]}),indent=2)+'\n')
    print(json.dumps(results,indent=2))


if __name__=='__main__': main()
