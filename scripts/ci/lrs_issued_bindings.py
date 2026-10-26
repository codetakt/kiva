"""Authored correspondence from issued rule IDs to concrete legacy witnesses.

Selectors locate tests, not proof of semantic completeness. The report retains
every rule condition, exception, dependency and unresolved interpretation.
"""
import re

TABLES = {
    'Statement':'statement', 'Agent':'agent', 'IdentifiedGroup':'identified-group',
    'AnonymousGroup':'anonymous-group', 'Account':'account', 'Verb':'verb',
    'Activity':'activity', 'ActivityDefinition':'definition', 'Interaction':'interaction',
    'InteractionComponent':'component', 'StatementRef':'statement-ref',
    'SubStatement':'substatement', 'Result':'result', 'Score':'score', 'Context':'context',
    'contextAgent':'context-agent', 'contextGroup':'context-group', 'Attachment':'attachment',
}

# The right side names independently authored duty records. Individual table
# facets are selected separately below, instead of inheriting a whole family.
DIRECT = '''
structural-validation all-table-validation
resource-table-validation resources-table
utf8 utf8
content-type-json attachment-alternatives document-type-repeat about-contract
json-attachment-admission json-no-attachment json-fileurl
multipart-no-attachments multipart-no-attachment
multipart-admission-first multipart-first-json
attachment-match hash-only-match
attachment-transmission-json attachment-alternatives get-attachments-false
multipart-rfc attachment-alternatives
multipart-first-json multipart-first-json
multipart-data multipart-input
multipart-x-experience-api-hash multipart-hash-header
multipart-content-transfer-encoding multipart-binary-header
multipart-deduplicate multipart-output
multipart-part-content-type multipart-json-header
multipart-match-contentType multipart-property-match
multipart-match-length multipart-property-match
multipart-match-sha2 multipart-property-match
attachment-response multipart-output
multipart-accept multipart-input multipart-no-attachment multipart-fileurl
multipart-missing-data unmatched-attachment
multipart-binary-default multipart-binary-header
document-etag document-etag
document-if-match if-match
document-precondition-status precondition-status
document-precondition-no-write precondition-preservation
document-unconditional-put-status unconditional-put-status
document-unconditional-put-explanation conflict-explanation
document-unconditional-put-no-write conflict-preservation
precondition-boundary if-match
unknown-parameter unknown-parameters
parameter-case parameter-case
batch-atomic-rejection batch-atomic-rejection
size-limit-choice size-denial
rate-limit-choice rate-denial
document-unknown-entity unknown-document-identities
document-representation document-type-repeat
statement-put-body statement-put-contract
statement-put-availability statement-put-contract
statement-put-duplicate duplicate-put
statement-put-compare duplicate-put
statement-put-conflict duplicate-put
statement-post-body statement-post-contract
statement-post-availability statement-post-contract
statement-post-duplicate duplicate-post
statement-post-compare duplicate-post
statement-post-conflict duplicate-post
statement-put-id-match statement-put-contract
batch-duplicate-id duplicate-batch-id
statement-get-shape query-returns
statement-more query-time-order-limit
get-param-statementId query-identity
get-param-voidedStatementId voided-hidden
get-param-agent query-identity
get-param-verb query-basic-filters
get-param-activity query-basic-filters
get-param-registration query-basic-filters
get-param-related_activities related-filters
get-param-related_agents related-filters
get-param-since query-time-order-limit
get-param-until query-time-order-limit
get-param-limit query-time-order-limit
get-param-format query-formats
get-param-attachments get-attachments-true get-attachments-false
get-param-ascending query-time-order-limit
get-boolean-wire query-returns
get-mutually-exclusive-ids single-id-exclusion
get-id-parameters single-id-parameters
query-permissions authorization-decisions
query-empty empty-statement-result
consistent-through watermark-header
consistent-through-load watermark-header
statement-last-modified statement-last-modified
format-exact exact-display query-formats
format-ids query-formats
format-canonical query-formats
statementref-filters reference-filtering
statementref-time reference-filtering
canonical-activity-language canonical-activity-one-language
canonical-map-maintenance canonical-other-one-language
canonical-other-language canonical-other-one-language
canonical-negotiation language-per-map
voided-visibility voided-hidden
voided-targeters voided-targeters
void-query-wording voided-hidden voided-targeters
state-single-since state-list
state-list-id-conflict state-list
document-group-ifi document-scope-binary
agents-get person-table
person-fallback person-fallback
person-array-members person-elements
person-multiple-identifiers person-elements
activities-get activity-get
activity-fallback activity-get
about-get about-contract
about-versions about-contract
about-extensions about-contract
about-version-exemption about-version-exception
version-scheme about-contract
response-version-header response-version-header
response-version-value response-version-header
version-alias version-alias
version-before-two older-version
version-after-two future-version
version-reject-status version-problem-description
version-missing-boundary response-version-header
authorization authorization-decisions
field-presence-meaning all-table-validation
preserve-actor immutable-components
preserve-verb immutable-components
preserve-object immutable-components
preserve-duration immutable-components duration-high-precision
object-default default-object-activity
Agent-one-ifi single-actor-ifi
IdentifiedGroup-one-ifi single-actor-ifi
anonymous-no-ifi no-group-member-group
agent-object-discriminator default-object-activity
interaction-table-label interaction-vocabulary
interaction-length-description interaction-vocabulary
interaction-correctness interaction-vocabulary
characterstring-validation characterstrings
characterstring-context characterstrings
interaction-no-components interaction-vocabulary
substatement-default-time substatement-timestamp-default
substatement-forbidden-fields substatement-contract
context-instructor-team independent-context-agent independent-context-group
context-activity-keys context-activity-types
context-activity-values context-activity-types
context-relationship-scope independent-context-agent independent-context-group
contextAgent-any-entity independent-context-agent
contextAgent-any-type independent-context-agent
contextGroup-any-entity independent-context-group
contextGroup-any-type independent-context-group
context-group-copy-errors independent-context-group
result-response-format interaction-vocabulary
contextAgent-generic-relationship independent-context-agent
contextAgent-types-recommended independent-context-agent
contextGroup-generic-relationship independent-context-group
contextGroup-types-recommended independent-context-group
statement-get-body query-returns
statement-put-parameter-type statement-put-contract
authority-authentication authority-determinism
authority-table-default authority-exists
hosted-metadata-optional hosted-metadata
hosted-metadata-canonical hosted-metadata
metadata-fetch hosted-metadata
metadata-incorporate hosted-metadata
metadata-other-document hosted-metadata
unauthorized-batch authorization-decisions
property-order property-order
void-missing-target unknown-void-target
property-size policy-exceptions
statement-size-security policy-exceptions
identifier-validation-waiver policy-exceptions
future-timestamp policy-exceptions
substatement-future utc-timestamps
empty-objects all-table-validation
return-serialization response-preservation
create-id id-creation
actor-multiple-ifi single-actor-ifi
nested-group no-group-member-group
number-precision binary32-precision
interaction-type interaction-validation-boundary
interaction-remaining interaction-validation-boundary
component-whitespace interaction-vocabulary
nested-substatement no-substatement-nesting
timestamp-default timestamp-default-stored
stored-overwrite stored-overwrite
stored-validate stored-overwrite
authority-present authority-exists
authority-overwrite authority-determinism
authority-deterministic authority-determinism
authority-trust authority-determinism
authority-trust-caution authority-determinism
context-array context-arrays
context-single-array context-single-array
version-admission version-retain-default
canonical-update-authority hosted-metadata
canonical-small-corrections hosted-metadata
canonical-major-changes hosted-metadata
version-retention version-retain-default
version-default version-retain-default
more-self-contained query-time-order-limit
more-length query-time-order-limit
more-rerun query-time-order-limit
more-cache query-time-order-limit
more-cache-voids query-time-order-limit voided-hidden
void-reject-type voiding-shape
voided-definition voiding-iff
void-rollback voiding-iff
signature-attachment signature-attachment
signature-compact jws-compact
signature-payload signature-payload
signature-algorithm signature-algorithms
signature-certificate-recommended signature-attachment
signature-chain-recommended signature-attachment
signature-reject signature-malformed
signature-error-message signature-malformed
signature-decode signature-validation
signature-equivalence signature-validation
signature-certificate-verify signature-validation
signature-requirements signature-validation
iri-comparison iri-comparison
iri-not-resolvable extension-values
extension-values-domain extension-values
extension-key extension-keys
extension-value-acceptance extension-values
language-map-shape language-tags
language-map-population language-tags
language-map-specificity language-tags
language-map-plain-text language-tags
language-invalid language-tags
language-minimum language-tags
uuid-form uuid-output-string
uuid-variant uuid-output-string
timestamp-utc utc-timestamps
timestamp-precision timestamp-precision
timestamp-format utc-timestamps
duration-format duration-primary-format
duration-alternative duration-alternative
duration-high-precision duration-high-precision
duration-truncate duration-high-precision
duration-compare duration-comparison
document-id document-type-repeat
document-updated document-type-repeat
document-contents document-type-repeat
Statement-stored-meaning stored-overwrite
IRL-definition query-time-order-limit
'''
DIRECT = {'IP-' + row.split()[0]:row.split()[1:] for row in DIRECT.strip().splitlines()}


