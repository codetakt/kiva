#!/usr/bin/env python3
"""Bind the merged ledger to concrete test definitions and actual oracles.

This inventory separates test-definition projection, executable assertions,
procedures, unresolved source oracles and target results. A count or target PASS
cannot approve a design. Official review is the GitHub PR merge, not a local
receipt or an acceptance flag supplied to this generator.
"""
import argparse
import hashlib
import json
from pathlib import Path

import yaml

from lrs_issued_bindings import select, DIRECT, identifier_format_bindings
from issued_test_design import obligations, review_binding
from issued_definition_contracts import load_contracts
from lrs_procedure_definitions import load_procedures

ROOT = Path(__file__).resolve().parents[2]
CATALOG = 'spec/requirements/ieee-39274-1-1-2025.yaml'
CATALOG_SHA = 'ceb16376c0cfdf9f6c1ccde4ffadfae0dd598a92d73ee42b275d3070080f74c9'
BASELINE = 'spec/conformance/issued-ledger-baseline.json'
BASELINE_SHA = 'e467bb1ce57d01bcff85a3087dd66c1859ddecfef49c953702380df73069d2c2'
OUTPUT = ROOT / 'spec/conformance/issued-test-projection.json'


def collect_execution_contract(root=ROOT):
    """Bind actual oracle inputs, including inherited workspace dependency settings."""
    return {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted([
        *root.glob('tools/xapi-lrs-conformance/src/*.rs'),
        *root.glob('scripts/ci/lrs_projection_*.py'),
        *root.glob('scripts/ci/test_*.py'),
        *[root / p for p in ['Cargo.toml','Cargo.lock','tools/xapi-lrs-conformance/Cargo.toml',
            'scripts/ci/generate_lrs_projection_cases.py','scripts/ci/lrs_issued_bindings.py',
            'scripts/ci/generate_issued_test_projection.py','scripts/ci/report_issued_test_results.py',
            'scripts/ci/report_lrs_interpretations.py',
            'scripts/ci/report_conformance_case_matrix.py',
            'scripts/ci/validate_lrs_projection.py', 'scripts/ci/audit_lrs_source_projection.py',
            'scripts/ci/lrs_generated_artifacts.py',
            'scripts/ci/check_conformance_regressions.py',
            'scripts/ci/conformance_lib.sh',
            'scripts/ci/run_memory_conformance.sh','scripts/ci/run_postgres_conformance.sh',
            'spec/conformance/ci-execution-baseline.json',
            'spec/conformance/endpoint-profile.example.yaml','spec/conformance/auth-profile.example.yaml',
            'docs/architecture/conformance-claim.md','.github/workflows/conformance.yml',
            'scripts/ci/lrs_procedure_definitions.py',
            'spec/conformance/lrs-generated-artifacts.json',
            'spec/conformance/issued-execution-evidence.md',
            'tests/conformance/fixtures/signing/cases.json',
            'spec/conformance/issued-ledger-baseline.json', 'nix/flake/checks.nix',
            'flake.nix', 'nix/flake/packages.nix', 'nix/flake/conformance.nix',
            'tests/conformance/fixtures/preservation-properties.json',
            'scripts/ci/issued_test_design.py','scripts/ci/lrs_issued_scenarios.py',
            'scripts/ci/issued_definition_contracts.py',
            'scripts/ci/lrs_issued_interpretations.py','scripts/ci/lrs_preservation.py',
            'scripts/ci/run_lrs_procedures.py','scripts/ci/probe_lrs_authority_filter.py',
            'scripts/ci/run_issued_revision_controls.py',
            'scripts/ci/run_lrs_person_identifier_controls.py',
            'tests/conformance/controls/person_identifier_fixture.json',
            'tests/conformance/controls/person_identifier_proxy.py',
            'spec/conformance/lrs-interpretation-policy.yaml',
            'spec/conformance/lrs-duty-review.yaml','tests/conformance/models/lrs-tables.yaml',
            'spec/conformance/lrs-review-tests.yaml','spec/conformance/issued-test-procedures.yaml',
            'spec/conformance/issued-supplemental-tests.yaml',
            'spec/conformance/issued-definition-tests.yaml',
            'spec/conformance/issued-definition-contracts.json',
            'spec/conformance/issued-test-definition-completion-amendment.md',
            'spec/conformance/issued-test-definition-source-residuals.json',
            'spec/conformance/definition-contract-vectors.yaml',
            'spec/conformance/issued-definition-source-anchors.json',
            'spec/conformance/reference/language-subtag-registry-20260917.txt',
            'spec/conformance/schemas/conformance-case.schema.json',
            'spec/conformance/schemas/endpoint-profile.schema.json',
            'spec/conformance/schemas/conformance-run-report.schema.json',
            'spec/conformance/schemas/conformance-case-matrix-report.schema.json']],
    ])}


