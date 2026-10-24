"""Regression boundaries for positive, admitted Statement visibility observations."""
import copy
import json
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch
from run_lrs_procedures import Executor

class VisibilityTests(unittest.TestCase):
    def test_relative_continuation_requires_a_base_and_keeps_origin_boundary(self):
        executor=Executor.__new__(Executor)
        executor.args=SimpleNamespace(endpoint='http://example.test/xapi')
        executor.headers={'Authorization':'fixture-credential'}
        for base,more in [
            (None,'?cursor=2'),
            ('http://example.test/xapi/statements','//other.test/next'),
            ('http://example.test/xapi/statements','https://example.test/next'),
        ]:
            with self.subTest(base=base,more=more), patch('run_lrs_procedures.http.client.HTTPConnection') as connection:
                with self.assertRaises(ValueError):
                    executor.http('GET','/statements',continuation=more,continuation_base=base)
                connection.assert_not_called()

    def test_relative_continuations_follow_each_page_url(self):
        for continuations,expected in [
            (['?cursor=2','pages/third?cursor=3','?cursor=4',''],
             ['/xapi/statements?limit=1','/xapi/statements?cursor=2',
              '/xapi/pages/third?cursor=3','/xapi/pages/third?cursor=4']),
            (['pages/','next/','next/',''],
             ['/xapi/statements?limit=1','/xapi/pages/',
              '/xapi/pages/next/','/xapi/pages/next/next/']),
        ]:
            with self.subTest(continuations=continuations), TemporaryDirectory() as directory:
                executor=Executor.__new__(Executor)
                executor.args=SimpleNamespace(endpoint='http://example.test/xapi')
                executor.out=Path(directory);executor.headers={};executor.docs=[]
                executor.results={'probe':{'checks':[]}};executor.current='probe';executor.ordinal=0
                connections=[]
                for more in continuations:
                    connection=Mock();response=connection.getresponse.return_value
                    response.status=200
                    response.getheaders.return_value=[('X-Experience-API-Version','2.0.0'),
                        ('X-Experience-API-Consistent-Through','2026-01-01T00:00:00Z')]
                    response.read.return_value=json.dumps({'statements':[],'more':more}).encode()
                    connections.append(connection)
                with patch('run_lrs_procedures.http.client.HTTPConnection',side_effect=connections) as factory:
                    self.assertEqual(executor.collect_statements({'limit':'1'},'probe'),[])
                self.assertEqual(factory.call_count,4)
                self.assertEqual([c.request.call_args.args[1] for c in connections],expected)
                records=[json.loads(line) for line in (executor.out/'http.jsonl').read_text().splitlines()]
                self.assertEqual([r['response']['url'] for r in records],['http://example.test'+p for p in expected])

    invalid_response_dates = [
        'Thu, 01 Jan 2026 00:00 GMT',
        '01 Jan 2026 00:00:00 GMT',
        'Thu, 1 Jan 2026 00:00:00 GMT',
        'Thu, 01 Jan 26 00:00:00 GMT',
        'Thu, 01 Jan 2026 00:00:00 +0000',
        'Thu, 01 Jan 2026 00:00:00 UTC',
        'Thu, 01 Jan 2026 00:00:00',
        'thu, 01 Jan 2026 00:00:00 GMT',
        'Thu, 01 jan 2026 00:00:00 GMT',
        'Thu, 01 Jan 2026 00:00:00 gmt',
        'Thu, 01 Jan 2026 00:00:00 GMT (comment)',
        'Thu,  01 Jan 2026 00:00:00 GMT',
        'Thu,\t01 Jan 2026 00:00:00 GMT',
        'Thu, 01 Jan 2026 00:00:00 GMT\n',
        'Thu, 01 Jan 2026 0:00:00 GMT',
        'Thu, 01 Jan 2026 00:00:00.0 GMT',
        'Thu, 01 Jan 2026 00:00:60 GMT',
        'Thu, 01 Jan 2026 24:00:00 GMT',
        'Thu, 30 Feb 2026 00:00:00 GMT',
        'Fri, 01 Jan 2026 00:00:00 GMT',
        'Thu, ０１ Jan 2026 00:00:00 GMT',
        # Legal obsolete receiver forms do not satisfy the HTTP sender duty.
        'Thursday, 01-Jan-26 00:00:00 GMT',
        'Thu Jan  1 00:00:00 2026',
    ]

    def test_response_http_date_syntax_precedes_page_comparison(self):
        for header in self.invalid_response_dates:
            for continuation in [False,True]:
                for items,incomplete in [([],False),([{'stored':'2026-01-01T00:00:00Z'}],False),([],True)]:
                    with self.subTest(header=header,continuation=continuation,items=items,incomplete=incomplete):
                        e,_=self.collect(items,header,continuation=continuation,incomplete=incomplete)
                        checks=[c for c in e.results['probe']['checks'] if c['id']=='probe-page-last-modified']
                        self.assertEqual(checks[-1]['outcome'],'failed')

    def test_complete_response_dates_keep_page_semantics(self):
        for header,stored in [
            ('Thu, 01 Jan 2026 00:00:00 GMT','2026-01-01T00:00:00Z'),
            ('Thu, 29 Feb 2024 23:59:59 GMT','2024-02-29T23:59:59.999Z'),
            ('Mon, 01 Jan 0001 00:00:00 GMT','0001-01-01T00:00:00Z'),
            (' \tThu, 01 Jan 2026 00:00:00 GMT\t ','2026-01-01T00:00:00Z'),
            ('\r\n Thu, 01 Jan 2026 00:00:00 GMT\r\n\t','2026-01-01T00:00:00Z'),
        ]:
            for items,incomplete in [([],False),([{'stored':stored}],False),([],True)]:
                with self.subTest(header=header,items=items,incomplete=incomplete):
                    e,_=self.collect(items,header,incomplete=incomplete)
                    checks=[c for c in e.results['probe']['checks'] if c['id']=='probe-page-last-modified']
                    self.assertEqual(checks[-1]['outcome'],'not-run' if incomplete else 'passed')
        for header,outcome in [('Sat, 01 Jan 0000 00:00:00 GMT','not-run'),
                               ('Sun, 01 Jan 0000 00:00:00 GMT','failed'),
                               ('Wed, 30 Feb 0000 00:00:00 GMT','failed')]:
            with self.subTest(header=header):
                e,_=self.collect([],header)
                checks=[c for c in e.results['probe']['checks'] if c['id']=='probe-page-last-modified']
                self.assertEqual(checks[-1]['outcome'],outcome)

    def test_document_last_modified_uses_same_sender_syntax(self):
        for header in [*self.invalid_response_dates,'Thu, 01 Jan 2026 00:00:00 GMT']:
            with self.subTest(header=header):
                e=Executor.__new__(Executor)
                e.results={'probe':{'checks':[]}};e.current='probe';e.fixture={};e.prefix='urn:fixture'
                e.docs=[('state','/activities/state',{},'stateId')]
                def http(method,path,query=None,*a,**kw):
                    status=404 if method=='GET' and query.get('stateId')=='clock' and getattr(e,'deleted',False) else 200 if method=='GET' else 204
                    if method=='DELETE':e.deleted=True
                    return {'status':status,'headers':{'last-modified':header},'json':{'v':'sibling'},
                            'before':'2026-01-01T00:00:00Z','after':'2026-01-01T00:00:00Z'}
                e.http=http;e.documents()
                checks=[c for c in e.results['probe']['checks'] if c['id']=='state-last-modified']
                self.assertEqual(checks[-1]['outcome'],'failed' if header in self.invalid_response_dates else 'not-run')

    def test_long_http_date_padding_is_bounded(self):
        import subprocess
        import sys
        from pathlib import Path
        # Keep a parser hang bounded in the test process as well. Both sides
        # exercise failure after long padding, not just a greedy valid match.
        script="""
from run_lrs_procedures import response_http_date
for value in [' '*4096+'Thu, 01 Jan 2026 00:00 GMT',
              'Thu, 01 Jan 2026 00:00:00 GMT'+' '*4096+'junk']:
    try: response_http_date(value)
    except ValueError: pass
    else: raise AssertionError('malformed date accepted')
response_http_date(' '*4096+'Thu, 01 Jan 2026 00:00:00 GMT'+' '*4096)
"""
        result=subprocess.run([sys.executable,'-B','-c',script],
                              cwd=Path(__file__).resolve().parent,capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stderr)

    def collect(self, items, header, *, incomplete=False, continuation=False):
        from run_lrs_procedures import Reply
        e=Executor.__new__(Executor)
        e.results={'probe':{'checks':[]}};e.current='probe'
        headers={'x-experience-api-version':'2.0.0',
                 'x-experience-api-consistent-through':'2026-01-01T00:00:00Z'}
        if header is not None:headers['last-modified']=header
        reply=Reply(status=200,headers=headers,json={'statements':items,'more':''},incomplete_body=incomplete)
        pages=[reply]
        if continuation:
            pages.insert(0,Reply(status=200,headers={k:v for k,v in headers.items() if k!='last-modified'},
                                 json={'statements':[],'more':'/next'}))
        e.http=lambda *a,**kw:pages.pop(0)
        result=e.collect_statements({},'probe')
        return e,result

    def test_page_header_syntax_survives_empty_or_interrupted_body(self):
        for continuation in [False,True]:
            for incomplete in [False,True]:
                for header in [None,'invalid','Thu, 01 Jan 2026 00:00:00 GMT']:
                    with self.subTest(continuation=continuation,incomplete=incomplete,header=header):
                        e,result=self.collect([],header,incomplete=incomplete,continuation=continuation)
                        self.assertEqual('failed' in self.outcomes(e),header=='invalid')
                        self.assertEqual(result,None if incomplete else [])
                        checks=[c for c in e.results['probe']['checks'] if c['id']=='probe-page-last-modified']
                        self.assertTrue(checks)
                        self.assertEqual(checks[-1]['outcome'],
                                         'failed' if header=='invalid' else 'not-run' if incomplete else 'passed')

    def test_unavailable_page_time_does_not_hide_definite_stored_or_max_defect(self):
        unknown={'stored':'2016-12-31T23:59:60Z'}
        for item in [{'stored':42},{},{'stored':'2026-02-30T00:00:00Z'},
                     {'stored':'2026-01-01T00:00:01Z'}]:
            for items in [[unknown,item],[item,unknown]]:
                with self.subTest(items=items):
                    e,_=self.collect(items,'Thu, 01 Jan 2026 00:00:00 GMT')
                    self.assertIn('failed',self.outcomes(e))
        for items,header,outcome in [
            ([unknown],None,'failed'),
            ([unknown],'invalid','failed'),
            ([unknown],'Thu, 01 Jan 2026 00:00:00 GMT','not-run'),
            ([unknown,{'stored':'2025-12-31T23:59:59Z'}],'Thu, 01 Jan 2026 00:00:00 GMT','not-run'),
            ([{'stored':'2026-01-01T00:00:00.999Z'}],'Thu, 01 Jan 2026 00:00:00 GMT','passed'),
            ([{'stored':'2025-12-31T23:59:59Z'}],'Thu, 01 Jan 2026 00:00:00 GMT','failed')]:
            with self.subTest(items=items,header=header):
                e,_=self.collect(items,header)
                checks=[c for c in e.results['probe']['checks'] if c['id']=='probe-page-last-modified']
                self.assertEqual(checks[-1]['outcome'],outcome)

    def executor(self, statuses, *, missing_header=False, wrong_id=False, seen=False,
                 first_consistent_through='1970-01-01T00:00:00Z',
                 stored='1969-12-31T23:59:59Z', later_consistent_through='1970-01-01T00:00:00Z',
                 incomplete=False):
        e=Executor.__new__(Executor)
        e.fixture={'statement_visibility_timeout_ms':0 if statuses==[404] else 1000}
        e.results={'probe':{'checks':[]}};e.current='probe'
        e.visible_statements={'s':stored} if seen else {}
        e.calls=[]
        def read(body,**query):
            e.calls.append(query)
            status=statuses[min(len(e.calls)-1,len(statuses)-1)]
            headers={'x-experience-api-version':'2.0.0','x-experience-api-consistent-through':later_consistent_through}
            if len(e.calls)==1:headers['x-experience-api-consistent-through']=first_consistent_through
            if missing_header and len(e.calls)==1:headers.pop('x-experience-api-version')
            from run_lrs_procedures import Reply
            return Reply(status=status,headers=headers,json={'id':'wrong' if wrong_id else 's','stored':stored} if status==200 else {},
                         incomplete_body=incomplete and status==404)
        e.read=read
        return e
    def outcomes(self,e):return [x['outcome'] for x in e.results['probe']['checks']]
    def test_initial_404_then_visible(self):
        e=self.executor([404,200])
        with patch('run_lrs_procedures.time.sleep'):
            self.assertIsNotNone(e.read_visible({'id':'s'},{'status':204}))
        self.assertEqual(len(e.calls),2);self.assertNotIn('failed',self.outcomes(e))
    def test_deadline_is_not_run(self):
        e=self.executor([404]);self.assertIsNone(e.read_visible({'id':'s'},{'status':204}))
        self.assertIn('not-run',self.outcomes(e));self.assertNotIn('failed',self.outcomes(e))
    def test_error_header_failure_is_not_retried(self):
        e=self.executor([404,200],missing_header=True);self.assertIsNone(e.read_visible({'id':'s'},{'status':204}))
        self.assertEqual(len(e.calls),1);self.assertIn('failed',self.outcomes(e))
    def test_first_200_wrong_identity_is_not_retried(self):
        e=self.executor([200,200],wrong_id=True);self.assertIsNone(e.read_visible({'id':'s'},{'status':204}))
        self.assertEqual(len(e.calls),1);self.assertIn('failed',self.outcomes(e))
    def test_previously_visible_disappearance_is_failure(self):
        e=self.executor([404,200],seen=True);self.assertIsNone(e.read_visible({'id':'s'},{'status':204}))
        self.assertEqual(len(e.calls),1);self.assertIn('failed',self.outcomes(e))
    def test_retrieval_guarantee_requires_a_strictly_later_watermark(self):
        for stored, watermark, outcome in [
            ('1970-01-01T00:00:00Z', '1970-01-01T00:00:00Z', 'not-run'),
            ('1970-01-01T09:00:00+09:00', '1970-01-01T00:00:00Z', 'not-run'),
            ('1970-01-01T00:00:00Z', '1969-12-31T23:59:59Z', 'not-run'),
            ('1970-01-01T00:00:00Z', '1970-01-01T00:00:00.0000000001Z', 'failed'),
            ('2016-12-31T23:59:60Z', '2017-01-01T00:00:01Z', 'not-run'),
            ('2017-01-01T00:00:00Z', '2016-12-31T23:59:60Z', 'not-run'),
            ('2016-12-31T23:59:60Z', '2026-02-30T00:00:00Z', 'failed'),
        ]:
            for incomplete in [False, True]:
                with self.subTest(stored=stored,watermark=watermark,incomplete=incomplete):
                    e=self.executor([200,404,200],stored=stored,
                                    later_consistent_through=watermark,incomplete=incomplete)
                    self.assertIsNotNone(e.read_visible({'id':'s'},{'status':204}))
                    self.assertIsNone(e.read_visible({'id':'s'},{'status':204}))
                    self.assertEqual(len(e.calls),2)  # Never retry a prior positive observation.
                    self.assertIn(outcome,self.outcomes(e))
                    if outcome=='not-run':self.assertNotIn('failed',self.outcomes(e))

    def test_invalid_stored_is_not_a_visibility_witness(self):
        for value in [None, 42, 'invalid', '2026-02-30T00:00:00Z']:
            with self.subTest(stored=value):
                e=self.executor([200],stored=value)
                self.assertIsNone(e.read_visible({'id':'s'},{'status':204}))
                self.assertIn('failed',self.outcomes(e))

    def test_rejected_admission_does_not_poll(self):
        e=self.executor([404]);self.assertIsNone(e.read_visible({'id':'s'},{'status':400}))
        self.assertFalse(e.calls);self.assertIn('not-run',self.outcomes(e))
    def test_unexpected_status_is_failure(self):
        e=self.executor([500,200]);self.assertIsNone(e.read_visible({'id':'s'},{'status':204}))
        self.assertEqual(len(e.calls),1);self.assertIn('failed',self.outcomes(e))

    def test_date_only_error_and_success_headers_fail_before_retry(self):
        for value in ['2026-10-01','20261001','2026-W40-4','2026W404',
                      '2026-10-01Z','2026-10-01+09:00']:
            for status in [404,200]:
                with self.subTest(value=value,status=status):
                    e=self.executor([status,200],first_consistent_through=value)
                    self.assertIsNone(e.read_visible({'id':'s'},{'status':204}))
                    self.assertEqual(len(e.calls),1)
                    self.assertIn('failed',self.outcomes(e))
                    self.assertNotIn('not-run',self.outcomes(e))

    def test_combined_forms_keep_existing_parser_acceptance(self):
        # Compatibility vectors for the delegated parser, not ISO certification.
        for value in ['2026-10-01T00:00:00Z','2026-10-01T09:00:00+09:00',
                      '2026-10-01T00:00:00.123456Z','20261001T000000Z',
                      '2026-10-01t00:00:00+00:00','2026-10-01 00:00:00+00:00',
                      '2026-W40-4T00:00:00Z','2026-10-01T00:00:00,123Z',
                      '2026-10-01T00:00:00']:
            with self.subTest(value=value):
                e=self.executor([404,200],first_consistent_through=value)
                with patch('run_lrs_procedures.time.sleep'):
                    self.assertIsNotNone(e.read_visible({'id':'s'},{'status':204}))
                self.assertEqual(len(e.calls),2)
                self.assertNotIn('failed',self.outcomes(e))

    def test_malformed_combined_header_never_retries(self):
        for value in [None,123,'','2026-10-01T','2026-10-01T25:00:00Z',
                      '2026-02-30T00:00:00Z','2026-10-01X00:00:00Z',
                      '2026-10-01T25Z','2026-10-01T25:00Z','2026-10-01T00:61Z',
                      '2026-10-01T25.5Z','2026-10-01T24.1Z','2026-10-01T09.5+24:00']:
            with self.subTest(value=value):
                e=self.executor([404,200],first_consistent_through=value)
                self.assertIsNone(e.read_visible({'id':'s'},{'status':204}))
                self.assertEqual(len(e.calls),1)
                self.assertIn('failed',self.outcomes(e))
                self.assertNotIn('not-run',self.outcomes(e))

if __name__=='__main__':unittest.main()
