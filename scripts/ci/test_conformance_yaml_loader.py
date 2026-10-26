"""The accelerated report loader retains safe construction and mapping checks."""

from pathlib import Path
import tempfile
import unittest

import yaml

from validate_conformance import ConformanceError, load_yaml


class ConformanceYamlLoaderControls(unittest.TestCase):
    def load(self, text):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixture.yaml"
            path.write_text(text, encoding="utf-8")
            return load_yaml(path)

    def test_scalar_types_aliases_and_merge_keys_match_safe_loader(self):
        text = '''
defaults: &defaults
  enabled: true
  absent: null
  count: 42
  precision: 0.1
  identifier: "00000000-0000-4000-8000-000000000001"
  timestamp: "2026-01-01T00:00:00Z"
case:
  <<: *defaults
  count: 0
  unicode: "試験"
  sequence: [false, "false", 1, "1"]
'''
        self.assertEqual(self.load(text), yaml.safe_load(text))

    def test_python_object_construction_is_rejected(self):
        for text in [
            'value: !!python/object:builtins.object {}',
            'value: !!python/name:builtins.print',
            'value: !!python/object/apply:builtins.str [unsafe]',
        ]:
            with self.subTest(text=text), self.assertRaises(yaml.YAMLError):
                self.load(text)

    def test_malformed_yaml_and_multiple_documents_are_rejected(self):
        for text in ['value: [unterminated', 'value: *missing', 'a: 1\n---\nb: 2']:
            with self.subTest(text=text), self.assertRaises(yaml.YAMLError):
                self.load(text)

    def test_non_mapping_roots_are_rejected(self):
        for text in ['', 'null', 'scalar', '[a, b]']:
            with self.subTest(text=text), self.assertRaises(ConformanceError):
                self.load(text)


if __name__ == "__main__":
    unittest.main()
