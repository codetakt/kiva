"""AFC execution completeness distinguishes executed failures from missing evidence."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from report_issued_test_results import file_sha256

ROOT = Path(__file__).resolve().parents[2]


class ExecutionBoundaryTests(unittest.TestCase):
    def test_real_reporter_gate(self):
        definition = 'scripts/ci/test_issued_execution_boundaries.py'
        comparator = 'scripts/ci/report_issued_test_results.py'
        projection = dict(
            cases={'probe': {'sha256': 'fixed', 'file': definition}}, procedures={},
            execution_contract={comparator: file_sha256(ROOT / comparator)},
            rules=[dict(id='IP-probe', strength='shall', semantic_coverage='not-established-by-static-checks',
                        cases=['probe'], procedures=[], issues=[], dependencies=[])],
            summary={'test_definition_projection_complete': True, 'semantic_coverage_complete': False})
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'projection.json').write_text(json.dumps(projection))
            (root / 'receipt.json').write_text(json.dumps(projection['cases']))
            scenarios = [
                ('execute-http', 'passed', 'passed', True),
                ('execute-http', 'failed', 'failed', True),
                ('execute-http', 'not-run', 'incomplete', False),
                ('execute-http', 'validated', 'validated', False),
                ('validate-only', 'validated', 'validated', False),
                (None, 'passed', 'passed', False),
                ('validate-only', 'passed', 'passed', False),
                ('execute-http', 'passed', 'incomplete', False),
            ]
            for mode, outcome, status, complete in scenarios:
                http = dict(mode=mode, status=status, suites=[{'cases': [{'id': 'probe', 'outcome': outcome}]}])
                (root / 'http.json').write_text(json.dumps(http))
                evidence = dict(
                    schema_version=1, kind='issued-test-execution-receipt',
                    projection_sha256=file_sha256(root / 'projection.json'),
                    input_sha256={p: file_sha256(ROOT / p) for p in (definition, comparator)},
                    artifacts_sha256={p: file_sha256(root / p) for p in ('http.json', 'receipt.json')})
                (root / 'evidence.json').write_text(json.dumps(evidence))
                for receipt_present in (False, True):
                    with self.subTest(mode=mode, outcome=outcome, status=status, receipt=receipt_present):
                        command = [sys.executable, '-B', str(ROOT / comparator),
                                   '--projection', str(root / 'projection.json'), '--http', str(root / 'http.json'),
                                   '--case-receipt', str(root / 'receipt.json'), '--output', str(root / 'report.json'),
                                   '--require-complete']
                        if receipt_present:
                            command += ['--evidence-receipt', str(root / 'evidence.json')]
                        result = subprocess.run(command, capture_output=True, text=True)
                        self.assertEqual(result.returncode, 0 if complete and receipt_present else 2, result.stderr)
                        report = json.loads((root / 'report.json').read_text())
                        self.assertFalse(report['rules'][0]['full_requirement_pass'])
                        self.assertEqual(report['summary']['http_outcomes'], {outcome: 1})
                        self.assertEqual(report['summary']['execution_evidence_verified'], receipt_present)


if __name__ == '__main__':
    unittest.main()
