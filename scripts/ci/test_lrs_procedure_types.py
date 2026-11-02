"""Untrusted response types must not become harness capability failures."""
import copy
from contextlib import redirect_stdout
import io
import json
import subprocess
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

import probe_lrs_authority_filter as authority_probe
from run_lrs_procedures import Executor, IFI, Reply
from lrs_preservation import preservation, snapshot_equal, load_json_members
from report_issued_test_results import assess, procedure_incomplete


class VersionExplanationTests(unittest.TestCase):
    message = 'X-Experience-API-Version must be 2.0 or a normal 2.0.x version; the requested version is unsupported or malformed'

    def observe(self, message, status=400):
        executor=Executor.__new__(Executor)
        executor.current='older-version-routing'
        executor.results={executor.current:{'checks':[]}}
        body={'error':message}
        executor.http=Mock(return_value={'status':status,'json':body,
            'body_hex':json.dumps(body).encode().hex()})
        executor.versions()
        self.assertEqual(executor.http.call_count,3)
        return executor.results[executor.current]['checks']

    def test_reviewed_response_identifies_version_cause_and_supported_family(self):
        checks=self.observe(self.message)
        self.assertEqual([c['id'] for c in checks],
            [version+'-rejection-explanation' for version in ('1.0.3','1.1.0','2.1.0')])
        self.assertTrue(all(c['outcome']=='passed' for c in checks))
        self.assertTrue(all(c['details']['message']==self.message for c in checks))

    def test_unknown_or_misleading_explanations_remain_unavailable(self):
        for message in ('','Bad request','unsupported',None,7,[],{},
                        'Not true: '+self.message,self.message+'; this request was accepted'):
            with self.subTest(message=message):
                self.assertTrue(all(c['outcome']=='not-run' for c in self.observe(message)))

    def test_version_admission_does_not_establish_prior_conformance(self):
        checks=self.observe(self.message,status=200)
        self.assertEqual([c['outcome'] for c in checks],['not-run','not-run','failed'])


