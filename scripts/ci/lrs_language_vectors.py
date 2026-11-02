"""Independent, finite RFC 5646 witnesses at every xAPI Language Map role.

The grammar/registry witness meanings come from DC-LANGUAGE and the pinned IANA
inventory. This enumerates schema roles, including Context Activities and every
interaction component; it does not call Kiva's language validator.
"""
import copy
import hashlib

TAGS = (
    ("ja", True), ("en-US", True), ("zh-Hant", True),
    ("en", True), ("JA-jp", True), ("zh-Hant-CN", True),
    ("ZH-CMN-HANS-CN", True), ("i-klingon", True), ("en-GB-oed", True),
    ("x-private", True), ("X-a-12345678", True), ("en-x-private", True),
    ("sl-rozaj", True), ("de-1901", True), ("qaa-Qaaa-QM", True),
    ("en-u-ca-gregory", True), ("en-rozaj", True),
    ("", False), ("abcdefgh", False), ("abcdefghi", False), ("en-abcdefghi", False),
    ("en_Us", False), ("en--US", False), ("en-", False),
    ("x", False), ("x-123456789", False), ("en-x-123456789", False),
    ("sl-rozaj-rozaj", False), ("en-a-foo-a-bar", False),
    ("en-Zzzz-999", False), ("en-cmn", False), ("zh-cmn-yue", False),
    ("en-unlisted", False), ("日本語", False),
)


def registry_evidence(text):
    """Retain reference records without calling the target's validity predicate.

    TAGS is the independently fixed outcome set. Records support membership;
    they do not replace the RFC syntax and duplicate-component propositions.
    """
    records=[]
    for raw in text.split('%%'):
        fields={}
        for line in raw.splitlines():
            if ': ' in line:
                key,value=line.split(': ',1)
                fields.setdefault(key,[]).append(value)
        if 'Type' in fields:
            records.append(fields)
    evidence=[]
    for tag,valid in TAGS:
        values={tag.lower(),*tag.lower().split('-')}
        found=[]
        for record in records:
            key=(record.get('Subtag') or record.get('Tag',['']))[0].lower()
            ranged=key.split('..')
            if key in values or len(ranged)==2 and any(
                    len(v)==len(ranged[0]) and ranged[0]<=v<=ranged[1] for v in values):
                found.append(record)
        evidence.append({'tag':tag,'expected_valid':valid,'registry_records':found,
                         'syntax_basis':'RFC5646 token/placement grammar, unique variants and extension singletons; extlang Prefix is required, variant Prefix is recommendation; private use needs no registration'})
    return {'file_date':text.splitlines()[0], 'sha256':hashlib.sha256(text.encode()).hexdigest(),
            'vectors':evidence,'role_boundary':'issued LRP invalid-tag wording remains producer-only; LRS rejection duty is unchanged'}


def placements(statement, prefix):
    """Yield (JSON pointer, fixture, mutable map owner/key) independently of tags."""
    for nested in (False, True):
        root = copy.deepcopy(statement)
        if nested:
            sub = copy.deepcopy(statement)
            sub.pop('id', None)
            sub['objectType'] = 'SubStatement'
            root['object'] = sub
        base = '/object' if nested else ''

        def owner(body):
            return body['object'] if nested else body

        body = copy.deepcopy(root)
        yield base + '/verb/display', body, owner(body)['verb'], 'display'
        for where in ('object', 'parent', 'grouping', 'category', 'other'):
            for slot in ('name', 'description', 'choices', 'scale', 'source', 'target', 'steps'):
                body = copy.deepcopy(root)
                current = owner(body)
                activity = {'id': prefix + '/language/' + where, 'definition': {}}
                if where == 'object':
                    current['object'] = activity
                    path = base + '/object/definition'
                else:
                    current['context'] = {'contextActivities': {where: [activity]}}
                    path = base + '/context/contextActivities/' + where + '/0/definition'
                definition = activity['definition']
                if slot in ('name', 'description'):
                    yield path + '/' + slot, body, definition, slot
                else:
                    definition['interactionType'] = {
                        'choices': 'choice', 'scale': 'likert', 'source': 'matching',
                        'target': 'matching', 'steps': 'performance'}[slot]
                    definition[slot] = [{'id': 'a'}]
                    if slot in ('source', 'target'):
                        definition['target' if slot == 'source' else 'source'] = [{'id': 'b'}]
                    yield path + '/' + slot + '/0/description', body, definition[slot][0], 'description'
        for field in ('display', 'description'):
            body = copy.deepcopy(root)
            attachment = {'usageType': prefix + '/attachment', 'display': {'en': 'attachment'},
                          'contentType': 'text/plain', 'length': 0,
                          'sha2': hashlib.sha256(b'').hexdigest(), 'fileUrl': prefix + '/file'}
            owner(body)['attachments'] = [attachment]
            yield base + '/attachments/0/' + field, body, attachment, field
