#!/usr/bin/env python3
"""Generate the pinned Unicode 3.2 tables used by the RFC4518 provider.

Normalization uses Python's explicit ucd_3_2_0 database. Case folding is read
from RFC3454 B.2, not str.lower()/casefold(), whose Unicode version can change.
"""
import argparse
import hashlib
import gzip
import json
from pathlib import Path
import re
import stringprep
import unicodedata

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'spec/conformance/reference/rfc3454.txt'
SOURCE_SHA = 'eb722fa698fb7e8823b835d9fd263e4cdb8f1c7b0d234edf7f0e3bd2ccbb2c79'
UCD = unicodedata.ucd_3_2_0
OUTPUTS = ('vendor/pkix-path/src/rfc4518/unicode32.rs',
           'tests/conformance/fixtures/pkix-unicode32.json.gz')


def locked_outputs_present():
    """Check the two build prerequisites without expanding all LRS cases."""
    lock = json.loads((ROOT/'spec/conformance/lrs-generated-artifacts.lock.json').read_text())
    for name in OUTPUTS:
        expected = lock['outputs'][name]
        path = ROOT/name
        if not path.is_file():
            return False
        data = path.read_bytes()
        if len(data) != expected['bytes'] or hashlib.sha256(data).hexdigest() != expected['sha256']:
            return False
    return True


def prepared(text, folding):
    """Reference using the independently implemented Unicode 3.2 normalizer."""
    mapped = []
    spaces = {9,10,11,12,13,0x85,0x20,0xa0,0x1680,0x2028,0x2029,0x202f,0x205f,0x3000} | set(range(0x2000,0x200b))
    removed = {0xad,0x1806,0x34f,0xfffc,0x6dd,0x70f,0x180e,0xfeff,0xe0001}
    for first,last in [(0x180b,0x180d),(0xfe00,0xfe0f),(0,8),(14,31),(0x7f,0x84),(0x86,0x9f),(0x200b,0x200f),(0x202a,0x202e),(0x2060,0x2063),(0x206a,0x206f),(0xfff9,0xfffb),(0x1d173,0x1d17a),(0xe0020,0xe007f)]:
        removed.update(range(first,last+1))
    for char in text:
        code = ord(char)
        if code in removed:
            continue
        code = 32 if code in spaces else code
        mapped.extend(map(chr, folding.get(code, [code])))
    value = UCD.normalize('NFKC', ''.join(mapped))
    predicates = (stringprep.in_table_a1,stringprep.in_table_c8,stringprep.in_table_c3,stringprep.in_table_c4,stringprep.in_table_c5)
    if any(char == '\ufffd' or any(predicate(char) for predicate in predicates) for char in value):
        return None
    # Tokenize SPACE+following marks as significant data before collapsing gaps.
    tokens = []
    for index,char in enumerate(value):
        insignificant = char == ' ' and not (index+1 < len(value) and UCD.category(value[index+1]).startswith('M'))
        tokens.append(None if insignificant else char)
    while tokens and tokens[0] is None: tokens.pop(0)
    while tokens and tokens[-1] is None: tokens.pop()
    output = ' '
    in_gap = False
    for token in tokens:
        if token is None:
            if not in_gap: output += '  '
            in_gap = True
        else:
            output += token
            in_gap = False
    return output + ' '


def ranges(points):
    result = []
    for point in points:
        if result and result[-1][1] + 1 == point:
            result[-1][1] = point
        else:
            result.append([point, point])
    return result


