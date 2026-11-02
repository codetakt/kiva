"""Positive product capabilities require enabled, executed native HTTP controls.

The adopted IEEE procedure gate remains distinct. This gate cannot promote an
advisory observation, a disabled configuration or an external peer into support.
"""
import hashlib
import json
from pathlib import Path

import yaml

CAPABILITIES = frozenset(('negotiated-errors', 'canonical-inventory',
    'resolvable-metadata', 'metadata-host', 'mailbox-hash-query', 'trusted-authority',
    'interaction-validation', 'person-enrichment', 'void-metadata-rollback',
    'pagination-modes', 'extension-namespace'))
MANIFEST = 'spec/conformance/lrs-product-capabilities.yaml'


def definitions(root):
    data = yaml.safe_load((root / MANIFEST).read_text())
    rows = data.get('capabilities', [])
    source = yaml.safe_load((root /
        'spec/requirements/ieee-39274-1-1-2025.yaml').read_text())['issued_contract']['rules']
    source_rules = {row['id']: row for row in source}
    projection = json.loads((root / 'spec/conformance/issued-test-projection.json').read_text())
    projection_indices = {row['id']: index for index, row in enumerate(projection['rules'])}
    rules = set(source_rules)
    expected_modal = {row['id'] for row in source if row['strength'] in ('should','should-not','may','permission')}
    modal = data.get('modal_requirements', [])
    if (len(modal) != len(expected_modal) or {row['rule'] for row in modal} != expected_modal
            or any(not row.get('disposition') or not row.get('adopted_test_projection') for row in modal)):
        raise ValueError('incomplete recommended/optional requirement scope')
    if (data.get('schema_version') != 1 or len(rows) != len(CAPABILITIES)
            or {row['id'] for row in rows} != CAPABILITIES):
        raise ValueError('incomplete product capability inventory')
    for row in modal:
        source_rule = source_rules[row['rule']]
        if (row.get('strength') != source_rule['strength']
                or row.get('source_units') != source_rule['source_units']
                or row['adopted_test_projection'] !=
                'spec/conformance/issued-test-projection.json#/rules/' + str(projection_indices[row['rule']])):
            raise ValueError('incorrect source/test trace: ' + row['rule'])
        capabilities = row.get('capabilities', [])
        if (not set(capabilities) <= CAPABILITIES
                or (row['disposition'] == 'required-product-capability' and not capabilities)
                or not (capabilities or row.get('implementation') or row.get('design'))):
            raise ValueError('missing implementation disposition: ' + row['rule'])
        for name in row.get('implementation', []):
            path = root / name
            if not path.resolve().is_relative_to(root.resolve()) or not path.is_file():
                raise ValueError('missing implementation anchor: ' + name)
        for capability in capabilities:
            linked = next(item for item in rows if item['id'] == capability)
            if row['rule'] not in linked['rules']:
                raise ValueError('inconsistent capability trace: ' + row['rule'])
    for row in rows:
        if (row.get('required') is not True or row.get('fixtures') != ['memory', 'postgres']
                or not row.get('rules') or not set(row['rules']) <= rules
                or not row.get('native_controls') or not row.get('implementation')):
            raise ValueError('incomplete capability definition: ' + row['id'])
        for name in row['implementation']:
            path = root / name
            if not path.resolve().is_relative_to(root.resolve()) or not path.is_file():
                raise ValueError('missing implementation anchor: ' + name)
    return rows


def evaluate(rows, backend, tests, observations, http_tests):
    if backend not in ('memory', 'postgres'):
        raise ValueError('product capabilities require an actual storage backend')
    result = {}
    for row in rows:
        controls = [name.format(backend=backend) for name in row['native_controls']]
        observed = [record for record in observations if
            'kiva-server::' + str(record.get('test', '')) in controls
            and record.get('kind') == 'product-capability'
            and record.get('observation', {}).get('enabled') is True
            and row['id'] in record['observation'].get('capabilities', [])]
        observed_controls = {'kiva-server::' + record['test'] for record in observed}
        errors = []
        for control in controls:
            if tests.get(control, {}).get('outcome') != 'passed':
                errors.append('control did not pass: ' + control)
            if control not in http_tests or control not in observed_controls:
                errors.append('missing enabled HTTP execution: ' + control)
        if row['id'] == 'pagination-modes' and {
                record['observation'].get('pagination_mode') for record in observed
        } != {'live', 'snapshot'}:
            errors.append('both pagination modes must execute')
        if row['id'] == 'pagination-modes' and not any(
                record['observation'].get('self_contained_short_and_opaque_long') is True
                for record in observed):
            errors.append('bounded self-contained continuation mode must execute')
        result[row['id']] = {'outcome': 'failed' if errors else 'passed',
            'rules': row['rules'], 'controls': controls, 'errors': errors}
    return {'backend': backend, 'capabilities': result,
        'complete': bool(result) and all(row['outcome'] == 'passed' for row in result.values())}


def assess(root, directory, backend, tests):
    observations = [json.loads(path.read_text()) for path in
                    sorted((directory / 'http/controls').glob('*.json'))]
    http_tests = set()
    for path in (directory / 'http').glob('*.json'):
        record = json.loads(path.read_text())
        if record.get('received_response') and record.get('response') and record.get('request'):
            http_tests.add('kiva-server::' + str(record.get('test', '')))
    result = evaluate(definitions(root), backend, tests, observations, http_tests)
    result['definition_sha256'] = hashlib.sha256((root / MANIFEST).read_bytes()).hexdigest()
    return result
