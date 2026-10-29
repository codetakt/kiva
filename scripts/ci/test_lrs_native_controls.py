import unittest
import io
from run_lrs_native_controls import parse_results
from run_lrs_python_controls import RecordedResult, validate

class NativeExecutionEvidence(unittest.TestCase):
    def test_each_selected_test_requires_an_actual_result(self):
        inventory=['one','two']
        self.assertEqual(parse_results('test one ... ok\ntest two ... FAILED\n',inventory),
                         {'one':'passed','two':'failed'})
        for log in ['test one ... ok\n', 'test one ... ok\ntest one ... ok\n',
                    'test one ... ok\ntest forged ... ok\n', 'test result: ok. 2 passed; 0 failed\n']:
            with self.subTest(log=log), self.assertRaises(ValueError):
                parse_results(log,inventory)

    def test_ignored_never_becomes_success(self):
        self.assertEqual(parse_results('test postgres_one ... ignored, requires a database\n', ['postgres_one']),
                         {'postgres_one':'not-run'})

    def test_python_observation_requires_every_selected_control(self):
        good={'inventory':['a','b'],'results':{'a':'passed','b':'passed'},'count':2}
        self.assertEqual(validate(good),good['results'])
        for changed in [dict(good,count=1),dict(good,results={'a':'passed'}),
                        dict(good,inventory=['a','a']),dict(good,results={'a':'passed','b':'forged'})]:
            with self.assertRaises(ValueError): validate(changed)

    def test_python_failed_subtest_and_skipped_subtest_cannot_pass(self):
        class Model(unittest.TestCase):
            def test_failure(self):
                with self.subTest(value=1): self.assertEqual(1,2)
            def test_skip(self):
                with self.subTest(value=1): self.skipTest('oracle unavailable')
            def test_error(self): raise ValueError('fixture unavailable')
            def test_ok(self): self.assertEqual(1,1)
        suite=unittest.defaultTestLoader.loadTestsFromTestCase(Model)
        result=unittest.TextTestRunner(stream=io.StringIO(),resultclass=RecordedResult).run(suite)
        observed={k.rsplit('.',1)[1]:v for k,v in result.observed.items()}
        self.assertEqual(observed,{'test_failure':'failed','test_skip':'not-run','test_error':'not-run','test_ok':'passed'})

if __name__ == '__main__':
    unittest.main()
