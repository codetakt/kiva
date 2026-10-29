"""About oracle faults, including two independent HTTP target controls."""
import json
import os
from pathlib import Path
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from lrs_about import about_content
from run_lrs_procedures import Executor


class AboutTests(unittest.TestCase):
    def test_required_content_and_extra_properties(self):
        for raw in (b'{"version":["2.0.0"]}',
                    b'{"version":["2.0.0"],"vendor":true}',
                    b'{"version":["2.0.0"],"extensions":{"name":"Kiva"}}',
                    b'{"version":["2.0.0"],"extensions":{"https://example.org/e":[null,true,1]}}'):
            self.assertIs(about_content(raw, ['2.0.0'])['required'], True)
        for raw in (b'[]', b'{}', b'{"version":"2.0.0"}', b'{"version":[]}',
                    b'{"version":[2]}', b'{"version":["2.0"]}',
                    b'{"version":["1.0.3"]}', b'{"version":["2.0.0"],"extensions":[]}',
                    b'{"version":["2.0.0"],"number":NaN}', b'{'):
            with self.subTest(raw=raw):
                self.assertIs(about_content(raw, ['2.0.0'])['required'], False)
        self.assertIs(about_content(b'{"version":["2.0.0"]}', ['1.0.3','2.0.0'])['required'], False)
        self.assertIs(about_content(b'{"version":["1.0.3","2.0.0"]}', ['1.0.3','2.0.0'])['required'], True)

    def test_duplicates_are_advisory_and_ambiguity_is_incomplete(self):
        for raw in (b'{"version":["2.0.0"],"vendor":1,"vendor":2}',
                    b'{"version":["2.0.0"],"version":["2.0.0"]}'):
            result=about_content(raw, ['2.0.0'])
            self.assertIs(result['required'],True)
            self.assertTrue(result['duplicate_names'])
        ambiguous=about_content(b'{"version":["1.0.3"],"version":["2.0.0"]}', ['2.0.0'])
        self.assertIsNone(ambiguous['required'])
        self.assertTrue(ambiguous['duplicate_names'])
        self.assertIs(about_content(b'{"version":null,"version":false}', ['2.0.0'])['required'],False)
        self.assertIsNone(about_content(b'{"version":["2.0.0"]}', None)['required'])

    def exercise_peer(self, fault):
        # This server has independent, intentionally faulty cause handling. It
        # is an oracle control, never evidence that the Kiva target passed.
        class Peer(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_GET(self):
                unsupported=self.headers.get('X-Experience-API-Version')=='9.9.9'
                unknown='auditUnknown=' in self.path
                refused=self.headers.get('Authorization')=='Bearer independently-refused'
                status=401 if refused else 400 if unknown else 200
                if fault=='reject-version' and unsupported and not unknown and not refused:
                    status=400
                if fault=='exempt-unknown' and unsupported and unknown and not refused:
                    status=200
                data=json.dumps({'version':['2.0.0']} if status==200 else {'error':'declared cause'}).encode()
                self.send_response(status)
                if fault!='missing-header':
                    self.send_header('X-Experience-API-Version','2.0.0')
                self.send_header('Content-Length',str(len(data)))
                self.end_headers()
                if self.command!='HEAD':
                    self.wfile.write(data)

            do_HEAD=do_GET

        with ThreadingHTTPServer(('127.0.0.1',0),Peer) as server, tempfile.TemporaryDirectory() as temp:
            thread=threading.Thread(target=server.serve_forever,daemon=True)
            thread.start()
            try:
                root=Path(temp)
                fixture=root/'fixture.json'
                fixture.write_text(json.dumps({'about_context':{'supported':'2.0.0','unsupported':'9.9.9',
                    'latest_versions':['2.0.0'],'unrecognized':'auditUnknown','missing_header':'route-default'}}))
                executor=Executor(SimpleNamespace(out=root,fixture=fixture,endpoint='http://127.0.0.1:'+str(server.server_port)))
                executor.current='about-control'
                executor.results[executor.current]={'checks':[]}
                with patch.dict(os.environ,KIVA_CONFORMANCE_INVALID_TOKEN='independently-refused'):
                    executor.about()
                checks=executor.results[executor.current]['checks']
                records=[json.loads(line) for line in (root/'http.jsonl').read_text().splitlines()]
                self.assertEqual(len(records),18)
                for record in records:
                    if record['request']['method']=='HEAD':
                        self.assertEqual(record['response']['body_hex'],'')
                absent=[r for r in records if 'X-Experience-API-Version' not in r['request']['headers']]
                self.assertEqual(len(absent),6)
                return checks
            finally:
                server.shutdown()
                thread.join()

    def test_about_cross_product_catches_two_version_exception_faults(self):
        good=self.exercise_peer(None)
        self.assertTrue(all(c['outcome'] in ('passed','observed') for c in good))
        from report_issued_test_results import procedure_incomplete
        self.assertFalse(procedure_incomplete({'outcome':'passed','checks':good},{'level':'source_settled'}))
        for fault in ('reject-version','exempt-unknown'):
            with self.subTest(fault=fault):
                checks=self.exercise_peer(fault)
                failed=[c for c in checks if c['outcome']=='failed']
                self.assertEqual(len(failed),2)  # GET and HEAD status obligations
                self.assertTrue(all(c['id'].endswith('-status') for c in failed))

    def test_required_headers_remain_independent_of_status(self):
        checks=self.exercise_peer('missing-header')
        failed=[c for c in checks if c['outcome']=='failed']
        self.assertEqual(len(failed),18)
        self.assertTrue(all(c['id'].endswith('-version-header') for c in failed))


if __name__=='__main__':
    unittest.main()