def identifier_format_bindings(cases, models, table_bindings, origins):
    """Locate format probes from their model facet/input, not status alone.

    These are interpretation dependencies, not a ruling that rejection is
    mandatory or that every failure in the case is caused by that dispute.
    Primitive type violations remain separate from string format probes.
    """
    result = {}

    def add(name, parameter=False):
        rules = ['IP-identifier-validation-waiver', 'IP-table-format']
        if parameter:
            rules += ['IP-parameter-format', 'IP-parameter-scheme']
        result.setdefault(name, set()).update(rules)

    for binding in table_bindings:
        field = binding['field']
        if field is None or '-value-' not in binding['condition']:
            continue
        vector = models[binding['model']]['fields'][field]
        fmt = vector.get('format')
        if fmt not in ['iri', 'irl', 'uri', 'uuid', 'mailto', 'iri-array', 'extensions']:
            continue
        case = cases[binding['case']]
        body = case.get('steps', [case])[0]['request']['body']
        node = body
        for token in binding['placement'].strip('/').split('/') if binding['placement'] else []:
            node = node[int(token)] if isinstance(node, list) else node[token]
        value = node[field]
        if fmt == 'mailto' and not isinstance(value, str):
            continue
        if fmt == 'iri-array' and not (isinstance(value, list) and all(isinstance(v, str) for v in value)):
            continue
        add(binding['case'])
    # Hand-written probes do not have model/table bindings. Bind their actual
    # field contract as well as the format dispute; a conservative case context
    # is not a ruling that every failure (e.g. another missing field) is waived.
    handwritten_formats = {
        'statement-post-rejects-non-uuid-statement-id': 'IP-Statement-id-type',
        'statement-post-rejects-invalid-verb-iri': 'IP-Verb-id-type',
        'statement-actor-rejects-invalid-openid': 'IP-Agent-openid-type',
        'statement-actor-rejects-invalid-account': 'IP-Account-homePage-type',
        'statement-actor-rejects-invalid-mbox': 'IP-Agent-mbox-type',
        'statement-context-rejects-invalid-registration': 'IP-Context-registration-type',
    }
    for name, field_rule in handwritten_formats.items():
        if name in cases:
            add(name)
            result[name].add(field_rule)
    # This compound fixture also omits Account.name. Its 400 can follow from
    # that undisputed obligation even when homePage format is not validated;
    # keep the presence observation visible rather than treating this case as
    # independent evidence of format discrimination.
    name = 'statement-actor-rejects-invalid-account'
    if name in cases:
        result[name].add('IP-Account-name-presence')
    name = 'statement-context-rejects-invalid-context-agent'
    if name in cases:
        result.setdefault(name, set()).update([
            'IP-contextAgent-relevantTypes-value', 'IP-contextAgent-relevantTypes-type'])
    for name, case in cases.items():
        for entry in case.get('steps', [case]):
            for key, value in entry['request'].get('query', {}).items():
                if not isinstance(value, str) or '{{' in value:
                    continue
                if key in ['activityId', 'activity', 'verb']:
                    if not re.match(r'^[A-Za-z][A-Za-z0-9+.-]*:', value) or re.search(r'\s', value):
                        add(name, parameter=True)
                elif key in ['statementId', 'voidedStatementId', 'registration']:
                    if not re.fullmatch(r'[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}', value):
                        add(name, parameter=True)
        if name.startswith('issued-completion-') and '-invalid-second-type-no-scheme-' in name:
            add(name)
    for name, original in origins.items():
        if original in result:
            result.setdefault(name, set()).update(result[original])
    return result


