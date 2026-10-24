"""Untrusted response types must not become harness capability failures."""
import copy
from contextlib import redirect_stdout
import io
import json
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace
import unittest
from unittest.mock import Mock

from run_lrs_procedures import Executor, IFI


class ResponseTypeTests(unittest.TestCase):
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
