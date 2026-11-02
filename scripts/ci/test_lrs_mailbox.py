import json
from pathlib import Path
import unittest
from lrs_preservation import mailbox_structure

class MailboxContract(unittest.TestCase):
    def test_reference_partitions(self):
        fixture = Path(__file__).resolve().parents[2]/'tests/conformance/fixtures/mailto-contract.json'
        for row in json.loads(fixture.read_text())['vectors']:
            with self.subTest(id=row['id'], iri=row['iri']):
                self.assertIs(mailbox_structure(row['iri']), row['valid'])

if __name__ == '__main__':
    unittest.main()
