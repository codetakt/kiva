"""A complete-looking HTTP bundle must not stand in for full issued coverage."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from validate_release_evidence_set import ReleaseEvidenceSetError, validate_conformance_release_scope


class IssuedReleaseControls(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.manifest = {'live_http': {'memory': True, 'postgres': True},
                         'verification': {'postgres': True}}

    def validate(self, static=False):
        validate_conformance_release_scope(self.manifest, self.root,
            allow_static_conformance=static, require_adl_run=False)

    def test_http_only_is_not_full_release_evidence(self):
        with self.assertRaisesRegex(ReleaseEvidenceSetError, 'lacks full issued-test execution'):
            self.validate()
        self.validate(static=True)

    def test_missing_raw_evidence_cannot_be_replaced_by_a_passed_summary(self):
        directory = self.root / 'issued/memory'
        directory.mkdir(parents=True)
        (directory / 'execution.json').write_text(json.dumps({'implementation': {'backend': 'memory'}}))
        (directory / 'issued-results.json').write_text(json.dumps({'summary': {'test_specification_conformant': True}}))
        with self.assertRaises((FileNotFoundError, ValueError)):
            self.validate()

    def test_both_backend_assessments_must_pass_and_match_their_backend(self):
        for backend in ('memory', 'postgres'):
            directory = self.root / 'issued' / backend
            directory.mkdir(parents=True)
            (directory / 'execution.json').write_text(json.dumps({'implementation': {'backend': backend}}))
        with patch('report_issued_test_results.assess_capture', side_effect=[
                {'summary': {'test_specification_conformant': True}},
                {'summary': {'test_specification_conformant': False, 'missing_procedures': 120}}]):
            with self.assertRaisesRegex(ReleaseEvidenceSetError, 'not fully passed: postgres'):
                self.validate()
        (self.root / 'issued/memory/execution.json').write_text(json.dumps({'implementation': {'backend': 'postgres'}}))
        with self.assertRaisesRegex(ReleaseEvidenceSetError, 'backend mismatch'):
            self.validate()


if __name__ == '__main__':
    unittest.main()
