"""Static projection of the authored interpretation/dependency definitions.

No target calls, cryptographic fixture generation or review verdicts are made
here. The adopted source ledger is immutable; these are successor definitions.
"""
import json
from pathlib import Path
import yaml

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = 'spec/conformance/issued-definition-contracts.json'


def load_contracts():
    return json.loads((ROOT / CONTRACT).read_text())


def branch_procedures():
    document = load_contracts()
    tests = []
    for branch in document['branches']:
        tests.append(dict(
            id='issued-definition-' + branch['id'],
            rules=branch['rules'], issues=[branch['parent_id']],
            level=branch['classification'],
            fixture=branch['test']['fixture'],
            steps=branch['test']['steps'], assertions=branch['test']['assertions'],
            contract_ref=CONTRACT + '#/branches/' + str(len(tests)),
            source_basis=branch['source_basis'],
            policy_choices=branch['decision_ids'],
            project_policy=branch.get('project_policy'),
            source_residual=branch['source_residual'],
            grading='Keep project-policy outcomes separate from disputed IEEE propositions. Independent IEEE requirements retain their strength; no blanket waiver or automatic PASS.',
            executor='manual; definition only; no target execution',
        ))
    return tests


def apply_interpretation_definitions(tests):
    """Keep the 25 stable parent IDs but replace superseded indeterminate cells."""
    document = load_contracts()
    for test in tests:
        parent = test['id'].removeprefix('issued-discriminator-')
        branches = [b for b in document['branches'] if b['parent_id'] == parent]
        if not branches:
            continue
        test['level'] = 'branch-specific-definition'
        test['steps'] = [
            'Use the explicitly defined inputs, conditions and exceptions in each linked branch procedure; record each branch separately.',
            *['Apply issued-definition-' + b['id'] for b in branches],
        ]
        test['assertions'] = [b['id'] + ': ' + b['oracle'] for b in branches]
        test['oracle_partition'] = dict(
            definitions=[dict(id=b['id'], classification=b['classification'],
                procedure='issued-definition-' + b['id'], decision_ids=b['decision_ids'],
                project_policy=b.get('project_policy')) for b in branches],
            source_residuals=[b['id'] for b in branches if b['source_residual']],
            policy='Kiva choices remain separate from IEEE allowed sets. Nonselection alone is not source indeterminacy; consult each branch source_residual and decision applicability.',
            execution='Definition only. Missing provider/fixture is not source indeterminacy and never PASS.',
        )
    return tests


def rule_definitions(rule_id):
    # Callers loading all rules should cache the document themselves.
    return [b['id'] for b in load_contracts()['branches'] if rule_id in b['rules']]


def dependency_procedures():
    document = load_contracts()
    vectors = yaml.load((ROOT / document['definition_vectors']).read_text(), Loader=yaml.CSafeLoader)['contracts']
    result = []
    for index, contract in enumerate(document['dependency_contracts']):
        examples = vectors[contract['id']]
        result.append(dict(id='issued-contract-' + contract['id'], rules=contract['rule_ids'],
            level='incorporated-contract',
            fixture='Abstract independently characterized inputs, trusted reference results, declared algorithm/profile and role; no provider selected or fixture constructed. Inputs: ' + json.dumps(contract['inputs'], ensure_ascii=False),
            steps=[v['id'] + ': ' + v['input'] for v in examples],
            assertions=[v['id'] + ': ALLOWED: ' + v['allowed'] + ' FORBIDDEN: ' + v['forbidden'] for v in examples],
            vectors=examples, contract_ref=CONTRACT + '#/dependency_contracts/' + str(index),
            executor='manual; provider integration and behavioral execution are later work',
            grading='Required/allowed/advisory/producer/policy conditions retain their contract strength. An unavailable required input yields incomplete execution, never valid.',
        ))
    return result
