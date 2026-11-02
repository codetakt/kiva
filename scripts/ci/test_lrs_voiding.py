"""Independent HTTP controls for both permitted voiding-of-voiding branches."""
import copy
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
import json
from pathlib import Path
import tempfile
import threading
from types import SimpleNamespace
import unittest
from urllib.parse import parse_qs,urlsplit

from run_lrs_procedures import Executor


class VoidingTests(unittest.TestCase):
    def exercise(self,mode):
        rows=[]
        class Peer(BaseHTTPRequestHandler):
            def log_message(self,*args):pass
            def reply(self,status,value=None):
                data=b'' if value is None else json.dumps(value).encode()
                self.send_response(status)
                self.send_header('X-Experience-API-Version','2.0.0')
                self.send_header('X-Experience-API-Consistent-Through','2026-10-20T00:00:01Z')
                if status==200 and value:
                    self.send_header('Last-Modified','Tue, 20 Oct 2026 00:00:00 GMT')
                self.send_header('Content-Length',str(len(data)))
                self.end_headers();self.wfile.write(data)
            def do_PUT(self):
                value=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                value['stored']='2026-10-20T00:00:00.125Z'
                rows.append(value)
                self.reply(400 if len(rows)==3 and mode in ('reject','rejected-leak') else 204)
            def do_GET(self):
                query=parse_qs(urlsplit(self.path).query)
                visible=copy.deepcopy(rows)
                if len(rows)>1:visible=[r for r in visible if r['id']!=rows[0]['id']]
                if len(rows)==3:
                    if mode=='reject':visible=[r for r in visible if r['id']!=rows[2]['id']]
                    elif mode=='void-b':visible=[r for r in visible if r['id']!=rows[1]['id']]
                    elif mode=='restore-a':visible.append(rows[0])
                if 'statementId' in query:
                    found=next((r for r in visible if r['id']==query['statementId'][0]),None)
                    self.reply(200 if found else 404,found);return
                selected=[r for r in visible if r['verb']['id']==query['verb'][0]]
                # An empty result has no known representation modification time.
                # The fixture follows Kiva D02 omission rather than fabricating it.
                data={'statements':selected,'more':''}
                if not selected:
                    raw=json.dumps(data).encode();self.send_response(200)
                    self.send_header('X-Experience-API-Version','2.0.0')
                    self.send_header('X-Experience-API-Consistent-Through','2026-10-20T00:00:01Z')
                    self.send_header('Content-Length',str(len(raw)))
                    self.end_headers();self.wfile.write(raw)
                else:self.reply(200,data)
        with ThreadingHTTPServer(('127.0.0.1',0),Peer) as server,tempfile.TemporaryDirectory() as temp:
            thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
            try:
                root=Path(temp);fixture=root/'fixture.json'
                fixture.write_text('{"statement_visibility_timeout_ms":0}')
                executor=Executor(SimpleNamespace(out=root,fixture=fixture,endpoint=f'http://127.0.0.1:{server.server_port}'))
                executor.current='void-control';executor.results[executor.current]={'checks':[]}
                executor.voiding()
                checks=executor.results[executor.current]['checks']
                self.assertEqual(len(checks),len({c['id'] for c in checks}))
                return checks
            finally:
                server.shutdown();thread.join()

    def test_both_admission_branches_and_three_state_violations(self):
        for mode in ('accept','reject','rejected-leak','void-b','restore-a'):
            with self.subTest(mode=mode):
                checks=self.exercise(mode)
                outcomes={c['outcome'] for c in checks}
                if mode in ('accept','reject'):
                    self.assertEqual(outcomes,{'passed'})
                else:
                    self.assertIn('failed',outcomes)
                if mode=='rejected-leak':
                    self.assertTrue(any(c['id']=='c-single-retrievability' and c['outcome']=='failed' for c in checks))
                if mode=='restore-a':
                    self.assertTrue(any(c['id'].startswith('a-query-observed-exclusions-') and c['outcome']=='failed' for c in checks))


if __name__=='__main__':unittest.main()
