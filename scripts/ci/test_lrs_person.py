"""Execute the six adopted Person association contrasts over independent HTTP.

Peers qualify the oracle, not the Kiva target. Native tests separately establish
Kiva's no-directory fallback inventory and exercise the four IFIs.
"""
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

from lrs_person import IFI, identity, person_associations
from run_lrs_procedures import Executor


VALUES = {
    'mbox': ['mailto:a@example.org', 'mailto:c@example.org', 'mailto:b@example.org'],
    'mbox_sha1sum': ['a'*40, 'c'*40, 'b'*40],
    'openid': ['https://example.org/a', 'https://example.org/c', 'https://example.org/b'],
    'account': [{'homePage':'https://example.org/accounts', 'name': n} for n in ('a','c','b')],
}


def fixture(field, mode):
    requested = {field: VALUES[field][0]}
    associated = {key: values[:2] for key, values in VALUES.items()} if mode == 'enriched' else {field: [VALUES[field][0]]}
    associated['name'] = ['Only A', 'Also A'] if mode == 'enriched' else ['Only A']
    return {'revision':'independent-person-inventory-v1', 'authority':'fixed peer fixture before HTTP',
            'queried': requested, 'mode':'enriched' if mode == 'enriched' else 'fallback',
            'no_additional_information': mode != 'enriched', 'complete':mode != 'unknown',
            'associated':associated, 'unrelated':{**{key:[values[2]] for key,values in VALUES.items()}, 'name':['Only B']}}


