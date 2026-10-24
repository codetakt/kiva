"""Untrusted response types must not become harness capability failures."""
import copy
from contextlib import redirect_stdout
import io
import json
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

from run_lrs_procedures import Executor, IFI, Reply
from lrs_preservation import preservation, snapshot_equal, load_json_members
from report_issued_test_results import procedure_incomplete


class ResponseTypeTests(unittest.TestCase):
    document_procedure = 'document-updated-time-boundary'

    def test_blocked_duplicate_retains_failures_and_unexecuted_checks(self):
        for failure_at in ['initial','PUT','POST','none']:
            with self.subTest(failure_at=failure_at),TemporaryDirectory() as directory:
                executor=self.person_executor(directory,{},False)
                name='duplicate-state-and-response-observation'
                executor.args.procedure=[name]
                reads=0;writes=[];original={}
                def http(method,path,query=None,body=None,*args,**kwargs):
                    nonlocal reads
                    headers={'x-experience-api-version':'2.0.0',
                        'x-experience-api-consistent-through':'2026-01-02T00:00:00Z'}
                    if method in ['PUT','POST']:
                        writes.append(method)
                        if not original:original.update(copy.deepcopy(body))
                        return Reply(status=204 if method=='PUT' else 200,headers=headers,json=[])
                    if path!='/statements':return Reply(status=200,headers=headers,json={})
                    position=['initial','PUT','POST'][reads];reads+=1
                    result=copy.deepcopy(original)
                    if position!=failure_at:result['stored']='2026-01-01T00:00:00Z'
                    return Reply(status=200,headers=headers,json=result)
                executor.http=Mock(side_effect=http)
                self.assertEqual(self.execute(executor),2 if failure_at=='none' else 1)
                result=executor.results[name]
                self.assertTrue(procedure_incomplete(result,executor.current_spec))
                checks={c['id']:c for c in result['checks']}
                self.assertNotIn('execution-unavailable',checks)
                self.assertEqual(checks['all-excluded-property-and-delayed-visibility-variants']['outcome'],'not-run')
                self.assertEqual(writes,['PUT'] if failure_at=='initial' else ['PUT','PUT','POST'])
                for method in ['PUT','POST']:
                    skipped=failure_at in ['initial',method]
                    self.assertEqual(checks[method+'-all-object-preservation']['outcome'],'not-run' if skipped else 'passed')
                if failure_at!='none':
                    self.assertTrue(any(c['outcome']=='failed' for c in result['checks']))

    def test_mime_nested_attachment_is_a_failed_response_without_aborting_probes(self):
        for mode in ['valid','nested','corrupted','interrupted']:
            with self.subTest(mode=mode), TemporaryDirectory() as directory:
                executor=self.person_executor(directory,{},False)
                executor.args.procedure=['mime-rfc-framing']
                executor.read_visible=Mock(return_value={})
                saved={}
                def http(method,path,query=None,body=None,headers=None,raw=None,**kwargs):
                    if method=='PUT':
                        saved.update(raw=raw,media=headers['Content-Type'])
                        return {'status':204}
                    content=saved['raw']
                    boundary=saved['media'].split('boundary=',1)[1].strip('"').encode()
                    if mode=='nested':
                        start=content.index(b'cOnTeNt-TyPe:')
                        end=content.rindex(b'\r\n--'+boundary+b'--')
                        content=(content[:start]+b'Content-Type: multipart/mixed; boundary=inner\r\n\r\n'
                                 b'--inner\r\nContent-Type: application/octet-stream\r\n\r\npayload\r\n--inner--'
                                 +content[end:])
                    elif mode=='corrupted':
                        content=content.replace(b'\x00binary\xff',b'\x00changed\xff')
                    return Reply(status=200,headers={'content-type':saved['media']},body_hex=content.hex(),
                                 incomplete_body=mode=='interrupted')
                executor.http=Mock(side_effect=http)
                self.assertEqual(self.execute(executor),1 if mode in ['nested','corrupted'] else 2)
                checks={c['id']:c for c in executor.results['mime-rfc-framing']['checks']}
                if mode=='interrupted':
                    self.assertFalse(any(c['outcome']=='failed' for c in checks.values()))
                    self.assertEqual(checks['execution-unavailable']['details']['kind'],'EvidenceUnavailable')
                else:
                    self.assertNotIn('execution-unavailable',checks)
                    self.assertEqual(executor.http.call_count,4)
                    for label in ['quoted','ordinary']:
                        self.assertEqual(checks[label+'-returned-binary-hash']['outcome'],
                                         'passed' if mode=='valid' else 'failed')
                    self.assertEqual(checks['legal-folding-transfer-variations-full-rfc']['outcome'],'not-run')

    def document_executor(self, directory, change=None, shared_clock=False):
        root=Path(directory)
        fixture=root/'fixture.json'
        fixture.write_text(json.dumps({'axis':'bounded-control',
            'host_clock_shared' if shared_clock else 'clock_control':True}))
        runner=root/'runner'; runner.write_text('unused adapter fixture')
        executor=Executor(SimpleNamespace(out=root, fixture=fixture, runner=runner,
            endpoint='http://fixture.test', procedure=[self.document_procedure]))
        executor.control=Mock()
        reads={}; writes={}; deleted=False
        def http(method, path, query=None, *args, **kwargs):
            nonlocal deleted
            key='stateId' if path=='/activities/state' else 'profileId'
            if method=='PUT': writes[path]=writes.get(path,0)+1
            if method=='DELETE': deleted=True
            stamp='2026-10-06T00:00:03Z' if writes.get(path,0)>1 else '2026-10-06T00:00:01Z'
            reply=Reply(status=200 if method=='GET' else 204, json=None,
                headers={'last-modified':'Tue, 06 Oct 2026 00:00:0'+('3' if writes.get(path,0)>1 else '1')+' GMT'},
                before=stamp, after='2026-10-06T00:00:04Z' if shared_clock and writes.get(path,0)>1 else stamp)
            if method=='GET' and 'since' in query:
                reads[path]=reads.get(path,0)+1
                reply['json']=['clock'] if query['since']<'2026-10-06T00:00:03.750Z' else []
                if change: change(path, reads[path], reply)
            elif method=='GET':
                reply['status']=404 if deleted and query.get(key)=='clock' else 200
                reply['json']={'v':'sibling'} if query.get(key)=='preserved' else {'v':writes.get(path,0)}
            return reply
        executor.http=Mock(side_effect=http)
        return executor

    def test_document_equal_null_is_a_failed_response_and_later_checks_continue(self):
        with TemporaryDirectory() as directory:
            def change(path, index, reply):
                if path=='/activities/state' and index==3: reply['json']=None
            executor=self.document_executor(directory, change)
            self.assertEqual(self.execute(executor),1)
            report=json.loads((Path(directory)/'summary.json').read_text())
            procedure=report['procedures'][self.document_procedure]
            self.assertEqual(procedure['outcome'],'failed')
            checks={c['id']:c['outcome'] for c in procedure['checks']}
            self.assertEqual(checks['state-since-equal'],'failed')
            self.assertNotIn('execution-unavailable',checks)
            self.assertEqual(checks['activity-profile-since-equal'],'passed')
            self.assertEqual(checks['state-delete-scope'],'passed')

    def test_document_list_types_fail_for_every_family_and_boundary(self):
        families=[('state','/activities/state'), ('agent-profile','/agents/profile'),
                  ('activity-profile','/activities/profile')]
        for family,path in families:
            for index,label in enumerate(['before','after','equal'],1):
                for bad in [None, {}, 'clock', 42, True, ['clock',None], ['clock',[]], ['clock',{}], ['clock',42]]:
                    with self.subTest(family=family,label=label,bad=bad), TemporaryDirectory() as directory:
                        def change(p,i,reply):
                            if (p,i)==(path,index): reply['json']=bad
                        executor=self.document_executor(directory,change)
                        self.assertEqual(self.execute(executor),1)
                        checks={c['id']:c['outcome'] for c in executor.results[self.document_procedure]['checks']}
                        self.assertEqual(checks[family+'-since-'+label],'failed')
                        self.assertNotIn('execution-unavailable',checks)
                        self.assertEqual(checks['state-delete-scope'],'passed')

    def test_document_list_valid_shapes_retain_exclusive_since_semantics(self):
        for index in [None,1,2,3]:
            with self.subTest(index=index), TemporaryDirectory() as directory:
                def change(path,i,reply):
                    if path=='/activities/state' and i==index:
                        reply['json']=[] if index==1 else ['clock']
                executor=self.document_executor(directory,change)
                self.assertEqual(self.execute(executor),0 if index is None else 1)
                checks=executor.results[self.document_procedure]['checks']
                self.assertFalse(any(c['id']=='execution-unavailable' for c in checks))

    def test_document_shared_clock_probes_preserve_observed_shape_violations(self):
        for invalid_index,interrupt in [(None,False),(1,False),(2,False),(3,False),(1,True)]:
            with self.subTest(index=invalid_index,interrupt=interrupt), TemporaryDirectory() as directory:
                def change(path,index,reply):
                    if path!='/activities/state': return
                    if index==invalid_index: reply['json']=None
                    if index==2 and interrupt: reply['incomplete_body']=True
                executor=self.document_executor(directory,change,shared_clock=True)
                self.assertEqual(self.execute(executor),2 if invalid_index is None else 1)
                checks=executor.results[self.document_procedure]['checks']
                self.assertEqual(any(c['id']=='execution-unavailable' for c in checks),interrupt)
                if invalid_index is not None:
                    self.assertTrue(any(c['outcome']=='failed' for c in checks))

    def test_document_list_interrupted_body_remains_unavailable(self):
        for index in [1,2,3]:
            with self.subTest(index=index), TemporaryDirectory() as directory:
                def change(path,i,reply):
                    if path=='/activities/state' and i==index: reply['incomplete_body']=True
                executor=self.document_executor(directory,change)
                self.assertEqual(self.execute(executor),2)
                checks=executor.results[self.document_procedure]['checks']
                self.assertFalse(any(c['outcome']=='failed' for c in checks))
                unavailable=[c for c in checks if c['id']=='execution-unavailable']
                self.assertEqual(len(unavailable),1)
                self.assertEqual(unavailable[0]['details']['kind'],'EvidenceUnavailable')
                self.assertFalse(unavailable[0]['details']['target_violation'])

    def test_http_decoding_retains_extension_duplicates_for_both_comparisons(self):
        original = {'result':{'extensions':{'urn:x':{'null':None}}}}
        for members, preserved in [('"null":null', True), ('"null":false,"null":null', False),
                                   ('"null":null,"null":null', False)]:
            with self.subTest(members=members), TemporaryDirectory() as directory:
                executor=Executor.__new__(Executor)
                executor.args=SimpleNamespace(endpoint='http://fixture.test')
                executor.out=Path(directory); executor.headers={}; executor.docs=[]
                executor.results={'probe':{'checks':[]}};executor.current='probe';executor.ordinal=0
                connection=Mock();response=connection.getresponse.return_value
                response.status=200
                response.getheaders.return_value=[('X-Experience-API-Version','2.0.0'),
                    ('X-Experience-API-Consistent-Through','2026-01-01T00:00:00Z')]
                raw=('{"result":{"extensions":{"urn:x":{'+members+'}}}}').encode()
                response.read.return_value=raw
                with patch('run_lrs_procedures.http.client.HTTPConnection',return_value=connection):
                    reply=executor.http('GET','/statements')
                self.assertIs(snapshot_equal(original,reply['json']),preserved)
                self.assertEqual(any(o['outcome']=='failed' for o in preservation(original,reply['json'])),not preserved)
                # Raw evidence retains the full text even for ordinary map-view consumers.
                self.assertEqual(reply['body_hex'],raw.hex())
                evidence=json.loads((executor.out/'http.jsonl').read_text())
                recovered=load_json_members(bytes.fromhex(evidence['response']['body_hex']))
                self.assertIs(snapshot_equal(original,recovered),preserved)

    def test_iri_returned_object_types_fail_without_aborting_later_inputs(self):
        for bad in [None, [], {}, 42, 'activity', {'id':[]}, {'id':{}}, {'id':None}]:
            with self.subTest(bad=bad), TemporaryDirectory() as directory:
                executor=self.person_executor(directory, {}, unavailable=False)
                executor.args.procedure=['iri-comparison-allowed-strategies']
                executor.store=Mock(return_value={})
                executor.read_visible=Mock(return_value={'json':{'object':bad}})
                self.assertEqual(self.execute(executor),1)
                checks=executor.results['iri-comparison-allowed-strategies']['checks']
                self.assertFalse(any(c['id']=='execution-unavailable' for c in checks))
                self.assertEqual(executor.read_visible.call_count,5)
                failures=[c for c in checks if c['id'].endswith('-received-identity')]
                self.assertEqual(len(failures),5)
                self.assertTrue(all(c['outcome']=='failed' for c in failures))

    def test_iri_valid_activity_retains_only_the_unimplemented_comparison_scope(self):
        with TemporaryDirectory() as directory:
            executor=self.person_executor(directory, {}, unavailable=False)
            executor.args.procedure=['iri-comparison-allowed-strategies']
            executor.store=Mock(return_value={})
            executor.read_visible=Mock(side_effect=lambda body,admission:{'json':body})
            self.assertEqual(self.execute(executor),2)
            checks=executor.results['iri-comparison-allowed-strategies']['checks']
            self.assertFalse(any(c['id']=='execution-unavailable' or c['outcome']=='failed' for c in checks))
            self.assertEqual(executor.read_visible.call_count,5)

    @staticmethod
    def execute(executor):
        with redirect_stdout(io.StringIO()):
            return executor.execute()

    @staticmethod
    def page(items, more=''):
        return {'status':200, 'url':'http://fixture.test/xapi/statements',
                'headers':{'x-experience-api-version':'2.0.0',
                    'x-experience-api-consistent-through':'2026-01-01T00:00:00Z',
                    'last-modified':'Thu, 01 Jan 2026 00:00:00 GMT'},
                'json':{'statements':items, 'more':more}}

    def test_invalid_page_members_do_not_hide_same_or_later_exclusions(self):
        forbidden={'id':'forbidden', 'stored':'2026-01-01T00:00:00Z'}
        malformed=[{'id':value, 'stored':forbidden['stored']} for value in [None, [], {}, 42, True]]
        malformed += [{'stored':forbidden['stored']}, None, [], 'not-a-statement']
        for bad in malformed:
            for first in [True, False]:
                with self.subTest(bad=bad, first=first):
                    executor=Executor.__new__(Executor)
                    executor.results={'probe':{'checks':[]}}; executor.current='probe'
                    items=[bad, forbidden] if first else [forbidden, bad]
                    executor.http=Mock(side_effect=[self.page(items, '?cursor=2'),self.page([forbidden])])
                    self.assertIsNone(executor.collect_statements({}, 'probe', {'forbidden'}))
                    self.assertEqual(executor.http.call_count, 2)
                    checks=executor.results['probe']['checks']
                    self.assertTrue(any(c['id']=='probe-page-shape' and c['outcome']=='failed' for c in checks))
                    for page in [0, 1]:
                        self.assertTrue(any(c['id']==f'probe-observed-exclusions-{page}'
                                            and c['outcome']=='failed' for c in checks))

    def test_valid_collected_ids_remain_safe_for_membership_consumers(self):
        items=[{'id':'a', 'stored':'2026-01-01T00:00:00Z'},
               {'id':'b', 'stored':'2026-01-01T00:00:00Z'}]
        executor=Executor.__new__(Executor)
        executor.results={'probe':{'checks':[]}}; executor.current='probe'
        executor.http=Mock(return_value=self.page(items))
        rows=executor.collect_statements({}, 'probe', {'excluded'})
        self.assertEqual({item['id'] for item in rows}, {'a','b'})
        self.assertFalse(any(c['outcome']=='failed' for c in executor.results['probe']['checks']))

    @staticmethod
    def person_executor(directory, changes, unavailable):
        root=Path(directory)
        fixture=root/'fixture.json'; fixture.write_text('{"axis":"bounded-control"}')
        runner=root/'runner'; runner.write_text('unused grammar adapter fixture')
        executor=Executor(SimpleNamespace(out=root, fixture=fixture, runner=runner,
            endpoint='http://fixture.test', procedure=['person-semantic-identifiers']))
        def http(method, path, query, **kwargs):
            agent=json.loads(query['agent'])
            got={'objectType':'Person', **{key:[value] for key,value in agent.items()}}
            got.update(copy.deepcopy(changes))
            return {'status':200, 'json':got}
        executor.http=Mock(side_effect=http)
        def syntax(got):
            if unavailable:
                raise OSError('controlled grammar adapter unavailable')
            result={}
            for key in IFI:
                entries=got.get(key, [])
                if not isinstance(entries, list): continue
                for item in entries:
                    value=item.get('homePage') if key=='account' and isinstance(item,dict) else item
                    if isinstance(value,str): result[value,key=='openid']=True
            return result
        executor.identifier_syntax=Mock(side_effect=syntax)
        return executor

    def test_person_type_defects_are_recorded_before_grammar_deferral(self):
        changes=[{key:value} for key in ['name',*IFI] for value in [None, {}, 42, 'not-an-array']]
        changes += [{key:[value]} for key in ['name',*IFI] for value in [None, [], {}, 42]]
        changes += [{'account':[{'homePage':[], 'name':'learner'}]},
                    {'account':[{'homePage':'https://example.test', 'name':None}]},
                    {'objectType':None}]
        for change in changes:
            with self.subTest(change=change), TemporaryDirectory() as directory:
                executor=self.person_executor(directory, change, unavailable=True)
                self.assertEqual(self.execute(executor), 1)
                checks=executor.results['person-semantic-identifiers']['checks']
                self.assertFalse(any(c['id']=='execution-unavailable' for c in checks))
                self.assertEqual(executor.http.call_count, 4)
                self.assertTrue(any(c['outcome']=='failed' for c in checks))
                report=json.loads((Path(directory)/'summary.json').read_text())
                self.assertEqual(report['procedures']['person-semantic-identifiers']['outcome'], 'failed')

    def test_person_valid_shape_and_unknown_grammar_are_not_target_failures(self):
        for unavailable in [False, True]:
            with self.subTest(unavailable=unavailable), TemporaryDirectory() as directory:
                executor=self.person_executor(directory, {'name':['one','two']}, unavailable)
                self.assertEqual(self.execute(executor), 2)  # Association/provider scope remains open.
                checks=executor.results['person-semantic-identifiers']['checks']
                self.assertFalse(any(c['outcome']=='failed' or c['id']=='execution-unavailable' for c in checks))
                self.assertEqual(executor.http.call_count, 4)

    def test_local_mailbox_and_sha_defects_survive_unavailable_grammar(self):
        for change in [{'mbox':['https://example.test']}, {'mbox_sha1sum':['not-a-sha1']}]:
            with self.subTest(change=change), TemporaryDirectory() as directory:
                executor=self.person_executor(directory, change, unavailable=True)
                self.assertEqual(self.execute(executor), 1)
                checks=executor.results['person-semantic-identifiers']['checks']
                self.assertTrue(any(c['id'].endswith('-0') and c['outcome']=='failed' for c in checks))


if __name__=='__main__': unittest.main()