class AuthorityCompletionTests(unittest.TestCase):
    procedure = 'related-agents-returned-authority'
    identifier = '00000000-0000-4000-8000-000000000001'

    def test_child_identity_is_stable_across_bootstrap_stopping_points(self):
        for stop in ('PUT','GET',None):
            with self.subTest(stop=stop),TemporaryDirectory() as directory:
                code,outcome,_,_=self.probe(directory,stop)
                _,result,_=self.parent(directory,code,outcome)
                children={c['id']:c for c in outcome['checks']}
                self.assertEqual(len(children),len(outcome['checks']))
                parent={c['id']:c for c in result['checks']}
                for identity,check in children.items():
                    self.assertEqual(parent['authority-helper-'+identity]['outcome'],check['outcome'])
                self.assertIn('authority-helper-bootstrap-put-version-1',parent)
                self.assertNotIn('authority-helper-0',parent)

    def test_missing_or_duplicate_child_ids_never_complete(self):
        for checks in [[{'outcome':'passed'}],
                       [{'id':'query','outcome':'passed'}]*2,
                       [{'id':[],'outcome':'failed'}]]:
            with self.subTest(checks=checks),TemporaryDirectory() as directory:
                outcome={'status':'failed','execution_complete':True,'checks':checks}
                _,result,incomplete=self.parent(directory,1,outcome)
                self.assertEqual(incomplete,[self.procedure])
                self.assertFalse(any(c['id']=='execution-unavailable' for c in result['checks']))

    def probe(self, directory, stop=None, members=False, unresolved=None, returned_id=None):
        output=Path(directory)/'child'
        identity={'mbox':'mailto:authority@example.org'}
        if members:
            identity={'objectType':'Group','member':[identity,{'mbox':'mailto:member@example.org'}]}
        stored={'id':self.identifier if returned_id is None else returned_id,'authority':identity}
        def http(request, **kwargs):
            headers={'X-Experience-API-Version':'2.0.0',
                     'X-Experience-API-Consistent-Through':'2026-01-01T00:00:00Z'}
            if stop==request.method:headers.pop('X-Experience-API-Version')
            response=Mock(status=204 if request.method=='PUT' else 200,headers=headers)
            response.read.return_value=json.dumps(stored).encode()
            response.__enter__=Mock(return_value=response)
            response.__exit__=Mock(return_value=False)
            return response
        def runner(command, **kwargs):
            steps=authority_probe.yaml.safe_load((output/'cases.yaml').read_text())['cases'][0]['steps']
            row={'id':self.procedure,'outcome':'passed','failures':[]}
            if unresolved is not None:
                row['unresolved_checks']=unresolved
            if stop in ('capture','default','false','true','oracle','unknown'):
                position=0 if stop=='capture' else 2 if stop=='oracle' else next(
                    (i for i,s in enumerate(steps) if s['id']=='authority-0-'+stop),1)
                row.update(outcome='not-run' if stop=='oracle' else 'failed')
                reason='ORACLE-UNAVAILABLE: controlled unavailable comparison' if stop=='oracle' else 'controlled assertion failure'
                row['not_run_reasons' if stop=='oracle' else 'failures']=[
                    'step '+steps[position]['id']+': '+reason if stop!='unknown' else 'unrecognized failure scope']
            status='incomplete' if row['outcome']=='not-run' else row['outcome']
            return subprocess.CompletedProcess(command,0 if status=='passed' else 1,
                json.dumps({'status':status,'suites':[{'cases':[row]}]}),'')
        with patch('sys.argv',['probe','--endpoint','http://fixture.test','--runner','/unused',
                               '--output',str(output)]),patch.object(authority_probe.uuid,'uuid4',return_value=self.identifier), \
                patch.object(authority_probe,'urlopen',side_effect=http) as calls, \
                patch.object(authority_probe.subprocess,'run',side_effect=runner) as child,redirect_stdout(io.StringIO()):
            code=authority_probe.main()
        return code,json.loads((output/'outcome.json').read_text()),calls.call_count,child.call_count

    def parent(self, directory, code, outcome):
        output=Path(directory)/'parent';output.mkdir()
        executor=ResponseTypeTests.person_executor(output,{},False)
        executor.args.procedure=[self.procedure]
        destination=output/'authority-filter';destination.mkdir()
        (destination/'outcome.json').write_text(json.dumps(outcome))
        with patch('run_lrs_procedures.subprocess.run',return_value=subprocess.CompletedProcess([],code,'','')):
            exit_code=ResponseTypeTests.execute(executor)
        result=executor.results[self.procedure]
        projection={'cases':{},'procedures':{self.procedure:{'file':'spec/conformance/lrs-review-tests.yaml'}},
                    'summary':{'test_definition_projection_complete':True,'semantic_coverage_complete':False},
                    'rules':[]}
        report=assess(projection,{'suites':[]},{},[{'procedures':executor.results}])
        return exit_code,result,report['incomplete_procedures']

    def test_bootstrap_uuid_identity_requires_standard_syntax(self):
        self.identifier='abcdef01-abcd-4abc-8abc-abcdef012345'
        for returned,valid in [(self.identifier.upper(),True),(self.identifier,True),
                               ('abcdef02-abcd-4abc-8abc-abcdef012345',False),
                               (self.identifier.replace('-',''),False),
                               ('{'+self.identifier+'}',False),('',False),([],False),(42,False)]:
            with self.subTest(returned=returned),TemporaryDirectory() as directory:
                code,outcome,_,children=self.probe(directory,returned_id=returned)
                self.assertEqual(code,0 if valid else 1)
                self.assertEqual(children,1 if valid else 0)
                self.assertEqual(outcome['status'],'passed' if valid else 'failed')
                self.assertEqual(outcome['execution_complete'],valid)

    def test_bootstrap_header_failure_retains_unexecuted_queries_through_collector(self):
        for stop in ('PUT','GET'):
            with self.subTest(stop=stop),TemporaryDirectory() as directory:
                code,outcome,requests,children=self.probe(directory,stop)
                self.assertEqual((code,requests,children),(1,1 if stop=='PUT' else 2,0))
                self.assertFalse(outcome['execution_complete'])
                self.assertEqual({c['id'] for c in outcome['checks'] if c['outcome']=='not-run'},
                                 {'authority-queries-'+mode for mode in ('default','false','true')})
                exit_code,result,incomplete=self.parent(directory,code,outcome)
                self.assertEqual(exit_code,1)
                self.assertEqual(result['outcome'],'failed')
                self.assertEqual(incomplete,[self.procedure])
                self.assertTrue(any(c['outcome']=='not-run' for c in result['checks']))
                self.assertFalse(any(c['id']=='execution-unavailable' for c in result['checks']))

    def test_child_stopping_step_distinguishes_completed_failure_from_skipped_queries(self):
        for stop,expected in [(None,['passed']*3),('capture',['not-run']*3),
                              ('default',['failed','not-run','not-run']),
                              ('false',['passed','failed','not-run']),
                              ('true',['passed','passed','failed']),
                              ('oracle',['passed','not-run','not-run']),('unknown',['not-run']*3)]:
            with self.subTest(stop=stop),TemporaryDirectory() as directory:
                code,outcome,_,_=self.probe(directory,stop)
                queries=[c for c in outcome['checks'] if c.get('id','').startswith('authority-')]
                self.assertEqual([c['outcome'] for c in queries],expected)
                complete='not-run' not in expected
                self.assertEqual(outcome['execution_complete'],complete)
                exit_code,result,incomplete=self.parent(directory,code,outcome)
                self.assertEqual(incomplete,[] if complete else [self.procedure])
                self.assertEqual(exit_code,0 if stop is None else 2 if stop=='oracle' else 1)

    def test_failure_for_one_member_leaves_later_member_queries_unexecuted(self):
        with TemporaryDirectory() as directory:
            code,outcome,_,_=self.probe(directory,'true',members=True)
            queries=[c for c in outcome['checks'] if c.get('id','').startswith('authority-')]
            self.assertEqual([c['outcome'] for c in queries],['passed','passed','failed']+['not-run']*3)
            self.assertEqual(self.parent(directory,code,outcome)[2],[self.procedure])

    def test_runner_unresolved_checks_survive_child_parent_and_collector(self):
        for stop in (None, 'true'):
            for unresolved in (['step authority-0-true: StatementResult aggregate response checks not evaluated'],
                               'invalid evidence', [None]):
                with self.subTest(stop=stop,unresolved=unresolved),TemporaryDirectory() as directory:
                    code,outcome,_,_=self.probe(directory,stop,unresolved=unresolved)
                    self.assertFalse(outcome['execution_complete'])
                    checks={c['id']:c for c in outcome['checks']}
                    self.assertEqual(checks['runner-unresolved-checks']['outcome'],'not-run')
                    self.assertEqual(checks['authority-0-true']['outcome'],'failed' if stop else 'passed')
                    exit_code,result,incomplete=self.parent(directory,code,outcome)
                    self.assertEqual(incomplete,[self.procedure])
                    self.assertEqual(exit_code,1 if stop else 2)
                    self.assertEqual(result['outcome'],'failed' if stop else 'incomplete')

    def test_mixed_legacy_failure_keeps_failure_and_unavailable_subcheck(self):
        steps=[{'id':name} for name in ['capture','query']]
        reasons=['step query: invalid page', 'step query: ORACLE-UNAVAILABLE: remaining pages']
        checks=authority_probe.query_observations(steps,{'outcome':'failed','failures':reasons})
        self.assertEqual([c['outcome'] for c in checks],['failed','not-run'])

    def test_parent_does_not_complete_legacy_or_inconsistent_child_reports(self):
        for outcome in [None,{'status':'failed'},
                        {'status':'failed','execution_complete':True,'checks':[]},
                        {'status':'failed','execution_complete':True,'checks':[{'outcome':'not-run'}]}]:
            with self.subTest(outcome=outcome),TemporaryDirectory() as directory:
                _,result,incomplete=self.parent(directory,1,outcome)
                self.assertEqual(incomplete,[self.procedure])
                if outcome is not None:self.assertEqual(result['outcome'],'failed')


