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


def assess(projection, http, case_receipt, procedure_reports):
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
    procedures = {}
    incomplete = set()
    for report in procedure_reports:
        for name, result in report['procedures'].items():
            if name in procedures or name not in projection['procedures']:
                raise ValueError('duplicate or unknown procedure: ' + name)
            # run_lrs_procedures uses compact insertion-order JSON for its rubric
            # receipt, so validate that format against the current actual spec.
            definitions = load_procedures(ROOT / projection['procedures'][name]['file'])
            spec = next(p for p in definitions if p['id'] == name)
            expected_hash = hashlib.sha256(json.dumps(spec, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()
            if result.get('rubric_sha256') != expected_hash:
                raise ValueError('stale procedure rubric: ' + name)
            procedures[name] = result
            if procedure_incomplete(result, spec):
                incomplete.add(name)
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
            http_report_complete=http.get('status') in ('passed', 'failed'),
            execution_evidence_verified=False,
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
    report['input_sha256'] = {str(p):file_sha256(p) for p in paths}
    if report['input_sha256'] != before:
        raise ValueError('execution evidence changed while being assessed')
    report['execution_evidence_scope'] = ('Input/output identities only; producer trust and built-runner provenance require separate execution acceptance. '
                                        'Absent receipts and unsupported applicability exemptions never establish completeness.')
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report['summary'], indent=2))
    if args.require_complete and not execution_complete(report):
        raise SystemExit(2)


if __name__ == '__main__':
    main()