def generate():
    source = SOURCE.read_bytes()
    assert hashlib.sha256(source).hexdigest() == SOURCE_SHA
    assert UCD.unidata_version == stringprep.unicodedata.unidata_version == '3.2.0'
    section = source.decode().rsplit('B.2 Mapping for case-folding used with NFKC', 1)[1].split('B.3 Mapping', 1)[0]
    folds = [(int(a, 16), [int(v, 16) for v in b.split()]) for a, b in re.findall(r'^\s+([0-9A-F]{4,6}); ([0-9A-F ]+);', section, re.M)]
    assert len(folds) == 1371
    decompositions, canonical_decompositions, compositions, combining, marks, prohibited = [], [], [], [], [], []
    nameprep_tables = {name: [] for name in ('B1', 'NAMEPREP_PROHIBITED', 'RANDAL', 'LCAT')}
    for point in range(0x110000):
        char = chr(point)
        normalized = UCD.normalize('NFKD', char)
        if normalized != char and not 0xAC00 <= point < 0xD7A4:
            decompositions.append((point, [ord(c) for c in normalized]))
        canonical = UCD.normalize('NFD', char)
        if canonical != char and not 0xAC00 <= point < 0xD7A4:
            canonical_decompositions.append((point, [ord(c) for c in canonical]))
        if stringprep.in_table_b1(char): nameprep_tables['B1'].append(point)
        if stringprep.in_table_d1(char): nameprep_tables['RANDAL'].append(point)
        if stringprep.in_table_d2(char): nameprep_tables['LCAT'].append(point)
        if any(p(char) for p in (stringprep.in_table_a1,stringprep.in_table_c12,
                stringprep.in_table_c22,stringprep.in_table_c3,stringprep.in_table_c4,
                stringprep.in_table_c5,stringprep.in_table_c6,stringprep.in_table_c7,
                stringprep.in_table_c8,stringprep.in_table_c9)):
            nameprep_tables['NAMEPREP_PROHIBITED'].append(point)
        decomposition = UCD.decomposition(char)
        if decomposition and not decomposition.startswith('<'):
            pair = [int(c, 16) for c in decomposition.split()]
            if len(pair) == 2 and UCD.normalize('NFC', ''.join(map(chr, pair))) == char:
                compositions.append((pair, point))
        if UCD.combining(char):
            combining.append((point, UCD.combining(char)))
        if UCD.category(char).startswith('M'):
            marks.append(point)
        if point == 0xFFFD or any(predicate(char) for predicate in (stringprep.in_table_a1, stringprep.in_table_c8, stringprep.in_table_c3, stringprep.in_table_c4, stringprep.in_table_c5)):
            prohibited.append(point)
    output = ['// Generated by scripts/ci/generate_pkix_unicode32.py; do not edit.',
              '// Unicode 3.2.0; RFC3454 SHA256: ' + SOURCE_SHA]
    for name, rows in [('FOLD', folds), ('DECOMPOSE', decompositions), ('CANONICAL_DECOMPOSE', canonical_decompositions)]:
        output += ['#[rustfmt::skip]', f'pub(super) const {name}: &[(u32, &[u32])] = &[']
        output += [f'    (0x{cp:X}, &[{", ".join(f"0x{v:X}" for v in sequence)}]),' for cp, sequence in rows]
        output += ['];']
    output += ['#[rustfmt::skip]', 'pub(super) const COMPOSE: &[((u32, u32), u32)] = &[']
    output += [f'    ((0x{pair[0]:X}, 0x{pair[1]:X}), 0x{cp:X}),' for pair, cp in sorted(compositions)]
    output += ['];', '#[rustfmt::skip]', 'pub(super) const COMBINING: &[(u32, u8)] = &[']
    output += [f'    (0x{cp:X}, {ccc}),' for cp, ccc in combining]
    output += ['];']
    for name, points in [('MARKS', marks), ('PROHIBITED', prohibited), *nameprep_tables.items()]:
        output += ['#[rustfmt::skip]', f'pub(super) const {name}: &[(u32, u32)] = &[']
        output += [f'    (0x{start:X}, 0x{end:X}),' for start, end in ranges(points)]
        output += ['];']
    target = ROOT / 'vendor/pkix-path/src/rfc4518/unicode32.rs'
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text('\n'.join(output) + '\n')
    print(f'{target.relative_to(ROOT)}: {len(folds)} folds, {len(decompositions)} decompositions, {len(compositions)} compositions')
    normalization = gzip.decompress((ROOT/'spec/conformance/reference/unicode-3.2-normalization-test.txt.gz').read_bytes())
    assert hashlib.sha256(normalization).hexdigest() == 'c4513869bb7098d19838be4a1fd5d760843c5804bfe03bd6bbb20623ceb6e57d'
    folding = dict(folds)
    vectors = []
    for index,line in enumerate(normalization.decode().splitlines(), 1):
        content = line.split('#',1)[0].strip()
        if not content or content.startswith('@'): continue
        columns = [''.join(chr(int(point,16)) for point in column.split()) for column in content.split(';')[:5]]
        for column in range(5):
            assert UCD.normalize('NFKC',columns[column]) == columns[3], f'Unicode3.2 provider mismatch at line {index}, column {column+1}'
            nfc = columns[1] if column < 3 else columns[3]
            assert UCD.normalize('NFC',columns[column]) == nfc
            vectors.append({'source_line':index,'column':column+1,'input':columns[column],
                            'nfkc':columns[3],'nfc':nfc,'prepared':prepared(columns[column],folding)})
    for point in (0xfffd,0xe000,0x1f600,0x1e9e,0x2f868,0x2136a,0x36fc):
        vectors.append({'boundary':f'U+{point:X}','input':chr(point),'nfkc':UCD.normalize('NFKC',chr(point)),
                        'nfc':UCD.normalize('NFC',chr(point)), 'prepared':prepared(chr(point),folding)})
    target = ROOT/'tests/conformance/fixtures/pkix-unicode32.json.gz'
    encoded = (json.dumps(vectors, ensure_ascii=False, separators=(',',':'))+'\n').encode()
    target.write_bytes(gzip.compress(encoded,mtime=0))
    print(f'{target.relative_to(ROOT)}: {len(vectors)} reference preparation vectors')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--if-needed', action='store_true',
                        help='materialize missing/stale build inputs and require locked output identities')
    args = parser.parse_args()
    if not args.if_needed or not locked_outputs_present():
        generate()
    if args.if_needed and not locked_outputs_present():
        raise SystemExit('Unicode outputs differ from the artifact lock; review and refresh the lock')
