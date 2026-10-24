"""Controls for the integration gate; no LRS or provider is started."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from check_conformance_regressions import capture, gate, compare, definitions, digest, expectations, load_json, run, sha, STANDARD, INITIAL_VISIBILITY_DELAY


class RegressionGateControls(unittest.TestCase):
    def setUp(self):
        self.suites = {'suite': 'tests/conformance/http/example.yaml'}
        self.cases = {('suite', name): {'id': name, 'requirements': ['R'],
            'safety': {'destructive': False, 'requires_isolated_lrs': False}}
            for name in ['good', 'gap', 'unavailable']}
        self.expected = {
            ('suite', 'gap'): {'outcome': 'failed', 'failures': ['step store: expected 204, received 400'], 'not_run_reasons': []},
            ('suite', 'unavailable'): {'outcome': 'not-run', 'failures': [], 'not_run_reasons': ['step read: ORACLE-UNAVAILABLE: exact token']}}
        cases = []
        for key, definition in self.cases.items():
            gap = self.expected.get(key, {'outcome': 'passed', 'failures': []})
            cases.append(dict(copy.deepcopy(definition), **copy.deepcopy(gap)))
        self.report = {'schema_version': 2, 'standard': STANDARD, 'mode': 'execute-http',
                       'status': 'failed', 'totals': {'total': 3, 'passed': 1, 'failed': 1, 'not_run': 1, 'validated': 0},
                       'suites': [{'id': 'suite', 'path': self.suites['suite'], 'cases': cases}]}

    def compare(self, report=None, code=1):
        return compare(self.report if report is None else report, code,
                       self.cases, self.suites, self.expected)

    def test_known_gaps_are_integration_only_and_do_not_mutate_report(self):
        before = copy.deepcopy(self.report)
        result = self.compare()
        self.assertEqual(result['regression_gate'], 'accepted')
        self.assertFalse(result['conformance_complete'])
        self.assertEqual(result['runner_status'], 'failed')
        self.assertEqual(result['runner_exit'], 1)
        self.assertEqual(self.report, before)

    def test_new_failures_unavailability_and_stale_gaps_are_rejected(self):
        for index, outcome in [(0, 'failed'), (0, 'not-run'), (1, 'passed'),
                               (1, 'not-run'), (2, 'passed'), (2, 'failed')]:
            with self.subTest(index=index, outcome=outcome):
                report = copy.deepcopy(self.report)
                report['suites'][0]['cases'][index]['outcome'] = outcome
                with self.assertRaisesRegex(ValueError, 'unexpected outcome or stale baseline'):
                    self.compare(report)

    def test_same_count_changed_reason_or_extra_violation_is_rejected(self):
        for index, field, messages in [
            (1, 'failures', ['step store: expected 204, received 500']),
            (1, 'failures', []), (1, 'not_run_reasons', ['transport unavailable']),
            (2, 'not_run_reasons', ['ORACLE-UNAVAILABLE: connection refused']),
            (2, 'failures', ['bad header']), (0, 'failures', ['hidden failure'])]:
            with self.subTest(index=index, field=field):
                report = copy.deepcopy(self.report)
                report['suites'][0]['cases'][index][field] = messages
                with self.assertRaisesRegex(ValueError, 'changed failure/unavailability reasons'):
                    self.compare(report)

    def test_unknown_missing_duplicate_cases_or_suites_are_rejected(self):
        changes = [lambda r: r['suites'][0]['cases'].pop(),
                   lambda r: r['suites'][0]['cases'].append(r['suites'][0]['cases'][0]),
                   lambda r: r['suites'][0]['cases'][0].update(id='unknown'),
                   lambda r: r['suites'].clear(),
                   lambda r: r['suites'].append(r['suites'][0]),
                   lambda r: r['suites'][0].update(id='unknown'),
                   lambda r: r['suites'][0].update(path='other.yaml')]
        for change in changes:
            report = copy.deepcopy(self.report)
            change(report)
            with self.assertRaises(ValueError):
                self.compare(report)

    def test_report_metadata_totals_and_process_exit_must_agree(self):
        for field, value in [('mode', 'validate-only'), ('schema_version', 1),
                             ('standard', 'other'), ('status', 'passed'),
                             ('totals', {'total': 3, 'passed': 3, 'failed': 0, 'validated': 0, 'not_run': 0})]:
            report = copy.deepcopy(self.report)
            report[field] = value
            with self.assertRaises(ValueError):
                self.compare(report)
        for code in [0, 2, 127, -9, True]:
            with self.assertRaises(ValueError):
                self.compare(code=code)
        for field, value in [('requirements', ['wrong']), ('safety', {'destructive': True})]:
            report = copy.deepcopy(self.report)
            report['suites'][0]['cases'][0][field] = value
            with self.assertRaisesRegex(ValueError, 'metadata drift'):
                self.compare(report)

    def test_attempt_failures_must_match_the_exact_known_reason_and_step(self):
        case = self.report['suites'][0]['cases'][2]
        case['retrieval_attempts'] = [{'step': 'read', 'status': 200,
            'pending_visibility': False, 'failures': ['ORACLE-UNAVAILABLE: exact token']}]
        self.compare()
        for field, value in [('failures', ['bad header']), ('step', 'different')]:
            report = copy.deepcopy(self.report)
            report['suites'][0]['cases'][2]['retrieval_attempts'][0][field] = value
            with self.assertRaisesRegex(ValueError, 'retrieval-attempt violation'):
                self.compare(report)
        self.report['suites'][0]['cases'][0]['diagnostic_statuses'] = [403]
        with self.assertRaisesRegex(ValueError, 'diagnostic status'):
            self.compare()

    def test_only_resolved_initial_visibility_delays_are_exempt(self):
        self.cases[('suite', 'good')]['steps'] = [{'id': 'read', 'request': {
            'method': 'GET', 'path': '/statements', 'query': {'statementId': 'id'},
            'await_retrieval_ms': 100}, 'expect': {'status': 200}}]
        pending = {'step': 'read', 'status': 404, 'pending_visibility': True,
                   'failures': [INITIAL_VISIBILITY_DELAY]}
        visible = {'step': 'read', 'status': 200, 'pending_visibility': False, 'failures': []}
        case = self.report['suites'][0]['cases'][0]
        case['retrieval_attempts'] = [copy.deepcopy(pending), copy.deepcopy(pending), copy.deepcopy(visible)]
        self.compare()
        invalid_sequences = [
            [pending], [visible, pending, visible],
            [dict(pending, pending_visibility=False), visible],
            [dict(pending, status=500), visible],
            [dict(pending, failures=[INITIAL_VISIBILITY_DELAY, 'bad header']), visible],
            [dict(pending, failures=['bad header']), visible],
            [pending, dict(visible, step='other')],
            [dict(pending, step='other'), visible],
            [pending, dict(visible, failures=['bad header'])]]
        for sequence in invalid_sequences:
            with self.subTest(sequence=sequence):
                case['retrieval_attempts'] = sequence
                with self.assertRaisesRegex(ValueError, 'retrieval-attempt violation'):
                    self.compare()
        case['retrieval_attempts'] = [pending, visible]
        request = self.cases[('suite', 'good')]['steps'][0]['request']
        for key, bad in [('method', 'HEAD'), ('path', '/agents'), ('await_retrieval_ms', 0), ('query', {})]:
            previous = request[key]
            request[key] = bad
            with self.assertRaisesRegex(ValueError, 'retrieval-attempt violation'):
                self.compare()
            request[key] = previous

    def test_resolved_delay_allows_only_same_case_and_step_baseline_reasons(self):
        self.cases[('suite', 'unavailable')]['steps'] = [{'id': 'read', 'request': {
            'method': 'GET', 'path': '/statements', 'query': {'statementId': 'id'},
            'await_retrieval_ms': 100}, 'expect': {'status': 200}}]
        pending = {'step': 'read', 'status': 404, 'pending_visibility': True,
                   'failures': [INITIAL_VISIBILITY_DELAY]}
        visible = {'step': 'read', 'status': 200, 'pending_visibility': False,
                   'failures': ['ORACLE-UNAVAILABLE: exact token']}
        case = self.report['suites'][0]['cases'][2]
        case['retrieval_attempts'] = [copy.deepcopy(pending), copy.deepcopy(pending), copy.deepcopy(visible)]
        before = copy.deepcopy(self.report)
        result = self.compare()
        self.assertEqual(result['regression_gate'], 'accepted')
        self.assertFalse(result['conformance_complete'])
        self.assertEqual(result['totals']['not_run'], 1)
        self.assertEqual(self.report, before)
        invalid_sequences = [
            [pending], [visible, pending, visible],
            [pending, dict(visible, status=500)],
            [pending, dict(visible, pending_visibility=True)],
            [pending, dict(visible, step='other')],
            [dict(pending, step='other'), visible],
            [pending, dict(visible, failures=['ORACLE-UNAVAILABLE: other token'])],
            [pending, dict(visible, failures=visible['failures'] + ['bad header'])],
            [dict(pending, failures=[INITIAL_VISIBILITY_DELAY, 'bad header']), visible],
            # A reason allowed for a different case/step is not trusted here.
            [pending, dict(visible, failures=['expected 204, received 400'])]]
        for sequence in invalid_sequences:
            with self.subTest(sequence=sequence):
                case['retrieval_attempts'] = sequence
                with self.assertRaisesRegex(ValueError, 'retrieval-attempt violation'):
                    self.compare()
        case['retrieval_attempts'] = [pending, visible]
        self.expected[('suite', 'unavailable')]['not_run_reasons'] = ['step other: ORACLE-UNAVAILABLE: exact token']
        case['not_run_reasons'] = ['step other: ORACLE-UNAVAILABLE: exact token']
        with self.assertRaisesRegex(ValueError, 'retrieval-attempt violation'):
            self.compare()

    def test_resolved_delay_keeps_known_failures_and_unstepped_reasons(self):
        operation = {'request': {'method': 'GET', 'path': '/statements',
            'query': {'voidedStatementId': 'id'}, 'await_retrieval_ms': 100},
            'expect': {'status': 200}}
        for outcome, field, index, name in [
                ('not-run', 'not_run_reasons', 2, 'unavailable'),
                ('failed', 'failures', 1, 'gap')]:
            with self.subTest(outcome=outcome):
                # Root requests have no step prefix; multiple exact baseline
                # reasons must remain unchanged, rather than becoming PASS.
                reasons = ['known first reason', 'known second reason']
                definition = self.cases[('suite', name)]
                definition.update(operation)
                self.expected[('suite', name)][field] = reasons
                case = self.report['suites'][0]['cases'][index]
                case[field] = reasons
                pending = {'step': None, 'status': 404, 'pending_visibility': True,
                           'failures': [INITIAL_VISIBILITY_DELAY]}
                visible = {'step': None, 'status': 200, 'pending_visibility': False,
                           'failures': reasons}
                case['retrieval_attempts'] = [pending, visible]
                self.compare()
                self.assertEqual(case['outcome'], outcome)
                case['retrieval_attempts'] = [pending, dict(visible, failures=reasons + ['unknown'])]
                with self.assertRaisesRegex(ValueError, 'retrieval-attempt violation'):
                    self.compare()
                case['retrieval_attempts'] = [pending, visible]

    def test_duplicate_json_keys_are_not_silently_overwritten(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'report.json'
            path.write_text('{"status":"failed","status":"passed"}')
            with self.assertRaisesRegex(ValueError, 'duplicate JSON key'):
                load_json(path)

    def test_baseline_binds_all_definitions_profiles_and_gap_identity(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            case_file = root / self.suites['suite']
            case_file.parent.mkdir(parents=True)
            case_file.write_text(json.dumps({'suite': {'id': 'suite'}, 'cases': list(self.cases.values())}))
            profiles = ['spec/conformance/endpoint-profile.example.yaml', 'spec/conformance/auth-profile.example.yaml']
            for name in profiles:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('{}')
            _, _, files = definitions(root)
            baseline = {'schema_version': 1, 'claim': 'known-gap-regression-only',
                'case_count': 3, 'case_files': files,
                'profiles': {p: sha(root / p) for p in profiles},
                'gaps': [dict(suite=key[0], case=key[1], case_sha256=digest(self.cases[key]),
                              backends=['memory', 'postgres'], finding='R5-01', **gap)
                         for key, gap in self.expected.items()]}
            path = root / 'spec/conformance/ci-execution-baseline.json'
            path.write_text(json.dumps(baseline))
            self.assertEqual(len(expectations(root, 'memory')[3]), 2)
            baseline['gaps'][0]['finding']='product-followup-duration'
            path.write_text(json.dumps(baseline))
            self.assertEqual(len(expectations(root, 'memory')[3]), 2)
            mutations = [lambda b: b.update(case_count=2),
                         lambda b: b['case_files'].clear(),
                         lambda b: b['profiles'].clear(),
                         lambda b: b['gaps'].append(b['gaps'][0]),
                         lambda b: b['gaps'][0].update(case='missing'),
                         lambda b: b['gaps'][0].update(case_sha256='stale'),
                         lambda b: b['gaps'][0].update(backends=['other']),
                         lambda b: b['gaps'][0].update(backends=[]),
                         lambda b: b['gaps'][0].update(outcome='passed'),
                         lambda b: b['gaps'][0].update(failures=[]),
                         lambda b: b['gaps'][0].update(reason='ignored typo'),
                         *[lambda b, value=value: b['gaps'][0].update(finding=value)
                           for value in ['', '  ', None, 42, [], {}]]]
            for mutate in mutations:
                value = copy.deepcopy(baseline)
                mutate(value)
                path.write_text(json.dumps(value))
                with self.assertRaises(ValueError):
                    expectations(root, 'memory')
            path.write_text(json.dumps(baseline))
            case_file.write_text(case_file.read_text() + '\n')
            with self.assertRaisesRegex(ValueError, 'case-file drift'):
                expectations(root, 'memory')

    def test_wrapper_requires_fresh_artifacts_and_preserves_raw_nonzero_result(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            names = ['spec/conformance/ci-execution-baseline.json',
                     'spec/conformance/schemas/conformance-run-report.schema.json',
                     'spec/conformance/lrs-generated-artifacts.lock.json',
                     'scripts/ci/check_conformance_regressions.py',
                     'scripts/ci/conformance_lib.sh', 'scripts/ci/run_memory_conformance.sh']
            for name in names:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('{}')
            out = root / 'out'
            out.mkdir()
            (out / 'xapi-memory-http.json').write_text('stale report')
            with patch('check_conformance_regressions.subprocess.run') as process:
                with self.assertRaisesRegex(ValueError, 'unused output paths'):
                    run(root, 'memory', out)
                process.assert_not_called()
            (out / 'xapi-memory-http.json').unlink()
            def execute(command, **kwargs):
                self.assertEqual(command, ['bash', 'scripts/ci/run_memory_conformance.sh'])
                raw = Path(kwargs['env']['KIVA_CONFORMANCE_JSON_REPORT'])
                self.assertFalse(raw.exists())
                self.assertNotEqual(raw, out / 'xapi-memory-http.json')
                raw.write_text(json.dumps(self.report))
                Path(kwargs['env']['KIVA_CONFORMANCE_JUNIT_REPORT']).write_text('<testsuite failures="1"/>')
                return type('Exit', (), {'returncode': 1})()
            with patch('check_conformance_regressions.subprocess.run', side_effect=execute), \
                 patch('check_conformance_regressions.check', return_value=self.compare()) as checker, \
                 patch.dict('os.environ', {}, clear=True):
                result = run(root, 'memory', out)
                self.assertEqual(result['runner_exit'], 1)
                self.assertEqual(checker.call_args.args[-1], 1)
                self.assertEqual(load_json(out / 'xapi-memory-http.json'), self.report)
            with patch('check_conformance_regressions.subprocess.run') as process, \
                 patch.dict('os.environ', {}, clear=True):
                process.return_value.returncode = 1
                with self.assertRaisesRegex(ValueError, 'complete report artifacts'):
                    run(root, 'memory', root / 'missing-output')

    def test_capture_survives_regression_and_infrastructure_failures(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            inputs = ['spec/conformance/ci-execution-baseline.json',
                      'spec/conformance/schemas/conformance-run-report.schema.json',
                      'spec/conformance/lrs-generated-artifacts.lock.json',
                      'scripts/ci/check_conformance_regressions.py',
                      'scripts/ci/conformance_lib.sh', 'scripts/ci/run_memory_conformance.sh']
            for name in inputs:
                f = root / name
                f.parent.mkdir(parents=True, exist_ok=True)
                f.write_text('{}')
            def execute(command, **kwargs):
                Path(kwargs['env']['KIVA_CONFORMANCE_JSON_REPORT']).write_text('{"regression":true}')
                Path(kwargs['env']['KIVA_CONFORMANCE_JUNIT_REPORT']).write_text('<testsuite failures="1"/>')
                kwargs['stdout'].write('captured server log\n')
                return type('Exit', (), {'returncode': 1})()
            with patch('check_conformance_regressions.subprocess.run', side_effect=execute), \
                 patch.dict('os.environ', {}, clear=True):
                out = root / 'regression'
                receipt = capture(root, 'memory', out)
            self.assertEqual(receipt['runner_exit'], 1)
            self.assertEqual(receipt['errors'], [])
            before = {f.name: f.read_bytes() for f in out.iterdir()}
            with patch('check_conformance_regressions.check', side_effect=ValueError('new regression')):
                with self.assertRaisesRegex(ValueError, 'new regression'):
                    gate(root, 'memory', out)
            self.assertEqual({f.name: f.read_bytes() for f in out.iterdir()}, before)
            (out / 'xapi-memory-http.json').write_text('{}')
            with self.assertRaisesRegex(ValueError, 'captured report drift'):
                gate(root, 'memory', out)
            for problem in ['no-reports', 'launch-error']:
                out = root / problem
                with patch('check_conformance_regressions.subprocess.run') as process, \
                     patch.dict('os.environ', {}, clear=True):
                    if problem == 'launch-error':
                        process.side_effect = OSError('unavailable shell')
                    else:
                        process.return_value.returncode = 2
                    receipt = capture(root, 'memory', out)
                self.assertTrue(receipt['errors'])
                self.assertTrue((out / 'xapi-memory-execution.json').is_file())
                self.assertTrue((out / 'xapi-memory-execution.log').is_file())
                with self.assertRaisesRegex(ValueError, 'incomplete execution capture'):
                    gate(root, 'memory', out)


if __name__ == '__main__':
    unittest.main()
