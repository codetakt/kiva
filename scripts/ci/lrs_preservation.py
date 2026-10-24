"""Independent, schema-scoped oracles for issued xAPI preservation allowances.

These compare values; they do not certify the runtime's ISO 8601/IEEE 754
implementation. Extension members never acquire semantics from their names.
"""

import datetime as dt
from decimal import Decimal, localcontext
import re
import struct
import math
import json
import ipaddress


def iri_component(value, extra='', private=False):
    """RFC3987 2.2 iunreserved / pct-encoded / sub-delims plus component delimiters."""
    if re.search(r'%(?![0-9A-Fa-f]{2})',value):
        return False
    for char in re.sub(r'%[0-9A-Fa-f]{2}','',value):
        code=ord(char)
        if char in "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~!$&'()*+,;="+extra:
            continue
        if (0xA0<=code<=0xD7FF or 0xF900<=code<=0xFDCF or 0xFDF0<=code<=0xFFEF
                or 0x10000<=code<=0xDFFFD and (code&0xFFFF)<=0xFFFD
                or 0xE1000<=code<=0xEFFFD):
            continue
        if private and (0xE000<=code<=0xF8FF or 0xF0000<=code<=0xFFFFD or 0x100000<=code<=0x10FFFD):
            continue
        return False
    return True


def iri_reference_valid(value):
    """Validate raw RFC3987 2.2 syntax before urllib strips or normalizes bytes.

    Scheme-specific routing, origin authorization and transport support remain
    separate capabilities; a valid reference need not be reachable by this host.
    """
    if not isinstance(value,str):
        return False
    hierarchy,_,fragment=value.partition('#')
    hierarchy,_,query=hierarchy.partition('?')
    if not iri_component(fragment,':@/?') or not iri_component(query,':@/?',private=True):
        return False
    if ':' in hierarchy.split('/',1)[0]:
        scheme,_,hierarchy=hierarchy.partition(':')
        if not re.fullmatch(r'[A-Za-z][A-Za-z0-9+.-]*',scheme):
            return False
    if hierarchy.startswith('//'):
        authority,slash,path=hierarchy[2:].partition('/')
        hierarchy=slash+path
        user,separator,host=authority.rpartition('@')
        if not separator:
            host=authority
        elif not iri_component(user,':'):
            return False
        if host.startswith('['):
            literal=re.fullmatch(r'\[([^\]]+)\](?::[0-9]*)?',host)
            if literal is None:
                return False
            address=literal[1]
            if not re.fullmatch(r"[vV][0-9A-Fa-f]+\.[A-Za-z0-9._~!$&'()*+,;=:\-]+",address):
                if '%' in address:
                    return False
                try:ipaddress.IPv6Address(address)
                except ValueError:return False
        else:
            name,separator,port=host.partition(':')
            if not iri_component(name) or separator and not re.fullmatch(r'[0-9]*',port):
                return False
    return iri_component(hierarchy,':@/')


class MemberObject(dict):
    """Read-only response view with the complete decoded member sequence.

    Ordinary consumers can inspect the mapping; preservation must use members()
    so the mapping's last-value view never determines duplicate equivalence.
    """
    def __init__(self, pairs):
        super().__init__(pairs)
        self.pairs = tuple(pairs)


def load_json_members(encoded):
    return json.loads(encoded, object_pairs_hook=MemberObject)


def members(value):
    return value.pairs if isinstance(value, MemberObject) else tuple(value.items())


def unique_members(value):
    return len(members(value)) == len(value)


class PrimitiveUnavailable(ValueError):
    """A defined delegated relation needs a provider beyond this bounded adapter."""


