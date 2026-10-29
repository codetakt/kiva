"""About response observations, separate from request-cause policy.

Repeated member names are an interoperability recommendation, not the
Statement duplicate-name rejection rule. Ambiguous required data stays unknown.
"""
import json
import re

from lrs_preservation import MemberObject, members


def about_content(raw, latest_versions):
    def invalid_constant(value):
        raise ValueError('non-JSON numeric token: '+value)

    try:
        value = json.loads(raw, object_pairs_hook=MemberObject,
                           parse_constant=invalid_constant)
    except (ValueError, UnicodeError):
        return {'required': False, 'reason': 'invalid complete JSON representation',
                'duplicate_names': []}
    duplicates = []

    def inspect(item, path):
        if isinstance(item, dict):
            seen = set()
            for key, child in members(item):
                if key in seen:
                    duplicates.append(path+'/'+key)
                seen.add(key)
                inspect(child, path+'/'+key)
        elif isinstance(item, list):
            for i, child in enumerate(item):
                inspect(child, path+'/'+str(i))
    inspect(value, '')
    if not isinstance(value, dict):
        return {'required': False, 'reason': 'About must be an object',
                'duplicate_names': duplicates}
    versions = [v for k, v in members(value) if k == 'version']
    if not versions:
        return {'required': False, 'reason': 'required version absent',
                'duplicate_names': duplicates}

    def version_valid(candidate):
        if not isinstance(candidate, list) or not candidate or not all(
                isinstance(v, str) and re.fullmatch(r'(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)', v)
                for v in candidate):
            return False
        if latest_versions is None:
            return None
        # The fixture declares supported versions before looking at responses.
        return all(v in candidate for v in latest_versions)

    decisions = [version_valid(v) for v in versions]
    # Different repeated required values cannot be resolved by last-value JSON
    # parsing. An unambiguous mandatory defect remains a failure.
    required = (False if all(v is False for v in decisions) else None
                if any(v != versions[0] for v in versions[1:]) or None in decisions
                or False in decisions else True)
    extensions = [v for k, v in members(value) if k == 'extensions']
    for extension in extensions:
        # About's 4.1 table says Object/map of other properties. Do not import
        # the Statement model's 4.2 IRI-keyed Extensions restriction here.
        valid = isinstance(extension, dict)
        if not valid:
            required = False
    return {'required': required, 'reason': 'required version/extensions; unknown top-level members allowed',
            'duplicate_names': duplicates, 'version_candidates': versions}
