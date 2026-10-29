"""Separate controlled producer/renderer role; not an LRS grading feature."""
from datetime import datetime
from decimal import Decimal
from html import escape
from html.parser import HTMLParser
import re


def experience_evidence(fixture):
    event = fixture['experience']
    start, end, timestamp = (datetime.fromisoformat(event[key]) for key in ('start', 'end', 'event'))
    elapsed = end - start
    seconds = Decimal(elapsed.days * 86400 + elapsed.seconds) + Decimal(elapsed.microseconds)/1000000
    score = {key: Decimal(str(value)) for key, value in event['score'].items()}
    normalized = 2*(score['raw']-score['min'])/(score['max']-score['min'])-1
    # This fixture has minute/second precision; it is not a general ISO duration
    # implementation or a replacement for the receiver's duration contract.
    duration = re.fullmatch(r'PT([0-9]+)M([0-9]+(?:\.[0-9]+)?)S', event['duration'])
    described = None if duration is None else Decimal(duration[1])*60 + Decimal(duration[2])
    return {'duration_seconds': str(seconds), 'event_inside': start <= timestamp <= end,
            'duration_matches_experience': described == seconds,
            'scaled_matches_declared_formula': normalized == score['scaled'],
            'language_when_known': event['known_language'], 'language_when_unknown': None}


class HumanText(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.text = []
        self.elements = []

    def handle_data(self, data):
        self.text.append(data)

    def handle_starttag(self, tag, attrs):
        self.elements.append(tag)


def render_text(value):
    """The controlled consumer always creates text, including literal markup."""
    if not isinstance(value, str):
        raise ValueError('declared text must be a String')
    return escape(value, quote=True)


def render_extension(value):
    if value.get('contract') != 'local owner text rendering v1' or value.get('mode') != 'text':
        raise ValueError('no owner rendering contract')
    return render_text(value['value'])


def assignment_observations(fixture):
    assignment = fixture['assignments']
    return {'canonical_account_page': assignment['account']['homePage'] == assignment['account_system_canonical_page'],
            'unique_names': len(assignment['account_unique_names']) == len(set(assignment['account_unique_names'])),
            'mailbox_assignment': assignment['account']['name'] == assignment['mailbox_assignee'],
            'identifier_roles': [{'id': row['id'], 'entity': row['entity'], 'producer_owned': row['owned_namespace'],
                                  'reused': row['reuse'], 'lrs_rejection_inferred': False}
                                 for row in assignment['identifier_log']]}
