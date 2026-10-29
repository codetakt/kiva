"""Actual HTTP precision controls with independent fixed returned instants."""
import copy,json,tempfile,threading,unittest
from pathlib import Path
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
from types import SimpleNamespace
from urllib.parse import parse_qs,urlsplit
from run_lrs_procedures import Executor

OUTPUTS={
 '2025-01-01T09:00:00+09:00':'2025-01-01T00:00:00Z',
 '2025-01-01T09:00:00.123+09:00':'2025-01-01T00:00:00.123Z',
 '2025-01-01T09:00:00.123456+09:00':'2025-01-01T00:00:00.123Z',
 '2025-01-01T09:00:00.123500+09:00':'2025-01-01T00:00:00.124Z',
 '2025-01-01T09:00:00.123456789012+09:00':'2025-01-01T00:00:00.123456789Z',
 '1969-12-31T23:59:59.999999999999Z':'1970-01-01T00:00:00Z',
 '2024-12-31T23:59:59.999999999999Z':'2025-01-01T00:00:00Z',
}
class PrecisionTests(unittest.TestCase):
    def exercise(self,fault):
        saved={}
        class Peer(BaseHTTPRequestHandler):
            def log_message(self,*args):pass
            def response(self,status,value=None):
                raw=b'' if value is None else json.dumps(value).encode()
                self.send_response(status);self.send_header('X-Experience-API-Version','2.0.0')
                self.send_header('X-Experience-API-Consistent-Through','2026-10-27T00:00:01Z')
                self.send_header('Content-Length',str(len(raw)));self.end_headers();self.wfile.write(raw)
            def write(self,post):
                value=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                body=value[0] if post else value;saved[body['id']]=body
                self.response(200 if post else 204,[body['id']] if post else None)
            def do_PUT(self):self.write(False)
            def do_POST(self):self.write(True)
            def do_GET(self):
                key=parse_qs(urlsplit(self.path).query)['statementId'][0];body=copy.deepcopy(saved[key])
                owner=body['object'] if body['object'].get('objectType')=='SubStatement' else body
                sent=owner['timestamp'];owner['timestamp']=OUTPUTS[sent]
                if fault=='non-utc':owner['timestamp']=sent
                elif fault=='coarse':owner['timestamp']='2025-01-01T00:00:00.12Z'
                elif fault=='unavailable':owner['timestamp']='2016-12-31T23:59:60Z'
                elif fault=='date-only':owner['timestamp']='2025-01-01'
                elif fault=='type':owner['timestamp']=17
                body['stored']='2026-10-27T00:00:00Z';self.response(200,body)
        with ThreadingHTTPServer(('127.0.0.1',0),Peer) as server,tempfile.TemporaryDirectory() as temp:
            thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
            try:
                root=Path(temp);fixture=root/'fixture.json';fixture.write_text('{}')
                e=Executor(SimpleNamespace(out=root,fixture=fixture,endpoint=f'http://127.0.0.1:{server.server_port}'))
                e.current='timestamp-precision-rounding';e.results[e.current]={'checks':[]};e.precision()
                checks=e.results[e.current]['checks'];self.assertEqual(len({c['id'] for c in checks}),len(checks))
                self.assertEqual(len(saved),28)
                records=[json.loads(line) for line in (root/'http.jsonl').read_text().splitlines()]
                self.assertEqual(len(records),56)
                return checks
            finally:server.shutdown();thread.join()
    def test_allowed_precision_and_known_defects_remain_separate_from_provider_gaps(self):
        for mode in ('valid','non-utc','coarse','unavailable','type','date-only'):
            with self.subTest(mode=mode):
                checks=self.exercise(mode);outcomes={c['outcome'] for c in checks}
                if mode=='valid':self.assertEqual(outcomes,{'passed'})
                elif mode=='unavailable':self.assertEqual(outcomes,{'passed','not-run'})
                else:self.assertIn('failed',outcomes)

if __name__=='__main__':unittest.main()
