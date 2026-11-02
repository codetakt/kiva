"""RFC6068 recipient grammar. The caller supplies a validated RFC3987 IRI.

URI delimiter recognition precedes percent decoding. UTF8 is decoded once;
quoted mailbox data and hfields do not acquire SMTP/composer obligations.
"""
import re
from urllib.parse import unquote_to_bytes


def component(value, address):
    permitted = "-._~!$'()*+,:@" + ('' if address else ';')
    if re.search(r'%(?![0-9a-fA-F]{2})', value):
        return None
    raw = re.sub(r'%[0-9a-fA-F]{2}', '', value)
    if any(char.isascii() and not (char.isalnum() or char in permitted) for char in raw):
        return None
    try:
        return unquote_to_bytes(value).decode('utf-8')
    except UnicodeDecodeError:
        return None


def dot_atom(value):
    return all(part and all(not char.isascii() or char.isalnum() or char in "!#$%&'*+-/=?^_`{|}~" for char in part)
               for part in value.split('.'))


def local_part(value):
    if not (value.startswith('"') and value.endswith('"') and len(value) >= 2):
        return dot_atom(value)
    characters = iter(value[1:-1])
    for char in characters:
        if char == '\\':
            escaped = next(characters, '')
            if len(escaped) != 1 or not 32 <= ord(escaped) <= 126:
                return False
        elif char == '"' or ord(char) < 32 or ord(char) == 127:
            return False
    return True


def domain(value):
    if value.startswith('[') and value.endswith(']'):
        return all(33 <= ord(char) <= 90 or 94 <= ord(char) <= 126 for char in value[1:-1])
    return dot_atom(value)


def mailbox(value):
    scheme, separator, rest = value.partition(':')
    if not separator or scheme.lower() != 'mailto' or '#' in rest:
        return False
    addresses, separator, fields = rest.partition('?')
    if not addresses:
        return False
    for address in addresses.split(','):
        local, sep, host = address.rpartition('@')
        if not sep:
            return False
        local, host = component(local, True), component(host, True)
        if local is None or host is None or not local_part(local) or not domain(host):
            return False
    if separator:
        for field in fields.split('&'):
            name, sep, content = field.partition('=')
            if not sep or component(name, False) is None or component(content, False) is None:
                return False
    return True
