"""Ground-truth producer and human renderer evidence, independently of Kiva."""
import copy
import json
from pathlib import Path
import unittest

from lrs_producer_fixture import (HumanText, assignment_observations, experience_evidence,
                                 render_extension, render_text)

FIXTURE = Path(__file__).resolve().parents[2]/'tests/conformance/fixtures/producer-meaning.json'


class ProducerMeaningTests(unittest.TestCase):
    def test_ground_truth_assignment_and_sender_only_counterexamples(self):
        fixture = json.loads(FIXTURE.read_text())
        observed = experience_evidence(fixture)
        self.assertEqual(observed['duration_seconds'], '90.25')
        self.assertTrue(observed['duration_matches_experience'])
        self.assertTrue(observed['event_inside'])
        self.assertTrue(observed['scaled_matches_declared_formula'])
        self.assertEqual(observed['language_when_known'], 'ja')
        self.assertIsNone(observed['language_when_unknown'])
        wrong = copy.deepcopy(fixture); wrong['experience']['score']['scaled'] = .85
        wrong['experience']['duration'] = 'PT1M30.24S'
        self.assertFalse(experience_evidence(wrong)['duration_matches_experience'])
        self.assertFalse(experience_evidence(wrong)['scaled_matches_declared_formula'])
        wrong['experience']['event'] = '2020-01-01T00:02:00Z'
        self.assertFalse(experience_evidence(wrong)['event_inside'])
        assignment = assignment_observations(fixture)
        for key in ('canonical_account_page', 'unique_names', 'mailbox_assignment'):
            self.assertTrue(assignment[key])
        log = assignment['identifier_roles']
        self.assertEqual(log[0]['id'], log[1]['id'])
        self.assertTrue(log[1]['reused'])
        self.assertFalse(log[3]['producer_owned'])
        self.assertTrue(all(not row['lrs_rejection_inferred'] for row in log))

    def test_literal_markup_is_text_and_owner_contract_is_separate(self):
        fixture = json.loads(FIXTURE.read_text())
        for literal in (fixture['literal_language_value'], '<script>alert("x")</script>', '<img src=x onerror="x">'):
            parser = HumanText(); parser.feed(render_text(literal))
            self.assertEqual(''.join(parser.text), literal)
            self.assertEqual(parser.elements, [])
            # Deliberately inserting raw markup is a distinct failed rendering.
            raw = HumanText(); raw.feed(literal)
            self.assertNotEqual(raw.elements, [])
        parser = HumanText(); parser.feed(render_extension(fixture['extension_rendering']))
        self.assertEqual(''.join(parser.text), fixture['extension_rendering']['value'])
        self.assertEqual(parser.elements, [])
        with self.assertRaises(ValueError): render_extension({'mode': 'text', 'value': 'no contract'})
        human = HumanText(); human.feed(fixture['more_info_html'])
        self.assertIn(fixture['human_description'], ''.join(human.text))
        self.assertNotIn('a', human.elements)  # a launch link is optional
