"""Mandatory response headers must cover queries appended to storage scenarios."""
import unittest

from lrs_projection_issued import issued_cases


class IssuedHeaderControls(unittest.TestCase):
    def test_both_ifi_queries_have_the_statement_response_contract(self):
        cases, _ = issued_cases([])
        queries = [case for case in cases if case['id'].startswith('issued-mbox-sha1-query-')]
        self.assertEqual(len(queries), 5)
        for case in queries:
            self.assertEqual([step['id'] for step in case['steps'][-2:]],
                             ['same-ifi-query', 'optional-cross-ifi-query'])
            for step in case['steps']:
                with self.subTest(case=case['id'],step=step['id']):
                    headers = step['expect']['headers']
                    self.assertEqual(headers.get('X-Experience-API-Consistent-Through'), {'iso8601': True})
                    self.assertEqual(headers.get('X-Experience-API-Version'), {'equals': '2.0.0'})


if __name__ == '__main__': unittest.main()