class ResponseTypeTests(unittest.TestCase):
    document_procedure = 'document-updated-time-boundary'

    def test_released_visibility_uses_standard_uuid_identity(self):
        identifier='abcdef01-abcd-4abc-8abc-abcdef012345'
        responses=[(200,{'id':identifier},True),(200,{'id':identifier.upper()},True),
                   (200,{'id':'abcdef02-abcd-4abc-8abc-abcdef012345'},False),
                   (200,{'id':identifier.replace('-','')},False),
                   (200,{'id':[]},False),(200,{},False),(200,None,False),
                   (200,[],False),(404,{'id':identifier},False)]
        responses=[(*row,False) for row in responses]+[(404,None,False,True),(200,None,None,True)]
        for submitted in (identifier,identifier.upper()):
            for status,document,valid,incomplete in responses:
                with self.subTest(submitted=submitted,status=status,document=document,incomplete=incomplete),TemporaryDirectory() as directory:
                    executor=self.person_executor(directory,{},False)
                    name='consistent-through-visibility'
                    executor.args.procedure=[name]
                    executor.fixture['clock_control']=True
                    executor.control=Mock()
                    executor.store=Mock(return_value=True)
                    executor.statement=Mock(return_value={'id':submitted,'verb':{'id':'https://example.test/verb'}})
                    executor.http=Mock(return_value=Reply(status=200,headers={
                        'x-experience-api-consistent-through':'1969-12-31T23:59:59Z'}))
                    executor.read=Mock(return_value=Reply(status=status,json=document,incomplete_body=incomplete))
                    self.assertEqual(self.execute(executor),2 if valid is None else 0 if valid else 1)
                    checks={c['id']:c for c in executor.results[name]['checks']}
                    if valid is None:
                        self.assertEqual(checks['execution-unavailable']['outcome'],'not-run')
                        self.assertNotIn('released-retrieval',checks)
                    else:
                        self.assertNotIn('execution-unavailable',checks)
                        self.assertEqual(checks['released-retrieval']['outcome'],'passed' if valid else 'failed')
                    self.assertEqual(executor.control.call_count,3)

    def test_authority_procedure_checks_every_returned_identifier(self):
        rows=json.loads((Path(__file__).resolve().parents[2]/'tests/conformance/fixtures/preservation-properties.json').read_text())['authority_identifiers']
        for row in rows:
            with self.subTest(id=row['id']),TemporaryDirectory() as directory:
                executor=self.person_executor(directory,{},False)
                executor.current='authority-output'
                executor.results[executor.current]={'checks':[]}
                executor.fixture={'authority_policy':'declared'}
                executor.store=Mock(return_value=True)
                executor.read_visible=Mock(return_value={'json':{'authority':row['authority']}})
                with patch.dict('os.environ',{'KIVA_CONFORMANCE_WRITER_A_TOKEN':'fixture'}):
                    executor.authority()
                checks=executor.results[executor.current]['checks']
                compared=[c for c in checks if '-authority-present-' in c['id'] or c['id'].endswith('-deterministic')]
                self.assertEqual(len(compared),6)
                self.assertTrue(all(c['outcome']==row['outcome'] for c in compared),compared)

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
                    self.assertTrue(any(c['id']=='probe-page-0-shape' and c['outcome']=='failed' for c in checks))
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

    def test_page_exclusions_use_uuid_identity_without_rewriting_response(self):
        identity='abcdef01-abcd-4abc-8abc-abcdef012345'
        for supplied,returned in [(identity,identity.upper()),(identity.upper(),identity)]:
            for interrupted in (False,True):
                with self.subTest(supplied=supplied,interrupted=interrupted):
                    item={'id':returned,'stored':'2026-01-01T00:00:00Z'}
                    executor=Executor.__new__(Executor)
                    executor.results={'probe':{'checks':[]}};executor.current='probe'
                    executor.http=Mock(side_effect=[self.page([item],'?cursor=2'),
                                                   OSError('transport') if interrupted else self.page([])])
                    rows=executor.collect_statements({},'probe',{supplied})
                    self.assertEqual(rows,None if interrupted else [item])
                    checks={c['id']:c for c in executor.results['probe']['checks']}
                    self.assertEqual(checks['probe-observed-exclusions-0']['outcome'],'failed')

    def test_voiding_and_reference_membership_use_uuid_identity(self):
        for upper_input in (False,True):
            with self.subTest(upper_input=upper_input):
                bodies=[{'id':f'abcdef0{i}-abcd-4abc-8abc-abcdef012345','verb':{'id':'verb'},
                         'object':{'id':'activity'},'actor':IFI} for i in range(4)]
                if upper_input:
                    for body in bodies:body['id']=body['id'].upper()
                returned=[dict(body,id=body['id'].swapcase()) for body in bodies]
                executor=Executor.__new__(Executor)
                executor.current='probe';executor.results={'probe':{'checks':[]}}
                executor.store=Mock(return_value=True)
                executor.read_visible=Mock(return_value={'status':200})
                executor.read=Mock(return_value={'status':404})
                executor.statement_headers=Mock()
                executor.http=Mock(return_value={'status':204})
                executor.statement=Mock(side_effect=bodies[:3])
                executor.collect_statements=Mock(side_effect=[[row] for row in returned[:3]])
                executor.voiding()
                checks={c['id']:c['outcome'] for c in executor.results['probe']['checks']}
                self.assertEqual([checks[name+'-retrievability'] for name in ('a','b','c')],
                                 ['failed','passed','passed'])
                executor.results['probe']['checks']=[]
                executor.statement=Mock(side_effect=bodies)
                executor.collect_statements=Mock(side_effect=[returned[:3]]*3+[[returned[3]]])
                executor.references()
                checks={c['id']:c['outcome'] for c in executor.results['probe']['checks']}
                self.assertEqual([checks[name+'-recursive-inheritance'] for name in ('verb','activity','agent')],
                                 ['passed']*3)
                self.assertEqual(checks['context-does-not-inherit'],'failed')

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
