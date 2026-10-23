"""Condition/exception traceability and identity of authored test definitions.

Bindings detect changes to obligations and their concrete tests. They do not
represent review decisions or certify that an oracle is correct. The historical
review_binding name remains part of the generated projection schema.
"""
import hashlib
import json


def digest(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


EXCEPTION_SCENARIOS = [
    ('Rejection for security', 'issued-scope-boundaries'),
    ('The header list', 'issued-http-context-matrix'),
    ('Documents may', 'issued-document-time-and-atomicity'),
    ('Attachment exchanges', 'issued-multipart-contract'),
    ('PUT single-Statement', 'issued-discriminator-OPEN-PUT'),
    ('StatementResult with attachments=false', 'issued-multipart-contract'),
    ('First part application/json', 'issued-multipart-contract'),
    ('The LRS may recognize', 'declared-limit-and-extension-contexts'),
    ('The empty-result case', 'issued-discriminator-OPEN-LASTMOD'),
    ('Explicit data normalization', 'issued-metadata-language-inventory'),
    ('No expansion for', 'issued-reference-graph-and-pages'),
    ('Context.statement references', 'issued-reference-graph-and-pages'),
    ('Explicit voidedStatementId', 'issued-reference-graph-and-pages'),
    ('Chapter 5 includes Identified Group', 'issued-discriminator-OPEN-GROUP'),
    ('Chapter 4 instead prints', 'issued-discriminator-OPEN-ACTIVITY-LIST'),
    ('Missing-header and non-version', 'issued-discriminator-POL-VERSION-MISSING'),
    ('Routing to a fully conformant', 'older-version-routing'),
    ('About exemption', 'issued-discriminator-OPEN-VERSION'),
    ('Group-member order', 'issued-replay-state-snapshot'),
    ('display', 'issued-metadata-language-inventory'),
    ('explicit data normalization', 'issued-replay-state-snapshot'),
    ('precision finer', 'issued-primitive-integration-boundaries'),
    ('Extension values retain', 'issued-primitive-integration-boundaries'),
    ('When the Agent is a Statement Object', 'issued-producer-boundaries'),
    ('Do not turn unverifiable', 'issued-producer-and-meaning'),
    ('LRP Activity objectType', 'issued-producer-boundaries'),
    ('After a valid interactionType', 'issued-interaction-interpreter'),
    ('Exact value and interaction', 'issued-discriminator-OPEN-SUBTIME'),
    ('The JSON transmission alternative', 'issued-multipart-contract'),
    ('Interaction-property validation', 'issued-interaction-interpreter'),
    ('Formatting is not', 'issued-interaction-interpreter'),
    ('The property remains optional', 'issued-producer-and-meaning'),
    ('The processing clause separately', 'issued-authority-and-security-fixtures'),
    ('Normal retention', 'issued-scope-boundaries'),
    ('Specific unconditional', 'issued-conformance-configuration'),
    ('The acceptable margin', 'rejection-policy-and-clock-margin'),
    ('4.1.6.1.6 still forbids', 'issued-reference-graph-and-pages'),
    ('Whether all 4.2 exclusions', 'issued-discriminator-POL-EQUALITY-SIGNATURE'),
    ('Whole-message JSON validity', 'issued-error-cause-matrix'),
    ('An extension owner', 'issued-producer-and-meaning'),
    ('The identifier-validation permission', 'issued-discriminator-OPEN-FORMAT'),
    ('LRP UTC-format requirement', 'issued-primitive-integration-boundaries'),
    ('LRP recommendation to send', 'issued-primitive-integration-boundaries'),
]

# These exceptions concern different operations. Prefix matching previously
# sent both to replay-state-snapshot, which does not exercise either branch.
EXCEPTION_BY_ID = {
    'IP-preserve-actor.exception.1': 'issued-actor-order-exception',
    'IP-preserve-object.exception.1': 'issued-object-preservation-exception',
    'IP-about-version-exemption.exception.1': 'issued-about-error-cross-product',
}

STRATEGIES = {
    'table-field-presence': 'Exercise valid supplied and omitted fields in the declared enclosing model. Required omissions reject; Recommended/Optional omissions are allowed, with field-specific defaults/IFI/discriminator/attachment exceptions applied. Acceptance readback checks managed fields and submitted values.',
    'table-field-type': 'Exercise each primitive/type/format/enum partition and legal child model, including nested child rejection. Inspect stored submitted values, not only acceptance status. Remaining interaction validation and identifier-waiver dependencies stay conditional.',
    'table-field-value': 'Exercise enumerated/relational value boundaries and table-specific meanings. Domain meaning belongs to the producer/consumer role; receiver structural validation is not historical truth verification.',
    'table-field-meaning': 'Use a controlled experience/producer with known facts, then verify xAPI boundary acceptance/preservation independently of domain truth.',
    'response-table': 'Inspect every Person array element and requested identifier using both portable shape assertions and the independent directory fixture. Multiple valid identifying arrays are permitted.',
    'object-discriminator': 'Exercise omitted and supplied discriminator by role: Actor Agent default, Object Agent/Group required discriminator, Object Activity default, including nested Object.',
    'resource-method': 'Use valid resource lifecycle requests for this method, applicable parameter rejections, effect/readback or scoped absence checks; HEAD additionally has no body and independently required xAPI headers; GET-equivalent entity metadata is a SHOULD observation.',
    'document-method-and-parameter': 'Exercise the named method/selector under its resource scope, presence/type/case partitions, and lifecycle side effects. Request-body descriptions do not invent a rejection rule for unexpected bodies.',
    'authored-duty-correspondence': 'Use the named curated duty tests and supplemental concrete scenarios. Required effects, recommendation observations, optional choices, source disputes and role-only observations are graded separately under this exact rule.',
}


def rationale_for(rule, strategy):
    if rule['id'] == 'IP-Statement-stored-meaning':
        return ('The LRS recording-time data contract uses issued-stored-recording-time and '
                'issued-recording-and-visibility-clock with an independent recording clock. '
                'Timestamp shape or a producer truth procedure is not the recording-time oracle.')
    for family in ['state', 'agent-profile', 'activity-profile']:
        if rule['id'] == 'IP-' + family + '-put-body':
            return ('This LRS storage data contract is exercised by ' + family + '-binary-put: '
                    'compare exact submitted and retrieved bytes and media type, not only status. '
                    'Document bodies need not be JSON; independent scope/time checks are separate.')
        if rule['id'] == 'IP-' + family + '-post-body':
            return ('This LRS update/storage data contract is exercised by ' + family +
                    '-merge-replaces-top-level-object and -post-creates-absent-document. '
                    'Inspect POST and readback effects; creation/precondition project readings '
                    'remain attached through the relevant rules and result reporter.')
    return STRATEGIES[strategy]


def obligations(rule, strategy, names, procedures, procedure_index, duties):
    if strategy not in STRATEGIES:
        raise ValueError('unreviewed projection strategy: ' + strategy)
    exceptions=[]
    for index, text in enumerate(rule['exceptions']):
        exception_id = rule['id'] + '.exception.' + str(index+1)
        choices=([EXCEPTION_BY_ID[exception_id]] if exception_id in EXCEPTION_BY_ID else
            [p for prefix,p in EXCEPTION_SCENARIOS if text.startswith(prefix)])
        if len(choices)!=1 or choices[0] not in procedure_index:
            raise ValueError('exception lacks an explicit test: ' + rule['id'] + ': ' + text)
        procedures.add(choices[0])
        exceptions.append(dict(id=exception_id, condition=text, procedure=choices[0],
            oracle='Use the concrete exception/ordinary branches in this procedure; do not promote a role, recommendation, permission or unresolved alternative into a mandatory receiver status.'))
    for issue in rule['issues']:
        name='issued-discriminator-' + issue
        if name not in procedure_index:
            raise ValueError('missing issue test: ' + issue)
        procedures.add(name)
    role = 'receiver'
    if rule['actor']!='LRS':
        role = 'role-specific'
        if not procedures:
            raise ValueError('role-specific rule lacks role procedure: ' + rule['id'])
    grade = ('required' if rule['strength'] in ['shall','shall-not','data-contract'] else
        'advisory' if rule['strength'] in ['should','should-not'] else
        'interpretation-dependent' if rule['strength']=='unresolved' else 'permission-or-context')
    return dict(applicability=dict(actor=rule['actor'],condition=rule['when'],role=role),
        effect=dict(requirement=rule['requirement'],strength=rule['strength'],grading=grade),
        partition_strategy=strategy, rationale=rationale_for(rule, strategy),
        duty_actions=[dict(id=d['id'],action=d['action_and_violation_observation']) for d in duties],
        exception_branches=exceptions,
        interpretation_branches=[dict(issue=i,procedure='issued-discriminator-' + i,verdict='indeterminate-until-disposition') for i in rule['issues']],
        dependency_boundary=dict(dependencies=rule['dependencies'],scope='xAPI calling boundary; runtime primitives delegated, not newly certified'),
        http_cases=sorted(names),procedures=sorted(procedures))


def review_binding(rule, design, case_index, procedure_index):
    return digest(dict(rule_sha256=digest(rule), design=design,
        cases={n:case_index[n] for n in design['http_cases']},
        procedures={n:digest(procedure_index[n]) for n in design['procedures']}))
