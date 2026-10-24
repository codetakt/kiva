"""Issued-ledger storage/readback witnesses; no implementation-derived oracle."""

import copy
import hashlib
import json

from generate_lrs_projection_cases import case, expect, request, step, read_statement, uuid_for, set_pointer
from lrs_projection_behavior import S


def storage_case(name, body, transport, requirements, *, following_steps=()):
    body = copy.deepcopy(body)
    identifier = uuid_for(name)
    if 'id' in body:
        body['id'] = identifier
    method = 'PUT' if transport == 'put' else 'POST'
    payload = [body] if transport == 'batch' else body
    query = {'statementId': identifier} if method == 'PUT' else {}
    write_expect = expect(204 if method == 'PUT' else 200)
    if method == 'POST':
        if 'id' in body:
            write_expect['checks'] = [{'kind':'statement-ids', 'expected':[identifier]}]
        else:
            write_expect['capture'] = {'issued_id': {'pointer': '/0'}}
            identifier = '{{issued_id}}'
    read = read_statement(identifier, checks=[{'kind':'json-preservation', 'expected':body}])
    read['request']['query']['format'] = 'exact'
    return case(name, requirements, steps=[step('store', request(method, '/statements', query, payload), write_expect), read, *following_steps])


def issued_cases(table_cases):
    """Every successful table vector is exercised through PUT, POST and batch.

    The original acceptance/rejection case remains. These additional cases
    examine the complete returned submitted value with allowed transformations.
    """
    cases, origins = [], {}
    for original in table_cases:
        first = original.get('steps', [original])[0]
        req, exp = first['request'], first['expect']
        if req['method'] != 'PUT' or req['path'] != '/statements' or not isinstance(req.get('body'), dict) or exp['status'] != 204:
            continue
        for transport in ['put', 'post', 'batch']:
            name = 'issued-readback-' + original['id'] + '-' + transport
            cases.append(storage_case(name, req['body'], transport, original['requirements']))
            origins[name] = original['id']

    # Extension keys that resemble xAPI properties must remain opaque, in every
    # supported Statement/SubStatement extension location.
    for location in ['/result/extensions', '/context/extensions', '/object/definition/extensions', '/object/result/extensions', '/object/context/extensions', '/object/object/definition/extensions']:
        for transport in ['put', 'post', 'batch']:
            name = 'issued-opaque-extension-' + location.strip('/').replace('/', '-') + '-' + transport
            body = copy.deepcopy(S)
            if location.startswith(('/object/result/', '/object/context/', '/object/object/')):
                body['object'] = dict(copy.deepcopy(S), objectType='SubStatement')
                body['object'].pop('id', None)
            value = {'timestamp':'not-an-xapi-timestamp', 'duration':'domain-owned', 'member':[{'x':1},{'x':2}], 'contextActivities':{'parent':{'id':'opaque'}}, 'result':{'score':{'scaled':500}}, 'null':None}
            set_pointer(body, location, {'urn:example:opaque':value})
            body['id'] = uuid_for(name)
            cases.append(storage_case(name, body, transport, ['xapi-lrs-4.2.7-extension-values']))
    for nested in [False, True]:
        # A recorded experience lasting 90.25 seconds, with a timestamp in its
        # interior. No oracle asks the LRS to verify truth or to grade a score.
        for language in [None, 'ja', 'en-GB']:
            for transport in ['put', 'post', 'batch']:
                name = 'issued-data-meaning-' + ('sub-' if nested else 'root-') + (language or 'omitted') + '-' + transport
                experience = dict(copy.deepcopy(S), timestamp='2020-01-01T00:00:30Z',
                    result={'score':{'scaled':0.7,'raw':17,'min':0,'max':20}, 'duration':'PT90.25S'})
                experience['object']['definition'] = {'moreInfo':'https://example.invalid/activity-information'}
                if language:
                    experience['context'] = {'language':language}
                body = dict(copy.deepcopy(S), object=dict(experience, objectType='SubStatement')) if nested else experience
                if nested: body['object'].pop('id', None)
                body['id'] = uuid_for(name)
                cases.append(storage_case(name, body, transport, ['xapi-lrs-4.2.1-statement-type-validation']))

    for interaction in ['fill-in', 'long-fill-in', 'performance']:
        prefixes = ['', '{order_matters=true}', '{order_matters=false}', '{order_matters=invalid}', '{lang=ja}', '{lang=bad_tag}', '{unknown=value}']
        if interaction != 'performance':
            prefixes += ['{case_matters=true}', '{case_matters=false}', '{case_matters=invalid}']
        for index, prefix in enumerate(prefixes):
            for nested in [False, True]:
                name = f'issued-characterstring-{interaction}-{index}-' + ('sub' if nested else 'root')
                definition = {'interactionType':interaction, 'correctResponsesPattern':[prefix + ('step[.]answer' if interaction == 'performance' else 'answer')]}
                if interaction == 'performance': definition['steps'] = [{'id':'step'}]
                experience = dict(copy.deepcopy(S), result={'response':prefix + ('step[.]answer' if interaction == 'performance' else 'answer')})
                experience['object']['definition'] = definition
                body = dict(copy.deepcopy(S), object=dict(experience, objectType='SubStatement')) if nested else experience
                if nested: body['object'].pop('id', None)
                body['id'] = uuid_for(name)
                cases.append(storage_case(name, body, 'put', ['xapi-lrs-4.2.1-statement-type-validation']))
    for pointer in ['/actor', '/object', '/context/instructor', '/object/actor', '/object/object']:
        name = 'issued-mbox-sha1-query-' + pointer.strip('/').replace('/', '-')
        mailbox = 'mailto:' + name + '@example.org'
        # SHA-1 is applied to the complete mailto IRI, not just its address.
        agent = {'objectType':'Agent', 'mbox_sha1sum':hashlib.sha1(mailbox.encode()).hexdigest()}
        body = copy.deepcopy(S)
        if pointer.startswith('/object/'):
            body['object'] = dict(copy.deepcopy(S), objectType='SubStatement')
            body['object'].pop('id', None)
        set_pointer(body, pointer, agent)
        body['id'] = uuid_for(name)
        body['verb'] = {'id':'urn:example:' + name}
        # Complete the sequence before case() applies every response contract.
        queries = [step('same-ifi-query', request('GET', '/statements', {
            'agent':json.dumps(agent), 'verb':body['verb']['id'], 'related_agents':'true',
        }), expect(200, {'/statements':{'contains':{'id':body['id']}}})),
        step('optional-cross-ifi-query', request('GET', '/statements', {
            'agent':json.dumps({'mbox':mailbox}), 'verb':body['verb']['id'], 'related_agents':'true',
        }), expect(200, checks=[{'kind':'json-shape', 'pointer':'/statements', 'shape':{
            'type':'array', 'max_items':1, 'items':{'type':'object','required':['id'],'properties':{'id':{'enum':[body['id']]}}},
        }}]))]
        cases.append(storage_case(name, body, 'put', ['xapi-lrs-4.1.6.1-statement-query-related-agents'], following_steps=queries))
    # RFC 3629 boundary classes. Binary Documents are deliberately exercised by
    # their existing byte cases rather than being subjected to this JSON rule.
    invalid_utf8 = ['80', 'c080', 'e08080', 'f0808080', 'eda080', 'f4908080', 'f5808080', 'ff', 'c2']
    for transport in ['put', 'post', 'batch']:
        for index, bad in enumerate(invalid_utf8):
            name = f'issued-utf8-invalid-{index}-{transport}'
            body = dict(copy.deepcopy(S), id=uuid_for(name), result={'response':'INVALID_UTF8_MARKER'})
            payload = [body] if transport == 'batch' else body
            raw = json.dumps(payload).encode().replace(b'INVALID_UTF8_MARKER', bytes.fromhex(bad))
            method = 'PUT' if transport == 'put' else 'POST'
            cases.append(case(name, ['xapi-lrs-4.1.2-utf8'], steps=[
                step('reject-invalid-encoding', request(method, '/statements', {'statementId':body['id']} if method == 'PUT' else {}, raw_body_hex=raw.hex(), headers={'Content-Type':'application/json','X-Experience-API-Version':'2.0.0'}), expect(400)),
                step('no-record-created', request('GET', '/statements', {'statementId':body['id']}), expect(404)),
            ]))
        name = 'issued-utf8-valid-boundaries-' + transport
        body = dict(copy.deepcopy(S), id=uuid_for(name), result={'response':'\u0000\u007f\u0080\u07ff\u0800\ud7ff\ue000\uffff\U00010000\U0010ffff'})
        cases.append(storage_case(name, body, transport, ['xapi-lrs-4.1.2-utf8']))
    return cases, origins