def _check_timestamp_components(value):
    """Reject known calendar/full-or-reduced clock defects before delegation."""
    date, clock = re.split('[Tt ]', value, maxsplit=1)
    if not re.fullmatch(r'(?:[0-9]{8}|[0-9]{4}-[0-9]{2}-[0-9]{2})', date):
        return  # Expanded years, week/ordinal dates still need their provider.
    date = date.replace('-', '')
    year, month, day = int(date[:4]), int(date[4:6]), int(date[6:])
    if not 1 <= month <= 12:
        raise ValueError('invalid calendar/time component')
    leap = year % 4 == 0 and (year % 100 != 0 or year % 400 == 0)
    days = (31, 29 if leap else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)
    if not 1 <= day <= days[month - 1]:
        raise ValueError('invalid calendar day')
    zone_start = re.search('[Zz+-]', clock)
    local, zone = (clock[:zone_start.start()], clock[zone_start.start():]) if zone_start else (clock, '')
    if re.fullmatch(r'[+-][0-9]{2}(?::?[0-9]{2})?', zone):
        offset = zone[1:].replace(':', '')
        if int(offset[:2]) > 23 or (len(offset) > 2 and int(offset[2:]) > 59):
            raise ValueError('invalid timezone offset component')
    parts = re.split('[.,]', local, maxsplit=1)
    if not re.fullmatch(r'(?:[0-9]{2}(?::[0-9]{2}){0,2}|[0-9]{4}|[0-9]{6})', parts[0]):
        return
    digits = parts[0].replace(':', '')
    hour, minute, second = int(digits[:2]), int(digits[2:4] or '0'), int(digits[4:] or '0')
    if minute > 59 or second > 60 or hour > 24:
        raise ValueError('invalid calendar/time component')
    fraction = parts[1] if len(parts) > 1 else ''
    if hour == 24 and (minute or second or any(digit != '0' for digit in fraction)):
        raise ValueError('end-of-day requires zero smaller components')


def timestamp_ns(value):
    if not isinstance(value, str) or not re.search(r'[Tt ]\d{2}', value):
        raise ValueError('combined date and time required')
    _check_timestamp_components(value)
    # A bounded standard-library adapter, not the definition of ISO validity.
    # Retain fractional digits outside datetime's microsecond precision.
    match = re.fullmatch(r'(\d{4}-?\d\d-?\d\d[Tt ]\d\d:?\d\d:?\d\d)(?:[.,](\d+))?([Zz]|[+-]\d\d(?::?\d\d)?)', value)
    if not match:
        raise PrimitiveUnavailable('ISO combined-date/time provider required for this representation')
    components = re.sub(r'[-:Tt ]', '', match[1])
    year, hour, second = int(components[:4]), int(components[8:10]), int(components[12:14])
    if hour == 24 or second == 60 or year == 0:
        raise PrimitiveUnavailable('ISO end-of-day, leap-second or expanded calendar provider required')
    try:
        instant = dt.datetime.fromisoformat(match[1] + match[3].upper().replace('Z', '+00:00'))
    except ValueError as error:
        # Unsupported spellings remain delegated after definite component
        # violations have been excluded; this is not complete ISO validation.
        raise PrimitiveUnavailable('ISO calendar/time provider required') from error
    epoch = instant - dt.datetime(1970, 1, 1, tzinfo=dt.timezone.utc)
    with localcontext() as context:
        context.prec = max(28, len(match[2] or '') + 30)
        return Decimal(epoch.days * 86400 + epoch.seconds) + Decimal('0.' + (match[2] or '0'))


def timestamp_pair(left, right):
    """Parse both operands; definite defects win over an unavailable peer."""
    instants = []
    for value in (left, right):
        try:
            instants.append(timestamp_ns(value))
        except PrimitiveUnavailable:
            instants.append(None)
    return instants


def timestamp_preserved(sent, got):
    """Exact instant, truncation or nearest rounding at >= 3 decimal digits."""
    try:
        if not isinstance(got, str):
            return False
        # The UTC duty is independent of calendar/instant conversion, including
        # legal end-of-day and leap-second forms delegated by this adapter.
        clock = re.split('[Tt ]', got)[-1]
        offset = re.search(r'([+-])([0-9]{2}(?::?[0-9]{2})?)$', clock)
        if offset and (offset[1] == '-' or any(d != '0' for d in offset[2].replace(':', ''))):
            return False
        # Defer unsupported representations independently, so either operand's
        # definite syntax/calendar defect still wins over provider uncertainty.
        instants = timestamp_pair(sent, got)
        if None in instants:
            return None
        before, after = instants
        if not got.endswith(('Z', 'z', '+00:00', '+0000', '+00')):
            return False
        if before == after:
            return True
        # Work from the nonnegative fractional second, including before 1970.
        with localcontext() as context:
            context.prec = max(28, len(sent) + len(got) + 30)
            for digits in range(3, max(3, -before.as_tuple().exponent) + 1):
                quantum = Decimal(1).scaleb(-digits)
                floor = before.quantize(quantum, rounding='ROUND_FLOOR')
                remainder = before - floor
                if after == floor or (remainder >= quantum / 2 and after == floor + quantum):
                    return True
        return False
    except (ValueError, ArithmeticError, TypeError):
        return False


