"""Finite schema-route and relational boundary tests for the issued ledger.

Route tests complement the exhaustive field partitions at the model's canonical
placements. They deliberately put an invalid child after a valid child: checking
only an array's first member must not satisfy the enclosing type contract.
"""
import copy

from generate_lrs_projection_cases import case, expect, request, step, set_pointer, uuid_for
from lrs_projection_behavior import S, A
from lrs_projection_issued import storage_case

FAMILY = 'xapi-lrs-4.2.1-statement-type-validation'


def completion_cases():
    from lrs_projection_audit_repairs import attachment_digest_cases
    cases, bindings = attachment_digest_cases()

    def emit(label, body, rules, invalid=False, optional=False):
        for transport in ['put', 'post', 'batch']:
            name = 'issued-completion-' + label + '-' + transport
            record = copy.deepcopy(body)
            record['id'] = uuid_for(name)
            if invalid:
                method = 'PUT' if transport == 'put' else 'POST'
                req = request(method, '/statements', {'statementId':record['id']} if method == 'PUT' else {}, [record] if transport == 'batch' else record)
                exp = expect(400, **({'status_in':[204 if method == 'PUT' else 200]} if optional else {}))
                checks = [step('reject-invalid-child', req, exp)]
                if not optional:
                    checks.append(step('not-stored', request('GET','/statements',{'statementId':record['id']}), expect(404)))
                vector = case(name, [FAMILY], steps=checks)
            else:
                vector = storage_case(name, record, transport, [FAMILY])
            cases.append(vector)
            bindings[name] = rules

    def experience(nested):
        return dict(copy.deepcopy(S), object=dict(copy.deepcopy(S), objectType='SubStatement')) if nested else copy.deepcopy(S)

    # Equal bodies do not exempt duplicate ids within one POST batch. A replay
    # in a separate request has a different contract and cannot cover this arm.
    name = 'issued-completion-identical-duplicate-batch-id'
    body = dict(copy.deepcopy(S), id=uuid_for(name))
    cases.append(case(name, ['xapi-lrs-4.1.6.1-statement-post-duplicate-batch-id'], steps=[
        step('reject-identical-members', request('POST', '/statements', {}, [body, copy.deepcopy(body)]), expect(400)),
        step('not-stored', request('GET', '/statements', {'statementId':body['id']}), expect(404)),
    ]))
    bindings[name] = ['IP-batch-duplicate-id', 'IP-batch-atomic-rejection']

    # Each forbidden property is otherwise valid as a Statement property, so
    # rejection discriminates the SubStatement exclusion, not a malformed value.
    for field, value in [('id', uuid_for('substatement-forbidden-id')),
                         ('stored', '2020-01-01T00:00:00Z'),
                         ('authority', copy.deepcopy(A)), ('version', '2.0.0')]:
        body = experience(True)
        body['object'][field] = value
        emit('sub-forbidden-' + field, body, ['IP-substatement-forbidden-fields'], True)

    for nested in [False, True]:
        root = '/object' if nested else ''
        label = 'sub' if nested else 'root'
        # ISO duration calling-boundary partitions. This does not certify the
        # entire delegated primitive; it distinguishes date/week/time/decimal
        # forms from a parser that accepts only PT...S.
        for duration_label, duration in [('calendar', 'P1Y2M3DT4H5M6S'),
                                         ('week', 'P1W'), ('day', 'P1D'),
                                         ('hour', 'PT1H'), ('comma', 'PT1,5S')]:
            body = experience(nested)
            set_pointer(body, root + '/result/duration', duration)
            emit(label + '-duration-' + duration_label, body,
                 ['IP-Result-duration-type', 'IP-duration-format', 'IP-preserve-duration'])
        for kind,obj in [
            ('agent',copy.deepcopy(A)),
            ('identified-group',{'objectType':'Group','mbox':'mailto:object-group@example.org'}),
            ('anonymous-group',{'objectType':'Group','member':[copy.deepcopy(A)]}),
            ('reference',{'objectType':'StatementRef','id':uuid_for('context-reference-target')}),
        ] + ([] if nested else [('substatement',dict(copy.deepcopy(S),objectType='SubStatement'))]):
            body = experience(nested)
            set_pointer(body,root + '/object',obj)
            set_pointer(body,root + '/context',{'revision':'revision on any Object','platform':'platform on any Object'})
            emit(label + '-context-on-' + kind,body,
                ['IP-Context-revision-presence','IP-Context-revision-type','IP-Context-platform-presence','IP-Context-platform-type'])
        # Every Activity position can carry a definition, an interaction and all
        # five component-list kinds. Existing table vectors cover each field;
        # these cover dispatch through every schema edge and nonfirst elements.
        for activity_path in ['/object'] + ['/context/contextActivities/' + k + '/1' for k in ['parent','grouping','category','other']]:
            for kind, component in [('choice','choices'),('sequencing','choices'),('likert','scale'),('matching','source'),('matching','target'),('performance','steps')]:
                pointer = root + activity_path
                body = experience(nested)
                if '/contextActivities/' in activity_path:
                    set_pointer(body, pointer.rsplit('/',1)[0], [{'id':'urn:example:first'},{'id':'urn:example:second'}])
                definition = {'name':{'ja':'活動'},'description':{'en':'Description'},'moreInfo':'https://example.invalid/info','type':'urn:example:kind',
                    'extensions':{'urn:example:opaque':{'timestamp':'opaque','v':[None,True,3]}},
                    'interactionType':kind, component:[{'id':'first'},{'id':'second','description':{'ja':'二番目'}}]}
                set_pointer(body, pointer, {'id':'urn:example:' + label + '-' + kind + '-' + component,'definition':definition})
                stem = label + '-' + activity_path.strip('/').replace('/','-') + '-' + kind + '-' + component
                common = ['IP-Activity-definition-type','IP-ActivityDefinition-name-type','IP-ActivityDefinition-description-type','IP-ActivityDefinition-extensions-type',
                    'IP-Interaction-' + component + '-type','IP-Interaction-' + component + '-value','IP-InteractionComponent-id-type','IP-InteractionComponent-description-type','IP-table-types','IP-return-serialization']
                emit(stem + '-valid',body,common)
                wrong = copy.deepcopy(body)
                set_pointer(wrong,pointer + '/definition/' + component + '/1/id',42)
                emit(stem + '-invalid-second-component',wrong,common,True,True)
                wrong = copy.deepcopy(body)
                set_pointer(wrong,pointer + '/definition/name',{'en':42})
                emit(stem + '-invalid-map-value',wrong,['IP-ActivityDefinition-name-type','IP-language-map-shape','IP-table-types'],True)
        # Both kinds of Groups and every Group-bearing role, with a nonfirst
        # member whose nested account is malformed. Groups cannot contain Groups.
        roles = ['/actor','/object','/context/instructor','/context/team','/context/contextGroups/1/group']
        if not nested: roles.append('/authority')
        for role in roles:
            for identified in [False,True]:
                body = experience(nested)
                if '/contextGroups/' in role:
                    set_pointer(body,root + '/context/contextGroups',[
                        {'objectType':'contextGroup','group':{'objectType':'Group','member':[copy.deepcopy(A)]}},
                        {'objectType':'contextGroup','group':{}}])
                group = {'objectType':'Group','member':[copy.deepcopy(A),{'objectType':'Agent','account':{'homePage':'https://example.invalid/accounts','name':'second'}}]}
                if identified: group['mbox']='mailto:group@example.org'
                set_pointer(body,root + role,group)
                stem = label + '-' + role.strip('/').replace('/','-') + ('-identified' if identified else '-anonymous')
                rules = ['IP-IdentifiedGroup-member-type' if identified else 'IP-AnonymousGroup-member-type','IP-Account-homePage-type','IP-Account-name-type','IP-table-types']
                emit(stem + '-valid',body,rules)
                wrong = copy.deepcopy(body)
                set_pointer(wrong,root + role + '/member/1/account/name',42)
                emit(stem + '-bad-second-account',wrong,rules,True)
                wrong = copy.deepcopy(body)
                set_pointer(wrong,root + role + '/member/1',{'objectType':'Group','member':[copy.deepcopy(A)]})
                emit(stem + '-nested-group',wrong,['IP-nested-group','IP-table-types'],True)
        for container in ['/context/contextAgents','/context/contextGroups']:
            group = container.endswith('Groups')
            field = 'group' if group else 'agent'
            actor = {'objectType':'Group','member':[copy.deepcopy(A)]} if group else copy.deepcopy(A)
            body = experience(nested)
            wrapper = 'contextGroup' if group else 'contextAgent'
            set_pointer(body,root + container,[{'objectType':wrapper,field:actor},
                {'objectType':wrapper,field:actor,'relevantTypes':['urn:example:first','urn:example:second']}])
            stem = label + '-' + field + '-relationship'
            rule = 'IP-contextGroup-' if group else 'IP-contextAgent-'
            emit(stem + '-valid',body,[rule + field + '-type',rule + 'relevantTypes-type'])
            for value in [42,None,'no-scheme']:
                wrong = copy.deepcopy(body)
                set_pointer(wrong,root + container + '/1/relevantTypes/1',value)
                emit(stem + '-invalid-second-type-' + str(value),wrong,[rule + 'relevantTypes-type','IP-table-types'],True)
        for index,(score,valid) in enumerate([
            ({'scaled':-1},True),({'scaled':1},True),({'scaled':-1.0001},False),({'scaled':1.0001},False),
            ({'raw':-1000000},True),({'raw':1000000},True),({'raw':1,'min':1},True),({'raw':2,'max':2},True),
            ({'raw':0,'min':1},False),({'raw':3,'max':2},False),({'min':2,'max':2},False),({'min':3,'max':2},False),
            ({'min':1,'max':2,'raw':1},True),({'min':1,'max':2,'raw':2},True),({'min':1},True),({'max':2},True),
        ]):
            body = experience(nested)
            set_pointer(body,root + '/result/score',score)
            emit(label + '-score-relation-' + str(index),body,
                ['IP-Score-' + f + '-' + facet for f in ['scaled','raw','min','max'] for facet in ['type','value']],not valid)
        for field in ['scaled','raw','min','max','success','completion']:
            boolean = field in ['success','completion']
            pointer = root + ('/result/' if boolean else '/result/score/') + field
            for index,value in enumerate(['true','false','0','1'] if boolean else ['0.5','0','-1','1e0']):
                body = experience(nested)
                set_pointer(body,pointer,value)
                emit(label + '-coerced-' + field + '-' + str(index),body,
                    ['IP-table-primitive','IP-table-types',('IP-Result-' if boolean else 'IP-Score-') + field + '-type'],True)
        # Parameter scope: list-wide flags precede the complete list; lang is
        # repeated at each String item, including performance responses.
        for kind in ['fill-in','long-fill-in','performance']:
            for index,flag in enumerate(['', '{order_matters=true}', '{order_matters=false}', '{order_matters=invalid}'] + ([] if kind == 'performance' else ['{case_matters=true}','{case_matters=false}','{case_matters=invalid}'])):
                text = flag + ('step1[.]{lang=ja}答え[,]step2[.]{lang=en}answer' if kind == 'performance' else '{lang=ja}答え[,]{lang=en}answer')
                body = experience(nested)
                definition = {'interactionType':kind,'correctResponsesPattern':[text]}
                if kind == 'performance': definition['steps']=[{'id':'step1'},{'id':'step2'}]
                set_pointer(body,root + '/object/definition',definition)
                set_pointer(body,root + '/result/response',text)
                emit(label + '-string-list-' + kind + '-' + str(index),body,['IP-characterstring-validation','IP-characterstring-context','IP-result-response-format'])
    return cases, bindings
