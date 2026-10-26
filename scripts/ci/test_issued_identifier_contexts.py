"""Identifier format interpretation must not swallow primitive type failures."""
import unittest
from pathlib import Path

import yaml

from lrs_issued_bindings import identifier_format_bindings
from report_issued_test_results import assess


class IdentifierContexts(unittest.TestCase):
    def test_irl_and_derived_transport_keep_the_dispute(self):
        cases = {'irl': {'request': {'body': {'moreInfo':'no-scheme'}}},
                 'post': {'request': {}}, 'batch': {'request': {}}}
        models = {'definition': {'fields': {'moreInfo': {'format':'irl'}}}}
        bindings = [dict(model='definition', field='moreInfo', condition='moreInfo-value-1',
                         placement='', case='irl')]
        mapped = identifier_format_bindings(cases, models, bindings, {'post':'irl', 'batch':'irl'})
        for name in cases:
            self.assertIn('IP-identifier-validation-waiver', mapped[name])

    def test_query_format_does_not_depend_on_case_name_or_status(self):
        cases = {str(i): {'request': {'query': query}} for i,query in enumerate([
            {'activityId':'no-scheme'}, {'registration':'not-a-uuid'},
            {'activityId':'https://example.org/good'}, {'statementId':'{{saved_id}}'},
        ])}
        mapped = identifier_format_bindings(cases, {}, [], {})
        self.assertEqual(set(mapped), {'0','1'})
        self.assertTrue(all('IP-parameter-format' in rules for rules in mapped.values()))

    def test_nonstring_array_element_is_a_type_violation(self):
        cases = {str(i): {'request': {'body': {'relevantTypes': value}}}
                 for i,value in enumerate([['no-scheme'], [None], [42]])}
        models = {'relationship': {'fields': {'relevantTypes': {'format':'iri-array'}}}}
        bindings = [dict(model='relationship', field='relevantTypes', condition='relevantTypes-value-' + n,
                         placement='', case=n) for n in cases]
        mapped = identifier_format_bindings(cases, models, bindings, {})
        self.assertEqual(set(mapped), {'0'})

    def test_numeric_language_map_value_is_not_an_identifier_waiver(self):
        cases = {'language-map': {'request': {'body': {'name':{'en':1}}}}}
        models = {'definition': {'fields': {'name': {'format':'language-map'}}}}
        bindings = [dict(model='definition', field='name', condition='name-value-1', placement='', case='language-map')]
        self.assertEqual(identifier_format_bindings(cases, models, bindings, {}), {})

    def test_nested_mailto_and_derived_transports_keep_format_context(self):
        cases = {'mail': {'request': {'body': {'object': {'mbox': 'learner@example.org'}}}},
                 'numeric': {'request': {'body': {'object': {'mbox': 42}}}},
                 'post': {'request': {}}, 'batch': {'request': {}}}
        models = {'agent': {'fields': {'mbox': {'format': 'mailto'}}}}
        bindings = [dict(model='agent', field='mbox', condition='mbox-value-' + n,
                         placement='/object', case=n) for n in ['mail','numeric']]
        mapped = identifier_format_bindings(cases, models, bindings, {'post':'mail','batch':'mail'})
        self.assertEqual(set(mapped), {'mail','post','batch'})
        for name in mapped:
            self.assertIn('IP-table-format', mapped[name])

    def test_real_handwritten_probes_reach_reporter_dispute_context(self):
        root = Path(__file__).resolve().parents[2]
        expected = {
            'statement-post-rejects-non-uuid-statement-id': 'OPEN-FORMAT',
            'statement-post-rejects-invalid-verb-iri': 'OPEN-FORMAT',
            'statement-actor-rejects-invalid-openid': 'OPEN-FORMAT',
            'statement-actor-rejects-invalid-account': 'OPEN-FORMAT',
            'statement-actor-rejects-invalid-mbox': 'OPEN-FORMAT',
            'statement-context-rejects-invalid-registration': 'OPEN-FORMAT',
            'statement-context-rejects-invalid-context-agent': 'OPEN-ARRAY',
        }
        cases = {}
        for file in ['statement-validation-isolated.yaml','statement-actor-validation-isolated.yaml',
                     'statement-context-validation-isolated.yaml']:
            for case in yaml.load((root/'tests/conformance/http'/file).read_text(), Loader=yaml.CSafeLoader)['cases']:
                if case['id'] in expected:
                    cases[case['id']] = case
        self.assertEqual(set(cases), set(expected))
        mapped = identifier_format_bindings(cases, {}, [], {})
        catalog = yaml.load((root/'spec/requirements/ieee-39274-1-1-2025.yaml').read_text(), Loader=yaml.CSafeLoader)
        # Exercise the real reporter with actual rule issues, rather than
        # declaring that the selector's output alone is a displayed context.
        rules = [dict(id=r['id'], strength=r['strength'], issues=r['issues'], dependencies=r['dependencies'],
                      cases=[n for n in cases if r['id'] in mapped.get(n, set())],
                      procedures=[], semantic_coverage='not-established-by-static-checks')
                 for r in catalog['issued_contract']['rules'] if any(r['id'] in ids for ids in mapped.values())]
        receipt = {n: {'sha256': 'unit-control'} for n in cases}
        projection = dict(cases=receipt, rules=rules, procedures={}, summary={'semantic_coverage_complete':False})
        result = assess(projection, {'suites':[{'cases':[{'id':n,'outcome':'failed'} for n in cases]}]}, receipt, [])
        for name, issue in expected.items():
            self.assertIn(issue, result['case_interpretation_contexts'][name])
        self.assertTrue(all(not r['full_requirement_pass'] for r in result['rules']))

    def test_compound_account_probe_reports_the_undisputed_missing_name(self):
        root = Path(__file__).resolve().parents[2]
        case_id = 'statement-actor-rejects-invalid-account'
        suite = yaml.load((root/'tests/conformance/http/statement-actor-validation-isolated.yaml').read_text(), Loader=yaml.CSafeLoader)
        case = next(c for c in suite['cases'] if c['id'] == case_id)
        account = case['request']['body']['actor']['account']
        self.assertNotIn('name', account)
        self.assertEqual(account['homePage'], 'relative/home')
        mapped = identifier_format_bindings({case_id:case}, {}, [], {})
        catalog = yaml.load((root/'spec/requirements/ieee-39274-1-1-2025.yaml').read_text(), Loader=yaml.CSafeLoader)
        rules = [dict(id=r['id'], strength=r['strength'], issues=r['issues'], dependencies=r['dependencies'],
                      cases=[case_id], procedures=[], semantic_coverage='not-established-by-static-checks')
                 for r in catalog['issued_contract']['rules'] if r['id'] in mapped[case_id]]
        receipt = {case_id:{'sha256':'unit-control'}}
        projection = dict(cases=receipt, rules=rules, procedures={}, summary={'semantic_coverage_complete':False})
        for outcome in ['passed', 'failed']:
            result = assess(projection, {'suites':[{'cases':[{'id':case_id, 'outcome':outcome}]}]}, receipt, [])
            presence = next(r for r in result['rules'] if r['id'] == 'IP-Account-name-presence')
            self.assertEqual(presence['http'], {outcome:1})
            self.assertEqual(presence['interpretation_dependencies'], [])
            self.assertFalse(presence['full_requirement_pass'])
            self.assertEqual(result['summary']['http_outcomes'], {outcome:1})
            self.assertEqual(result['case_interpretation_contexts'][case_id], ['OPEN-FORMAT'])


if __name__ == '__main__':
    unittest.main()
