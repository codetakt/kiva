"""Parser changes must invalidate otherwise identical interpretation evidence."""
import json
from pathlib import Path
import tempfile
import unittest

from audit_lrs_source_projection import execution_contract_files
from report_lrs_interpretations import classify_report, file_digest


ROOT = Path(__file__).resolve().parents[2]
PARSER = 'tools/xapi-lrs-conformance/src/lossless_json.rs'
SCHEMA = 'spec/conformance/schemas/conformance-run-report.schema.json'
MANIFEST = 'spec/conformance/lrs-assertion-manifest.yaml'
CASE_FILE = 'tests/conformance/http/control.yaml'


class InterpretationEvidenceTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        for name in set(execution_contract_files()) | {PARSER, SCHEMA}:
            destination = self.root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes((ROOT / name).read_bytes())
        self.contract = execution_contract_files(self.root)
        self.write_manifest(self.contract)
        source = {'id': 'probe', 'requirements': ['statement-immutable'],
                  'safety': {'destructive': False, 'requires_isolated_lrs': False}}
        definition = self.root / CASE_FILE
        definition.parent.mkdir(parents=True, exist_ok=True)
        definition.write_text(json.dumps({'id': 'control', 'cases': [source]}))
        self.cases = {'probe': (CASE_FILE, source)}
        self.report_path = self.root / 'run.json'
        report = {'schema_version': 2, 'standard': 'IEEE/ISO/IEC 39274-1-1-2025',
                  'mode': 'execute-http', 'status': 'passed',
                  'totals': {'total': 1, 'passed': 1, 'failed': 0, 'validated': 0, 'not_run': 0},
                  'suites': [{'id': 'control', 'path': CASE_FILE,
                              'cases': [dict(source, outcome='passed', failures=[])]}]}
        self.report_path.write_text(json.dumps(report))
        self.receipt_path = self.root / 'receipt.json'
        self.receipt = {'input_sha256': dict(self.contract),
                        'artifacts_sha256': {'run.json': file_digest(self.report_path)}}
        self.receipt['input_sha256'][CASE_FILE] = file_digest(definition)

    def write_manifest(self, contract):
        (self.root / MANIFEST).write_text(json.dumps({'execution_contract_files': contract}))

    def classify(self):
        self.receipt_path.write_text(json.dumps(self.receipt))
        return classify_report(self.root, self.report_path, self.receipt_path,
                               self.cases, {'dependencies': {}})

    def test_matching_parser_evidence_is_qualified_without_a_conformance_verdict(self):
        self.assertIn(PARSER, self.contract)
        self.assertEqual(self.contract[PARSER], file_digest(self.root / PARSER))
        result = self.classify()
        self.assertEqual(result['evidence_kind'], 'qualified-existing-execution')
        self.assertEqual(result['conformance_verdict'], 'not-issued')

    def test_parser_only_change_rejects_old_run_after_manifest_regeneration(self):
        self.assertEqual(self.classify()['evidence_kind'], 'qualified-existing-execution')
        parser = self.root / PARSER
        parser.write_bytes(parser.read_bytes() + b'\n// Parser-only mutation control.\n')
        changed = execution_contract_files(self.root)
        self.assertEqual({p for p in self.contract.keys() | changed.keys()
                          if self.contract.get(p) != changed.get(p)}, {PARSER})
        self.write_manifest(changed)
        with self.assertRaisesRegex(ValueError, 'execution contract not bound.*lossless_json.rs'):
            self.classify()

    def test_missing_parser_receipt_cannot_qualify_existing_execution(self):
        self.receipt['input_sha256'].pop(PARSER, None)
        with self.assertRaisesRegex(ValueError, 'execution contract not bound.*lossless_json.rs'):
            self.classify()


if __name__ == '__main__':
    unittest.main()
