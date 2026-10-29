"""Table replay controls preserve omission preconditions and MIME payloads."""
import copy
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import tempfile
import threading
from types import SimpleNamespace
import unittest
from unittest.mock import patch
from urllib.parse import parse_qs, urlsplit
import uuid

from lrs_table_vectors import accepted_vectors, statement_template, write_payload
from run_lrs_procedures import Executor, ROOT


class TableVectorTests(unittest.TestCase):
    def test_raw_omissions_and_two_distinct_parts_survive_batch_expansion(self):
        rows=list(accepted_vectors(ROOT))
        by_id={name:(request,source) for name,request,source in rows}
        self.assertEqual(len(rows),len(by_id))
        omitted,_=statement_template(by_id['statement-root-id-omitted-accepted'][0])
        self.assertNotIn('id',omitted)
        raw=[(name,request) for name,request,_ in rows if 'raw_body_hex' in request]
        self.assertTrue(any('fileUrl-omitted' in name for name,_ in raw))
        self.assertIn('attachment-two-file-urls',by_id)
        self.assertIn('attachment-two-multipart',by_id)
        for name,request in raw:
            original,envelope=statement_template(request)
            bodies=[copy.deepcopy(original),copy.deepcopy(original)]
            for body in bodies:
                body['id']=str(uuid.uuid4())
            payload=write_payload(bodies,envelope)
            before,after,_=envelope
            self.assertTrue(payload['raw'].startswith(before))
            self.assertTrue(payload['raw'].endswith(after))
            self.assertEqual(json.loads(payload['raw'][len(before):-len(after)]),bodies)
            if name=='attachment-two-multipart':
                self.assertIn(b'first attachment\x00\r\n',after)
                self.assertIn(b'second different attachment\xff\x80',after)
                self.assertEqual(after.count(b'X-Experience-API-Hash:'),2)

    def exercise_peer(self, corrupt=False):
        original={'actor':{'mbox':'mailto:a@example.org','name':'Preserved A'},
                  'verb':{'id':'urn:example:verb'},'object':{'id':'urn:example:activity'}}
        rows=[('statement-root-id-omitted-accepted',{'method':'PUT','path':'/statements','body':original},
               {'source_anchor':'4.2.2','placement':'','condition':'id-omitted-accepted'})]
        writes=[]; reads=[]; storage={}
        class Peer(BaseHTTPRequestHandler):
            def log_message(self,*args):
                pass

            def reply(self,status,value=None):
                data=json.dumps(value).encode() if value is not None else b''
                self.send_response(status)
                self.send_header('X-Experience-API-Version','2.0.0')
                self.send_header('X-Experience-API-Consistent-Through','2026-10-20T00:00:01Z')
                self.send_header('Content-Length',str(len(data)))
                self.end_headers();self.wfile.write(data)

            def do_PUT(self):
                self.store()

            def do_POST(self):
                self.store()

            def store(self):
                submitted=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                writes.append((self.command,submitted))
                bodies=submitted if isinstance(submitted,list) else [submitted]
                if any('id' in body for body in bodies):
                    # Independent fixture detects the former executor bug.
                    self.reply(400);return
                ids=[]
                for body in bodies:
                    identifier=(parse_qs(urlsplit(self.path).query)['statementId'][0]
                                if self.command=='PUT' else str(uuid.uuid4()))
                    ids.append(identifier)
                    storage[identifier]={**body,'id':identifier,'stored':'2026-10-20T00:00:00.125Z',
                        'timestamp':'2026-10-20T00:00:00.125Z','version':'2.0.0',
                        'authority':{'objectType':'Agent','openid':'https://example.org/authority'}}
                self.reply(204 if self.command=='PUT' else 200,None if self.command=='PUT' else ids)

            def do_GET(self):
                query=parse_qs(urlsplit(self.path).query);reads.append(query)
                value=copy.deepcopy(storage[query['statementId'][0]])
                if corrupt is True:value['actor']['name']='Lost original name'
                elif corrupt in ('stored','timestamp'):value[corrupt]='2025-01-01'
                self.reply(200,value)

        with ThreadingHTTPServer(('127.0.0.1',0),Peer) as server,tempfile.TemporaryDirectory() as temp:
            thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
            try:
                root=Path(temp);fixture=root/'fixture.json';fixture.write_text('{}')
                executor=Executor(SimpleNamespace(out=root,fixture=fixture,endpoint=f'http://127.0.0.1:{server.server_port}'))
                executor.current='table-control';executor.results[executor.current]={'checks':[]}
                with patch('lrs_table_vectors.accepted_vectors',return_value=rows):executor.tables()
                checks=executor.results[executor.current]['checks']
                self.assertEqual(len({c['id'] for c in checks}),len(checks))
                self.assertEqual(len(writes),2)
                self.assertEqual(len(writes[1][1]),2)
                self.assertEqual(len(reads),6)
                self.assertEqual(sum('format' not in query for query in reads),3)
                self.assertEqual(sum(query.get('format')==['exact'] for query in reads),3)
                if corrupt=='stored':
                    self.assertTrue(any(c['outcome']=='failed' and c['id'].endswith('-stored') for c in checks))
                    return checks
                observations=[json.loads(line) for line in (root/'preservation.jsonl').read_text().splitlines()]
                self.assertEqual(len(observations),6)
                self.assertTrue(all(o['source']['source_anchor']=='4.2.2' for o in observations))
                return checks
            finally:
                server.shutdown();thread.join()

    def test_actual_http_omissions_defaults_batches_and_preservation_faults(self):
        good=self.exercise_peer()
        self.assertTrue(all(c['outcome']=='passed' for c in good))
        bad=self.exercise_peer(True)
        failures=[c for c in bad if c['outcome']=='failed']
        self.assertEqual(len(failures),6)
        self.assertTrue(all(c['id'].endswith('-preservation') for c in failures))

    def test_date_only_populated_fields_fail_after_accepted_omission(self):
        for field in ('stored','timestamp'):
            with self.subTest(field=field):
                checks=self.exercise_peer(field)
                self.assertTrue(any(c['outcome']=='failed' for c in checks))


if __name__=='__main__':
    unittest.main()
