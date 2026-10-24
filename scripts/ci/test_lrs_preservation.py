#!/usr/bin/env python3
"""Conforming and single-defect controls for preservation oracles."""
import copy
import json
from pathlib import Path
import unittest
from decimal import Decimal, localcontext

from lrs_preservation import (
    PrimitiveUnavailable, preservation, timestamp_ns, timestamp_preserved, number_preserved,
    statement_response_headers, snapshot_equal, duration_preserved, duration_lexically_valid, load_json_members,
    agent_validity, agent_identity_equal,
)


class PreservationControls(unittest.TestCase):
    def test_generated_and_dedicated_authority_formats_agree(self):
        import yaml
        from generate_lrs_projection_cases import actor_output_shape, read_statement
        root=Path(__file__).resolve().parents[2]
        case=yaml.safe_load((root/'tests/conformance/http/statement-authz-authority-isolated.yaml').read_text())['cases'][0]
        shape=actor_output_shape(identifiers=True)
        for position in (1,3):
            self.assertEqual(case['steps'][position]['expect']['checks'][0]['shape'],shape)
        read=read_statement('00000000-0000-4000-8000-000000000001')
        self.assertEqual(read['expect']['checks'][0]['shape']['properties']['authority'],shape)

    def test_authority_identifiers_precede_identity_success_or_deferral(self):
        fixture=json.loads((Path(__file__).resolve().parents[2]/'tests/conformance/fixtures/preservation-properties.json').read_text())['authority_identifiers']
        for row in fixture:
            expected={'passed':True,'failed':False,'not-run':None}[row['outcome']]
            value=row['authority']
            variants=[value]
            if value.get('objectType','Agent')=='Agent':
                variants += [dict(value,objectType='Group'), {'objectType':'Group','member':[value]}]
            for variant in variants:
                with self.subTest(id=row['id'],variant=variant):
                    self.assertIs(agent_validity(variant),expected)
                    self.assertIs(agent_identity_equal(variant,variant),expected)
        unknown={'mbox':'mailto:%22not%20me%22@example.org'}
        invalid={'mbox_sha1sum':'xyz'}
        self.assertIs(agent_identity_equal(unknown,invalid),False)
        self.assertIs(agent_identity_equal(invalid,unknown),False)

    def test_duration_designator_grammar_shared_controls(self):
        fixture=json.loads((Path(__file__).resolve().parents[2]/'tests/conformance/fixtures/preservation-properties.json').read_text())['duration_grammar']
        for group,values in fixture.items():
            valid=group=='valid'
            for value in values:
                with self.subTest(value=value):
                    self.assertIs(duration_lexically_valid(value),valid)
                    self.assertIs(duration_preserved(value,value),valid)
                    if not valid:
                        for other in ['PT1S','P1M']:
                            for before,after in [(value,other),(other,value)]:
                                for pointer in ['/result/duration','/object/result/duration']:
                                    self.assertEqual(preservation(before,after,pointer)[0]['outcome'],'failed')
                                    self.assertIs(snapshot_equal(before,after,pointer),False)

    def test_extension_member_multiplicity_survives_decoding(self):
        fixture = json.loads((Path(__file__).resolve().parents[2] / 'tests/conformance/fixtures/preservation-properties.json').read_text())
        for row in fixture['duplicate_members']:
            for placement in ['context', 'result']:
                def wrap(raw):
                    return load_json_members('{"' + placement + '":{"extensions":{"urn:x":' + raw + '}}}')
                with self.subTest(id=row['id'], placement=placement):
                    before, after = wrap(row['submitted']), wrap(row['returned'])
                    outcomes = {o['outcome'] for o in preservation(before, after)}
                    actual = 'failed' if 'failed' in outcomes else 'not-run' if 'not-run' in outcomes else 'passed'
                    self.assertEqual(actual, row['outcome'])
                    # Snapshot equality grants no second storage rounding.
                    if row['id'] != 'greedy-matching-counterexample':
                        expected = {'passed':True, 'failed':False, 'not-run':None}[row['outcome']]
                        self.assertIs(snapshot_equal(before, after), expected)

    def test_duplicate_core_and_extension_map_members_are_not_opaque(self):
        for raw in ['{"actor":{},"actor":{}}',
                    '{"context":{"extensions":{"urn:x":1,"urn:x":1}}}']:
            value = load_json_members(raw)
            self.assertIs(snapshot_equal(value, value), False)
            self.assertTrue(any(o['outcome']=='failed' for o in preservation(value, value)))

    def test_duration_lexical_defects_precede_comparison_or_deferral(self):
        for bad in ['PTé1S','PT🙂1S','PT１S','PT1éS','PT1.éS','PT1\x00S','PT1e3S','not-duration',
                    'PT1.S','PT1,S','PT1.H','P1,D','PT1.']:
            for other in ['PT1.239S','P1M',bad]:
                for left,right in [(other,bad),(bad,other)]:
                    with self.subTest(left=left,right=right):
                        self.assertIs(duration_preserved(left,right),False)
                        for pointer in ['/result/duration','/object/result/duration']:
                            self.assertEqual(preservation(left,right,pointer)[0]['outcome'],'failed')
        for left,right in [('PT1.239S','PT1.23S'),('PT1.239S','PT1,23S'),('P1W','P1W'),
                           ('PT1S','PT1.00S'),('PT1S','PT1,00S')]:
            self.assertIs(duration_preserved(left,right),True)
        self.assertIsNone(duration_preserved('P1M','P30D'))

    def test_snapshot_validates_both_timestamp_operands_before_deferral(self):
        unknowns = ['2016-12-31T23:59:60Z', '2026-01-01T24:00:00Z']
        invalid = ['not-a-timestamp', '2026-02-30T00:00:00Z', 42, None]
        for pointer in ['/stored', '/timestamp', '/object/timestamp']:
            for unknown in unknowns:
                for bad in invalid:
                    for left, right in [(unknown, bad), (bad, unknown)]:
                        with self.subTest(pointer=pointer, left=left, right=right):
                            self.assertIs(snapshot_equal(left, right, pointer), False)
                for other in [unknown, '2026-01-02T00:00:00Z']:
                    self.assertIsNone(snapshot_equal(unknown, other, pointer))
                    self.assertIsNone(snapshot_equal(other, unknown, pointer))
            self.assertIs(snapshot_equal('2026-01-01T09:00:00+09:00', '2026-01-01T00:00:00Z', pointer), True)
            self.assertIs(snapshot_equal('2026-01-01T00:00:00Z', '2026-01-01T00:00:01Z', pointer), False)
        # Extension names do not acquire timestamp semantics.
        self.assertIs(snapshot_equal({'extensions': {'urn:x': {'stored': 'opaque'}}},
                                     {'extensions': {'urn:x': {'stored': 'opaque'}}}), True)

    def test_snapshot_keeps_json_type_changes_when_numbers_need_a_provider(self):
        for nonnumber in ['original', True, False, None, {}, []]:
            for left, right in [(nonnumber, 1.5), (1.5, nonnumber)]:
                with self.subTest(left=left, right=right):
                    self.assertIs(snapshot_equal(left, right), False)
                    self.assertIs(snapshot_equal({'definition': {'name': {'en': left}}},
                                                 {'definition': {'name': {'en': right}}}), False)
        for number in [1, 1.5, Decimal('1.5')]:
            self.assertIsNone(snapshot_equal(number, 1.5))
            self.assertIsNone(snapshot_equal(1.5, number))
        self.assertIs(snapshot_equal({'score': 1.5, 'name': 'original'},
                                     {'score': 1.5, 'name': 1.5}), False)

    def test_known_timestamp_defects_win_over_either_unavailable_operand(self):
        unknown = ['2026-01-01T24:00:00Z', '2016-12-31T23:59:60Z']
        invalid = ['2026-02-30T00:00:00Z', '2026-01-01T25:00:00Z',
                   '2026-01-01T00:61:00Z', '2026-01-01T00:00:00+24:00',
                   '2026-01-01', 'invalid']
        for unavailable in unknown:
            for bad in invalid:
                for sent, got in [(unavailable, bad), (bad, unavailable)]:
                    with self.subTest(sent=sent, got=got):
                        self.assertIs(timestamp_preserved(sent, got), False)
                        for pointer in ['/timestamp', '/object/timestamp']:
                            self.assertEqual(preservation(sent, got, pointer)[0]['outcome'], 'failed')
            for valid in ['2026-01-02T00:00:00Z', *unknown]:
                for sent, got in [(unavailable, valid), (valid, unavailable)]:
                    self.assertIsNone(timestamp_preserved(sent, got))

    def test_explicit_non_utc_does_not_need_instant_conversion(self):
        for clock in ['2026-01-01T24:00:00', '20260101T240000', '2016-12-31T23:59:60']:
            for offset in ['+09:00', '+0900', '+09', '-09:00', '-0900', '-09',
                           '-00:00', '-0000', '-00', '+00:00', '+0000', '+00', 'Z']:
                got = clock + offset
                non_utc = offset.startswith('-') or '9' in offset
                with self.subTest(got=got):
                    expected = False if non_utc else None
                    self.assertIs(timestamp_preserved('2026-01-01T00:00:00Z', got), expected)
                    # The input instant can also require a provider; known UTC defects still win.
                    self.assertIs(timestamp_preserved(clock + 'Z', got), expected)
                    for prefix in ['', 'object']:
                        sent, returned = {'timestamp': clock + 'Z'}, {'timestamp': got}
                        if prefix:
                            sent, returned = {prefix: sent}, {prefix: returned}
                        outcomes = preservation(sent, returned)
                        self.assertEqual(outcomes[0]['outcome'], 'failed' if non_utc else 'not-run')

    def test_case_schema_rejects_partial_preservation_expectations(self):
        import jsonschema
        root = Path(__file__).resolve().parents[2]
        schema = json.loads((root / 'spec/conformance/schemas/conformance-case.schema.json').read_text())
        submitted = {'actor': {'mbox': 'mailto:test@example.org'},
                     'verb': {'id': 'https://example.org/verb'},
                     'object': {'id': 'https://example.org/activity'},
                     'result': {'duration': 'PT1S'}}
        # Use the referenced check schema without the suite-level sibling rules.
        validator = jsonschema.Draft202012Validator({'$defs': schema['$defs'], '$ref': '#/$defs/check'})
        validator.validate({'kind': 'json-preservation', 'expected': submitted})
        for key in ['actor', 'verb', 'object']:
            with self.subTest(missing=key), self.assertRaises(jsonschema.ValidationError):
                validator.validate({'kind': 'json-preservation',
                                    'expected': {k: v for k, v in submitted.items() if k != key}})

    def test_exact_property_additions_and_managed_boundaries(self):
        path = Path(__file__).resolve().parents[2] / 'tests/conformance/fixtures/preservation-properties.json'
        for row in json.loads(path.read_text(), parse_float=Decimal)['cases']:
            with self.subTest(id=row['id']):
                observations = preservation(row['submitted'], row['returned'])
                outcome = 'failed' if any(o['outcome']=='failed' for o in observations) else 'not-run' if any(o['outcome']=='not-run' for o in observations) else 'passed'
                self.assertEqual(outcome, 'not-run' if row['preserved'] is None else 'passed' if row['preserved'] else 'failed')
                if row['preserved'] is False:
                    self.assertTrue(any(o['outcome'] == 'failed' for o in observations))

    def test_added_values_retain_type_and_format_contracts(self):
        fixture = json.loads((Path(__file__).resolve().parents[2] / 'tests/conformance/fixtures/preservation-properties.json').read_text())
        for row in fixture['added_values']:
            for nested in [False, True]:
                with self.subTest(id=row['id'], nested=nested):
                    sent = copy.deepcopy(fixture['cases'][0]['submitted'])
                    if nested:
                        sent['object'] = dict(copy.deepcopy(sent), objectType='SubStatement')
                    parent = sent['object'] if nested else sent
                    if row['key'] == 'fileUrl':
                        parent['attachments'] = [{'sha2':'a'*64}]
                    got = copy.deepcopy(sent)
                    parent = got['object'] if nested else got
                    if row['key'] == 'fileUrl':parent = parent['attachments'][0]
                    parent[row['key']] = row['value']
                    checks = preservation(sent, got)
                    outcome = 'failed' if any(c['outcome']=='failed' for c in checks) else 'not-run' if any(c['outcome']=='not-run' for c in checks) else 'passed'
                    self.assertEqual(outcome, row['outcome'])
        # Neither a delegated instant nor a locator may hide the other defect.
        sent = {'object':{'objectType':'SubStatement','attachments':[{}]}}
        for timestamp, url in [('2016-12-31T23:59:60Z',None), (None,'https://example.org/file')]:
            got = {'object':{'objectType':'SubStatement','timestamp':timestamp,'attachments':[{'fileUrl':url}]}}
            self.assertTrue(any(c['outcome']=='failed' for c in preservation(sent,got)))

    def test_integer_endpoint_serializations_and_definite_changes(self):
        for sent, got, valid in (
                (100000008, 100000010, True), (-100000008, -100000010, True),
                (100000007, 100000010, True), (100000009, 100000020, True),
                (-100000009, -100000020, True),
                (-9223372036854775808, -9223372000000000000, True),
                (18446744073709551615, 18446742970000000000, True),
                (100000008, 100000008, True), (16777217, 16777216, True),
                (16777217, 16777218, True), (0, 0, True),
                (100000008, 100000009, False), (-100000008, -100000009, False),
                (100000008, 100000000, False), (100000008, 100000020, False),
                (100000008, 0, False), (100000008, -100000010, False),
                (9007199254740992, 9007199254740993, False),
                (-9007199254740992, -9007199254740993, False),
                (1, 2, False), (0, 1, False),
                # Exact decimal multiples have coincident floor/ceiling bounds.
                (1000000000, 1000000000, True), (-1000000000, -1000000000, True),
                (1000000000, 1000000010, False), (1000000000, 999999990, False),
                (-1000000000, -999999990, False), (-1000000000, -1000000010, False),
                (10000000000, 10000000010, False), (-10000000000, -9999999990, False),
                (10000000000, 10000000100, False), (-10000000000, -9999999900, False),
                (999999999, 1000000010, False), (-1000000001, -999999990, False)):
            with self.subTest(sent=sent, got=got):
                self.assertIs(number_preserved(sent, got), valid)
                self.assertEqual(preservation(
                    {'result': {'score': {'raw': sent}}},
                    {'result': {'score': {'raw': got}}})[0]['outcome'],
                    'passed' if valid else 'failed')

    def test_timestamp_precision_is_not_an_error_tolerance(self):
        for prefix in ['2025-01-01T00:00:00', '1969-12-31T23:59:59']:
            for suffix in ['.123Z', '.123456Z', '.12346Z', '.1235Z']:
                self.assertTrue(timestamp_preserved(prefix + '.123456Z', prefix + suffix))
            for suffix in ['.124Z', '.122Z', '.123457Z', '.12Z']:
                self.assertFalse(timestamp_preserved(prefix + '.123456Z', prefix + suffix))
            self.assertFalse(timestamp_preserved(prefix + '.123Z', prefix + '.124Z'))
        self.assertTrue(timestamp_preserved('2025-01-01T09:00:00.9999+09:00', '2025-01-01T00:00:01Z'))
        self.assertFalse(timestamp_preserved('2025-01-01T00:00:00Z', '2025-01-01T00:00:00-00:00'))
        self.assertTrue(timestamp_preserved('2025-01-01T00:00:00.1234567819Z', '2025-01-01T00:00:00.123456782Z'))
        self.assertFalse(timestamp_preserved('2025-01-01T00:00:00.123456789012345678901234567891Z', '2025-01-01T00:00:00.123456789012345678901234567892Z'))

    def test_binary32_floor_is_allowed_but_lower_precision_is_not(self):
        self.assertTrue(number_preserved(Decimal('.1'), Decimal('.10000000149011612')))
        self.assertFalse(number_preserved(Decimal('.123456789'), Decimal('.1234')))
        self.assertFalse(number_preserved(1, True))
        self.assertTrue(number_preserved(Decimal('-.1'), Decimal('-.10000000149011612')))
        for sign in ('', '-'):
            # An exactly represented binary32 input may be serialized directly
            # as binary32 or after widening to the runtime's binary64 float.
            exact = Decimal(sign + '.100000001490116119384765625')
            self.assertTrue(number_preserved(exact, Decimal(sign + '.1')))
            self.assertTrue(number_preserved(exact, Decimal(sign + '.10000000149011612')))
            self.assertTrue(number_preserved(exact, Decimal(sign + '.100000002')))
        # Sharing the binary32 rounding bin alone must not allow arbitrary drift.
        self.assertFalse(number_preserved(Decimal('.1'), Decimal('.1000000025')))
        self.assertTrue(number_preserved(Decimal('.3'), Decimal('.300000012')))

    def test_binary32_serialization_is_not_limited_to_shortest_or_nine_digits(self):
        # Independent fixed decimal outputs: ordinary, maximum and subnormal
        # endpoints, nearest and directed rounding, both signs and >9 digits.
        cases = [
            (Decimal('.3'), '.300000012'),
            (Decimal('.3'), '.3000000119209289551'),
            (Decimal('.1'), '.100000001'),
            (Decimal('.1'), '.100000002'),
            (Decimal('.1'), '.10000000149011611938476563'),
            (2**128 - 2**104, '3.40282347e38'),
            (2**128 - 2**104, '3.4028234663852885981e38'),
            (Decimal('1.40129846432481707092372958328991613128026194187651577175706828388979108268586060148663818836212158203125e-45'),
             '1.40129846e-45'),
        ]
        for sent, spelling in cases:
            for sign in ('', '-'):
                with self.subTest(sent=sent, spelling=sign + spelling):
                    original = Decimal(sign + str(sent))
                    got = Decimal(sign + spelling)
                    for rounding in ('ROUND_DOWN', 'ROUND_UP'):
                        with localcontext() as context:
                            context.prec = 6
                            context.rounding = rounding
                            self.assertIs(number_preserved(original, got), True)
                    self.assertEqual(preservation(original, got)[0]['outcome'], 'passed')
        for sent, got in [('.3', '.30000005'), ('.33333334', '.33333337'),
                          ('.1', '.1000000025'), ('1', '1.0000000000000001')]:
            self.assertIs(number_preserved(Decimal(sent), Decimal(got)), False)

    def test_maximum_finite_binary32_keeps_decidable_changes(self):
        maximum = 2**128 - 2**104
        for endpoint in (maximum, -maximum):
            with self.subTest(endpoint=endpoint):
                sign = '-' if endpoint < 0 else ''
                # Intermediate four-digit candidates overflow binary32, while
                # the shortest correct representation needs eight digits.
                self.assertIs(number_preserved(endpoint, 0), False)
                self.assertIs(number_preserved(endpoint, -endpoint), False)
                self.assertIs(number_preserved(endpoint, Decimal(sign + '3.4028235e38')), True)
                self.assertIs(number_preserved(endpoint, Decimal(repr(float(endpoint)))), True)
                self.assertIs(number_preserved(endpoint, endpoint), True)
                self.assertIs(number_preserved(endpoint, Decimal(sign + '3.40282347e38')), True)

    def test_definite_calendar_errors_are_distinct_from_delegated_forms(self):
        for value in ('2026-02-30T00:00:00Z', '2025-02-29T00:00:00Z',
                      '2026-10-01T25:00:00Z', '2026-10-01T24:00:00.1Z',
                      '2026-10-01T00:61:00Z', '2026-10-01T00:00:61Z',
                      '2026-10-01T00:00:00+09:60'):
            with self.subTest(value=value):
                try:
                    timestamp_ns(value)
                except PrimitiveUnavailable:
                    self.fail('definite invalidity was hidden as provider unavailability')
                except ValueError:
                    pass
                else:
                    self.fail('invalid timestamp accepted')
        for value in ('2026-10-01T24:00:00Z', '2026-06-30T23:59:60Z'):
            with self.assertRaises(PrimitiveUnavailable):
                timestamp_ns(value)

    def test_reduced_clock_defects_fail_before_provider_delegation(self):
        for suffix in ('T25Z', 'T25:00Z', 'T00:61Z', 'T25.5Z', 'T24.1Z',
                       'T09.5+24:00', 'T2500Z', 'T24:01Z'):
            value = '2026-10-01' + suffix
            with self.subTest(value=value):
                self.assertEqual(statement_response_headers({
                    'X-Experience-API-Version': '2.0.0',
                    'X-Experience-API-Consistent-Through': value}), (True, False))
        for value in ('2026-10-01T09Z', '2026-10-01T09:30Z',
                      '2026-10-01T09.5Z', '2026-10-01T09:30.5Z',
                      '2026-10-01T24Z', '2026-10-01T24:00:00Z',
                      '2026-06-30T23:59:60Z', '0000-02-29T00:00:00Z',
                      '+010000-10-01T09:00:00Z'):
            with self.subTest(value=value):
                with self.assertRaises(PrimitiveUnavailable):
                    timestamp_ns(value)
        for value in ('2026-02-30T09Z', '0000-02-30T09Z', '2026-13-01T09.5Z'):
            with self.subTest(value=value):
                self.assertIs(statement_response_headers({
                    'X-Experience-API-Consistent-Through': value})[1], False)

    def test_binary64_collapse_is_not_lossless_evidence(self):
        self.assertFalse(number_preserved(9007199254740992, 9007199254740993))
        self.assertIsNone(number_preserved(1, float('1.0000000000000001')))
        self.assertIsNone(number_preserved(.1, .1))
        self.assertEqual(preservation(.1, .1)[0]['outcome'], 'not-run')

    def test_extension_names_do_not_acquire_xapi_semantics(self):
        value = {'result': {'extensions': {'https://example.org/value': {
            'timestamp': 'not a timestamp', 'duration': 'custom',
            'member': [{'mbox': 'a'}, {'mbox': 'b'}],
            'contextActivities': {'parent': {'id': 'opaque'}},
        }}}}
        self.assertTrue(all(o['outcome'] == 'passed' for o in preservation(value, value)))
        for key, replacement in [('timestamp', 'other'), ('duration', 'other'), ('member', [{'mbox':'b'}, {'mbox':'a'}]), ('contextActivities', {'parent':[{'id':'opaque'}]})]:
            changed = copy.deepcopy(value)
            changed['result']['extensions']['https://example.org/value'][key] = replacement
            self.assertTrue(any(o['outcome'] == 'failed' for o in preservation(value, changed)), key)

    def test_real_locations_retain_their_allowances(self):
        sent = {'timestamp': '2025-01-01T09:00:00.123456+09:00', 'result': {'duration':'PT1.239S'}, 'context': {'contextActivities': {'parent': {'id':'urn:example:activity'}}}}
        got = {'timestamp': '2025-01-01T00:00:00.123Z', 'result': {'duration':'PT1.23S'}, 'context': {'contextActivities': {'parent': [{'id':'urn:example:activity'}]}}}
        self.assertTrue(all(o['outcome'] == 'passed' for o in preservation(sent, got)))
        got['result']['duration'] = 'PT1.24S'
        self.assertTrue(any(o['outcome'] == 'failed' for o in preservation(sent, got)))


if __name__ == '__main__':
    unittest.main()
