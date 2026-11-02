"""Independent resource declarations and actual HTTP field-oracle controls."""
import copy
import json
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
from pathlib import Path
import tempfile
import threading
from types import SimpleNamespace
import unittest
from urllib.parse import quote
from lrs_locator import locator_observations
from lrs_preservation import locator_validity
from run_lrs_procedures import Executor

ROOT=Path(__file__).resolve().parents[2]
DECLARATIONS=json.loads((ROOT/'tests/conformance/fixtures/locator-semantics.json').read_text())['resources']


def document(locator):
    agent={'account':{'homePage':locator,'name':'assigned-learner'}}
    group={'objectType':'Group',**agent,'member':[agent]}
    activity={'id':'urn:example:activity','definition':{'moreInfo':locator}}
    value={'id':'00000000-0000-4000-8000-000000000001','actor':group,
           'authority':group,'verb':{'id':'urn:example:verb'},'object':activity,
           'context':{'instructor':group,'team':group,
                      'contextAgents':[{'objectType':'contextAgent','agent':agent,'relevantTypes':['urn:example:relationship']}],
                      'contextGroups':[{'objectType':'contextGroup','group':group,'relevantTypes':['urn:example:relationship']}],
                      'contextActivities':{key:[activity] for key in ('parent','grouping','category','other')}},
           'attachments':[{'fileUrl':locator}],
           'result':{'extensions':{'urn:example:owner':{'account':{'homePage':'opaque'},'moreInfo':None,'fileUrl':17}}}}
    return copy.deepcopy(value)


class LocatorEvidenceTests(unittest.TestCase):
    def test_exact_resource_semantics_do_not_become_scheme_rules(self):
        for value in ('https://example.invalid/account','https://example.invalid/学習'):
            self.assertIs(locator_validity(value),True)
        name='urn:example:account'
        self.assertIs(locator_validity(name,DECLARATIONS),False)
        self.assertIsNone(locator_validity(name))
        self.assertIsNone(locator_validity('urn:example:other',DECLARATIONS))
        self.assertIsNone(locator_validity('custom:fixture-resource'))
        resolver={'custom:fixture-resource':b'controlled resource'}
        self.assertEqual(resolver['custom:fixture-resource'],b'controlled resource')
        self.assertIs(locator_validity('custom:fixture-resource',DECLARATIONS),True)
        self.assertEqual(quote('https://example.invalid/学習',safe=":/"),
                         'https://example.invalid/%E5%AD%A6%E7%BF%92')
        for key,value in (('mapped_uri','urn:example:wrong'),('source',''),('classification','unknown'),('access_mechanism','contradicts pure-name')):
            broken=copy.deepcopy(DECLARATIONS);broken[name][key]=value
            self.assertIsNone(locator_validity(name,broken))
        self.assertIs(locator_validity('https://[',DECLARATIONS),False)

    def test_actual_http_each_locator_role_and_person_element(self):
        holder={}
        class Peer(BaseHTTPRequestHandler):
            def log_message(self,*args):pass
            def do_GET(self):
                raw=json.dumps(holder['value']).encode()
                self.send_response(200)
                self.send_header('X-Experience-API-Version','2.0.0')
                self.send_header('X-Experience-API-Consistent-Through','2026-10-27T00:00:00Z')
                self.send_header('Content-Type','application/json')
                self.send_header('Content-Length',str(len(raw)))
                self.end_headers();self.wfile.write(raw)
        with ThreadingHTTPServer(('127.0.0.1',0),Peer) as server,tempfile.TemporaryDirectory() as temp:
            thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
            try:
                root=Path(temp);fixture=root/'fixture.json';fixture.write_text('{}')
                executor=Executor(SimpleNamespace(out=root,fixture=fixture,endpoint=f'http://127.0.0.1:{server.server_port}'))
                executor.current='locator-control';executor.results[executor.current]={'checks':[]}
                for iri,semantics,expected in [('https://example.invalid/account',None,True),
                    ('https://example.invalid/学習',None,True),('urn:example:account',DECLARATIONS,False),
                    ('urn:example:account',None,None),('custom:fixture-resource',DECLARATIONS,True)]:
                    for role in ('statement','object-group','substatement','substatement-object-group','person'):
                        if role=='person':
                            value={'objectType':'Person','account':[{'homePage':iri,'name':'A'},{'homePage':iri,'name':'B'}]}
                        else:
                            value=document(iri)
                            if role.startswith('substatement'):
                                child=document(iri)
                                for key in ('id','authority'):child.pop(key)
                                child['objectType']='SubStatement'
                                if role.endswith('object-group'):child['object']=child['actor']
                                value['object']=child
                            elif role=='object-group':value['object']=value['actor']
                        holder['value']=value
                        received=executor.http('GET','/agents' if role=='person' else '/statements',expected_statuses=(200,))
                        observations=locator_observations(received['json'],'person' if role=='person' else 'statement',semantics)
                        locators=[o for o in observations if o['kind']=='locator']
                        self.assertTrue(locators)
                        account_paths=['/actor','/actor/member/0','/authority','/authority/member/0',
                            '/context/instructor','/context/instructor/member/0','/context/team','/context/team/member/0',
                            '/context/contextAgents/0/agent','/context/contextGroups/0/group','/context/contextGroups/0/group/member/0']
                        expected_paths={p+'/account/homePage' for p in account_paths}|{'/object/definition/moreInfo','/attachments/0/fileUrl'}|{
                            '/context/contextActivities/'+k+'/0/definition/moreInfo' for k in ('parent','grouping','category','other')}
                        if role=='person':expected_paths={'/account/0/homePage','/account/1/homePage'}
                        elif role.startswith('substatement'):
                            child_paths={p for p in expected_paths if not p.startswith('/authority')}
                            if role.endswith('object-group'):
                                child_paths.remove('/object/definition/moreInfo')
                                child_paths|={'/object/account/homePage','/object/member/0/account/homePage'}
                            expected_paths.remove('/object/definition/moreInfo')
                            expected_paths|={'/object'+p for p in child_paths}
                        elif role=='object-group':
                            expected_paths.remove('/object/definition/moreInfo')
                            expected_paths|={'/object/account/homePage','/object/member/0/account/homePage'}
                        self.assertEqual({o['pointer'] for o in locators},expected_paths)
                        for item in locators:self.assertIs(item['valid'],expected,(role,item))
                        self.assertFalse(any('/extensions/' in o['pointer'] for o in observations))
                        if role=='person':
                            broken=copy.deepcopy(received['json']);broken['account'][0].pop('name');broken['account'][1]['homePage']='https://['
                            checks=locator_observations(broken,'person')
                            self.assertIs(next(c['valid'] for c in checks if c['pointer']=='/account/0/name'),False)
                            self.assertIs(next(c['valid'] for c in checks if c['pointer']=='/account/1/homePage'),False)
            finally:server.shutdown();thread.join()


if __name__=='__main__':unittest.main()
