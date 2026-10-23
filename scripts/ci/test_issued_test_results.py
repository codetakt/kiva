#!/usr/bin/env python3
"""Controls for the separation between test completeness and target failure."""
import copy
from contextlib import redirect_stdout
import io
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

from report_issued_test_results import (
    assess, execution_complete, file_sha256, procedure_incomplete,
    verify_execution_receipt,
)


class ResultControls(unittest.TestCase):
    def setUp(self):
        self.projection = dict(
            cases={'case-a': {'sha256': 'a'}, 'case-b': {'sha256': 'b'}},
            procedures={'fixture-required': {}},
            summary={'test_definition_projection_complete': True, 'semantic_coverage_complete': False},
            rules=[dict(id='rule-a', strength='shall', cases=['case-a', 'case-b'],
                        procedures=['fixture-required'], semantic_coverage='not-established-by-static-checks',
                        issues=['OPEN-EXAMPLE'], dependencies=['dependency-a'])],
        )
        self.receipt = copy.deepcopy(self.projection['cases'])

    def report(self, cases):
        return assess(self.projection, {'suites': [{'cases': cases}]}, self.receipt, [])

    def test_failure_is_an_observation_not_a_missing_case(self):
        report = self.report([{'id': 'case-a', 'outcome': 'failed'}, {'id': 'case-b', 'outcome': 'passed'}])
        self.assertEqual(report['summary']['missing_http_cases'], 0)
        self.assertEqual(report['rules'][0]['http'], {'failed': 1, 'passed': 1})
        self.assertEqual(report['rules'][0]['external_dependencies'], ['dependency-a'])
        self.assertFalse(report['rules'][0]['full_requirement_pass'])
        self.assertEqual(report['case_interpretation_contexts'], {
            'case-a':['OPEN-EXAMPLE'], 'case-b':['OPEN-EXAMPLE'],
        })
        self.projection['rules'].append(dict(self.projection['rules'][0],
            id='unrelated',cases=['case-b'],issues=['OPEN-OTHER']))
        self.assertEqual(self.report([{'id':'case-a','outcome':'failed'}])['case_interpretation_contexts'],
            {'case-a':['OPEN-EXAMPLE']})

    def test_all_observed_passes_do_not_discharge_missing_evidence(self):
        report = self.report([{'id': 'case-a', 'outcome': 'passed'}])
        self.assertEqual(report['missing_http_cases'], ['case-b'])
        self.assertEqual(report['rules'][0]['missing_procedures'], ['fixture-required'])
        self.assertFalse(report['summary']['semantic_coverage_complete'])

    def test_changed_case_cannot_reuse_previous_result(self):
        self.receipt['case-a']['sha256'] = 'old'
        report = self.report([{'id': 'case-a', 'outcome': 'passed'}])
        self.assertEqual(report['stale_cases'], ['case-a'])
        self.assertEqual(report['rules'][0]['http']['stale'], 1)

    def test_unknown_and_duplicate_observations_are_rejected(self):
        with self.assertRaises(ValueError):
            self.report([{'id': 'unknown', 'outcome': 'passed'}])
        with self.assertRaises(ValueError):
            self.report([{'id': 'case-a', 'outcome': 'passed'}] * 2)


