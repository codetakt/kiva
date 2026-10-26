"""Wire mutation witnesses require their intended diagnostic and known suffix."""
import copy
import unittest

from run_issued_wire_controls import CASES, validate_control_results

FAILURE = 'step reject-invalid-child: expected status 400, received 204'
SUFFIX = 'step not-stored: not evaluated after step reject-invalid-child stopped execution'


class WireControlTests(unittest.TestCase):
    def fixture(self, mutation):
        failed = mutation == 'coerce-string'
        rows = [{'id': name, 'outcome': 'failed' if failed else 'passed',
                 'failures': [FAILURE] if failed else [],
                 'unresolved_checks': [SUFFIX] if failed else [], 'not_run_reasons': []}
                for name in CASES]
        return {'suites': [{'cases': rows}], 'status': 'failed' if failed else 'passed',
                'totals': {'total': 2, 'passed': 0 if failed else 2,
                           'failed': 2 if failed else 0, 'validated': 0, 'not_run': 0}}

    def test_strict_and_coercing_controls_keep_exact_witnesses(self):
        for mode, code in [('strict', 0), ('coerce-string', 1)]:
            outcomes = validate_control_results(self.fixture(mode), code, mode)
            self.assertEqual(set(outcomes), set(CASES))

    def test_unrelated_and_unavailable_results_are_not_mutation_witnesses(self):
        for mode, code in [('strict', 0), ('coerce-string', 1)]:
            for name in CASES:
                for mutation in ['replace-failure', 'extra-failure', 'extra-unresolved',
                                 'unavailable-reason', 'diagnostic-status', 'missing-suffix',
                                 'wrong-suffix', 'duplicate', 'totals', 'status']:
                    if mode == 'strict' and mutation in ['missing-suffix', 'wrong-suffix']:
                        continue
                    with self.subTest(mode=mode, name=name, mutation=mutation):
                        report = self.fixture(mode)
                        rows = report['suites'][0]['cases']
                        row = next(c for c in rows if c['id'] == name)
                        if mutation == 'replace-failure': row['failures'] = ['missing required Version header']
                        elif mutation == 'extra-failure': row['failures'].append('unrelated header violation')
                        elif mutation == 'extra-unresolved': row['unresolved_checks'].append('ORACLE-UNAVAILABLE: body')
                        elif mutation == 'unavailable-reason': row['not_run_reasons'] = ['ORACLE-UNAVAILABLE: body']
                        elif mutation == 'diagnostic-status': row['diagnostic_statuses'] = [{'status': 400}]
                        elif mutation == 'missing-suffix': row['unresolved_checks'] = []
                        elif mutation == 'wrong-suffix': row['unresolved_checks'] = ['step other: not evaluated']
                        elif mutation == 'duplicate': rows.append(copy.deepcopy(row))
                        elif mutation == 'totals': report['totals']['total'] = 3
                        elif mutation == 'status': report['status'] = 'incomplete'
                        with self.assertRaises(ValueError): validate_control_results(report, code, mode)

    def test_selection_mode_outcome_and_exit_must_match(self):
        for mutation in ['missing', 'unexpected', 'outcome', 'mode', 'exit']:
            with self.subTest(mutation=mutation):
                report = self.fixture('strict'); mode, code = 'strict', 0
                rows = report['suites'][0]['cases']
                if mutation == 'missing': rows.pop()
                elif mutation == 'unexpected': rows[0]['id'] = 'other'
                elif mutation == 'outcome': rows[0]['outcome'] = 'not-run'
                elif mutation == 'mode': mode = 'unknown'
                elif mutation == 'exit': code = 2
                with self.assertRaises(ValueError): validate_control_results(report, code, mode)


if __name__ == '__main__': unittest.main()
