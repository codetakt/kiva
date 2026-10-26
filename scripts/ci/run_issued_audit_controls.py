#!/usr/bin/env python3
"""Bounded wire controls for AX-01..05; not a reference LRS or target result.

Final Suite cases are executed unchanged. Procedure realizations are explicitly
private fixtures. Every mode gets an isolated peer and a new evidence directory.
"""
import argparse
import copy
import hashlib
import json
from email import policy
from email.parser import BytesParser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import subprocess
import threading
from urllib.parse import parse_qs, urlsplit
import uuid

import yaml
from generate_lrs_projection_cases import STANDARD, case, expect, request, step, uuid_for
from lrs_projection_behavior import S

ROOT = Path(__file__).resolve().parents[2]


def dump(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def canonical(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()


class Fixture:
    def __init__(self, mode):
        self.mode, self.statements, self.parts, self.documents = mode, {}, {}, {}
        self.pending, self.wire, self.receipts = [], [], []

    def release_and_drain(self, request_hash):
        count = len(self.pending)
        for statement in self.pending:
            self.statements[statement['id']] = statement
        self.pending.clear()
        receipt = dict(kind='private-peer-completion', request_sha256=request_hash,
            pending_before=count, pending_after=0, all_commit_and_visibility_work_drained=True)
        self.receipts.append(receipt)
        return receipt

    def handler(self):
        fixture = self

        class Peer(BaseHTTPRequestHandler):
            def log_message(self, *_): pass

            def reply(self, status, value=None, raw=None, content_type='application/json'):
                data = raw if raw is not None else (json.dumps(value).encode() if value is not None else b'')
                self.send_response(status)
                self.send_header('X-Experience-API-Version', '2.0.0')
                self.send_header('X-Experience-API-Consistent-Through', '2020-01-01T00:00:00Z')
                self.send_header('Last-Modified', 'Wed, 01 Jan 2020 00:00:00 GMT')
                self.send_header('Content-Type', content_type)
                self.send_header('Content-Length', str(len(data)))
                self.end_headers()
                if self.command != 'HEAD': self.wfile.write(data)

            def exchange(self):
                parsed = urlsplit(self.path)
                query = {k:v[0] for k,v in parse_qs(parsed.query).items()}
                raw = self.rfile.read(int(self.headers.get('Content-Length', 0)))
                fixture.wire.append(dict(method=self.command, path=self.path,
                    headers=dict(self.headers), body_hex=raw.hex()))
                mode = fixture.mode
                if parsed.path == '/about':
                    version = self.headers.get('X-Experience-API-Version')
                    unknown = 'auditUnknown' in query
                    if mode == 'reject-about-version' and version == '2.1.0' and not unknown:
                        return self.reply(400)
                    if unknown and not (mode == 'about-blanket-exemption' and version == '2.1.0'):
                        return self.reply(400)
                    return self.reply(200, {'version':['2.0.0']})
                if parsed.path != '/statements':
                    registration = query.get('registration')
                    if registration:
                        try: uuid.UUID(registration)
                        except ValueError: return self.reply(400)
                        if mode == 'reject-valid-registration' and self.command == 'POST':
                            return self.reply(400)
                    scope = {k:v for k,v in query.items() if mode != 'ignore-registration-scope' or k != 'registration'}
                    key = parsed.path + json.dumps(scope, sort_keys=True)
                    if self.command == 'GET':
                        return self.reply(200, fixture.documents[key]) if key in fixture.documents else self.reply(404)
                    value = json.loads(raw)
                    if self.command == 'PUT': fixture.documents[key] = value
                    elif mode != 'discard-post-body':
                        fixture.documents.setdefault(key, {}).update(value)
                    return self.reply(204)
                if self.command == 'GET':
                    identifier = query.get('statementId')
                    if identifier not in fixture.statements: return self.reply(404)
                    body = copy.deepcopy(fixture.statements[identifier])
                    if mode == 'rewrite-sha2' and body.get('attachments'):
                        body['attachments'][0]['sha2'] = '0' * len(body['attachments'][0]['sha2'])
                    if query.get('attachments') == 'true':
                        boundary = 'independent-peer-boundary'
                        output = b'--' + boundary.encode() + b'\r\nContent-Type: application/json\r\n\r\n' + json.dumps(body).encode() + b'\r\n'
                        for metadata in body.get('attachments', []):
                            digest = metadata['sha2']
                            data = fixture.parts[digest]
                            if mode == 'corrupt-raw-output': data = bytes([data[0] ^ 1]) + data[1:]
                            output += (b'--' + boundary.encode() + b'\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: ' + digest.encode() + b'\r\n\r\n' + data + b'\r\n')
                        output += b'--' + boundary.encode() + b'--\r\n'
                        return self.reply(200, raw=output, content_type='multipart/mixed; boundary=' + boundary)
                    return self.reply(200, body)
                raw_parts = []
                if self.headers.get('Content-Type', '').startswith('multipart/'):
                    mime = BytesParser(policy=policy.default).parsebytes(
                        ('Content-Type: ' + self.headers['Content-Type'] + '\r\nMIME-Version: 1.0\r\n\r\n').encode() + raw)
                    parts = list(mime.iter_parts())
                    value = json.loads(parts[0].get_payload(decode=True))
                    raw_parts = [(p['X-Experience-API-Hash'], p.get_payload(decode=True)) for p in parts[1:]]
                else: value = json.loads(raw)
                values = value if isinstance(value, list) else [value]
                for body in values:
                    body.setdefault('id', query.get('statementId', str(uuid.uuid4())))
                ids = [b['id'] for b in values]
                invalid_batch = len(ids) != len(set(ids)) or any(not isinstance(b['verb']['id'], str) for b in values)
                if invalid_batch:
                    if mode == 'synchronous-commit-after-reject': fixture.statements[values[0]['id']] = values[0]
                    if mode == 'delayed-commit-after-reject': fixture.pending.append(values[0])
                    return self.reply(400)
                for body in values:
                    for metadata in body.get('attachments', []):
                        length = len(metadata['sha2'])
                        if mode == 'reject-sha384' and length == 96 or mode == 'reject-sha512' and length == 128:
                            return self.reply(400)
                for digest, data in raw_parts:
                    algorithm = {64:'sha256',96:'sha384',128:'sha512'}[len(digest)]
                    if hashlib.new(algorithm, data).hexdigest() != digest and mode != 'ignore-raw-digest':
                        return self.reply(400)
                    fixture.parts[digest] = data
                for body in values:
                    body.setdefault('stored','2020-01-01T00:00:00.000Z')
                    body.setdefault('timestamp',body['stored'])
                    body.setdefault('version','2.0.0')
                    body.setdefault('authority',{'objectType':'Agent','mbox':'mailto:peer@example.org'})
                    fixture.statements[body['id']] = body
                return self.reply(204 if self.command == 'PUT' else 200, None if self.command == 'PUT' else ids)

            do_GET = exchange
            do_HEAD = exchange
            do_PUT = exchange
            do_POST = exchange
        return Peer


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runner', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    dump(args.out/'fixture-declaration.json', dict(
        unrecognized_about_parameter='auditUnknown', supported_version='2.0.0', unsupported_version='2.1.0',
        endpoint_role='private bounded peer, not a reference LRS',
        cause_evidence='The peer recognizes only its declared fields. A named mutant changes exactly the version/parameter rejection choice.',
        unsettled_and_missing_header='Not graded by these private HTTP cases; see the manual procedure and explicit source-indeterminate branches.',
        control_script_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()))
    cases = {}
    source_files = {}
    for path in [ROOT/'tests/conformance/http/generated/issued-completion.yaml', ROOT/'tests/conformance/http/generated/behaviors.yaml']:
        source_files[str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()
        cases.update({c['id']:c for c in yaml.load(path.read_text(), Loader=yaml.CSafeLoader)['cases']})
    sha_cases = [c for n,c in cases.items() if n.startswith('issued-audit-sha')]
    assert len(sha_cases) == 6
    doc_cases = [cases[f + '-' + tail] for f in ['state','agent-profile','activity-profile']
                 for tail in ['merge-replaces-top-level-object','post-creates-absent-document']]
    registration = [cases['state-registration-' + t + '-with-valid-selectors-post'] for t in ['valid','invalid']]
    about = []
    for method in ['GET','HEAD']:
        for version in ['2.0.0','2.1.0']:
            for unknown in [False, True]:
                name = 'private-about-' + method.lower() + '-' + version + ('-unknown' if unknown else '-valid')
                about.append(case(name, ['xapi-lrs-4.1.6.7-about-version-header-not-rejected'],
                    req=request(method,'/about', {'auditUnknown':'1'} if unknown else {},
                        headers={'X-Experience-API-Version':version}), exp=expect(400 if unknown else 200)))
    summary = {}

    def run(label, fixture, server, vectors, failed):
        out = args.out/label;out.mkdir()
        suite = out/'cases.yaml'
        suite.write_text(yaml.dump(dict(suite=dict(id='issued-audit-controls',standard=STANDARD,
            description='Bounded peer and private procedure realization'),cases=vectors),Dumper=yaml.CSafeDumper,sort_keys=False))
        dump(out/'receipt.json', {'case_hashes':{c['id']:canonical(c) for c in vectors},
            'runner_sha256':hashlib.sha256(args.runner.read_bytes()).hexdigest(), 'source_files':source_files,
            'origin':{c['id']:('final-suite' if c['id'] in cases else 'private-procedure-realization') for c in vectors}})
        process = subprocess.run([str(args.runner.resolve()),'--case',str(suite),'--endpoint',
            f'http://127.0.0.1:{server.server_port}','--allow-destructive','--json'],capture_output=True,text=True,timeout=90)
        (out/'raw-report.json').write_text(process.stdout);(out/'stderr.txt').write_text(process.stderr)
        dump(out/'wire.json',fixture.wire)
        result = {c['id']:c for s in json.loads(process.stdout)['suites'] for c in s['cases']}
        assert set(result) == {c['id'] for c in vectors}
        observed = {n for n,c in result.items() if c['outcome']=='failed'}
        assert observed == set(failed), (label, observed, failed)
        assert all(c['outcome'] in ['passed','failed'] for c in result.values())
        assert process.returncode == (1 if observed else 0)
        summary[label] = dict(cases=len(vectors),failed=sorted(observed),failures={n:result[n]['failures'] for n in observed})
        return result

    def peer(mode, callback):
        fixture = Fixture(mode);server=ThreadingHTTPServer(('127.0.0.1',0),fixture.handler())
        worker=threading.Thread(target=server.serve_forever,daemon=True);worker.start()
        try: callback(fixture,server)
        finally: server.shutdown();worker.join();server.server_close()

    for mode in ['baseline','reject-sha384','reject-sha512','rewrite-sha2','ignore-raw-digest','corrupt-raw-output']:
        vectors = sha_cases
        if mode in ['rewrite-sha2','corrupt-raw-output']:
            vectors = [c for c in sha_cases if c['id'].endswith('-raw-match')]
        failed = [c['id'] for c in vectors if (mode == 'reject-sha384' and 'sha384' in c['id'] and not c['id'].endswith('mismatch'))
            or (mode == 'reject-sha512' and 'sha512' in c['id'] and not c['id'].endswith('mismatch'))
            or mode in ['rewrite-sha2','corrupt-raw-output'] or (mode=='ignore-raw-digest' and c['id'].endswith('mismatch'))]
        peer(mode, lambda f,s: run('sha-'+mode,f,s,vectors,failed))
    for mode in ['baseline','discard-post-body']:
        peer(mode, lambda f,s: run('documents-'+mode,f,s,doc_cases,[] if mode=='baseline' else [c['id'] for c in doc_cases]))
    for mode in ['baseline','reject-valid-registration']:
        peer(mode, lambda f,s: run('registration-'+mode,f,s,registration,[] if mode=='baseline' else [registration[0]['id']]))
    scoped = {'activityId':'urn:example:scope', 'agent':json.dumps(S['actor']), 'stateId':'private-registration-scope'}
    a = dict(scoped, registration=uuid_for('scope-a'))
    b = dict(scoped, registration=uuid_for('scope-b'))
    scope_case = case('private-state-post-registration-scope', ['xapi-lrs-4.1.6.2-state-post-json-merge'], steps=[
        step('create-a',request('PUT','/activities/state',a,{'scope':'A','value':0}),expect(204)),
        step('create-b',request('PUT','/activities/state',b,{'scope':'B','value':0}),expect(204)),
        step('post-a',request('POST','/activities/state',a,{'value':1},headers={'X-Experience-API-Version':'2.0.0','Content-Type':'application/json','If-Match':'*'}),expect(204)),
        step('unchanged-b',request('GET','/activities/state',b),expect(200,{'':{'equals':{'scope':'B','value':0}}})),
        step('updated-a',request('GET','/activities/state',a),expect(200,{'':{'equals':{'scope':'A','value':1}}})),
    ])
    for mode in ['baseline','ignore-registration-scope']:
        peer(mode, lambda f,s: run('registration-scope-'+mode,f,s,[scope_case],[] if mode=='baseline' else [scope_case['id']]))
    for mode in ['baseline','reject-about-version','about-blanket-exemption']:
        failed = [c['id'] for c in about if '2.1.0' in c['id'] and c['id'].endswith('-valid' if mode=='reject-about-version' else '-unknown')] if mode!='baseline' else []
        peer(mode, lambda f,s: run('about-'+mode,f,s,about,failed))

    for mode in ['baseline','synchronous-commit-after-reject','delayed-commit-after-reject']:
        def settled(fixture, server):
            for variant in ['accepted','invalid-middle','identical-duplicate','different-duplicate']:
                label = 'batch-' + mode + '-' + variant
                values = [dict(copy.deepcopy(S),id=uuid_for(label+str(i))) for i in range(3)]
                if variant=='invalid-middle': values[1]['verb']['id']=42
                if variant.endswith('duplicate'):
                    values=[values[0],copy.deepcopy(values[0])]
                    if variant=='different-duplicate': values[1]['verb']['id']='urn:example:other'
                req=request('POST','/statements',body=values)
                vector=case('private-'+label,['xapi-lrs-4.1.6.1-statement-post-duplicate-batch-id'],
                    req=req,exp=expect(200 if variant=='accepted' else 400))
                run(label+'-submit',fixture,server,[vector],[])
                ids=list(dict.fromkeys(v['id'] for v in values))
                if variant != 'accepted':
                    immediate = [case('private-'+label+'-immediate-'+str(i),vector['requirements'],
                        req=request('GET','/statements',{'statementId':identifier}),exp=expect(404)) for i,identifier in enumerate(ids)]
                    run(label+'-immediate',fixture,server,immediate,
                        [immediate[0]['id']] if mode=='synchronous-commit-after-reject' else [])
                fixture.release_and_drain(canonical(req))
                checks=[case('private-'+label+'-'+str(i),vector['requirements'],
                    req=request('GET','/statements',{'statementId':identifier}),exp=expect(200 if variant=='accepted' else 404)) for i,identifier in enumerate(ids)]
                failed=[] if mode=='baseline' or variant=='accepted' else [checks[0]['id']]
                run(label+'-settled',fixture,server,checks,failed)
                dump(args.out/(label+'-completion.json'),fixture.receipts[-1])
        peer(mode,settled)
    # This is a fixture-precondition result, not a synthetic successful run.
    summary['batch-no-settlement-fixture'] = dict(outcome='not-run',reason='No correlated completion receipt or quiescent durable snapshot; absence cannot be graded.')
    dump(args.out/'summary.json',summary)
    print(json.dumps({'groups':len(summary),'case_instances':sum(v.get('cases',0) for v in summary.values()),'expected_outcomes_observed':True}))


if __name__ == '__main__': main()
