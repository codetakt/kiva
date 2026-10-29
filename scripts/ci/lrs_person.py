"""Person association checks against a separately supplied fixture inventory.

These checks do not infer associations from Person responses, random IFIs, or
syntactic validity. Grammar and status checks remain independently necessary.
"""

IFI = ('mbox', 'mbox_sha1sum', 'openid', 'account')


def identity(field, value):
    if field == 'account':
        if not isinstance(value, dict) or not all(isinstance(value.get(k), str) for k in ('homePage', 'name')):
            return None
        return value['homePage'], value['name']
    return value if isinstance(value, str) else None


def person_associations(requested, person, inventory):
    """Return per-property/index decisions; unknown evidence never masks FAIL."""
    checks = []

    def add(identifier, valid, reason, details=None):
        checks.append({'id': identifier, 'valid': valid, 'reason': reason, 'details': details})

    if not isinstance(person, dict) or person.get('objectType') != 'Person':
        add('person-object', False, 'A Person Object is required')
        return checks
    queried = [field for field in IFI if field in requested]
    for field in queried:
        expected = identity(field, requested[field])
        entries = person.get(field)
        present = (expected is not None and isinstance(entries, list)
                   and any(identity(field, item) == expected for item in entries))
        add(field+'-requested', present, 'The complete queried IFI must be present')
    trusted = (isinstance(inventory, dict)
               and all(isinstance(inventory.get(k), str) and inventory[k].strip() for k in ('revision','authority'))
               and inventory.get('queried') == requested)
    mode = inventory.get('mode') if trusted else None
    complete = (trusted and inventory.get('complete') is True and
                (mode == 'enriched' or mode == 'fallback' and inventory.get('no_additional_information') is True))
    associated = inventory.get('associated', {}) if trusted else {}
    unrelated = inventory.get('unrelated', {}) if trusted else {}
    if not isinstance(associated, dict) or not isinstance(unrelated, dict):
        associated, unrelated, complete = {}, {}, False
    if complete:
        # A complete inventory contradicting its own requested identity is a
        # fixture defect, not evidence that an otherwise valid target failed.
        for field in queried:
            known=associated.get(field,[])
            if (not isinstance(known,list)
                    or not any(identity(field,v)==identity(field,requested[field]) for v in known)):
                complete=False
    for field in ('name',*IFI):
        known,other=associated.get(field,[]),unrelated.get(field,[])
        if (isinstance(known,list) and isinstance(other,list)
                and any(identity(field,a) is not None and identity(field,a)==identity(field,b) for a in known for b in other)):
            complete=False
            unrelated={}  # Conflicting positive/negative fixture assertions.
            break
    add('inventory-applicability', True if complete else None,
        'Independently declared association inventory and fallback/enrichment precondition',
        {'revision': inventory.get('revision') if trusted else None, 'mode': mode})
    for field in ('name', *IFI):
        entries = person.get(field, [])
        if not isinstance(entries, list):
            add(field+'-array', False, 'Person properties are arrays')
            continue
        known = associated.get(field, [])
        other = unrelated.get(field, [])
        usable = (isinstance(known, list) and isinstance(other, list)
                  and all(identity(field, entry) is not None for entry in [*known, *other]))
        for index, item in enumerate(entries):
            value = identity(field, item)
            if value is None:
                valid = False
            elif usable and any(identity(field, entry) == value for entry in other):
                valid = False
            elif complete and usable:
                valid = any(identity(field, entry) == value for entry in known)
            else:
                valid = None
            add(f'{field}-{index}-association', valid,
                'Each returned value needs association evidence; complete Account pair is compared',
                {'value': item, 'inventory_revision': inventory.get('revision') if trusted else None})
    return checks