def load(path):
    return yaml.load((ROOT / path).read_text(), Loader=yaml.CSafeLoader)


def digest(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def write(path, content, check):
    if check:
        if not path.exists() or path.read_text() != content:
            raise ValueError('generated projection drift: ' + str(path.relative_to(ROOT)))
    else:
        path.write_text(content)


def project():
    if hashlib.sha256((ROOT / CATALOG).read_bytes()).hexdigest() != CATALOG_SHA:
        raise ValueError('adopted source catalog changed; rebaseline explicitly')
    if hashlib.sha256((ROOT / BASELINE).read_bytes()).hexdigest() != BASELINE_SHA:
        raise ValueError('ledger baseline authority changed; rebaseline explicitly')
    for entry in load(BASELINE)['files']:
        if hashlib.sha256((ROOT / entry['path']).read_bytes()).hexdigest() != entry['sha256']:
            raise ValueError('merged ledger input changed; rebaseline explicitly: ' + entry['path'])
    catalog = load(CATALOG)['issued_contract']
    review = load('spec/conformance/lrs-duty-review.yaml')
    duties = {d['id']:d for d in review['prose_duties']}
    fields = {d['id']:d for d in review['table_field_duties']}
    procedures = load_procedures(ROOT / 'spec/conformance/lrs-review-tests.yaml')
    additional = load('spec/conformance/issued-test-procedures.yaml')['tests']
    supplemental = load('spec/conformance/issued-supplemental-tests.yaml')['tests']
    definitions = load('spec/conformance/issued-definition-tests.yaml')['tests']
    definition_contracts = load_contracts()
    procedure_files = {p['id']:file for file, entries in [
        ('spec/conformance/lrs-review-tests.yaml', procedures),
        ('spec/conformance/issued-test-procedures.yaml', additional),
        ('spec/conformance/issued-supplemental-tests.yaml', supplemental),
        ('spec/conformance/issued-definition-tests.yaml', definitions),
    ] for p in entries}
    additional += supplemental + definitions
    rule_ids = {r['id'] for r in catalog['rules']}
    for procedure in additional:
        if (not procedure['rules'] and not procedure.get('issues')) or set(procedure['rules']) - rule_ids:
            raise ValueError('unknown or absent detailed rule in procedure: ' + procedure['id'])
        if not all(procedure.get(key) for key in ['fixture', 'steps', 'assertions']):
            raise ValueError('procedure lacks fixture, steps or oracle: ' + procedure['id'])
    procedure_index = {p['id']:p for p in procedures + additional}
    if len(procedure_index) != len(procedures) + len(additional):
        raise ValueError('duplicate procedure id')
    cases, case_index = {}, {}
    for path in sorted((ROOT / 'tests/conformance/http').rglob('*.yaml')):
        for case in load(path)['cases']:
            if case['id'] in cases:
                raise ValueError('duplicate case: ' + case['id'])
            cases[case['id']] = case
            assertions = []
            for index, step in enumerate(case.get('steps', [case])):
                prefix = f'/steps/{index}' if 'steps' in case else ''
                for key, value in step['expect'].items():
                    if key != 'capture':
                        assertions.append({'pointer':prefix + '/expect/' + key, 'sha256':digest(value)})
            case_index[case['id']] = dict(file=str(path.relative_to(ROOT)), sha256=digest(case), assertions=assertions)
    families = {r['id'] for r in load(CATALOG)['requirements']}
    for case in cases.values():
        if set(case['requirements']) - families:
            raise ValueError('unknown requirement family in ' + case['id'])
    origins = load('spec/conformance/issued-readback-origins.json')['origins']
    origins.update(load('spec/conformance/issued-rejection-origins.json')['origins'])
    format_bindings = identifier_format_bindings(cases,
        load('tests/conformance/models/lrs-tables.yaml')['models'],
        load('spec/conformance/lrs-table-bindings.yaml')['bindings'], origins)
    completion = load('spec/conformance/issued-completion-bindings.json')['bindings']
    for name, ids in completion.items():
        if name not in cases or set(ids) - rule_ids:
            raise ValueError('invalid completion binding: ' + name + ': ' + str(set(ids) - rule_ids))
    by_origin = {}
    for name, original in origins.items():
        if name not in cases or original not in cases:
            raise ValueError('unknown readback origin: ' + name)
        by_origin.setdefault(original, []).append(name)
    records = []
    for rule in catalog['rules']:
        names, proc, strategy = select(rule, cases, duties, fields)
        names, proc = set(names), set(proc)
        names.update(name for name, ids in format_bindings.items() if rule['id'] in ids)
        names.update(name for name, ids in completion.items() if rule['id'] in ids)
        for procedure in additional:
            if rule['id'] in procedure['rules']:
                proc.add(procedure['id'])
        if rule['id'] == 'IP-mbox-hash-query':
            names.update(n for n in cases if n.startswith('issued-mbox-sha1-query-'))
        if rule['id'] == 'IP-utf8':
            names.update(n for n in cases if n.startswith('issued-utf8-'))
        if rule['id'] == 'IP-batch-atomic-rejection':
            names.update(n for n in cases if n.startswith('issued-reject-') and n.endswith('-batch'))
        if rule['id'] in ['IP-characterstring-validation', 'IP-characterstring-context']:
            names.update(n for n in cases if n.startswith('issued-characterstring-'))
        if rule['id'].endswith('-meaning') and rule['id'] != 'IP-Statement-stored-meaning':
            names.update(n for n in cases if n.startswith('issued-data-meaning-'))
        if rule['id'] in ['IP-extension-values-domain', 'IP-extension-value-acceptance', 'IP-return-serialization']:
            names.update(n for n in cases if n.startswith('issued-opaque-extension-'))
        if rule['id'] == 'IP-return-serialization':
            names.update(n for n in cases if n.startswith(('issued-readback-', 'issued-data-meaning-', 'issued-characterstring-')))
        for original in list(names):
            names.update(by_origin.get(original, []))
        design = obligations(rule, strategy, names, proc, procedure_index,
            [duties[n] for n in DIRECT.get(rule['id'], [])])
        branches = [b for b in definition_contracts['branches'] if rule['id'] in b['rules']]
        design['interpretation_branches'] = [dict(id=b['id'], issue=b['parent_id'],
            procedure='issued-definition-' + b['id'], classification=b['classification'],
            oracle=b['oracle'], source_residual=b['source_residual'], decision_ids=b['decision_ids'],
            policy_selections={d['id']: d['selection'] for d in definition_contracts['decision_options'] if d['id'] in b['decision_ids']},
            project_policy=b.get('project_policy'),
            definition_state='authored expectations; source residuals remain explicit') for b in branches]
        design['incorporated_contracts'] = [d['id'] for d in definition_contracts['dependency_contracts'] if rule['id'] in d['rule_ids']]
        if not names and not proc:
            raise ValueError('rule has no concrete test or procedure: ' + rule['id'])
        if names - cases.keys() or proc - procedure_index.keys():
            raise ValueError('unknown witness for ' + rule['id'])
        records.append(dict(
            id=rule['id'], rule_sha256=digest(rule), actor=rule['actor'], strength=rule['strength'],
            source_units=rule['source_units'], when=rule['when'], requirement=rule['requirement'],
            exceptions=rule['exceptions'], issues=rule['issues'], dependencies=rule['dependencies'],
            correspondence=strategy, cases=sorted(names), procedures=sorted(proc),
            test_design=design,
            review_binding_sha256=review_binding(rule, design, case_index, procedure_index),
            semantic_coverage='not-established-by-static-checks', target_execution='not-established',
            review_obligations=[
                'Verify each concrete request establishes the stated actor and applicability condition.',
                'Verify each assertion tests the required effect, not only the HTTP status.',
                *['Verify this exception has both the applicable branch and its boundary: ' + e for e in rule['exceptions']],
                'Verify mandatory, recommended, optional, unresolved and role-only outcomes remain distinct.',
            ],
        ))
    execution_contract = collect_execution_contract()
    reverse = {name:[] for name in cases}
    for record in records:
        for name in record['cases']:
            reverse[name].append(record['id'])
    if any(not ids for ids in reverse.values()):
        raise ValueError('HTTP case has no issued-rule correspondence')
    return dict(schema_version=2, standard='IEEE Std 9274.1.1-2023', catalog=CATALOG, catalog_sha256=CATALOG_SHA,
        ledger_baseline=BASELINE, ledger_baseline_sha256=BASELINE_SHA,
        claim='All ledger rules have concrete test/procedure correspondence, explicit conditions, exceptions and interpretation/dependency boundaries. Static checks establish structural correspondence and identity, not semantic approval, target execution or universal conformance. Official review is the GitHub PR merge.',
        execution_contract=execution_contract,
        definition_files={name:hashlib.sha256((ROOT / name).read_bytes()).hexdigest()
            for name in sorted({entry['file'] for entry in case_index.values()} | set(procedure_files.values()))},
        summary=dict(rules=len(records), rules_with_http=sum(bool(r['cases']) for r in records),
            rules_with_procedures=sum(bool(r['procedures']) for r in records),
            interpretation_dependent_rules=sum(bool(r['issues']) for r in records),
            cases=len(cases), procedures=len(procedure_index), semantic_coverage_complete=False,
            test_definition_projection_complete=True, unconditional_oracle_complete=False,
            source_questions=len(catalog['open_questions']),
            source_questions_scope='historical issue IDs; not a count of current unresolved branches',
            unresolved_source_branches=sum(bool(b['source_residual']) for b in definition_contracts['branches']),
            project_defined_source_residual_branches=sum(bool(b['source_residual']) and bool(b.get('project_policy')) for b in definition_contracts['branches']),
            exception_branches=sum(len(r['exceptions']) for r in records),
            automatic_execution_complete=False),
        rules=records, cases=case_index, reverse_case_rules=reverse,
        procedures={p['id']:{'file':procedure_files[p['id']], 'sha256':digest(p),
            'executor':('scripts/ci/run_lrs_procedures.py (partial; inspect NOT-RUN checks)' if p in procedures else 'manual; no executor')} for p in procedures+additional})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--require-complete', action='store_true', help='Require concrete ledger/test correspondence, including condition/exception and source-residual branches; no review receipt or target execution is implied')
    args = parser.parse_args()
    document = project()
    write(OUTPUT, json.dumps(document, ensure_ascii=False, indent=2) + '\n', args.check)
    rows = ['# Issued ledger to test correspondence', '', document['claim'], '',
        'Generated by `scripts/ci/generate_issued_test_projection.py`. Conditions, exceptions and assertion hashes are in [issued-test-projection.json](issued-test-projection.json).', '',
        '| Rule | Strength | HTTP cases | Procedures | Interpretation | Semantic coverage |',
        '| --- | --- | ---: | --- | --- | --- |']
    rows += [f"| {r['id']} | {r['strength']} | {len(r['cases'])} | {', '.join(r['procedures']) or '—'} | {', '.join(r['issues']) or '—'} | {r['semantic_coverage']} |" for r in document['rules']]
    write(ROOT / 'spec/conformance/issued-test-matrix.md', '\n'.join(rows) + '\n', args.check)
    print(json.dumps(document['summary'], indent=2))
    if args.require_complete and not document['summary']['test_definition_projection_complete']:
        raise SystemExit('incomplete: ledger/test correspondence is missing')


if __name__ == '__main__':
    main()