def statement_response_headers(headers):
    """Shared per-response contract, including Statement resource error replies."""
    lowered = {name.lower(): value for name, value in headers.items()}
    version = lowered.get('x-experience-api-version') == '2.0.0'
    try:
        value = lowered['x-experience-api-consistent-through']
        timestamp_ns(value)
        consistent = True
    except PrimitiveUnavailable:
        consistent = None
    except (KeyError, TypeError, ValueError, ArithmeticError):
        consistent = False
    return version, consistent


def binary32_decimal_spellings(value):
    """Decimal round-trip outputs of a known finite binary32 endpoint.

    Include every significant precision, not just the shortest spelling or
    nine digits. Directed decimal rounding brackets every nearest/tie choice.
    Once the exact terminating expansion is reached, additional digits only
    add zeros. Round-trip filtering excludes output with insufficient precision;
    sharing a rounding bin alone does not establish a valid serialization.
    """
    bits = struct.pack('!f', value)
    exact = Decimal.from_float(value)
    spellings = {exact, Decimal(repr(value))}
    with localcontext() as context:
        # Formatting the exact Decimal also avoids binary64 pre-rounding and
        # makes the result independent of the ambient Decimal rounding mode.
        for rounding in ('ROUND_FLOOR', 'ROUND_CEILING'):
            context.rounding = rounding
            for digits in range(1, len(exact.as_tuple().digits) + 1):
                spelling = format(exact, f'.{digits}g')
                try:
                    candidate = struct.pack('!f', float(spelling))
                except OverflowError:
                    # Coarse output can overflow the maximum finite endpoint.
                    continue
                if candidate == bits:
                    spellings.add(Decimal(spelling))
    return spellings


def number_preserved(sent, got):
    """Exact value or the enclosing binary32 interval, without relative epsilon.

    The interval admits directed as well as nearest rounding and higher
    precision results, including decimal serializations of either binary32
    endpoint at any significant precision that round-trips, directly or widened
    to binary64. It does
    not round both values and equate their bins.
    Outside this finite adapter's range, request the delegated numeric oracle.
    """
    if type(sent) not in (int, float, Decimal) or type(got) not in (int, float, Decimal):
        return False
    # json.loads/YAML float parsing may already have erased decimal digits.
    # Equality of those floats is not evidence of the lossless relation.
    if type(sent) is float or type(got) is float:
        if any(type(value) is float and not math.isfinite(value) for value in (sent, got)):
            return False
        return None
    left, right = Decimal(str(sent)), Decimal(str(got))
    if not left.is_finite() or not right.is_finite():
        return False
    if left == right:
        return True
    try:
        rounded = struct.unpack('!f', struct.pack('!f', float(left)))[0]
        if not math.isfinite(rounded):
            return None
        exact = Decimal.from_float(rounded)
        if exact == left:
            return right in binary32_decimal_spellings(rounded)
        bits = struct.unpack('!I', struct.pack('!f', rounded))[0]
        upward = exact < left
        if rounded == 0:
            adjacent_bits = 1 if upward else 0x80000001
        else:
            adjacent_bits = bits + (1 if (rounded > 0) == upward else -1)
        adjacent = struct.unpack('!f', struct.pack('!I', adjacent_bits))[0]
        if not math.isfinite(adjacent):
            return None
        bound = Decimal.from_float(adjacent)
        return (min(exact, bound) <= right <= max(exact, bound)
                or right in binary32_decimal_spellings(rounded)
                or right in binary32_decimal_spellings(adjacent))
    except (OverflowError, struct.error, ValueError):
        return None


def duration_lexically_valid(value):
    """ISO 8601:2004 4.4.3.2 designators, without calendar conversion."""
    if not isinstance(value, str):
        return False
    number = r'[0-9]+(?:[.,][0-9]+)?'
    if re.fullmatch('P' + number + 'W', value):
        return True
    pattern = 'P' + ''.join('(' + number + unit + ')?' for unit in 'YMD')
    pattern += '(?:T(?=[0-9])' + ''.join('(' + number + unit + ')?' for unit in 'HMS') + ')?'
    match = re.fullmatch(pattern, value)
    if not match:
        return False
    components = [part for part in match.groups() if part is not None]
    return bool(components) and not any('.' in part or ',' in part for part in components[:-1])