class ProcedureCompletionControls(unittest.TestCase):
    def test_unknown_or_unfinished_outcomes_do_not_discharge_required_checks(self):
        for outcome in ('not-run', 'unresolved', 'not-applicable', 'incomplete',
                        'observed', 'future-status', None):
            with self.subTest(outcome=outcome):
                result = {'outcome': 'passed', 'checks': [
                    {'outcome': outcome, 'reason': 'A free-text claim is not applicability evidence'}]}
                self.assertTrue(procedure_incomplete(result, {'level': 'required'}))
                result = {'outcome': outcome, 'checks': [{'outcome': 'passed'}]}
                self.assertTrue(procedure_incomplete(result, {'level': 'required'}))

    def test_failures_are_executed_but_partial_failures_are_incomplete(self):
        result = {'outcome': 'failed', 'checks': [{'outcome': 'failed'}, {'outcome': 'passed'}]}
        self.assertFalse(procedure_incomplete(result, {'level': 'required'}))
        result['checks'].append({'outcome': 'unresolved'})
        self.assertTrue(procedure_incomplete(result, {'level': 'required'}))
        for checks in ([], None, [{}], [None]):
            self.assertTrue(procedure_incomplete({'outcome': 'passed', 'checks': checks}, {'level': 'required'}))

    def test_observed_requires_an_exact_item_binding_even_in_an_advisory_procedure(self):
        spec = {'level': 'required', 'check_levels': {'recommendation': 'advisory'}}
        result = {'outcome': 'passed', 'checks': [
            {'id': 'required-header', 'outcome': 'passed'},
            {'id': 'recommendation', 'outcome': 'observed'}]}
        self.assertFalse(procedure_incomplete(result, spec))
        for unknown in ('recommendation-extra', 'other', None, ''):
            with self.subTest(id=unknown):
                changed = copy.deepcopy(result)
                changed['checks'][1].update(id=unknown, level='advisory')
                self.assertTrue(procedure_incomplete(changed, spec))
        for outcome in ('observed', 'not-run', 'unresolved', 'not-applicable', 'incomplete'):
            with self.subTest(outcome=outcome):
                changed = copy.deepcopy(result)
                changed['level'] = 'advisory'
                changed['checks'][0].update(outcome=outcome, level='advisory')
                self.assertTrue(procedure_incomplete(changed, spec))
                # A procedure-level advisory label does not exempt required headers.
                changed['outcome'] = 'observed'
                self.assertTrue(procedure_incomplete(changed, dict(spec, level='advisory')))
        result['outcome'] = 'observed'
        self.assertTrue(procedure_incomplete(result, spec))
        self.assertFalse(procedure_incomplete(result, dict(spec, level='advisory')))
        result['checks'][1]['outcome'] = 'unresolved'
        self.assertTrue(procedure_incomplete(result, dict(spec, level='advisory')))

    def test_executor_and_collector_bind_head_observations_to_the_real_definition(self):
        from run_lrs_procedures import Executor
        root = Path(__file__).resolve().parents[2]
        name = 'http-rfc-interoperability'
        for required, expected_exit, incomplete in ((True, 0, False), (False, 1, False), (None, 2, True)):
            with self.subTest(required=required), tempfile.TemporaryDirectory() as directory:
                output = Path(directory)
                fixture = output / 'fixture.json'
                fixture.write_text('{"axis":"unit-control"}')
                executor = Executor.__new__(Executor)
                executor.args = SimpleNamespace(procedure=[name], endpoint='unused-unit-control',
                                                fixture=fixture, runner=Path(__file__))
                executor.fixture = {'axis': 'unit-control'}
                executor.out = output
                executor.results = {}

                def emit_control_observations():
                    # No network: only the production rubric/result path runs.
                    for family in ('state', 'agent-profile', 'activity-profile'):
                        executor.check(family + '-head-metadata-recommendation', False)
                    executor.check('state-head-version', required)

                executor.http_rfc = emit_control_observations
                with redirect_stdout(io.StringIO()):
                    self.assertEqual(executor.execute(), expected_exit)
                result = executor.results[name]
                self.assertEqual([check['outcome'] for check in result['checks'][:3]], ['observed'] * 3)
                self.assertTrue(all(check['level'] == 'advisory' for check in result['checks'][:3]))
                self.assertEqual(result['checks'][3]['level'], 'required')
                projection = {'cases': {}, 'procedures': {name: {'file': 'spec/conformance/lrs-review-tests.yaml'}},
                              'rules': [], 'summary': {'test_definition_projection_complete': True, 'semantic_coverage_complete': False}}
                with patch('report_issued_test_results.ROOT', root):
                    report = assess(projection, {'suites': []}, {}, [{'procedures': executor.results}])
                self.assertEqual(report['summary']['incomplete_procedures'], int(incomplete))

    def test_all_existing_advisory_items_are_bound_individually(self):
        from lrs_procedure_definitions import load_procedures
        root = Path(__file__).resolve().parents[2]
        count = 0
        for spec in load_procedures(root / 'spec/conformance/lrs-review-tests.yaml'):
            for identifier, level in spec.get('check_levels', {}).items():
                self.assertEqual(level, 'advisory')
                count += 1
                result = {'outcome': 'observed' if spec['level'] == 'advisory' else 'passed',
                          'checks': [{'id': identifier, 'outcome': 'observed'},
                                     {'id': 'response-1-version-header', 'outcome': 'passed'}]}
                self.assertFalse(procedure_incomplete(result, spec))
                result['checks'][1]['outcome'] = 'observed'
                self.assertTrue(procedure_incomplete(result, spec))
        self.assertEqual(count, 6)

    def test_advisory_keys_have_production_emitters_in_the_dispatched_method(self):
        import ast
        from lrs_procedure_definitions import load_procedures
        root = Path(__file__).resolve().parents[2]
        source = ast.parse((root / 'scripts/ci/run_lrs_procedures.py').read_text())
        executor = next(node for node in source.body
                        if isinstance(node, ast.ClassDef) and node.name == 'Executor')
        methods = {node.name: node for node in executor.body if isinstance(node, ast.FunctionDef)}
        dispatch = next(node.value.elts for node in ast.walk(methods['execute'])
                        if isinstance(node, ast.Assign) and any(
                            isinstance(target, ast.Name) and target.id == 'functions'
                            for target in node.targets))
        docs = next(node.value.elts for node in ast.walk(methods['__init__'])
                    if isinstance(node, ast.Assign) and any(
                        isinstance(target, ast.Attribute) and target.attr == 'docs'
                        for target in node.targets))
        families = [ast.literal_eval(item.elts[0]) for item in docs]
        for spec, function in zip(load_procedures(root / 'spec/conformance/lrs-review-tests.yaml'),
                                  dispatch, strict=True):
            declared = set(spec.get('check_levels', {}))
            if not declared:
                continue
            with self.subTest(procedure=spec['id']):
                self.assertIsInstance(function, ast.Attribute)
                emitted = set()
                for call in ast.walk(methods[function.attr]):
                    if not (isinstance(call, ast.Call) and isinstance(call.func, ast.Attribute)
                            and isinstance(call.func.value, ast.Name) and call.func.value.id == 'self'
                            and call.func.attr == 'check' and call.args):
                        continue
                    identifier = call.args[0]
                    if isinstance(identifier, ast.Constant) and isinstance(identifier.value, str):
                        emitted.add(identifier.value)
                    elif (isinstance(identifier, ast.BinOp) and isinstance(identifier.op, ast.Add)
                          and isinstance(identifier.left, ast.Name) and identifier.left.id == 'family'
                          and isinstance(identifier.right, ast.Constant)
                          and isinstance(identifier.right.value, str)):
                        emitted.update(family + identifier.right.value for family in families)
                self.assertFalse(declared - emitted, 'advisory key has no production emitter')

    def test_unreferenced_procedure_still_blocks_global_completion(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            spec = {'id': 'procedure-a', 'level': 'required'}
            (root / 'procedure.json').write_text(json.dumps({'tests': [spec]}))
            projection = {'cases': {}, 'procedures': {'procedure-a': {'file': 'procedure.json'}},
                          'rules': [], 'summary': {'test_definition_projection_complete': True, 'semantic_coverage_complete': False}}
            result = {'outcome': 'incomplete', 'checks': [{'outcome': 'passed'}],
                      'rubric_sha256': hashlib.sha256(json.dumps(spec, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()}
            with patch('report_issued_test_results.ROOT', root):
                report = assess(projection, {'mode': 'execute-http', 'status': 'passed', 'suites': []},
                                {}, [{'procedures': {'procedure-a': result}}])
            report['summary']['execution_evidence_verified'] = True
            self.assertEqual(report['incomplete_procedures'], ['procedure-a'])
            self.assertFalse(execution_complete(report))

    def test_receipt_and_report_completion_are_required_even_when_every_case_ran(self):
        projection = {'cases': {'case-a': {'sha256': 'a'}}, 'procedures': {}, 'rules': [],
                      'summary': {'test_definition_projection_complete': True, 'semantic_coverage_complete': False}}
        http = {'mode': 'execute-http', 'status': 'failed',
                'suites': [{'cases': [{'id': 'case-a', 'outcome': 'failed'}]}]}
        report = assess(projection, http, projection['cases'], [])
        self.assertFalse(execution_complete(report))
        report['summary']['execution_evidence_verified'] = True
        self.assertTrue(execution_complete(report))
        self.assertFalse(report['summary']['semantic_coverage_complete'])
        report['summary']['test_definition_projection_complete'] = False
        report['summary']['semantic_coverage_complete'] = True
        self.assertFalse(execution_complete(report))
        report['summary']['test_definition_projection_complete'] = True
        report['summary']['http_report_complete'] = False
        self.assertFalse(execution_complete(report))


class SharedProcedureControls(unittest.TestCase):
    def test_shared_text_expands_to_the_original_effective_rubric(self):
        from lrs_procedure_definitions import load_procedures
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'procedures.json'
            procedure = {'id': 'procedure-a', 'expected': 'Before. {{common:response}} After.'}
            path.write_text(json.dumps({'common_expectations': {'response': 'Exact response contract.'},
                                        'tests': [procedure]}))
            self.assertEqual(load_procedures(path), [dict(procedure, expected='Before. Exact response contract. After.')])

    def test_invalid_shared_references_fail_closed(self):
        from lrs_procedure_definitions import load_procedures
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'procedures.json'
            for common, expected in [({}, '{{common:missing}}'), ({'x': ''}, '{{common:x}}'),
                                     ({'x': '{{common:y}}'}, '{{common:x}}'), ({'x': 'text'}, '{{common:x}')]:
                with self.subTest(common=common, expected=expected):
                    path.write_text(json.dumps({'common_expectations': common,
                                                'tests': [{'id': 'a', 'expected': expected}]}))
                    with self.assertRaises(ValueError):
                        load_procedures(path)


    def test_malformed_item_levels_are_rejected_when_loading_definitions(self):
        from lrs_procedure_definitions import load_procedures
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'procedures.json'
            for levels in ([], {'x': 'optional'}, {'': 'advisory'}, {1: 'advisory'}, {'x': None}):
                with self.subTest(levels=levels):
                    # YAML retains non-string mapping keys for the key-type control.
                    import yaml
                    path.write_text(yaml.safe_dump({'tests': [{'id': 'a', 'check_levels': levels}]}))
                    with self.assertRaises(ValueError):
                        load_procedures(path)


class ExecutionReceiptControls(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name, data in {'comparator.py': 'current comparator', 'cases.yaml': 'case definitions',
                           'procedures.yaml': 'procedure definitions', 'http.json': 'HTTP result',
                           'case-receipt.json': 'case map', 'procedure.json': 'procedure result'}.items():
            (self.root / name).write_text(data)
        self.projection = {'execution_contract': {'comparator.py': file_sha256(self.root / 'comparator.py')},
                           'cases': {'case-a': {'file': 'cases.yaml'}},
                           'procedures': {'procedure-a': {'file': 'procedures.yaml'}}}
        self.projection_path = self.root / 'projection.json'
        self.projection_path.write_text(json.dumps(self.projection))
        self.artifacts = [self.root / name for name in ('http.json', 'case-receipt.json', 'procedure.json')]
        self.receipt = {'schema_version': 1, 'kind': 'issued-test-execution-receipt',
                        'projection_sha256': file_sha256(self.projection_path),
                        'input_sha256': {name: file_sha256(self.root / name) for name in
                                         ('comparator.py', 'cases.yaml', 'procedures.yaml')},
                        'artifacts_sha256': {path.name: file_sha256(path) for path in self.artifacts}}

    def verify(self):
        verify_execution_receipt(self.projection, self.projection_path, self.artifacts,
                                 self.receipt, root=self.root)

    def test_exact_receipt_binds_all_inputs_and_outputs(self):
        self.verify()

    def test_inherited_workspace_features_invalidate_contract_and_old_receipt(self):
        from generate_issued_test_projection import collect_execution_contract
        from issued_test_design import digest
        repository = Path(__file__).resolve().parents[2]
        contract = collect_execution_contract(repository)
        self.assertIn('Cargo.toml', contract)
        for name in contract:
            destination = self.root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes((repository / name).read_bytes())
        self.projection['execution_contract'] = contract
        self.projection_path.write_text(json.dumps(self.projection))
        self.receipt['projection_sha256'] = file_sha256(self.projection_path)
        self.receipt['input_sha256'].update(contract)
        self.verify()

        # This manifest-only feature change leaves Cargo.lock and all cases
        # unchanged but can alter the runner's JSON number representation.
        manifest = self.root / 'Cargo.toml'
        source = manifest.read_text()
        original = 'serde_json = "1.0"'
        self.assertIn(original, source)
        manifest.write_text(source.replace(original,
            'serde_json = { version = "1.0", features = ["arbitrary_precision"] }', 1))
        changed = collect_execution_contract(self.root)
        self.assertEqual(contract['Cargo.lock'], changed['Cargo.lock'])
        self.assertNotEqual(digest(contract), digest(changed))
        with self.assertRaisesRegex(ValueError, 'current execution contract drift: Cargo.toml'):
            self.verify()

        # Renewing the definition/projection hashes cannot renew an old run's
        # input receipt. The collector must still reject that old evidence.
        self.projection['execution_contract'] = changed
        self.projection_path.write_text(json.dumps(self.projection))
        self.receipt['projection_sha256'] = file_sha256(self.projection_path)
        with self.assertRaisesRegex(ValueError, 'execution input not bound to current bytes: Cargo.toml'):
            self.verify()

    def test_changed_comparator_rejects_unchanged_case_results(self):
        (self.root / 'comparator.py').write_text('corrected comparator')
        with self.assertRaisesRegex(ValueError, 'current execution contract drift'):
            self.verify()
        self.projection['execution_contract']['comparator.py'] = file_sha256(self.root / 'comparator.py')
        self.projection_path.write_text(json.dumps(self.projection))
        self.receipt['projection_sha256'] = file_sha256(self.projection_path)
        with self.assertRaisesRegex(ValueError, 'execution input not bound'):
            self.verify()

    def test_changed_definition_or_missing_input_is_not_current_evidence(self):
        for name in ('comparator.py', 'cases.yaml', 'procedures.yaml'):
            with self.subTest(name=name):
                digest = self.receipt['input_sha256'].pop(name)
                with self.assertRaisesRegex(ValueError, 'execution input not bound'):
                    self.verify()
                self.receipt['input_sha256'][name] = digest

    def test_every_report_and_case_receipt_must_be_bound(self):
        for path in self.artifacts:
            with self.subTest(path=path.name):
                digest = self.receipt['artifacts_sha256'].pop(path.name)
                with self.assertRaisesRegex(ValueError, 'execution artifact not bound'):
                    self.verify()
                self.receipt['artifacts_sha256'][path.name] = digest
        self.artifacts[-1].write_text('substituted result')
        with self.assertRaisesRegex(ValueError, 'execution artifact not bound'):
            self.verify()

    def test_projection_identity_and_receipt_version_are_required(self):
        self.receipt['projection_sha256'] = 'old projection'
        with self.assertRaisesRegex(ValueError, 'current projection'):
            self.verify()
        self.receipt['schema_version'] = 0
        with self.assertRaisesRegex(ValueError, 'unsupported execution receipt'):
            self.verify()


class EndpointProfileVocabularyControls(unittest.TestCase):
    def test_runner_reporter_and_report_schema_recognize_the_same_prerequisites(self):
        import ast
        import re
        root = Path(__file__).resolve().parents[2]
        rust = (root / 'tools/xapi-lrs-conformance/src/lib.rs').read_text()
        python = (root / 'scripts/ci/report_conformance_case_matrix.py').read_text()
        schema = json.loads((root / 'spec/conformance/schemas/conformance-case-matrix-report.schema.json').read_text())

        def compare(rust_source, python_source, report_schema):
            # Bounded source-shape guard, not a Rust parser: unfamiliar syntax
            # fails this control instead of silently dropping a profile name.
            constants = dict(re.findall(r'const (ENDPOINT_PROFILE_[A-Z_]+): &str = "([^"]+)";', rust_source))
            validator = rust_source.split('fn validate_endpoint_profile_name(', 1)[1].split('\nfn ', 1)[0]
            match = re.search(r'if !matches!\(\s*name,\s*(.*?)\s*\) \{', validator, re.S)
            self.assertIsNotNone(match, 'unrecognized Rust prerequisite validator shape')
            rust_names = set()
            for token in match[1].split('|'):
                token = token.strip()
                if re.fullmatch(r'"[a-z0-9-]+"', token):
                    rust_names.add(json.loads(token))
                else:
                    self.assertRegex(token, r'^ENDPOINT_PROFILE_[A-Z_]+$')
                    rust_names.add(constants[token])
            assignments = [node.value for node in ast.parse(python_source).body
                           if isinstance(node, ast.Assign) and any(
                               isinstance(target, ast.Name) and target.id == 'VALID_ENDPOINT_PROFILES'
                               for target in node.targets)]
            self.assertEqual(len(assignments), 1)
            reporter_names = ast.literal_eval(assignments[0])
            schema_names = report_schema['$defs']['case']['properties']['requires_endpoint_profiles']['items']['enum']
            self.assertTrue(rust_names)
            self.assertEqual(rust_names, reporter_names)
            self.assertEqual(rust_names, set(schema_names))
            self.assertEqual(len(schema_names), len(set(schema_names)))
            return rust_names

        names = compare(rust, python, schema)
        profile_schema = json.loads((root / 'spec/conformance/schemas/endpoint-profile.schema.json').read_text())
        declarable = set(profile_schema['properties']['profiles']['properties'])
        self.assertLessEqual(declarable, names)
        self.assertEqual(names - declarable, {'signature-validation-context'})
        # A typo in any one of the three authoritative consumers is detected.
        marker = 'signature-validation-context'
        changed_schema = copy.deepcopy(schema)
        changed_schema['$defs']['case']['properties']['requires_endpoint_profiles']['items']['enum'].remove(marker)
        for sources in ((rust.replace(marker, marker + '-typo'), python, schema),
                        (rust, python.replace(marker, marker + '-typo'), schema),
                        (rust, python, changed_schema)):
            with self.assertRaises(AssertionError):
                compare(*sources)


if __name__ == '__main__':
    unittest.main()
