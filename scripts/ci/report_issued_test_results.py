#!/usr/bin/env python3
"""Keep issued-rule test design, HTTP results and procedure coverage separate.

FAIL is a valid test execution outcome. Missing tests, stale case receipts,
unexecuted procedures and open interpretations never become a coverage PASS.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

from lrs_procedure_definitions import check_level, load_procedures

ROOT = Path(__file__).resolve().parents[2]


def load(path):
    return json.loads(path.read_text())


def file_sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_execution_receipt(projection, projection_path, artifact_paths, receipt, root=ROOT):
    """Check a run-time receipt, never manufacture one from current code/results.

    The execution producer must capture inputs around the actual run and bind
    its output bytes. Identity checks cannot establish that producer's honesty
    or prove a binary was built from the recorded source; that provenance is a
    separate execution acceptance requirement.
    """
    if receipt.get('schema_version') != 1 or receipt.get('kind') != 'issued-test-execution-receipt':
        raise ValueError('unsupported execution receipt')
    if receipt.get('errors'):
        raise ValueError('incomplete execution capture: ' + str(receipt['errors']))
    if receipt.get('projection_sha256') != file_sha256(projection_path):
        raise ValueError('execution receipt does not bind the current projection')
    inputs = receipt.get('input_sha256', {})
    contract = projection.get('execution_contract')
    if not isinstance(contract, dict) or not contract:
        raise ValueError('projection lacks an execution contract')
    definitions = {entry['file'] for group in ('cases', 'procedures')
                   for entry in projection[group].values()}
    definition_files = projection.get('definition_files')
    if not isinstance(definition_files, dict) or set(definition_files) != definitions:
        raise ValueError('projection lacks the exact definition file inventory')
    for name in sorted(set(contract) | definitions):
        relative = Path(name)
        path = root / relative
        if relative.is_absolute() or '..' in relative.parts or not path.resolve().is_relative_to(root.resolve()):
            raise ValueError('unsafe execution input path: ' + name)
        current = file_sha256(path)
        if name in definitions and definition_files[name] != current:
            raise ValueError('current definition differs from projection: ' + name)
        if name in contract and contract[name] != current:
            raise ValueError('current execution contract drift: ' + name)
        if inputs.get(name) != current:
            raise ValueError('execution input not bound to current bytes: ' + name)
    artifacts = receipt.get('artifacts_sha256', {})
    if not isinstance(artifacts, dict):
        raise ValueError('execution receipt lacks artifact hashes')
    for path in artifact_paths:
        if file_sha256(path) not in artifacts.values():
            raise ValueError('execution artifact not bound by receipt: ' + str(path))


def procedure_incomplete(result, spec):
    # The overall outcome and each check have separate definition-bound levels.
    # Neither an advisory procedure nor a report-supplied level weakens an
    # unlisted/required check (including mandatory response headers).
    completed = {'passed', 'failed'}
    if spec.get('level') == 'advisory':
        completed.add('observed')
    checks = result.get('checks')
    # No applicability-evidence verifier exists yet. A not-applicable label or
    # free-text reason alone therefore cannot complete this execution gate.
    return (result.get('outcome') not in completed
            or not isinstance(checks, list) or not checks
            or any(not isinstance(check, dict) or not (
                       check.get('outcome') in ('passed', 'failed')
                       or (check.get('outcome') == 'observed'
                           and check_level(spec, check.get('id')) == 'advisory'))
                   for check in checks))


def execution_complete(report):
    summary = report['summary']
    return (summary['test_definition_projection_complete']
            and summary['execution_evidence_verified']
            and summary['http_execution_mode'] and summary['http_report_complete']
            and not any(summary[key] for key in (
                'unexecuted_http_cases', 'incomplete_http_cases', 'missing_http_cases', 'stale_cases',
                'missing_procedures', 'incomplete_procedures')))


def definition_obligations(spec):
    """Address every authored rubric item without interpreting or weakening it."""
    obligations = {}
    def visit(value, pointer):
        if isinstance(value, dict):
            for key, child in value.items():
                visit(child, pointer + '/' + key.replace('~', '~0').replace('/', '~1'))
        elif isinstance(value, list):
            for index, child in enumerate(value):
                visit(child, pointer + '/' + str(index))
        else:
            obligations[pointer] = hashlib.sha256(json.dumps(value, ensure_ascii=False,
                sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    for field in ('setup', 'action', 'expected', 'fixture', 'steps', 'assertions',
                  'rubric', 'vectors', 'association_controls'):
        if field in spec:
            visit(spec[field], '/' + field)
    return obligations


def procedure_coverage_complete(result, spec):
    """Require explicit evidence links for the whole rubric, not one PASS label.

    These links remain author evidence: a hash cannot establish that a check
    semantically discharges the linked prose. Independent review remains separate.
    """
    obligations = definition_obligations(spec)
    coverage = result.get('definition_coverage')
    checks = result.get('checks', [])
    if not obligations or not isinstance(coverage, dict) or set(coverage) != set(obligations):
        return False
    identifiers = [c.get('id') for c in checks if isinstance(c, dict)]
    if (len(identifiers) != len(checks) or not all(isinstance(i, str) and i for i in identifiers)
            or len(set(identifiers)) != len(identifiers)):
        return False
    for pointer, digest in obligations.items():
        entry = coverage[pointer]
        if not isinstance(entry, dict) or entry.get('sha256') != digest:
            return False
        links = entry.get('checks')
        if (not isinstance(links, list) or not links
                or any(not isinstance(i, str) or i not in identifiers for i in links)
                or len(set(links)) != len(links)):
            return False
    return True


def test_specification_conformant(report):
    """All current mandatory test observations pass; this is not an IEEE ruling."""
    summary = report['summary']
    return (execution_complete(report)
            and summary.get('implementation_evidence_verified') is True
            and summary.get('http_evidence_verified') is True
            and summary.get('wrapper_exit') == 0
            and summary.get('http_report_status') == 'passed'
            and set(summary['http_outcomes']) <= {'passed'}
            and summary.get('http_cases_observed', 0) > 0
            and not summary.get('inconsistent_http_cases', 0)
            and not summary.get('failed_procedure_checks', 0)
            and not summary.get('incomplete_procedure_coverage', 1)
            and not summary['procedure_outcomes'].get('failed', 0))


def implementation_inputs(root=ROOT):
    """Bind all local implementation/build/database inputs, including new files."""
    paths = {'Cargo.toml', 'Cargo.lock', 'flake.nix', 'flake.lock', 'rust-toolchain.toml', 'db/schema.sql',
             'spec/conformance/implementation-procedure-bindings.json'}
    for pattern in ('.cargo/**/*', 'crates/**/*', 'vendor/**/*', 'tools/xapi-lrs-conformance/**/*',
                    'db/migrations/*', 'nix/**/*.nix', 'tests/conformance/controls/**/*',
                    'tests/conformance/fixtures/**/*', 'tests/postgres/**/*',
                    'scripts/ci/**/*.py', 'scripts/ci/**/*.sh', 'spec/openapi/**/*',
                    'generated/**/*', 'spec/conformance/reference/*'):
        paths.update(str(path.relative_to(root)) for path in root.glob(pattern) if path.is_file())
    return {name: file_sha256(root / name) for name in sorted(paths) if (root / name).is_file()}


def verify_implementation_receipt(receipt, root=ROOT):
    implementation = receipt.get('implementation', {})
    if implementation.get('backend') not in ('memory', 'postgres'):
        raise ValueError('implementation receipt needs the executed backend')
    if implementation.get('input_sha256') != implementation_inputs(root):
        raise ValueError('implementation source/build/database input drift')
    for key in ('server_binary_sha256', 'runner_binary_sha256'):
        value = implementation.get(key)
        if not isinstance(value, str) or len(value) != 64 or any(c not in '0123456789abcdef' for c in value):
            raise ValueError('implementation receipt lacks ' + key)


def verify_passing_http(http, root=ROOT):
    # Reuse the exhaustive HTTP gate with NO known-gap exceptions. Validate
    # attempts and totals as well as the final labels, against actual definitions.
    import jsonschema
    from check_conformance_regressions import SCHEMA, compare, definitions
    try:
        jsonschema.Draft202012Validator(load(root / SCHEMA)).validate(http)
    except jsonschema.ValidationError as error:
        raise ValueError('invalid HTTP evidence: ' + error.message) from error
    cases, suites, _ = definitions(root)
    compare(http, 0, cases, suites, {})


def assess_capture(directory, root=ROOT):
    """Reassess original captured evidence, never trust a supplied PASS summary."""
    projection_path = root / 'spec/conformance/issued-test-projection.json'
    projection = load(projection_path)
    paths = [directory / name for name in
             ('http.json', 'case-receipt.json', 'procedure-evidence.json', 'fixture.json')]
    receipt = load(directory / 'execution.json')
    artifacts = receipt.get('artifacts_sha256', {})
    for name in ('http.json', 'http.xml', 'case-receipt.json', 'fixture.json',
                 'execution.log', 'procedures/summary.json', 'procedures/http.jsonl',
                 'native/receipt.json', 'procedure-evidence.json'):
        if name not in artifacts:
            raise ValueError('missing capture artifact: ' + name)
    for name, digest in artifacts.items():
        path = directory / name
        if (Path(name).is_absolute() or '..' in Path(name).parts
                or not path.resolve().is_relative_to(directory.resolve())
                or file_sha256(path) != digest):
            raise ValueError('capture artifact drift: ' + name)
    verify_execution_receipt(projection, projection_path, paths, receipt, root)
    verify_implementation_receipt(receipt, root)
    if receipt.get('fixture_sha256') != file_sha256(paths[-1]):
        raise ValueError('execution fixture drift')
    http = load(paths[0])
    workers = receipt.get('workers', 1)
    if type(workers) is not int or not 1 <= workers <= 8:
        raise ValueError('invalid capture worker count')
    if workers > 1:
        import jsonschema
        from check_conformance_regressions import SCHEMA, definitions
        from run_parallel_conformance import verify_aggregate
        cases, suites, files = definitions(root)
        verify_aggregate(directory / 'shards', workers, http, directory / 'http.xml',
                         cases, suites, files, jsonschema.Draft202012Validator(load(root / SCHEMA)))
    from collect_lrs_procedure_evidence import collect_capture, combined_wrapper_exit
    reconstructed=collect_capture(directory,receipt['implementation']['backend'],root)
    if reconstructed != load(paths[2]):
        raise ValueError('procedure discharge summary differs from original execution witnesses')
    if receipt.get('wrapper_exit') != combined_wrapper_exit(receipt.get('partial_wrapper_exit'),reconstructed,http,root):
        raise ValueError('procedure discharge exit mismatch')
    report = assess(projection, http, load(paths[1])['cases'], [reconstructed], root=root)
    report['summary'].update(execution_evidence_verified=True,
                             implementation_evidence_verified=True,
                             wrapper_exit=receipt.get('wrapper_exit'))
    try:
        verify_passing_http(http, root)
        report['summary']['http_evidence_verified'] = True
    except (ValueError, KeyError, TypeError) as error:
        report['http_validation_error'] = str(error)
    report['summary']['test_specification_conformant'] = test_specification_conformant(report)
    return report


def unresolved_http_checks(case):
    """Retain partial execution independently of the case's FAIL verdict.

    Read final case diagnostics, not recovered intermediate polling attempts.
    Also recognize the existing mixed failure spelling so omission of the new
    structured field cannot hide an explicit ORACLE-UNAVAILABLE diagnosis.
    """
    reasons = []
    for field in ('unresolved_checks', 'not_run_reasons', 'failures'):
        values = case.get(field, [])
        if not isinstance(values, list) or any(not isinstance(value, str) for value in values):
            raise ValueError('invalid HTTP check evidence: ' + field)
        for value in values:
            diagnostic = value.partition(': ')[2] if value.startswith('step ') else value
            if field != 'failures' or diagnostic.startswith('ORACLE-UNAVAILABLE:'):
                if value not in reasons:
                    reasons.append(value)
    return reasons


def assess(projection, http, case_receipt, procedure_reports, root=None):
    root = root or ROOT
    observations = {}
    for suite in http['suites']:
        for case in suite['cases']:
            if case['id'] in observations:
                raise ValueError('duplicate HTTP result: ' + case['id'])
            observations[case['id']] = case
    unknown = set(observations) - projection['cases'].keys()
    if unknown:
        raise ValueError('HTTP results reference unknown cases: ' + ', '.join(sorted(unknown)))
    stale = {name for name in observations if case_receipt.get(name, {}).get('sha256') != projection['cases'][name]['sha256']}
    unexecuted = {name for name, case in observations.items() if case.get('outcome') not in ('passed', 'failed')}
    unresolved = {name: reasons for name, case in observations.items()
                  if (reasons := unresolved_http_checks(case))}
    incomplete_http = unexecuted | unresolved.keys()
    inconsistent_http = {name for name, case in observations.items()
                         if case.get('outcome') == 'passed' and case.get('failures')}
    procedures = {}
    incomplete = set()
    uncovered = set()
    failed_checks = []
    for report in procedure_reports:
        for name, result in report['procedures'].items():
            if name in procedures or name not in projection['procedures']:
                raise ValueError('duplicate or unknown procedure: ' + name)
            # run_lrs_procedures uses compact insertion-order JSON for its rubric
            # receipt, so validate that format against the current actual spec.
            definitions = load_procedures(root / projection['procedures'][name]['file'])
            spec = next(p for p in definitions if p['id'] == name)
            expected_hash = hashlib.sha256(json.dumps(spec, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()
            if result.get('rubric_sha256') != expected_hash:
                raise ValueError('stale procedure rubric: ' + name)
            procedures[name] = result
            if procedure_incomplete(result, spec):
                incomplete.add(name)
            if not procedure_coverage_complete(result, spec):
                uncovered.add(name)
            for check in result.get('checks', []) or []:
                if (isinstance(check, dict) and check.get('outcome') == 'failed'
                        and check_level(spec, check.get('id')) != 'advisory'):
                    failed_checks.append({'procedure': name, 'check': check.get('id')})
    rows = []
    case_contexts = {n:set() for n in observations}
    for rule in projection['rules']:
        branches = rule.get('test_design', {}).get('interpretation_branches', [])
        # Historical parent issue IDs are not blanket waivers after disposition.
        # Keep concrete residual branches and policy choices as separate data;
        # neither changes an observed raw outcome or creates review acceptance.
        successor = any('classification' in b for b in branches)
        dependencies = ([b['id'] for b in branches if b.get('source_residual')]
            if successor else rule['issues'])
        for name in rule['cases']:
            if name in case_contexts:
                case_contexts[name].update(dependencies)
        counts = Counter('stale' if n in stale else observations.get(n, {}).get('outcome', 'not-run') for n in rule['cases'])
        missing_procedures = [n for n in rule['procedures'] if n not in procedures]
        incomplete_procedures = [n for n in rule['procedures'] if n in incomplete]
        rows.append(dict(id=rule['id'], strength=rule['strength'], semantic_coverage=rule['semantic_coverage'],
            http=dict(counts), missing_procedures=missing_procedures,
            incomplete_http_cases=sorted(set(rule['cases']) & incomplete_http),
            procedure_outcomes={n:procedures[n].get('outcome', 'not-run') for n in rule['procedures'] if n in procedures},
            incomplete_procedures=incomplete_procedures, interpretation_dependencies=dependencies,
            historical_issue_ids=rule['issues'], definition_dispositions=branches,
            pending_policy_choices=sorted({d for b in branches for d in b.get('decision_ids', []) if b.get('policy_selections', {}).get(d) is None}),
            selected_project_policies={d: selection for b in branches for d, selection in b.get('policy_selections', {}).items() if selection is not None},
            disputed_ieee_facets={b['id']: 'source-indeterminate' for b in branches if b.get('source_residual')},
            project_policy_contexts=[b['project_policy'] for b in branches if b.get('project_policy')],
            grading_scope='Raw observations are unchanged. Selected policy outcomes do not settle disputed IEEE facets; independently decidable IEEE violations remain reportable.',
            external_dependencies=rule['dependencies'],
            full_requirement_pass=False))
    return dict(schema_version=1, claim='Execution observations only; no rule-wide or whole-standard conformance verdict.',
        summary=dict(rules=len(rows), http_cases_observed=len(observations), http_outcomes=dict(Counter(c['outcome'] for c in observations.values())),
            http_mode=http.get('mode'), http_execution_mode=http.get('mode') == 'execute-http',
            http_report_status=http.get('status'),
            http_report_complete=http.get('status') in ('passed', 'failed'),
            execution_evidence_verified=False,
            implementation_evidence_verified=False,
            http_evidence_verified=False,
            test_specification_conformant=False,
            inconsistent_http_cases=len(inconsistent_http),
            failed_procedure_checks=len(failed_checks),
            incomplete_procedure_coverage=len(uncovered),
            unexecuted_http_cases=len(unexecuted),
            incomplete_http_cases=len(incomplete_http),
            stale_cases=len(stale), missing_http_cases=len(set(projection['cases']) - observations.keys()),
            missing_procedures=len(set(projection['procedures']) - procedures.keys()),
            incomplete_procedures=len(incomplete),
            procedure_outcomes=dict(Counter(p.get('outcome', 'not-run') for p in procedures.values())),
            source_interpretation_dependent_rules=sum(bool(r['interpretation_dependencies']) for r in rows),
            test_definition_projection_complete=projection['summary'].get('test_definition_projection_complete', False),
            semantic_coverage_complete=projection['summary']['semantic_coverage_complete']),
        stale_cases=sorted(stale), unexecuted_http_cases=sorted(unexecuted), missing_http_cases=sorted(set(projection['cases']) - observations.keys()),
        incomplete_http_cases=sorted(incomplete_http), unresolved_http_checks=unresolved,
        incomplete_procedures=sorted(incomplete),
        inconsistent_http_cases=sorted(inconsistent_http),
        failed_procedure_checks=failed_checks,
        incomplete_procedure_coverage=sorted(uncovered),
        case_interpretation_contexts={n:sorted(ids) for n,ids in sorted(case_contexts.items()) if ids},
        case_context_scope='Conservative union of issues on corresponding detailed rules, not proof that every issue affects every assertion. Raw FAIL remains a fixture/profile observation; no unqualified IEEE verdict is inferred.',
        rules=rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--projection', type=Path, default=ROOT / 'spec/conformance/issued-test-projection.json')
    parser.add_argument('--http', type=Path, required=True)
    parser.add_argument('--case-receipt', type=Path, required=True, help='Case map or complete projection captured for this execution, not regenerated from the current tree')
    parser.add_argument('--evidence-receipt', type=Path,
                        help='Run-time input/output receipt; required for execution completeness, distinct from the case map')
    parser.add_argument('--procedures', type=Path, action='append', default=[])
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--require-complete', action='store_true')
    parser.add_argument('--require-conforming', action='store_true',
                        help='Require current implementation evidence and PASS for the complete test specification, including every procedure rubric item')
    args = parser.parse_args()
    paths = [args.projection, args.http, args.case_receipt, *args.procedures]
    if args.evidence_receipt:
        paths.append(args.evidence_receipt)
    before = {str(p):file_sha256(p) for p in paths}
    receipt = load(args.case_receipt)
    report = assess(load(args.projection), load(args.http), receipt.get('cases', receipt), [load(p) for p in args.procedures])
    if args.evidence_receipt:
        verify_execution_receipt(load(args.projection), args.projection,
                                 [args.http, args.case_receipt, *args.procedures],
                                 load(args.evidence_receipt))
        report['summary']['execution_evidence_verified'] = True
        if args.require_conforming or 'implementation' in load(args.evidence_receipt):
            verify_implementation_receipt(load(args.evidence_receipt))
            report['summary']['implementation_evidence_verified'] = True
            report['summary']['wrapper_exit'] = load(args.evidence_receipt).get('wrapper_exit')
    report['input_sha256'] = {str(p):file_sha256(p) for p in paths}
    if report['input_sha256'] != before:
        raise ValueError('execution evidence changed while being assessed')
    report['execution_evidence_scope'] = ('Input/output identities only; producer trust and built-runner provenance require separate execution acceptance. '
                                        'Absent receipts and unsupported applicability exemptions never establish completeness.')
    # A failed HTTP run is still retained as execution evidence; malformed
    # reports are rejected by schema validation rather than accepted as PASS.
    try:
        verify_passing_http(load(args.http))
        report['summary']['http_evidence_verified'] = True
    except (ValueError, KeyError, TypeError) as error:
        report['http_validation_error'] = str(error)
    report['summary']['test_specification_conformant'] = test_specification_conformant(report)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report['summary'], indent=2))
    if args.require_complete and not execution_complete(report):
        raise SystemExit(2)
    if args.require_conforming and not report['summary']['test_specification_conformant']:
        raise SystemExit(2)


if __name__ == '__main__':
    main()