def duration_preserved(sent, got):
    """Recognize exact duration and permitted truncation of fractional seconds.

    This intentionally does not convert calendar months or years to seconds.
    Seconds spelling is compared without rounding. Other equivalent calendar
    representations require the delegated ISO duration oracle.
    """
    if not isinstance(sent, str) or not isinstance(got, str):
        return False
    if not all(duration_lexically_valid(value) for value in (sent, got)):
        return False
    if sent == got:
        return True
    pattern = r'(P(?:\d+(?:[.,]\d+)?[YMD])*T(?:\d+(?:[.,]\d+)?[HM])*)(\d+)(?:[.,](\d+))?S'
    before, after = re.fullmatch(pattern, sent), re.fullmatch(pattern, got)
    if not before or not after or before[1] != after[1]:
        return None
    seconds, fraction = before[2].lstrip('0'), (before[3] or '').rstrip('0')
    other_seconds, other_fraction = after[2].lstrip('0'), (after[3] or '').rstrip('0')
    return seconds == other_seconds and other_fraction in (fraction, fraction[:2].rstrip('0'))


def all_decisions(values):
    """False dominates unknown; consume all independently decidable operands."""
    values = list(values)
    return False if False in values else None if None in values else True


def multiset_equal(left, right, compare):
    if len(left) != len(right):
        return False
    edges = [[compare(a, b) for b in right] for a in left]
    def perfect(possible):
        owners = [None] * len(right)
        def augment(row, seen):
            for col, value in enumerate(edges[row]):
                if col in seen or not (value is True or possible and value is None):
                    continue
                seen.add(col)
                if owners[col] is None or augment(owners[col], seen):
                    owners[col] = row
                    return True
            return False
        return all(augment(row, set()) for row in range(len(left)))
    return True if perfect(False) else None if perfect(True) else False


def agent_structure(value, member=False):
    """Structural identifying fields; lexical IFI validation is a separate duty."""
    if not isinstance(value, dict):
        return False
    kind = value.get('objectType', 'Agent')
    if kind not in ('Agent', 'Group') or member and kind != 'Agent':
        return False
    keys = [k for k in ('mbox', 'mbox_sha1sum', 'openid', 'account') if k in value]
    if len(keys) > 1 or kind == 'Agent' and len(keys) != 1:
        return False
    if keys:
        key = keys[0]; item = value[key]
        if key == 'account':
            if not isinstance(item, dict) or not all(isinstance(item.get(k), str) for k in ('homePage', 'name')):
                return False
        elif not isinstance(item, str):
            return False
    if 'name' in value and not isinstance(value['name'], str):
        return False
    if kind == 'Group':
        members = value.get('member')
        if not keys and (not isinstance(members, list)):
            return False
        if 'member' in value and (not isinstance(members, list) or not all(agent_structure(v, True) for v in members)):
            return False
    return True


def identifier_equal(left, right, field):
    if left == right:
        return True
    if field == 'mbox_sha1sum':
        return left.lower() == right.lower()
    if field == 'mbox':
        # Only unescaped single mailboxes are decided by this adapter. The
        # full RFC3987/DC-MAILTO relation remains required outside this subset.
        simple = r"(?i:mailto):([A-Za-z0-9.!$'*+_~-]+)@([A-Za-z0-9.-]+)"
        a, b = re.fullmatch(simple, left), re.fullmatch(simple, right)
        if a and b:
            return a[1] == b[1] and a[2].lower() == b[2].lower()
    return None


def _agent_identity_core(left, right):
    if not agent_structure(left) or not agent_structure(right):
        return False
    if left.get('objectType', 'Agent') != right.get('objectType', 'Agent'):
        return False
    keys = ('mbox', 'mbox_sha1sum', 'openid', 'account')
    a, b = next((k for k in keys if k in left), None), next((k for k in keys if k in right), None)
    if a != b:
        return False
    if a is None:
        return multiset_equal(left['member'], right['member'], agent_identity_equal)
    if a == 'account':
        return all_decisions([left[a]['name'] == right[a]['name'], identifier_equal(left[a]['homePage'], right[a]['homePage'], 'homePage')])
    return identifier_equal(left[a], right[a], a)


