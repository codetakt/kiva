"""The bounded peer permits only its explicit signature prerequisite gap."""
import copy
import unittest
from run_issued_revision_controls import (
    DUPLICATE, SIGNATURE, SIGNATURE_UNAVAILABLE, VERSION, validate_control_results,
)

VERSION_FAILURE = 'step preserve-accepted-version: JSON pointer /version equals failed for "1.1.0"'


class RevisionControlTests(unittest.TestCase):
    def fixture(self, failed=False):
        selected={SIGNATURE:{'requires_endpoint_profiles':['signature-validation-context']},
                  VERSION:{'steps':[
                      {'id':'submit-nondefault-version', 'request':{'method':'POST'}, 'expect':{'status':200}},
                      {'id':'preserve-accepted-version', 'request':{'method':'GET'},
                       'expect':{'status':200, 'json':{'pointers':{'/version':{'equals':'1.1.0'}}}}},
                  ]}}
        cases=[{'id':SIGNATURE, 'outcome':'not-run', 'failures':[],
                'not_run_reasons':[SIGNATURE_UNAVAILABLE]},
               {'id':VERSION, 'outcome':'failed' if failed else 'passed',
                'failures':[VERSION_FAILURE] if failed else [], 'not_run_reasons':[]}]
        report={'suites':[{'cases':cases}], 'status':'failed' if failed else 'incomplete',
                'totals':{'total':2,'passed':0 if failed else 1,'failed':int(failed),'validated':0,'not_run':1}}
        return selected, report

    def test_exact_gap_retains_raw_nonzero_exit_with_or_without_mutations(self):
        for failed in [False,True]:
            selected,report=self.fixture(failed)
            outcomes=validate_control_results(report, selected, {VERSION} if failed else set(), 1)
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
            validate_control_results(report,selected,{VERSION},1)

    def test_unrelated_or_additional_failure_cannot_witness_the_mutation(self):
        unrelated='step preserve-accepted-version: missing response header X-Experience-API-Version'
        unavailable='step preserve-accepted-version: ORACLE-UNAVAILABLE: response body interrupted'
        for failures in [[unrelated], [unrelated, unavailable], [VERSION_FAILURE, unrelated],
                         [VERSION_FAILURE, unavailable], [VERSION_FAILURE, VERSION_FAILURE]]:
            with self.subTest(failures=failures):
                selected,report=self.fixture(True)
                report['suites'][0]['cases'][1]['failures']=failures
                with self.assertRaises(ValueError):
                    validate_control_results(report,selected,{VERSION},1)

    def test_unavailable_observations_never_count_as_completed_controls(self):
        for failed in [False,True]:
            for index in [0,1]:
                with self.subTest(failed=failed,index=index):
                    selected,report=self.fixture(failed)
                    report['suites'][0]['cases'][index]['unresolved_checks']=[
                        'ORACLE-UNAVAILABLE: response body interrupted']
                    with self.assertRaises(ValueError):
                        validate_control_results(report,selected,{VERSION} if failed else set(),1)

    def admission_fixture(self, name, method, step_id, expected, received):
        selected,report=self.fixture(True)
        selected.pop(VERSION)
        selected[name]={'steps':[
            {'id':step_id, 'request':{'method':method}, 'expect':{'status':expected}},
            {'id':'remaining-check', 'request':{'method':'GET'}, 'expect':{'status':200}},
        ]}
        case=report['suites'][0]['cases'][1]
        case.update(id=name, failures=[f'step {step_id}: expected status {expected}, received {received}'],
                    unresolved_checks=[f'step remaining-check: not evaluated after step {step_id} stopped execution'])
        return selected,report

    def test_admission_mutations_require_the_exact_status_and_unvisited_suffix(self):
        vectors=[(DUPLICATE,'POST','reject-identical-members',400,200)]
        for method in ['PUT','POST']:
            success=204 if method=='PUT' else 200
            vectors.extend([
                (f'issued-completion-sub-forbidden-id-{method.lower()}',method,'reject-invalid-child',400,success),
                (f'issued-completion-root-duration-day-{method.lower()}',method,'store',success,400),
            ])
        for vector in vectors:
            name=vector[0]
            with self.subTest(vector=vector):
                selected,report=self.admission_fixture(*vector)
                validate_control_results(report,selected,{name},1)
                for mutation in ['wrong-status','wrong-step','missing-suffix','extra-unavailable','wrong-suffix']:
                    bad=copy.deepcopy(report)
                    case=bad['suites'][0]['cases'][1]
                    if mutation=='wrong-status': case['failures']=[case['failures'][0].rsplit(' ',1)[0]+' 500']
                    elif mutation=='wrong-step': case['failures']=[case['failures'][0].replace(vector[2],'other-step')]
                    elif mutation=='missing-suffix': case['unresolved_checks']=[]
                    elif mutation=='extra-unavailable': case['unresolved_checks'].append('ORACLE-UNAVAILABLE: body')
                    elif mutation=='wrong-suffix': case['unresolved_checks']=['step other: not evaluated']
                    with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                        validate_control_results(bad,selected,{name},1)

    def test_failure_witness_must_still_match_the_selected_definition(self):
        for mutation in ['unknown-case','missing-step','changed-status','changed-pointer','duplicate-step']:
            with self.subTest(mutation=mutation):
                selected,report=self.fixture(True)
                steps=selected[VERSION]['steps']
                name=VERSION
                if mutation=='unknown-case':
                    name='unregistered-control'
                    selected[name]=selected.pop(VERSION)
                    report['suites'][0]['cases'][1]['id']=name
                elif mutation=='missing-step': steps.pop()
                elif mutation=='changed-status': steps[1]['expect']['status']=201
                elif mutation=='changed-pointer': steps[1]['expect']['json']['pointers']['/version']['equals']='2.0.0'
                elif mutation=='duplicate-step': steps.append(copy.deepcopy(steps[1]))
                with self.assertRaises(ValueError):
                    validate_control_results(report,selected,{name},1)


if __name__=='__main__': unittest.main()
