"""Parallel execution must preserve raw failures and require the entire inventory."""

from collections import Counter
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET
import jsonschema

from check_conformance_regressions import (
    STANDARD, capture, capture_artifacts, gate, load_json,
)
from run_parallel_conformance import merge_junit, merge_reports, partitions, totals_and_status, verify_aggregate


class ParallelConformanceControls(unittest.TestCase):
    def setUp(self):
        self.suites = {'suite': 'tests/conformance/http/example.yaml'}
        self.cases = {('suite', name): dict(id=name, requirements=['R'],
            safety=dict(destructive=False, requires_isolated_lrs=False))
            for name in ['good', 'gap', 'unknown', 'also-good']}
        self.groups = partitions(self.cases, 2)
        self.rows = {}
        for key, definition in self.cases.items():
            outcome = {'gap': 'failed', 'unknown': 'not-run'}.get(key[1], 'passed')
            self.rows[key] = dict(copy.deepcopy(definition), outcome=outcome,
                failures=['first violation', 'second violation'] if outcome == 'failed' else [],
                not_run_reasons=['missing provider', 'missing capability'] if outcome == 'not-run' else [],
                unresolved_checks=['unvisited suffix'] if outcome == 'failed' else [],
                diagnostic_statuses=[])
        self.reports = []
        for group in self.groups:
            rows = [self.rows[key] for key in group]
            totals, status = totals_and_status(rows)
            self.reports.append(dict(schema_version=2, standard=STANDARD, mode='execute-http',
                status=status, totals=totals, suites=[dict(id='suite', path=self.suites['suite'], cases=rows)]))
        self.exits = [1, 1]

    def merge(self, reports=None, exits=None, groups=None):
        return merge_reports(self.reports if reports is None else reports,
                             self.exits if exits is None else exits,
                             self.groups if groups is None else groups, self.cases, self.suites)

    def test_partition_is_disjoint_complete_and_preserves_relative_order(self):
        self.assertEqual(self.groups, [[('suite', 'good'), ('suite', 'unknown')],
                                      [('suite', 'gap'), ('suite', 'also-good')]])
        self.assertEqual(Counter(key for group in self.groups for key in group), Counter(self.cases.keys()))
        for workers in [0, 1, 5, 9, True]:
            with self.subTest(workers=workers), self.assertRaises(ValueError):
                partitions(self.cases, workers)
        duplicate = dict(self.cases)
        duplicate[('other', 'good')] = duplicate[('suite', 'good')]
        with self.assertRaisesRegex(ValueError, 'globally unique'):
            partitions(duplicate, 2)

    def test_merge_retains_every_diagnostic_without_relabeling(self):
        before = copy.deepcopy(self.reports)
        merged = self.merge()
        self.assertEqual(merged['suites'][0]['cases'], list(self.rows.values()))
        self.assertEqual(merged['totals'], dict(total=4, passed=2, failed=1, not_run=1, validated=0))
        self.assertEqual(merged['status'], 'failed')
        self.assertEqual(self.reports, before)

    def test_missing_duplicate_foreign_and_wrongly_assigned_cases_fail(self):
        mutations = [lambda r: r[0]['suites'][0]['cases'].pop(),
                     lambda r: r[0]['suites'][0]['cases'].append(r[0]['suites'][0]['cases'][0]),
                     lambda r: r[0]['suites'][0]['cases'][0].update(id='foreign'),
                     lambda r: r.reverse(),
                     lambda r: r[0]['suites'].append(r[0]['suites'][0]),
                     lambda r: r[0]['suites'][0].update(path='wrong.yaml'),
                     lambda r: r[0]['suites'][0]['cases'][0].update(requirements=['other']),
                     lambda r: r.pop()]
        for mutate in mutations:
            reports = copy.deepcopy(self.reports)
            mutate(reports)
            with self.subTest(mutation=mutate), self.assertRaises(ValueError):
                self.merge(reports)
        for groups in [[self.groups[0], self.groups[0]], [self.groups[0], []]]:
            with self.assertRaises(ValueError):
                self.merge(groups=groups)

    def test_invalid_totals_modes_statuses_and_exits_fail(self):
        for field, value in [('totals', {}), ('mode', 'validate-only'), ('status', 'passed'),
                             ('standard', 'other'), ('schema_version', 1)]:
            reports = copy.deepcopy(self.reports)
            reports[0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.merge(reports)
        for code in [0, 2, 127, -9, True, None]:
            with self.subTest(code=code), self.assertRaises(ValueError):
                self.merge(exits=[code, 1])

    def junit(self, report):
        root = ET.Element('testsuites', name=STANDARD, tests=str(report['totals']['total']),
                          failures=str(report['totals']['failed']))
        props = ET.SubElement(root, 'properties')
        for name, value in dict(schema_version='2', standard=STANDARD, mode='execute-http',
                                status=report['status']).items():
            ET.SubElement(props, 'property', name=name, value=value)
        for row in report['suites']:
            suite = ET.SubElement(root, 'testsuite', name=row['id'], tests=str(len(row['cases'])),
                                  failures=str(sum(c['outcome'] == 'failed' for c in row['cases'])))
            ET.SubElement(ET.SubElement(suite, 'properties'), 'property', name='path', value=row['path'])
            for case in row['cases']:
                element = ET.SubElement(suite, 'testcase', classname=row['id'], name=case['id'])
                props = ET.SubElement(element, 'properties')
                ET.SubElement(props, 'property', name='outcome', value=case['outcome'])
                ET.SubElement(props, 'property', name='requirements', value=','.join(case['requirements']))
                ET.SubElement(element, 'property', name='diagnostic_statuses', value='[]')
                for message in case['failures']:
                    ET.SubElement(element, 'failure', message=message).text = message
                for message in case['not_run_reasons']:
                    ET.SubElement(element, 'skipped', message=message)
        return root

    def test_junit_keeps_all_failure_and_skip_messages(self):
        documents = [self.junit(report) for report in self.reports]
        merged = merge_junit(documents, self.reports, self.merge())
        self.assertEqual([row.get('name') for row in merged.findall('./testsuite/testcase')],
                         [key[1] for key in self.cases])
        self.assertEqual([e.get('message') for e in merged.findall('.//failure')],
                         ['first violation', 'second violation'])
        self.assertEqual([e.get('message') for e in merged.findall('.//skipped')],
                         ['missing provider', 'missing capability'])

    def test_junit_drift_missing_and_duplicate_rows_are_rejected(self):
        changes = [lambda d: d[0].set('tests', '0'),
                   lambda d: d[0].find('testsuite').remove(d[0].find('./testsuite/testcase')),
                   lambda d: d[0].find('testsuite').append(copy.deepcopy(d[0].find('./testsuite/testcase'))),
                   lambda d: d[0].find('./testsuite/testcase').set('name', 'foreign'),
                   lambda d: d[1].find('.//failure').set('message', 'different'),
                   lambda d: d[0].find('.//skipped').set('message', 'different'),
                   lambda d: d[0].find('./testsuite/testcase/property').set('value', '[403]')]
        for change in changes:
            documents = [self.junit(report) for report in self.reports]
            change(documents)
            with self.subTest(change=change), self.assertRaises(ValueError):
                merge_junit(documents, self.reports, self.merge())

    def prepare_inputs(self, root):
        for name in ['spec/conformance/ci-execution-baseline.json',
                     'spec/conformance/schemas/conformance-run-report.schema.json',
                     'spec/conformance/lrs-generated-artifacts.lock.json',
                     'scripts/ci/check_conformance_regressions.py', 'scripts/ci/conformance_lib.sh',
                     'scripts/ci/run_memory_conformance.sh', 'scripts/ci/run_parallel_conformance.py']:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('{}')

    def test_gate_reconstructs_the_aggregate_from_raw_shards(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            files = {'case.yaml': 'identity'}
            plan = dict(schema_version=1, strategy='ordered-round-robin-v1', workers=2,
                        case_files=files, assignments=[[list(key) for key in group] for group in self.groups])
            (directory / 'plan.json').write_text(json.dumps(plan))
            documents = []
            for index, (group, report) in enumerate(zip(self.groups, self.reports, strict=True)):
                child = directory / f'{index:02}'
                child.mkdir()
                (child / 'case-ids.txt').write_text(''.join(key[1] + '\n' for key in group))
                (child / 'result.json').write_text(json.dumps(dict(index=index, runner_exit=1)))
                (child / 'http.json').write_text(json.dumps(report))
                document = self.junit(report)
                ET.ElementTree(document).write(child / 'http.xml')
                documents.append(document)
            report = self.merge()
            xml = directory / 'merged.xml'
            ET.ElementTree(merge_junit(documents, self.reports, report)).write(xml)
            def verify(value=report):
                verify_aggregate(directory, 2, value, xml, self.cases, self.suites, files,
                                 jsonschema.Draft202012Validator({}))
            verify()
            changed = copy.deepcopy(report)
            changed['suites'][0]['cases'][1]['failures'] = ['rewritten']
            with self.assertRaisesRegex(ValueError, 'aggregate JSON differs'):
                verify(changed)
            for name, replacement in [('plan.json', '{}'), ('00/case-ids.txt', 'foreign\n'),
                                      ('01/result.json', '{"index":0,"runner_exit":1}')]:
                path = directory / name
                original = path.read_bytes()
                path.write_text(replacement)
                with self.subTest(name=name), self.assertRaises(ValueError):
                    verify()
                path.write_bytes(original)
            damaged = ET.parse(xml)
            damaged.getroot().find('.//failure').set('message', 'rewritten')
            damaged.write(xml)
            with self.assertRaisesRegex(ValueError, 'aggregate JUnit differs'):
                verify()

    def test_capture_binds_every_shard_artifact_before_strict_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.prepare_inputs(root)
            output = root / 'reports'
            def execute(command, **kwargs):
                self.assertIn('run_parallel_conformance.py', command[2])
                self.assertEqual(command[-3], '2')
                for name in capture_artifacts('memory', 2) - {'xapi-memory-execution.log'}:
                    path = output / name
                    if name == 'xapi-memory-http.json':
                        path = Path(kwargs['env']['KIVA_CONFORMANCE_JSON_REPORT'])
                    elif name == 'xapi-memory-http.xml':
                        path = Path(kwargs['env']['KIVA_CONFORMANCE_JUNIT_REPORT'])
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text('{}')
                return type('Exit', (), {'returncode': 1})()
            with patch('check_conformance_regressions.subprocess.run', side_effect=execute), \
                    patch.dict('os.environ', {}, clear=True):
                receipt = capture(root, 'memory', output, 2)
            self.assertEqual(receipt['schema_version'], 2)
            self.assertEqual(set(receipt['artifacts']), capture_artifacts('memory', 2))
            before = {str(p.relative_to(output)): p.read_bytes() for p in output.rglob('*') if p.is_file()}
            with patch('check_conformance_regressions.check', side_effect=ValueError('new regression')):
                with self.assertRaisesRegex(ValueError, 'new regression'):
                    gate(root, 'memory', output)
            self.assertEqual(before, {str(p.relative_to(output)): p.read_bytes()
                                     for p in output.rglob('*') if p.is_file()})
            for name in ['shards/plan.json', 'shards/00/http.json', 'shards/01/http.xml',
                         'shards/01/result.json', 'shards/00/case-ids.txt']:
                path = output / name
                old = path.read_bytes()
                path.write_text('changed')
                with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'captured report drift'):
                    gate(root, 'memory', output)
                path.write_bytes(old)

    def test_parallel_infrastructure_failure_preserves_partial_logs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.prepare_inputs(root)
            output = root / 'reports'
            def execute(command, **kwargs):
                path = output / 'shards/00/execution.log'
                path.parent.mkdir(parents=True)
                path.write_text('partial failing peer')
                return type('Exit', (), {'returncode': 2})()
            with patch('check_conformance_regressions.subprocess.run', side_effect=execute), \
                    patch.dict('os.environ', {}, clear=True):
                receipt = capture(root, 'memory', output, 2)
            self.assertTrue(receipt['errors'])
            self.assertIn('shards/00/execution.log', receipt['artifacts'])
            with self.assertRaisesRegex(ValueError, 'incomplete execution capture'):
                gate(root, 'memory', output)
            self.assertEqual((output / 'shards/00/execution.log').read_text(), 'partial failing peer')

    def test_external_selection_and_shared_fixture_overrides_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.prepare_inputs(root)
            for key in ['KIVA_CONFORMANCE_CASE_IDS_FILE', 'KIVA_CONFORMANCE_ARTIFACTS_READY',
                        'KIVA_MEMORY_CONFORMANCE_ADDR', 'KIVA_POSTGRES_CONFORMANCE_PGDATA']:
                with self.subTest(key=key), patch.dict('os.environ', {key: 'override'}, clear=True), \
                        patch('check_conformance_regressions.subprocess.run') as process:
                    with self.assertRaises(ValueError):
                        capture(root, 'memory', root / key, 2)
                    process.assert_not_called()


if __name__ == '__main__':
    unittest.main()