def agent_identity_equal(left, right):
    """Bounded authority relation, not the signature exclusion mask.

    Identity differences known by this adapter fail. Equivalent Group member
    order succeeds. Other changing display facts require the full declared
    deterministic mapping, rather than being silently ignored or rejected.
    """
    identity = _agent_identity_core(left, right)
    if identity is False:
        return False
    ignored = {'objectType', 'mbox', 'mbox_sha1sum', 'openid', 'account', 'member'}
    facts = {k:v for k,v in left.items() if k not in ignored}
    other = {k:v for k,v in right.items() if k not in ignored}
    extra = True if facts == other else None
    if 'member' in left and 'member' in right:
        members = multiset_equal(left['member'], right['member'], agent_identity_equal)
        # An identified Group's display membership is not its IFI. A changed
        # membership needs the mapping contract; order alone is fully decided.
        if members is False and any(k in left for k in ('mbox','mbox_sha1sum','openid','account')):
            members = None
    else:
        members = True if ('member' in left) == ('member' in right) else None
    return all_decisions([identity, extra, members])


def snapshot_equal(left, right, pointer='', opaque=False):
    """Same stored value, not input preservation or signature equivalence.

    Includes managed fields, every property and array multiplicity; permits
    only equivalent timestamp/duration spelling and designated unordered arrays.
    No signature exclusion mask or a second storage-rounding allowance is used.
    """
    if not opaque and pointer in ('/stored', '/timestamp', '/object/timestamp'):
        try:
            before, after = timestamp_pair(left, right)
            return None if before is None or after is None else before == after
        except (TypeError, ValueError, ArithmeticError):
            return False
    if not opaque and pointer in ('/result/duration', '/object/result/duration'):
        return all_decisions([duration_preserved(left, right), duration_preserved(right, left)])
    if isinstance(left, dict) and isinstance(right, dict):
        if opaque:
            return multiset_equal(members(left), members(right), lambda a,b:
                all_decisions([a[0] == b[0], snapshot_equal(a[1], b[1], pointer, True)]))
        return all_decisions([left.keys() == right.keys(), *[
            snapshot_equal(v, right[k], pointer+'/'+k.replace('~','~0').replace('/','~1'), pointer.endswith('/extensions'))
            for k,v in left.items() if k in right], unique_members(left), unique_members(right)])
    if isinstance(left, list) and isinstance(right, list):
        if not opaque and (pointer.endswith('/member') or pointer in ('/attachments', '/object/attachments')):
            return multiset_equal(left, right, lambda a,b:snapshot_equal(a,b,pointer+'/0',opaque))
        return all_decisions([len(left)==len(right), *[snapshot_equal(a,b,pointer+'/'+str(i),opaque) for i,(a,b) in enumerate(zip(left,right))]])
    if type(left) is float or type(right) is float:
        # Delegate only a number/number relation. Lost decimal tokens cannot
        # conceal a definite change to another JSON type (including boolean).
        numeric = (int, float, Decimal)
        return None if type(left) in numeric and type(right) in numeric else False
    return type(left) is type(right) and left == right


def added_value_valid(value, key):
    """Validate population without inventing a default instant or locator proof."""
    if key == 'timestamp':
        return timestamp_preserved(value, value)
    if key == 'fileUrl':
        if (not isinstance(value, str) or not re.match(r'[A-Za-z][A-Za-z0-9+.-]*:', value)
                or not iri_reference_valid(value)):
            return False
        # Like the Rust shape adapter: absolute IRI syntax is necessary, but
        # DC-IRL locator/access-mechanism evidence still requires its provider.
        return None
    # Root id/stored/authority/version retain their independent managed checks.
    return True


