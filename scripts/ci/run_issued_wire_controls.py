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


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runner',type=Path,required=True)
    parser.add_argument('--out',type=Path,required=True)
    args=parser.parse_args()
    args.out.mkdir(parents=True,exist_ok=True)
    if (args.out/'summary.json').exists(): raise SystemExit('use a fresh output directory')
    source=ROOT/'tests/conformance/http/generated/issued-completion.yaml'
    doc=yaml.load(source.read_text(),Loader=yaml.CSafeLoader)
    names=['issued-completion-root-coerced-scaled-3-put','issued-completion-sub-coerced-scaled-3-put']
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
        outcomes={c['id']:c['outcome'] for s in report['suites'] for c in s['cases']}
        expected='passed' if mutation=='strict' else 'failed'
        assert outcomes=={n:expected for n in names},outcomes
        assert len(observed)==len(names) and all(o['json_type']=='str' and o['value']=='1e0' for o in observed),observed
        assert run.returncode==(0 if mutation=='strict' else 1)
        results[mutation]=dict(outcomes=outcomes,wire_string_preserved=True,exit_code=run.returncode)
    (args.out/'summary.json').write_text(json.dumps(dict(scope='Two raw-wire scalar-type controls; not an LRS reference implementation',results=results,input_sha256={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in [source,args.runner,Path(__file__),suite]}),indent=2)+'\n')
    print(json.dumps(results,indent=2))


if __name__=='__main__': main()