class PersonAssociationTests(unittest.TestCase):
    def exercise(self, mode, fault=None, locator_evidence=None, sha1_case=None):
        inventories = {field:fixture(field, mode) for field in IFI}
        class Peer(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_GET(self):
                requested = json.loads(parse_qs(urlsplit(self.path).query)['agent'][0])
                field = next(key for key in IFI if key in requested)
                value = {'objectType':'Person', **copy.deepcopy(inventories[field]['associated'])}
                if mode == 'same-array':
                    value[field].append(VALUES[field][2])
                elif mode == 'other-property':
                    other = 'openid' if field != 'openid' else 'mbox'
                    value[other] = [VALUES[other][2]]
                    value['name'].append('Only B')
                elif mode == 'missing':
                    value[field] = [VALUES[field][2]]
                elif mode == 'unknown':
                    value[field].append(VALUES[field][1])
                if fault == 'account':
                    value['account'] = [{'homePage':'https://example.org/accounts'}]
                elif fault == 'type':
                    value['name'].append(False)
                elif fault == 'grammar':
                    value['openid'] = ['https://[']
                elif fault == 'sha1-grammar':
                    value['mbox_sha1sum'] = ['G'*40]
                if sha1_case is not None and 'mbox_sha1sum' in value:
                    value['mbox_sha1sum'] = [sha1_case(item) for item in value['mbox_sha1sum']]
                data = json.dumps(value).encode()
                self.send_response(200)
                self.send_header('X-Experience-API-Version','2.0.0')
                self.send_header('Content-Length',str(len(data)))
                self.end_headers()
                self.wfile.write(data)

        with ThreadingHTTPServer(('127.0.0.1',0),Peer) as server, tempfile.TemporaryDirectory() as temp:
            thread = threading.Thread(target=server.serve_forever,daemon=True)
            thread.start()
            try:
                root=Path(temp); path=root/'fixture.json'
                path.write_text(json.dumps({'person_inventories':inventories,'locator_evidence':locator_evidence}))
                executor=Executor(SimpleNamespace(out=root,fixture=path,endpoint=f'http://127.0.0.1:{server.server_port}'))
                executor.current='person-control'; executor.results[executor.current]={'checks':[]}
                # Fixed grammar-provider handover for these finite literals only.
                # There is no inference from target output or blanket True stub.
                known={(v,False):True for v in VALUES['mbox']}
                known.update({(v,True):True for v in VALUES['openid']})
                known.update({(v['homePage'],False):True for v in VALUES['account']})
                known[('https://[',True)]=False
                executor.identifier_syntax=lambda _got:known
                executor.person()
                records=[json.loads(line) for line in (root/'http.jsonl').read_text().splitlines()]
                self.assertEqual(len(records),4)
                self.assertTrue(all(record['response']['body_hex'] for record in records))
                if sha1_case is not None:
                    for record in records:
                        body = json.loads(bytes.fromhex(record['response']['body_hex']))
                        for digest in body.get('mbox_sha1sum', []):
                            self.assertEqual(digest, sha1_case(digest))
                return executor.results[executor.current]['checks']
            finally:
                server.shutdown(); thread.join()

    def test_six_association_contrasts_for_all_four_queried_ifis(self):
        for mode in ('fallback','same-array','other-property','enriched','missing','unknown'):
            with self.subTest(mode=mode):
                checks=self.exercise(mode)
                for field in IFI:
                    outcomes={check['outcome'] for check in checks if check['id'].startswith(field+'-')}
                    if mode in ('fallback','enriched'):
                        self.assertEqual(outcomes,{'passed'})
                    elif mode=='unknown':
                        self.assertEqual(outcomes,{'passed','not-run'})
                    else:
                        self.assertIn('failed',outcomes)
                        self.assertNotIn('not-run',outcomes)

    def test_person_locator_semantics_need_independent_resource_evidence(self):
        declarations=json.loads((Path(__file__).resolve().parents[2]/
            'tests/conformance/fixtures/locator-semantics.json').read_text())['resources']
        with patch.dict(VALUES,{'account':[{'homePage':'urn:example:account','name':n} for n in ('a','c','b')]}):
            for evidence,expected in ((None,'not-run'),(declarations,'failed')):
                checks=self.exercise('enriched',locator_evidence=evidence)
                for field in IFI:
                    for index in (0,1):
                        check=next(c for c in checks if c['id']==f'{field}-account-{index}')
                        self.assertEqual(check['outcome'],expected)
                # Lack of locator evidence never hides a missing Account.name.
                malformed=self.exercise('enriched','account',evidence)
                self.assertTrue(any(c['outcome']=='failed' and c['id'].endswith('account-0-type') for c in malformed))

    def test_sha1_identity_accepts_hex_case_without_rewriting_responses(self):
        for query_case, response_case in ((str.lower, str.upper), (str.upper, str.lower),
                                          (str.lower, lambda v: ''.join(c.upper() if i % 2 else c.lower()
                                                                      for i, c in enumerate(v)))):
            values = [query_case(v) for v in VALUES['mbox_sha1sum']]
            with self.subTest(query=values[0]), patch.dict(VALUES, {'mbox_sha1sum': values}):
                for mode in ('fallback', 'enriched'):
                    checks = self.exercise(mode, sha1_case=response_case)
                    self.assertEqual({c['outcome'] for c in checks}, {'passed'})
        # Different digests, malformed hex and contradictory fixture evidence
        # remain failures or unknown; only hexadecimal spelling is equivalent.
        checks = self.exercise('same-array', sha1_case=str.upper)
        self.assertTrue(any(c['outcome']=='failed' and c['id'].endswith('mbox_sha1sum-1-association') for c in checks))
        malformed = self.exercise('unknown', 'sha1-grammar', sha1_case=str.upper)
        self.assertTrue(any(c['outcome']=='failed' and c['id'].endswith('mbox_sha1sum-0') for c in malformed))
        inventory = fixture('mbox_sha1sum', 'fallback')
        inventory['unrelated']['mbox_sha1sum'] = ['A'*40]
        person = {'objectType':'Person', 'mbox_sha1sum':['aA'*20]}
        result = person_associations(inventory['queried'], person, inventory)
        self.assertTrue(any(c['valid'] is None for c in result))
        self.assertFalse(any(c['valid'] is False for c in result))
        self.assertEqual(person['mbox_sha1sum'], ['aA'*20])

    def test_non_hash_identities_remain_case_sensitive(self):
        for field, first, second in (
                ('mbox', 'mailto:A@example.org', 'mailto:a@example.org'),
                ('openid', 'https://example.org/A', 'https://example.org/a'),
                ('name', 'Alice', 'alice'),
                ('account', {'homePage':'https://example.org/a', 'name':'Alice'},
                 {'homePage':'https://example.org/a', 'name':'alice'})):
            with self.subTest(field=field):
                self.assertNotEqual(identity(field, first), identity(field, second))

    def test_unknown_association_cannot_cancel_local_format_failures(self):
        for fault in ('account','type','grammar'):
            checks=self.exercise('unknown',fault)
            for field in IFI:
                outcomes={check['outcome'] for check in checks if check['id'].startswith(field+'-')}
                self.assertIn('failed',outcomes)
                self.assertIn('not-run',outcomes)

    def test_account_pair_and_inventory_applicability_are_not_inferred(self):
        inventory=fixture('account','fallback')
        person={'objectType':'Person','account':[VALUES['account'][0],
                {'homePage':'https://example.org/another','name':'a'}]}
        checks=person_associations(inventory['queried'],person,inventory)
        self.assertIs(checks[-1]['valid'],False)
        for mutation in ('revision','authority','queried','no_additional_information','complete'):
            changed=copy.deepcopy(inventory); changed.pop(mutation)
            result=person_associations(inventory['queried'],person,changed)
            self.assertTrue(any(c['valid'] is None for c in result))
            self.assertIs(result[0]['valid'],True)
        for mutation in ('missing-requested','contradictory','bad-authority'):
            changed=copy.deepcopy(inventory)
            if mutation=='missing-requested':changed['associated']['account']=[]
            elif mutation=='contradictory':changed['unrelated']['account']=changed['associated']['account']
            else:changed['authority']=True
            valid_person={'objectType':'Person','account':[VALUES['account'][0]]}
            result=person_associations(inventory['queried'],valid_person,changed)
            self.assertTrue(any(c['valid'] is None for c in result))
            self.assertFalse(any(c['valid'] is False for c in result))
        # Even partial trusted evidence can establish that a value is B-only.
        unknown=fixture('mbox','unknown')
        result=person_associations(unknown['queried'],{'objectType':'Person','mbox':[VALUES['mbox'][0],VALUES['mbox'][2]]},unknown)
        self.assertTrue(any(c['valid'] is False for c in result))
        self.assertTrue(any(c['valid'] is None for c in result))


if __name__=='__main__':
    unittest.main()
