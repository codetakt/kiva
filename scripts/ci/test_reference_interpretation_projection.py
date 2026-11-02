"""Interpretation provenance must survive projection and actual outcome reporting."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import yaml

from issued_definition_contracts import branch_interpretations, branch_procedures, load_contracts
from report_issued_test_results import assess


class ReferenceInterpretationProjection(unittest.TestCase):
    def test_defined_branch_has_a_fixed_oracle_and_bound_competing_reading(self):
        document = load_contracts()
        branch = next(b for b in document['branches'] if b['id'] == 'REF-FILTER-01')
        interpretation, = branch_interpretations(document, branch)
        self.assertEqual(interpretation['id'], 'SP-REFERENCE-CONJUNCTION')
        self.assertTrue(interpretation['reasoning'])
        self.assertTrue(interpretation['competing_reading'])
        self.assertEqual(branch['classification'], 'source_settled')
        self.assertEqual(branch['source_residual'], '')
        self.assertEqual(branch['decision_ids'], [])
        procedure = next(p for p in branch_procedures()
                         if p['id'] == 'issued-definition-REF-FILTER-01')
        self.assertEqual(procedure['source_interpretations'], [interpretation])
        self.assertIn('IP-statementref-filters', procedure['rules'])
        self.assertIn('IP-statementref-time', procedure['rules'])

    def test_unknown_duplicate_and_wrong_branch_interpretations_are_rejected(self):
        document = load_contracts()
        branch = next(b for b in document['branches'] if b['id'] == 'REF-FILTER-01')
        for changes in [{'interpretation_ids':['missing']}, {'id':'unrelated-branch'},
                        {'interpretation_ids':['SP-REFERENCE-CONJUNCTION'] * 2}]:
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                branch_interpretations(document, dict(branch, **changes))

    def test_pass_failure_and_unavailability_retain_the_same_interpretation(self):
        document = load_contracts()
        branch = next(b for b in document['branches'] if b['id'] == 'REF-FILTER-01')
        spec = next(p for p in branch_procedures() if p['id'] == 'issued-definition-REF-FILTER-01')
        identity = spec['id']
        projected_branch = dict(id=branch['id'], procedure=identity,
            classification=branch['classification'], source_residual='', decision_ids=[],
            policy_selections={}, source_interpretations=branch_interpretations(document, branch))
        projection = dict(cases={'case':{'sha256':'exact-case'}},
            procedures={identity:{'file':'procedures.yaml'}},
            summary={'test_definition_projection_complete':True,'semantic_coverage_complete':False},
            rules=[dict(id='IP-statementref-filters', strength='data-contract',
                semantic_coverage='not-established-by-static-checks', cases=['case'],
                procedures=[identity], issues=[], dependencies=[],
                test_design={'interpretation_branches':[projected_branch]})])
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'procedures.yaml').write_text(yaml.safe_dump({'tests':[spec]}, sort_keys=False))
            for outcome in ['passed','failed','not-run']:
                with self.subTest(outcome=outcome):
                    http = {'suites':[{'cases':[{'id':'case','outcome':outcome}]}]}
                    result = dict(outcome=outcome, checks=[{'id':'witness','outcome':outcome}],
                        rubric_sha256=hashlib.sha256(json.dumps(spec,ensure_ascii=False,
                            separators=(',', ':')).encode()).hexdigest())
                    report = assess(projection,http,projection['cases'],
                        [{'procedures':{identity:result}}],root=root)
                    context = report['procedure_interpretation_contexts'][identity]
                    self.assertEqual(context['outcome'], outcome)
                    self.assertEqual(context['interpretations'], projected_branch['source_interpretations'])
                    self.assertEqual(report['rules'][0]['http'], {outcome:1})
                    self.assertEqual(report['rules'][0]['disputed_ieee_facets'], {})
                    self.assertFalse(report['rules'][0]['full_requirement_pass'])
                    # A bare PASS is still insufficient evidence for the rubric.
                    self.assertEqual(report['incomplete_procedure_coverage'], [identity])
                    self.assertEqual(bool(report['failed_procedure_checks']), outcome == 'failed')
                    self.assertEqual(bool(report['incomplete_procedures']), outcome == 'not-run')

    def test_existing_source_residual_is_not_erased_by_interpretation_display(self):
        document = load_contracts()
        branch = next(b for b in document['branches'] if b['id'] == 'OPEN-PUT-02')
        self.assertTrue(branch['source_residual'])
        self.assertEqual(branch['classification'], 'source_conflict_with_project_policy')
        self.assertEqual(branch_interpretations(document, branch), [])


if __name__ == '__main__':
    unittest.main()