def select(rule, cases, duties, fields):
    identifier = rule['id']
    names, procedures = set(), set()
    mode = 'authored-duty-correspondence'

    def duty(name):
        record = duties[name]
        names.update(w['case'] for w in record['automated_witnesses'])
        if record.get('semantic_procedure'):
            procedures.add(record['semantic_procedure'])

    for name in DIRECT.get(identifier, []):
        duty(name)
    if identifier == 'IP-property-order':
        names.add('statement-member-order-is-unrestricted')
    if identifier in ('IP-void-reject-id', 'IP-StatementRef-id-presence'):
        names.add('statement-post-rejects-voiding-statementref-without-id')
    for prefix, model in TABLES.items():
        match = re.fullmatch('IP-' + prefix + r'-(.+)-(presence|type|value|meaning)', identifier)
        if not match:
            continue
        field, facet = match.groups()
        record = fields.get(model + '.' + field)
        if record is None:
            break
        mode = 'table-field-' + facet
        for placement in record['placements']:
            names.update(w['case'] for w in placement['acceptance_witnesses'])
            for binding in placement['rejection_bindings']:
                condition = binding['condition']
                if (facet == 'presence' and condition.endswith('-missing')
                    or facet == 'type' and ('-type-' in condition or condition.endswith('-null') or '-value-' in condition)
                    or facet == 'value' and '-value-' in condition):
                    names.add(binding['case'])
        if facet == 'type':
            # A table's Object/Array type includes its legal child variants and
            # recursively constrained children, not just a {} versus [] check.
            # The curated placement records determine the schema edge; this is
            # deliberately independent of the implementation's validator.
            for placement in record['placements']:
                parent = placement['pointer'] + '/' + field
                for child in fields.values():
                    for child_placement in child['placements']:
                        location = child_placement['pointer']
                        if location == parent or location.startswith(parent + '/'):
                            names.update(w['case'] for w in child_placement['acceptance_witnesses'])
                            names.update(b['case'] for b in child_placement['rejection_bindings'])
        # Acceptance records include omitted/enum cases and the exact model
        # placements. Meaning-only rules still need their role-aware procedure.
    if identifier.startswith('IP-Person-'):
        duty('person-table')
        mode = 'response-table'
    if identifier.startswith('IP-ActorObject-') or identifier == 'IP-agent-object-discriminator':
        for name in cases:
            if re.match(r'(agent|identified-group|anonymous-group)-(object|object-object)-objectType-', name):
                names.add(name)
        mode = 'object-discriminator'
    if identifier.startswith('IP-resource-') and identifier != 'IP-resource-table-validation':
        resource, method = identifier.removeprefix('IP-resource-').rsplit('-', 1)
        path = '/' + resource.replace('-', '/')
        for name, case in cases.items():
            if any(s['request']['path'] == path and s['request']['method'] == method.upper() for s in case.get('steps', [case])):
                names.add(name)
        mode = 'resource-method'
    if identifier.startswith('IP-request-header-') or identifier == 'IP-response-header-context':
        duty('contextual-headers')
    if identifier.startswith('IP-parameter-') and identifier not in DIRECT:
        duty('parameter-structure')
    if identifier.startswith('IP-table-'):
        duty('all-table-validation')
    if identifier.startswith('IP-equivalence-ignore-'):
        duty('logical-equivalence-exemptions')
    if identifier.startswith('IP-interaction-pattern-'):
        duty('interaction-vocabulary')
        procedures.add('interaction-validation-policy')
    if identifier.startswith('IP-error-'):
        duty('appropriate-errors')
        code = identifier.removeprefix('IP-error-')
        if code.isdigit():
            names = {name for name in cases if any(s['expect']['status'] == int(code) for s in cases[name].get('steps',[cases[name]]))}
    for family in ['state', 'agent-profile', 'activity-profile']:
        prefix = 'IP-' + family + '-'
        if not identifier.startswith(prefix) or identifier in DIRECT:
            continue
        suffix = identifier.removeprefix(prefix)
        if suffix == 'last-modified':
            duty(family + '-modified-header')
        elif suffix == 'since-exclusive' or suffix.startswith('get-list'):
            duty(family + '-list')
        elif suffix.startswith('merge'):
            duty(family + '-merge-rejection' if 'invalid' in suffix else family + '-merge')
        elif suffix.startswith(('delete-all', 'delete-list')):
            duty('state-delete-all')
        else:
            # Exact method/parameter selector witnesses, plus resource lifecycle
            # cases for the scope/dispatch/body behavior.
            selected = suffix.replace('get-one', 'get').replace('delete-one', 'delete')
            names.update(name for name in cases if name.startswith(family + '-' + selected))
            if suffix.startswith(('put', 'post')) or suffix == 'dispatch':
                duty(family + '-write-contract')
            elif suffix.startswith('get-one'):
                duty('state-single-get' if family == 'state' else family + '-get')
            elif suffix.startswith('delete-one'):
                duty('state-single-delete' if family == 'state' else family + '-delete')
        if suffix == 'post-body':
            # A request-body contract includes the POST effect. PUT lifecycle
            # witnesses alone cannot detect discarded merge/create bodies.
            names.update(family + '-' + n for n in [
                'merge-replaces-top-level-object', 'post-creates-absent-document'])
            procedures.add('issued-document-time-and-atomicity')
        if family == 'state' and suffix == 'post-registration':
            names.update('state-registration-' + n + '-with-valid-selectors-post'
                         for n in ['valid', 'invalid'])
            procedures.add('issued-document-time-and-atomicity')
        mode = 'document-method-and-parameter'
    if identifier in ('IP-response-version-header', 'IP-response-version-value'):
        names.update(['statement-unknown-local-offset-policy', 'deterministic-rate-limit-profile-returns-429'])
        procedures.add('related-agents-returned-authority')
    if identifier == 'IP-consistent-through':
        names.add('statement-unknown-local-offset-policy')
        procedures.add('related-agents-returned-authority')
    # Advisory/permission observations retain their source strength; this link
    # does not make one canonical-metadata or cursor strategy mandatory.
    if identifier in (
        'IP-canonical-update-authority',
        'IP-canonical-small-corrections',
        'IP-canonical-major-changes',
        'IP-more-rerun',
        'IP-more-cache',
        'IP-more-cache-voids',
    ):
        procedures.add('canonical-metadata-and-cursor-policy')
    return sorted(names), sorted(procedures), mode
