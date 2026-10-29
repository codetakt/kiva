"""Negative controls for rubric discharge, independent of target successes."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from collect_lrs_procedure_evidence import (assemble, rubric_hash, resolve_witness,
    expand_obligations, combined_wrapper_exit, http_observation, native_http)


class ProcedureEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.spec = {'id': 'clock', 'level': 'required', 'fixture': 'Independent clock',
                     'steps': ['Pause admission', 'Read response']}
        self.catalog = {'clock': self.spec}
        self.native = {'tests': {'server::clock': {'outcome': 'passed', 'log': 'actual.log'}}}
        self.binding = {'obligations': ['/fixture', '/steps'],
                        'rationale': 'Actual injected clock and HTTP observations',
                        'witnesses': [{'kind': 'native', 'id': 'server::clock', 'http': True}]}
        self.manifest = {'schema_version': 1, 'procedures': {'clock': {
            'rubric_sha256': rubric_hash(self.spec), 'bindings': [self.binding]}}}
        self.traces = {'server::clock': [{'outcome': 'passed', 'artifact': 'raw.json', 'sha256': 'a'*64}]}
        self.partial = {'procedures': {}}

    def result(self):
        return assemble(self.catalog, self.manifest, self.partial, self.native,
                        self.traces, {}, 'memory')['procedures']['clock']

    def old(self, outcome='not-run'):
        check = {'id': 'clock-unavailable', 'outcome': outcome, 'reason': 'No independent clock'}
        self.partial['procedures']['clock'] = {'rubric_sha256': rubric_hash(self.spec), 'checks': [check]}
        self.binding['supersedes'] = [{'id': check['id'], 'reason': check['reason']}]

    def test_complete_actual_observation(self):
        self.assertTrue(self.result()['execution_complete'])
        self.assertTrue(self.result()['coverage_complete'])

    def test_no_map_no_completion(self):
        self.manifest['procedures'] = {}
        self.assertFalse(self.result()['execution_complete'])

    def test_missing_native_or_trace_is_incomplete(self):
        for native, traces in [({'tests': {}}, self.traces), (self.native, {})]:
            result = assemble(self.catalog, self.manifest, self.partial, native, traces, {}, 'memory')
            self.assertFalse(result['procedures']['clock']['execution_complete'])

    def test_passed_headers_cannot_erase_failed_native(self):
        self.native['tests']['server::clock']['outcome'] = 'failed'
        self.assertEqual(self.result()['outcome'], 'failed')

    def test_failed_headers_cannot_be_discarded(self):
        self.traces['server::clock'].append({'outcome': 'failed'})
        self.assertEqual(self.result()['outcome'], 'failed')

    def test_exact_capability_supersession_keeps_original(self):
        self.old()
        original = copy.deepcopy(self.partial)
        self.assertTrue(self.result()['execution_complete'])
        self.assertEqual(self.partial, original)
        # Partial witnesses must still see the ORIGINAL observation, including
        # after a successful binding supersedes it in the assembled result.
        other = copy.deepcopy(self.binding)
        other['obligations'] = ['/steps/1']
        other['witnesses'] = [{'kind': 'partial', 'id': 'clock-unavailable'}]
        other.pop('supersedes')
        self.binding['obligations'] = ['/fixture', '/steps/0']
        self.manifest['procedures']['clock']['bindings'].append(other)
        self.assertFalse(self.result()['execution_complete'])

    def test_observed_failure_cannot_be_superseded(self):
        self.old('failed')
        self.assertEqual(self.result()['outcome'], 'failed')

    def test_changed_reason_cannot_be_superseded(self):
        self.old()
        self.partial['procedures']['clock']['checks'][0]['reason'] = 'Transport failed during execution'
        with self.assertRaises(ValueError):
            self.result()

    def test_stale_rubric_or_partial_fails(self):
        self.manifest['procedures']['clock']['rubric_sha256'] = '0'*64
        with self.assertRaises(ValueError):
            self.result()
        self.manifest['procedures']['clock']['rubric_sha256'] = rubric_hash(self.spec)
        self.old()
        self.partial['procedures']['clock']['rubric_sha256'] = '0'*64
        with self.assertRaises(ValueError):
            self.result()

    def test_invalid_selectors_or_blank_evidence_fail(self):
        for selectors in [[], ['/missing'], ['/steps', '/steps/0']]:
            with self.assertRaises(ValueError):
                expand_obligations(selectors, {'/steps/0': 'x'})
        self.binding['witnesses'] = []
        with self.assertRaises(ValueError):
            self.result()

    def test_unavailable_partial_does_not_pass(self):
        for checks in [[], [{'id': 'check', 'outcome': 'not-run'}], [{'id': 'check', 'outcome': 'failed'}]]:
            result = resolve_witness({'kind': 'partial', 'id': 'check'}, {}, {}, {}, checks, 'memory')
            self.assertNotEqual(result['outcome'], 'passed')

    def test_http_pass_label_cannot_hide_failure(self):
        self.assertEqual(http_observation({'outcome':'passed','failures':['bad status']}),'failed')
        self.assertEqual(http_observation({'outcome':'passed','unresolved_checks':['unknown']}),'not-run')

    def test_head_uses_received_octets_not_the_preframing_entity(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            (root/'http').mkdir()
            headers=[{'name':'x-experience-api-version','value_hex':b'2.0.0'.hex()}]
            row={'test':'clock','request':{'method':'HEAD','uri':'/agents'},
                 'response':{'status':200,'headers':headers,'body_hex':b'entity'.hex()},
                 'received_response':{'status':200,'headers':headers,'body_hex':''}}
            def observe():
                (root/'http/one.json').write_text(json.dumps(row))
                return native_http(root)['kiva-server::clock'][0]['outcome']
            self.assertEqual(observe(),'passed')
            row['received_response']['body_hex']='00'
            self.assertEqual(observe(),'failed')
            del row['received_response']
            self.assertEqual(observe(),'not-run')

    def test_partial_exit_is_resolved_only_by_complete_verified_evidence(self):
        result={'procedures':{'clock':self.result()}}
        with patch('collect_lrs_procedure_evidence.verify_passing_http') as verified:
            self.assertEqual(combined_wrapper_exit(2,result,{}),0)
            verified.assert_called_once()
            verified.reset_mock()
            self.assertEqual(combined_wrapper_exit(1,result,{}),1)
            verified.assert_not_called()
            result['procedures']['clock']['coverage_complete']=False
            self.assertEqual(combined_wrapper_exit(0,result,{}),2)
            verified.assert_not_called()
        result['procedures']['clock']['coverage_complete']=True
        with patch('collect_lrs_procedure_evidence.verify_passing_http',side_effect=ValueError('HTTP gap')):
            self.assertEqual(combined_wrapper_exit(2,result,{}),2)
        self.assertEqual(combined_wrapper_exit(0,{'procedures':{}},{}),2)

    def branch(self):
        child={'id':'branch','level':'required','steps':['actual behavior']}
        self.catalog['branch']=child
        self.manifest['procedures']['branch']={'rubric_sha256':rubric_hash(child),
            'bindings':[dict(self.binding,obligations=['/steps'])]}
        self.binding['witnesses']=[{'kind':'procedure','id':'branch'}]

    def test_branch_is_evaluated_before_parent_regardless_of_catalog_order(self):
        self.branch()
        self.assertTrue(self.result()['execution_complete'])
        self.assertEqual(self.result()['checks'][0]['details']['witnesses'][0]['id'],'branch')

    def test_missing_uncovered_failed_and_stale_branches_never_pass_parent(self):
        self.branch()
        original=copy.deepcopy(self.manifest)
        del self.manifest['procedures']['branch']
        self.assertFalse(self.result()['execution_complete'])
        self.manifest=copy.deepcopy(original)
        self.catalog['branch']['steps'].append('uncovered obligation')
        with self.assertRaises(ValueError):
            self.result()
        self.manifest['procedures']['branch']['rubric_sha256']=rubric_hash(self.catalog['branch'])
        self.manifest['procedures']['branch']['bindings'][0]['obligations']=['/steps/0']
        self.assertFalse(self.result()['execution_complete'])
        self.catalog['branch']['steps'].pop()
        self.manifest=original
        self.native['tests']['server::clock']['outcome']='failed'
        self.assertEqual(self.result()['outcome'],'failed')

    def test_unknown_and_circular_branches_are_rejected(self):
        self.branch()
        self.manifest['procedures']['branch']['bindings'][0]['witnesses']=[{'kind':'procedure','id':'clock'}]
        with self.assertRaisesRegex(ValueError,'circular'):
            self.result()
        self.manifest['procedures']['branch']['bindings'][0]['witnesses']=[{'kind':'procedure','id':'absent'}]
        with self.assertRaisesRegex(ValueError,'unknown'):
            self.result()


if __name__ == '__main__':
    unittest.main()
