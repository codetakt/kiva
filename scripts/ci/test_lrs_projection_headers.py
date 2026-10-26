"""Storage response contracts and complete query witnesses."""
import copy
import json
from pathlib import Path
import unittest

import jsonschema
import yaml

from lrs_projection_issued import issued_cases, storage_case
from lrs_projection_behavior import S


class IssuedHeaderControls(unittest.TestCase):
    def test_post_response_id_cannot_replace_the_submitted_readback_target(self):
        for transport in ['post','batch']:
            with self.subTest(transport=transport):
                original=copy.deepcopy(S); original['id']='00000000-0000-4000-8000-000000000001'
                definition=storage_case('post-id-control-'+transport,original,transport,['fixture'])
                write,read=definition['steps']
                body=write['request']['body']
                submitted=body[0] if transport=='batch' else body
                self.assertIn({'kind':'statement-ids','expected':[submitted['id']]},write['expect']['checks'])
                self.assertNotIn('capture',write['expect'])
                self.assertEqual(read['request']['query']['statementId'],submitted['id'])
                self.assertEqual(original['id'],'00000000-0000-4000-8000-000000000001')

    def test_post_without_submitted_id_retains_checked_server_assignment(self):
        for transport in ['post','batch']:
            with self.subTest(transport=transport):
                original=copy.deepcopy(S); original.pop('id',None)
                definition=storage_case('assigned-id-'+transport,original,transport,['fixture'])
                write,read=definition['steps']
                self.assertEqual(write['expect']['checks'],[{'kind':'json-shape','status':200,'shape':{
                    'type':'array','min_items':1,'max_items':1,'items':{'type':'string','format':'uuid'}}}])
                self.assertEqual(write['expect']['capture'],{'issued_id':{'pointer':'/0'}})
                self.assertEqual(read['request']['query']['statementId'],'{{issued_id}}')

    def test_query_definitions_require_bounded_collection_in_the_schema(self):
        root=Path(__file__).resolve().parents[2]
        schema=json.loads((root/'spec/conformance/schemas/conformance-case.schema.json').read_text())
        data=yaml.safe_load((root/'tests/conformance/http/statement-query-filter-isolated.yaml').read_text())
        validator=jsonschema.Draft202012Validator(schema)
        validator.validate(data)
        found=0
        for case in data['cases']:
            for step in case.get('steps',[]):
                if not any(check['kind']=='statement-query' for check in step['expect'].get('checks',[])): continue
                found+=1
                self.assertEqual(step['request']['collect_statement_pages'],100)
                for root_case in [False,True]:
                    bad=copy.deepcopy(step)
                    bad['request'].pop('collect_statement_pages')
                    if root_case: bad.update(requirements=case['requirements'],safety=case['safety'])
                    test_schema={'$defs':schema['$defs'],'$ref':'#/$defs/'+('case' if root_case else 'step')}
                    with self.assertRaises(jsonschema.ValidationError):
                        jsonschema.Draft202012Validator(test_schema).validate(bad)
        self.assertEqual(found,4)

    def test_ifi_ids_use_typed_identity_and_require_complete_collection(self):
        root=Path(__file__).resolve().parents[2]
        schema=json.loads((root/'spec/conformance/schemas/conformance-case.schema.json').read_text())
        validator=jsonschema.Draft202012Validator({'$defs':schema['$defs'],'$ref':'#/$defs/step'})
        cases,_=issued_cases([])
        for case in cases:
            if not case['id'].startswith('issued-mbox-sha1-query-'):continue
            identity=case['steps'][0]['request']['body']['id']
            for step in case['steps'][-2:]:
                with self.subTest(case=case['id'],step=step['id']):
                    validator.validate(step)
                    check=next(c for c in step['expect']['checks'] if c['kind']=='statement-result-ids')
                    key='required' if step['id']=='same-ifi-query' else 'allowed'
                    self.assertEqual(check[key],[identity])
                    bad=copy.deepcopy(step);bad['request'].pop('collect_statement_pages')
                    with self.assertRaises(jsonschema.ValidationError):validator.validate(bad)
                    bad=copy.deepcopy(step)
                    next(c for c in bad['expect']['checks'] if c['kind']=='statement-result-ids')[key]=['not-a-uuid']
                    with self.assertRaises(jsonschema.ValidationError):validator.validate(bad)

    def test_both_ifi_queries_have_the_statement_response_contract(self):
        cases, _ = issued_cases([])
        queries = [case for case in cases if case['id'].startswith('issued-mbox-sha1-query-')]
        self.assertEqual(len(queries), 5)
        for case in queries:
            self.assertEqual([step['id'] for step in case['steps'][-2:]],
                             ['same-ifi-query', 'optional-cross-ifi-query'])
            for step in case['steps'][-2:]:
                self.assertEqual(step['request']['collect_statement_pages'], 100)
            for step in case['steps']:
                with self.subTest(case=case['id'],step=step['id']):
                    headers = step['expect']['headers']
                    self.assertEqual(headers.get('X-Experience-API-Consistent-Through'), {'iso8601': True})
                    self.assertEqual(headers.get('X-Experience-API-Version'), {'equals': '2.0.0'})


if __name__ == '__main__': unittest.main()
