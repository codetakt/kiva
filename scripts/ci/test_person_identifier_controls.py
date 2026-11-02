"""HTTP locator support must not hide malformed OpenIDs or unavailable observations."""
import copy
import unittest

from check_conformance_regressions import STANDARD
from run_lrs_person_identifier_controls import (
    ACCOUNT, CASES, CHECKS, PROCEDURE_GAPS,
    validate_control_results, validate_procedure_checks,
)


class PersonIdentifierControlTests(unittest.TestCase):
    def fixture(self, mode='valid', focused=True):
        names = sorted(CASES) + ([] if focused else ['ordinary', 'unrelated-known-gap'])
        definitions = {('person', name): {'requirements': ['person-arrays'], 'safety': 'isolated'}
                       for name in names}
        gaps = {}
        if not focused:
            gaps[('person', 'unrelated-known-gap')] = {
                'outcome': 'not-run', 'failures': [], 'not_run_reasons': ['exact unrelated prerequisite']}
        rows = []
        for name in names:
            row = dict(definitions['person', name], id=name, outcome='passed', failures=[], not_run_reasons=[])
            if ('person', name) in gaps:
                row.update(copy.deepcopy(gaps['person', name]))
            if mode == 'invalid' and name in CASES:
                index = 1 if name.endswith('-openid') else 0
                row.update(outcome='failed', failures=[f'/openid/{index} is not valid uri direct syntax'],
                           not_run_reasons=[], unresolved_checks=[])
            rows.append(row)
        report = {'schema_version': 2, 'standard': STANDARD, 'mode': 'execute-http',
                  'suites': [{'id': 'person', 'path': 'tests/person.yaml', 'cases': rows}],
                  'status': 'failed' if mode == 'invalid' else 'passed' if focused else 'incomplete',
                  'totals': {'total': len(rows), 'validated': 0,
                             **{kind.replace('-', '_'): sum(c['outcome'] == kind for c in rows)
                                for kind in ['passed', 'failed', 'not-run']}}}
        return report, definitions, {'person': 'tests/person.yaml'}, gaps

    def validate(self, fixture, mode='valid', focused=True, exit_code=None):
        if exit_code is None: exit_code = 0 if mode == 'valid' and focused else 1
        report, definitions, suites, gaps = fixture
        return validate_control_results(report, exit_code, definitions, suites, gaps, mode, focused)

    def test_resolved_http_locator_and_all_four_defects_in_both_selection_modes(self):
        for focused in [True, False]:
            valid = self.validate(self.fixture(focused=focused), focused=focused)
            invalid = self.validate(self.fixture('invalid', focused), 'invalid', focused)
            self.assertEqual(valid[ACCOUNT]['outcome'], 'passed')
            self.assertEqual(valid[ACCOUNT]['not_run_reasons'], [])
            self.assertTrue(all(invalid[name]['outcome'] == 'failed' for name in CASES))
            self.assertEqual({name for name in valid if valid[name]['outcome'] != invalid[name]['outcome']}, CASES)

    def test_resolved_provider_never_allows_failure_or_unavailable_reason(self):
        for mutation in ['wrong-reason', 'extra-reason', 'hidden-failure',
                         'account-not-run', 'ordinary-not-run', 'pass-unresolved', 'missing',
                         'duplicate', 'totals', 'status', 'exit', 'baseline', 'unknown-mode']:
            with self.subTest(mutation=mutation):
                fixture = self.fixture(); report, _, _, gaps = fixture
                rows = report['suites'][0]['cases']; by_id = {c['id']: c for c in rows}
                account = by_id[ACCOUNT]; ordinary = by_id['person-response-all-identifiers-mbox']
                exit_code, mode = 0, 'valid'
                if mutation == 'wrong-reason': account['not_run_reasons'] = ['transport unavailable']
                elif mutation == 'extra-reason': account['not_run_reasons'].append('another unavailable check')
                elif mutation == 'hidden-failure': account['failures'] = ['unexpected target violation']
                elif mutation == 'account-not-run': account['outcome'] = 'not-run'
                elif mutation == 'ordinary-not-run': ordinary['outcome'] = 'not-run'
                elif mutation == 'pass-unresolved': ordinary['unresolved_checks'] = ['body not evaluated']
                elif mutation == 'missing': rows.pop()
                elif mutation == 'duplicate': rows.append(copy.deepcopy(account))
                elif mutation == 'totals': report['totals']['not_run'] = 1
                elif mutation == 'status': report['status'] = 'incomplete'
                elif mutation == 'exit': exit_code = 1
                elif mutation == 'baseline': gaps['person', ACCOUNT] = {'outcome':'not-run','failures':[],'not_run_reasons':['unknown gap']}
                elif mutation == 'unknown-mode': mode = 'other'
                with self.assertRaises(ValueError): self.validate(fixture, mode=mode, exit_code=exit_code)

    def test_each_malformed_openid_requires_its_own_exact_failure(self):
        for name in CASES:
            for mutation in ['not-run', 'passed', 'different-failure', 'extra-failure']:
                with self.subTest(name=name, mutation=mutation):
                    fixture = self.fixture('invalid')
                    row = next(c for c in fixture[0]['suites'][0]['cases'] if c['id'] == name)
                    if mutation in ('not-run', 'passed'): row['outcome'] = mutation
                    elif mutation == 'different-failure': row['failures'] = ['some other defect']
                    else: row['failures'].append('some other defect')
                    with self.assertRaises(ValueError): self.validate(fixture, 'invalid')

    def test_full_suite_preserves_unrelated_baseline_and_rejects_regressions(self):
        for name, mutation in [('ordinary', 'not-run'), ('unrelated-known-gap', 'passed'),
                               ('unrelated-known-gap', 'changed-reason')]:
            with self.subTest(name=name, mutation=mutation):
                fixture = self.fixture(focused=False)
                row = next(c for c in fixture[0]['suites'][0]['cases'] if c['id'] == name)
                if mutation == 'changed-reason': row['not_run_reasons'] = ['new failure reason']
                else: row['outcome'] = mutation
                with self.assertRaises(ValueError): self.validate(fixture, focused=False)

    def test_expected_results_reject_unrelated_unresolved_observations(self):
        for focused in [True, False]:
            for mode in ['valid', 'invalid']:
                for name in CASES:
                    with self.subTest(focused=focused, mode=mode, name=name):
                        fixture = self.fixture(mode, focused)
                        row = next(c for c in fixture[0]['suites'][0]['cases'] if c['id'] == name)
                        row.setdefault('unresolved_checks', []).append('ORACLE-UNAVAILABLE: interrupted response')
                        with self.assertRaises(ValueError): self.validate(fixture, mode, focused)

    def test_valid_account_rejects_any_unavailable_observation(self):
        for unresolved in [['old IRL gap'], ['old IRL gap', 'old IRL gap'], ['another unavailable check']]:
            for focused in [True, False]:
                with self.subTest(unresolved=unresolved, focused=focused):
                    fixture = self.fixture(focused=focused)
                    row = next(c for c in fixture[0]['suites'][0]['cases'] if c['id'] == ACCOUNT)
                    row['unresolved_checks'] = unresolved
                    with self.assertRaises(ValueError): self.validate(fixture, focused=focused)

    def checks(self, mode):
        return ([{'id': name, 'outcome': 'failed' if mode == 'invalid' else 'passed'} for name in sorted(CHECKS)]
                + [{'id': name, 'outcome': 'not-run', 'reason': reason} for name, reason in PROCEDURE_GAPS.items()])

    def test_procedure_retains_locator_gap_and_independent_openid_contrasts(self):
        for mode, exit_code in [('valid', 2), ('invalid', 1)]:
            failed = validate_procedure_checks(self.checks(mode), exit_code, mode)
            self.assertEqual(failed, CHECKS if mode == 'invalid' else set())

    def test_procedure_rejects_missing_witnesses_and_unexpected_gaps(self):
        for mode, exit_code in [('valid', 2), ('invalid', 1)]:
            for mutation in ['missing', 'duplicate', 'unknown-gap', 'changed-reason', 'hidden-defect', 'exit']:
                with self.subTest(mode=mode, mutation=mutation):
                    checks = self.checks(mode); code = exit_code
                    if mutation == 'missing': checks.pop(0)
                    elif mutation == 'duplicate': checks.append(copy.deepcopy(checks[0]))
                    elif mutation == 'unknown-gap': checks.append({'id': 'transport', 'outcome': 'not-run'})
                    elif mutation == 'changed-reason': checks[-1]['reason'] = 'transport unavailable'
                    elif mutation == 'hidden-defect': checks.append({'id': 'unrelated', 'outcome': 'failed'})
                    elif mutation == 'exit': code = 0
                    with self.assertRaises(ValueError): validate_procedure_checks(checks, code, mode)


if __name__ == '__main__': unittest.main()
