"""The bounded peer permits only its explicit signature prerequisite gap."""
import copy
import unittest
from run_issued_revision_controls import SIGNATURE, SIGNATURE_UNAVAILABLE, validate_control_results


class RevisionControlTests(unittest.TestCase):
    def fixture(self, failed=False):
        selected={SIGNATURE:{'requires_endpoint_profiles':['signature-validation-context']}, 'ordinary':{}}
        cases=[{'id':SIGNATURE, 'outcome':'not-run', 'failures':[],
                'not_run_reasons':[SIGNATURE_UNAVAILABLE]},
               {'id':'ordinary', 'outcome':'failed' if failed else 'passed',
                'failures':['controlled defect'] if failed else [], 'not_run_reasons':[]}]
        report={'suites':[{'cases':cases}], 'status':'failed' if failed else 'incomplete',
                'totals':{'total':2,'passed':0 if failed else 1,'failed':int(failed),'validated':0,'not_run':1}}
        return selected, report

    def test_exact_gap_retains_raw_nonzero_exit_with_or_without_mutations(self):
        for failed in [False,True]:
            selected,report=self.fixture(failed)
            outcomes=validate_control_results(report, selected, {'ordinary'} if failed else set(), 1)
            self.assertEqual(outcomes[SIGNATURE]['outcome'],'not-run')
            self.assertEqual(outcomes[SIGNATURE]['not_run_reasons'],[SIGNATURE_UNAVAILABLE])

    def test_unexpected_case_outcomes_reasons_and_evidence_are_rejected(self):
        for mutation in ['signature-pass','signature-fail','other-not-run','unknown-reason',
                         'extra-reason','missing-reason','hidden-failure','pass-reason','missing',
                         'duplicate','unexpected','totals','status','exit','prerequisite']:
            with self.subTest(mutation=mutation):
                selected,report=self.fixture()
                cases=report['suites'][0]['cases']; exit_code=1
                if mutation=='signature-pass': cases[0]['outcome']='passed'
                elif mutation=='signature-fail': cases[0]['outcome']='failed'
                elif mutation=='other-not-run': cases[1]['outcome']='not-run'
                elif mutation=='unknown-reason': cases[0]['not_run_reasons']=['transport unavailable']
                elif mutation=='extra-reason': cases[0]['not_run_reasons'].append('another gap')
                elif mutation=='missing-reason': cases[0]['not_run_reasons']=[]
                elif mutation=='hidden-failure': cases[0]['failures']=['target defect']
                elif mutation=='pass-reason': cases[1]['not_run_reasons']=['hidden gap']
                elif mutation=='missing': cases.pop()
                elif mutation=='duplicate': cases.append(copy.deepcopy(cases[0]))
                elif mutation=='unexpected': cases[1]['id']='unknown'
                elif mutation=='totals': report['totals']['not_run']=0
                elif mutation=='status': report['status']='passed'
                elif mutation=='exit': exit_code=0
                elif mutation=='prerequisite': selected[SIGNATURE]['requires_endpoint_profiles']=[]
                with self.assertRaises(ValueError):
                    validate_control_results(report, selected, set(), exit_code)

    def test_a_missing_mutation_witness_is_not_a_success(self):
        selected,report=self.fixture()
        with self.assertRaises(ValueError):
            validate_control_results(report,selected,{'ordinary'},1)


if __name__=='__main__': unittest.main()