def issued_rejection_cases(table_cases):
    """Exercise every mandatory table rejection through POST and atomic batch.

    An invalid middle member is surrounded by two valid new Statements, so a
    400 response alone cannot hide a partially committed batch. Optional
    interaction validation vectors retain their existing alternative outcomes
    and are not promoted to mandatory batch rejections.
    """
    cases, origins = [], {}
    for original in table_cases:
        if 'steps' in original or original['expect']['status'] != 400 or original['expect'].get('status_in'):
            continue
        source = original['request']
        if source['method'] != 'PUT' or source['path'] != '/statements':
            continue
        for transport in ['post', 'batch']:
            name = 'issued-reject-' + original['id'] + '-' + transport
            identifier = uuid_for(name)
            if 'raw_body_hex' in source:
                raw = bytes.fromhex(source['raw_body_hex'])
            else:
                raw = json.dumps(source['body'], ensure_ascii=False, separators=(',', ':')).encode()
            raw = raw.replace(source['query']['statementId'].encode(), identifier.encode())
            siblings = [dict(copy.deepcopy(S), id=uuid_for(name + '-before')), dict(copy.deepcopy(S), id=uuid_for(name + '-after'))]
            if transport == 'batch':
                raw = b'[' + json.dumps(siblings[0]).encode() + b',' + raw + b',' + json.dumps(siblings[1]).encode() + b']'
            steps = [step('reject', request('POST', '/statements', raw_body_hex=raw.hex(), headers=copy.deepcopy(source['headers'])), expect(400))]
            for record in [*siblings, {'id':identifier}] if transport == 'batch' else [{'id':identifier}]:
                steps.append(step('not-recorded-' + record['id'], request('GET', '/statements', {'statementId':record['id']}), expect(404)))
            cases.append(case(name, list(dict.fromkeys([*original['requirements'], 'xapi-lrs-4.1.5-batch-rejection'] if transport == 'batch' else original['requirements'])), steps=steps))
            origins[name] = original['id']
    return cases, origins
