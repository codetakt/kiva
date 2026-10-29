"""Controls against renewing old evidence or losing failed-run diagnostics."""
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from capture_issued_test_execution import capture
from report_issued_test_results import file_sha256, verify_execution_receipt


class CaptureControls(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        for name in ('Cargo.toml', 'case.yaml', 'oracle.py', 'fixture.json', 'server', 'runner'):
            (self.root / name).write_text(name)
        self.projection = {'execution_contract': {'oracle.py': file_sha256(self.root / 'oracle.py')},
                           'definition_files': {'case.yaml': file_sha256(self.root / 'case.yaml')},
                           'cases': {'a': {'file': 'case.yaml'}}, 'procedures': {}}
        self.path = self.root / 'spec/conformance/issued-test-projection.json'
        self.path.parent.mkdir(parents=True)
        self.path.write_text(json.dumps(self.projection))
        self.output = self.root / 'out'
        # Collector semantics have separate negative controls. These producer
        # tests isolate artifact capture, source drift and wrapper ordering.
        def collect(directory, backend, root):
            if not (directory/'native/receipt.json').is_file():
                raise FileNotFoundError('native capture missing')
            return {'procedures':{'uncovered':{'outcome':'incomplete',
                'execution_complete':False,'coverage_complete':False}}}
        collector=patch('capture_issued_test_execution.collect_capture', side_effect=collect)
        collector.start()
        self.addCleanup(collector.stop)

    def execute(self, mutation=None, missing=False):
        def wrapper(command, **kwargs):
            self.assertEqual(command, ['bash', 'scripts/ci/run_memory_conformance.sh'])
            self.assertEqual(kwargs['cwd'], self.root)
            env = kwargs['env']
            if not missing:
                Path(env['KIVA_CONFORMANCE_JSON_REPORT']).write_text('{"status":"failed"}')
                Path(env['KIVA_CONFORMANCE_JUNIT_REPORT']).write_text('<testsuites/>')
                procedures = Path(env['KIVA_CONFORMANCE_PROCEDURE_REPORT_ROOT'])
                procedures.mkdir()
                (procedures / 'summary.json').write_text('{"procedures":{}}')
                native=Path(env['KIVA_CONFORMANCE_NATIVE_REPORT_ROOT'])
                native.mkdir()
                (native/'receipt.json').write_text('{}')
            if mutation:
                mutation()
            return SimpleNamespace(returncode=2)
        with patch.dict(os.environ, {}, clear=True), patch('capture_issued_test_execution.subprocess.run', side_effect=wrapper):
            return capture('memory', self.output, self.root / 'fixture.json',
                           self.root / 'server', self.root / 'runner', self.root)

    def test_failed_target_results_are_captured_and_bound_without_becoming_passes(self):
        receipt = self.execute()
        self.assertEqual(receipt['wrapper_exit'], 2)
        self.assertEqual(receipt['errors'], [])
        self.assertEqual(receipt['artifacts_sha256']['http.json'], file_sha256(self.output / 'http.json'))
        verify_execution_receipt(self.projection, self.path, [self.output / 'http.json'], receipt, self.root)
        with self.assertRaises(FileExistsError):
            self.execute()

    def test_source_drift_cannot_renew_unchanged_reports(self):
        receipt = self.execute(lambda: (self.root / 'Cargo.toml').write_text('changed build'))
        self.assertIn('implementation input drift during execution', receipt['errors'])
        with self.assertRaisesRegex(ValueError, 'incomplete execution capture'):
            verify_execution_receipt(self.projection, self.path, [], receipt, self.root)

    def test_missing_binary_and_partial_outputs_retain_the_failed_capture(self):
        receipt = self.execute(lambda: (self.root / 'server').unlink(), missing=True)
        self.assertTrue(any('unavailable' in e for e in receipt['errors']))
        self.assertIn('missing execution artifact: http.json', receipt['errors'])
        self.assertTrue((self.output / 'execution.log').is_file())
        self.assertTrue((self.output / 'execution.json').is_file())

    def test_parallel_http_and_procedure_setup_keep_separate_evidence(self):
        calls = []
        def run(command, **kwargs):
            env = kwargs['env']
            calls.append(command)
            if len(calls) == 1:
                self.assertIn('scripts/ci/run_parallel_conformance.py', command)
                self.assertNotIn('KIVA_CONFORMANCE_PROCEDURE_REPORT_ROOT', env)
                Path(env['KIVA_CONFORMANCE_JSON_REPORT']).write_text('{"label":"full HTTP"}')
                Path(env['KIVA_CONFORMANCE_JUNIT_REPORT']).write_text('full JUnit')
                return SimpleNamespace(returncode=1)
            self.assertEqual(Path(env['KIVA_CONFORMANCE_CASE_IDS_FILE']).read_text(), 'a\n')
            Path(env['KIVA_CONFORMANCE_JSON_REPORT']).write_text('setup HTTP')
            Path(env['KIVA_CONFORMANCE_JUNIT_REPORT']).write_text('setup JUnit')
            procedures = Path(env['KIVA_CONFORMANCE_PROCEDURE_REPORT_ROOT'])
            procedures.mkdir()
            (procedures / 'summary.json').write_text('{}')
            native=Path(env['KIVA_CONFORMANCE_NATIVE_REPORT_ROOT'])
            native.mkdir()
            (native/'receipt.json').write_text('{}')
            return SimpleNamespace(returncode=0)
        with patch.dict(os.environ, {}, clear=True), patch('capture_issued_test_execution.subprocess.run', side_effect=run):
            receipt = capture('memory', self.output, self.root / 'fixture.json',
                self.root / 'server', self.root / 'runner', self.root, workers=4)
        self.assertEqual(receipt['wrapper_exit'], 1)
        self.assertEqual(receipt['workers'], 4)
        self.assertEqual(receipt['errors'], [])
        self.assertEqual(json.loads((self.output / 'http.json').read_text())['label'], 'full HTTP')
        self.assertEqual((self.output / 'procedure-setup-http.json').read_text(), 'setup HTTP')


if __name__ == '__main__':
    unittest.main()