def preservation(sent, got, pointer=''):
    observations = []

    def record(ok, reason):
        observations.append(dict(pointer=pointer, outcome='not-run' if ok is None else 'passed' if ok else 'failed', rule=reason, submitted=sent, returned=got))

    opaque = '/extensions/' in pointer
    if pointer in ('/authority', '/stored'):
        record(True, 'LRS-managed value; independent authority/stored checks required')
    elif not opaque and re.fullmatch(r'(?:/object)?/timestamp', pointer) and isinstance(sent, str) and isinstance(got, str):
        record(timestamp_preserved(sent, got), 'UTC conversion and exact permitted rounding/truncation')
    elif not opaque and re.fullmatch(r'(?:/object)?/result/duration', pointer) and isinstance(sent, str) and isinstance(got, str):
        record(duration_preserved(sent, got), 'duration unchanged or fractional-second truncation; other forms need delegated oracle')
    elif isinstance(sent, dict):
        if not opaque and re.fullmatch(r'(?:/object)?/context/contextActivities/(parent|grouping|category|other)', pointer) and isinstance(got, list):
            if len(got) == 1:
                return preservation(sent, got[0], pointer + '/0')
            record(False, 'contextActivities singleton expands to one element')
        elif not isinstance(got, dict):
            record(False, 'object type preserved')
        elif opaque and (not unique_members(sent) or not unique_members(got)):
            def equal_pair(a, b):
                if a[0] != b[0]:
                    return False
                checks = preservation(a[1], b[1], pointer + '/' + a[0].replace('~','~0').replace('/','~1'))
                return all_decisions([None if o['outcome']=='not-run' else o['outcome']=='passed' for o in checks])
            record(multiset_equal(members(sent), members(got), equal_pair),
                   'Extension decoded member multiset with multiplicity preserved')
        else:
            if not unique_members(sent) or not unique_members(got):
                record(False, 'Duplicate core or extension-map property')
            for key, value in sent.items():
                sub = pointer + '/' + key.replace('~', '~0').replace('/', '~1')
                if key not in got:
                    observations.append(dict(pointer=sub, outcome='failed', rule='submitted property missing', submitted=value))
                else:
                    observations.extend(preservation(value, got[key], sub))
            for key in got.keys() - sent.keys():
                # Population/transmission exceptions do not waive the added
                # value's type or format. Implicit object types do not permit
                # changing an exact serialization's property set.
                allowed = (pointer == '' and key in {'id', 'stored', 'authority', 'timestamp', 'version'}
                           or pointer == '/object' and sent.get('objectType') == 'SubStatement' and key == 'timestamp'
                           or re.fullmatch(r'(?:/object)?/attachments/[0-9]+', pointer) and key == 'fileUrl')
                if opaque or not allowed:
                    observations.append(dict(pointer=pointer + '/' + key.replace('~', '~0').replace('/', '~1'),
                                             outcome='failed', rule='unexpected returned property', returned=got[key]))
                else:
                    decision = added_value_valid(got[key], key)
                    observations.append(dict(pointer=pointer + '/' + key.replace('~', '~0').replace('/', '~1'),
                        outcome='not-run' if decision is None else 'passed' if decision else 'failed',
                        rule='Added timestamp UTC/type/format or fileUrl IRL contract; locator/provider uncertainty remains NOT-RUN',
                        returned=got[key]))
    elif isinstance(sent, list):
        if not isinstance(got, list) or len(sent) != len(got):
            record(False, 'array type and cardinality preserved')
        elif not opaque and (pointer.endswith('/member') or pointer in ('/attachments', '/object/attachments')):
            candidates = [[preservation(value, item, pointer + '/' + str(index)) for item in got] for index, value in enumerate(sent)]
            def perfect(allow_unknown):
                owners = [None] * len(got)
                def augment(row, seen):
                    for col, checks in enumerate(candidates[row]):
                        eligible = all(o['outcome'] != 'failed' if allow_unknown else o['outcome'] == 'passed' for o in checks)
                        if not eligible or col in seen:
                            continue
                        seen.add(col)
                        if owners[col] is None or augment(owners[col], seen):
                            owners[col] = row
                            return True
                    return False
                return all(augment(row, set()) for row in range(len(sent)))
            record(True if perfect(False) else None if perfect(True) else False,
                   'Complete metadata/member multiset with multiplicity; only explicitly designated arrays are unordered')
        else:
            for index, value in enumerate(sent):
                observations.extend(preservation(value, got[index], pointer + '/' + str(index)))
    else:
        numeric = type(sent) in (int, float, Decimal) or type(got) in (int, float, Decimal)
        record(number_preserved(sent, got) if numeric else type(sent) is type(got) and sent == got, 'scalar preserved with binary32 minimum precision; unsupported primitive is NOT-RUN')
    return observations
