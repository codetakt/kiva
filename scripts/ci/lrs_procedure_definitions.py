"""Expand shared procedure expectations before hashing or interpreting a rubric."""

import re

import yaml


def check_level(spec, identifier):
    """Exact definition-bound check levels; unlisted checks remain required."""
    levels = spec.get('check_levels', {})
    if not isinstance(levels, dict) or any(
            not isinstance(name, str) or not name or level not in ('required', 'advisory')
            for name, level in levels.items()):
        raise ValueError('invalid procedure check_levels')
    return levels.get(identifier, 'required') if isinstance(identifier, str) else 'required'


def load_procedures(path):
    document = yaml.load(path.read_text(), Loader=yaml.CSafeLoader)
    common = document.get('common_expectations', {})
    for procedure in document['tests']:
        check_level(procedure, None)
        expected = procedure.get('expected')
        if not isinstance(expected, str):
            continue

        def replace(match):
            name = match[1]
            if name not in common or not isinstance(common[name], str) or not common[name].strip():
                raise ValueError('unknown or empty common procedure expectation: ' + name)
            if '{{common:' in common[name]:
                raise ValueError('nested common procedure expectations are not supported')
            return common[name]

        procedure['expected'] = re.sub(r'\{\{common:([^{}]+)\}\}', replace, expected)
        if '{{common:' in procedure['expected']:
            raise ValueError('malformed common procedure expectation')
    return document['tests']
