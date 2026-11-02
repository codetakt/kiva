import copy
import unittest
from pathlib import Path

from lrs_product_capabilities import definitions, evaluate

ROOT = Path(__file__).resolve().parents[2]


class ProductCapabilityGate(unittest.TestCase):
    def setUp(self):
        self.rows = definitions(ROOT)
        self.controls = {name.format(backend='memory') for row in self.rows for name in row['native_controls']}
        self.tests = {name: {'outcome': 'passed'} for name in self.controls}
        self.observations = []
        for row in self.rows:
            for name in row['native_controls']:
                for mode in ('live', 'snapshot') if row['id'] == 'pagination-modes' else (None,):
                    self.observations.append({'test': name.format(backend='memory').removeprefix('kiva-server::'),
                        'kind': 'product-capability', 'observation': {'enabled': True,
                            'capabilities': [row['id']], 'pagination_mode': mode,
                            'self_contained_short_and_opaque_long': mode == 'live'}})

    def evaluate(self, tests=None, observations=None, http=None):
        return evaluate(self.rows, 'memory', self.tests if tests is None else tests,
                        self.observations if observations is None else observations,
                        self.controls if http is None else http)

    def test_complete_positive_controls(self):
        self.assertTrue(self.evaluate()['complete'])

    def test_missing_failed_ignored_or_advisory_controls_cannot_pass(self):
        for control in self.controls:
            for outcome in (None, 'failed', 'not-run', 'observed', 'not-applicable'):
                with self.subTest(control=control, outcome=outcome):
                    tests = copy.deepcopy(self.tests)
                    if outcome is None:
                        del tests[control]
                    else:
                        tests[control]['outcome'] = outcome
                    self.assertFalse(self.evaluate(tests=tests)['complete'])

    def test_disabled_unobserved_external_or_incomplete_http_cannot_pass(self):
        for field, value in [('enabled', False), ('enabled', 'true'), ('capabilities', [])]:
            records = copy.deepcopy(self.observations)
            for record in records:
                record['observation'][field] = value
            self.assertFalse(self.evaluate(observations=records)['complete'])
        self.assertFalse(self.evaluate(observations=[])['complete'])
        self.assertFalse(self.evaluate(http=set())['complete'])
        for backend in ('peer', 'none', ''):
            with self.assertRaises(ValueError):
                evaluate(self.rows, backend, self.tests, self.observations, self.controls)

    def test_both_modes_are_required(self):
        records = [row for row in self.observations if row['observation']['pagination_mode'] != 'snapshot']
        self.assertFalse(self.evaluate(observations=records)['complete'])


if __name__ == '__main__':
    unittest.main()
