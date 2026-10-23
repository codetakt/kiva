"""Executable discrimination procedures for every adopted OPEN/POL question.

These procedures preserve uncertainty; they do not authorize arbitrary behavior.
Common explicit requirements remain mandatory. Disputed observations cannot be
used as an unqualified IEEE PASS/FAIL until the particular choice is recorded.
"""
# Historical seed templates below are not current unresolved-status authority.
# interpretation_tests always applies the explicit current branch definitions;
# their oracle/project_policy/source_residual partition supersedes these seeds.
# id: concrete experiment, undisputed oracle, competing readings/remaining evidence
EXPERIMENTS = {
'OPEN-EXT': (
 'Run the pinned identifier corpus and root/nested duration, timestamp and numeric boundary vectors through the actual runtime integration. Record runtime/package/version, delegated clause, input bytes, output and adapter identity.',
 'Explicit xAPI acceptance, rejection, normalization and precision requirements remain enforced at the integration boundary.',
 'Full external primitive-domain certification is delegated, per project scope; unknown external edge grammars require versioned provider evidence, not an invented xAPI oracle.'),
'OPEN-PUT': (
 'Use identical otherwise-valid Statement S in PUT body S, PUT body [S] with statementId, and POST body [S]. Repeat JSON and multipart first-part arrays.',
 'Single-Statement PUT and batch POST have the specified successful responses and retrieval; known replay state must not mutate.',
 'General batch wording suggests PUT arrays; method-specific wording describes a single Statement. Record both candidate outcomes for array PUT without changing the single/POST oracles.'),
'OPEN-RELATED': (
 'Store A with Activity only in contextActivities and B with Agent only in Context; query related_activities=true/false separately with Activity and Agent selectors and disjoint control ids.',
 'Default false and direct Object/Actor matching remain independently testable.',
 'Chapter 5 broadens Activities; chapter 4 repeats Agent language. Keep results under both readings and the existing project choice distinguishable from issuer correction.'),
'OPEN-LASTMOD': (
 'GET Statements for a unique unmatched verb; retain status, statements=[], Last-Modified presence/value and watermark. Compare with a one-Statement result whose stored is known.',
 'Empty result is 200 with statements []; singleton Last-Modified equals its stored time at HTTP-date precision.',
 'There is no maximum stored in the empty set. Record missing, epoch, current or other header conventions without asserting one is the IEEE-defined maximum.'),
'OPEN-RESULT': (
 'With three matching Statements and maximum one, traverse more until exhausted; record envelope fields, relative/absolute link form, empty/absent terminal more and delayed access after the declared continuation lifetime.',
 'Additional results have a usable IRL; required statements/results and void suppression remain tested.',
 'Exact terminal/envelope/relative-link and service-lifetime details lack the old defining table. No imported 24-hour mandatory minimum; retain each observed choice.'),
'OPEN-PRECONDITION': (
 'Across State and both Profile resources run existing/missing document x PUT/POST/DELETE x absent/If-Match(current,stale,*)/If-None-Match(current,other,*). Keep method/headers and conditional trigger receipts.',
 'Existing unconditional PUT gives 409 with meaningful retry instructions and no change; a failing applicable supplied precondition gives 412 and no change.',
 'Other headerless methods and the scope of inherited RFC If-None-Match semantics need method-specific interpretation. State is not automatically exempted from the explicit PUT rule.'),
'OPEN-STATE': (
 'Create two State ids. GET with neither stateId nor since, with since only, with stateId only, and with both; retain membership and response shape.',
 'Single id returns its document, and since is not used with stateId for single-document retrieval.',
 'Dispatch text selects list by missing id; the Required table cell conflicts. The list interpretation must be identified on corresponding test results.'),
'OPEN-GROUP': (
 'Repeat State and Agent Profile write/read/list/delete using a real Agent, an Identified Group with IFI and members, the same IFI without members and changed members, then an Anonymous Group.',
 'Ordinary Agent selectors work. If Identified Group access is supported, same IFI addresses the same document regardless of member omission/order/content.',
 'Chapter 4 Agent-only conflicts with chapter 5 Identified Group support. Do not infer Anonymous Group support from either reading.'),
'OPEN-ACTIVITY-LIST': (
 'Populate Activity Profiles under two Activities; list with activityId, agent, both and neither. Use distinct Agent and Activity scopes so the result cannot pass accidentally.',
 'Single-document Activity Profile access and isolation have their independent known contract.',
 'Chapter 4 list cell names agent; chapter 5 and the resource description name activityId. Record the chosen list oracle as interpretation-dependent.'),
'OPEN-EQUALITY': (
 'For paired complete Statements change only root id, timestamp, authority, stored, version, attachments, Activity definition, Group order and string case; repeat nested fields separately. Run comparator, replay and output paths independently.',
 'Replay cannot mutate any stored state; specific default/UTC/precision and mandatory data contracts remain distinct from comparison.',
 'Broad exclusions may differ in field/nesting/operation scope. A comparator exclusion never automatically settles output permission; retain separate POL cases.'),
'OPEN-FORMAT': (
 'For IRI/IRL/UUID fields and corresponding query parameters submit empty, no-scheme, syntactically malformed, well-formed-unreachable and well-formed-unassigned values. Exercise valid controls in each schema role.',
 'Valid values and mandatory types remain testable; IRI identifiers need not resolve over a network.',
 'Mandatory format rejection coexists with the IRI/UUID validation waiver. Separate syntax from real-world assignment and record any strict project oracle without calling the conflict resolved.'),
'OPEN-MAY-NOT': (
 'Serialize the identical JSON Statement with alphabetical, reverse and randomized property ordering, under fresh ids; repeat duplicate member controls separately.',
 'Parsed data and actual duplication are distinguishable; capture order as bytes before parsing.',
 'The negated may wording admits disputed modal readings. The project prohibition reading is not an issuer correction; do not erase the input-order discriminator.'),
'OPEN-ARRAY': (
 'At root and nested contextAgent/contextGroup use relevantTypes omitted, [], [one IRI], [two IRIs]. Separately test empty Group members, correctResponsesPattern and each component list; do not share an assumed minimum.',
 'Omitted relevantTypes is permitted generic relationship; valid typed entries remain valid and malformed item types do not become acceptable.',
 'One-or-more table wording conflicts with zero-or-more relationship prose. Cardinality is adjudicated per field, not by a universal nonempty-array rule.'),
'OPEN-EXTENSION-DUP': (
 'Send raw JSON duplicate names at core Statement, at extensions-map keys and inside an extension-owned object. Pair each with an ordinary nonduplicate opaque object containing null, arrays and xAPI-like names.',
 'Core duplicates reject; ordinary extension-owned JSON values remain opaque and accepted within whole-message/security limits.',
 'Repeated names inside extension-owned data need a parser/scope judgment. Do not deserialize away the duplicate before testing or silently apply the core rule everywhere.'),
'OPEN-LANGUAGE': (
 'Run overlong/empty language tokens, valid en/ja/zh-Hant/private-use/grandfathered forms and well-shaped registry-unknown tags at every language-map placement. Record the registry snapshot if used.',
 'The LRS token-length floor and string map values have definite oracles.',
 'The LRP invalid-tag wording is a source defect; full registry-level rejection needs separate evidence. Keep sender and receiver duties distinct.'),
'OPEN-IRI-STRENGTH': (
 'Inspect a controlled producer identifier assignment log: reuse versus new id, owned versus unowned namespace, resolvable versus nonresolvable IRI. Submit valid nonresolvable identifiers to the LRS.',
 'No LRS ownership or network-reachability rejection requirement follows from producer governance advice.',
 'The producer best-practice preamble gives should the force of shall despite modal definitions. Preserve producer-strength alternatives and do not impose them on the receiver.'),
'OPEN-SUBTIME': (
 'Submit a root timestamp T and a SubStatement without timestamp at independent recording time R different from T. Read the nested object, then repeat with an explicit nested timestamp U.',
 'Omitted nested timestamp is populated with a valid UTC timestamp; explicit U is retained under allowed normalization; no nested stored property is invented.',
 'Whether default is T, R or another justified time is unspecified. Capture it without requiring equality to top-level stored.'),
'OPEN-VERSION': (
 'On ordinary resources and About separately vary absent, 2.0, 2.0.0, 2.0.1, 1.0.3 and 2.1.0 headers; independently vary body version absent, 2.0, 2.0.0 and another accepted version.',
 'Header 2.0 alias, required unsupported-version rejection with explanation, About header exemption, default body 2.0.0 and retention of accepted body version remain distinct.',
 'Missing header, all-patch acceptance and body short form are not inferred from one another. Record routing and admission choices without converting should-not-reject into unconditional acceptance.'),
'OPEN-JWS': (
 'With independently generated RS256/384/512 JWS, compare good/malformed x5c, mismatched signing key, tampered bytes, untrusted self-signed but mathematically valid certificate and chain variants. Keep raw base64url/certificate bytes.',
 'If a certificate is supplied, the signature must be checked against it. Malformed signatures reject; missing recommended x5c alone is not malformed.',
 'Transitive certificate/chain/encoding requirements and deployment trust need their exact calling scope; do not equate cryptographic verification with a mandatory public-CA trust policy.'),
'POL-EQUALITY-REPLAY': (
 'Store A with timestamp T, replay the same id with timestamp U through PUT and POST, and compare all stored-object snapshots. Repeat with only a nested timestamp changed.',
 'No replay mutates any state. PUT allows 204/409 and POST also 200; 409 comparison advice remains advisory.',
 'Literal root timestamp exclusion versus stricter compatibility comparison and nested scope are separate candidate comparator policies.'),
'POL-EQUALITY-SIGNATURE': (
 'Sign complete A with root id I/timestamp T; submit with only root id J or timestamp U changed. Pair with unchanged signed bytes, substantive actor changes and nested timestamp changes.',
 'Cryptographic checks, full pre-signature payload shape and definite actor/verb/object non-equivalence remain enforced.',
 '4.2 exclusions may or may not govern 4.2.6 logically equivalent; the missing cross-reference is not restored silently. Both candidate results stay recorded.'),
'POL-EQUALITY-OUTPUT': (
 'Round-trip supplied id, timestamp, authority, stored, version, Activity definitions and two ordered attachments in exact/default/canonical; compare with input bytes and independently known defaults.',
 'Specific overwrite/default/UTC/precision and replay invariance remain enforced individually.',
 'Strict preservation except explicit transformations versus broader 4.2 latitude affects output assertions. A project-preservation failure alone is not an unqualified IEEE verdict.'),
'POL-VERSION-MISSING': (
 'Repeat valid ordinary GET/PUT/POST/HEAD and About GET/HEAD without X-Experience-API-Version; retain route selection and compare with explicit 2.0.0 controls.',
 'Explicit supported-header behavior and required response headers remain checked.',
 'Ordinary default acceptance versus 400 policy, and About missing-header scope, remain separately recorded choices.'),
'POL-ABOUT': (
 'Inspect raw About response bytes for version/extensions, additional properties and repeated names before JSON parsing; validate known required fields independently.',
 'A version array containing the latest supported minor/patch for each supported major is required. Optional extensions is allowed.',
 'Closed versus extensible response table and duplicate-key scope need separate oracle decisions; removing the ADL prohibition is not itself permission or prohibition.'),
'OPEN-VOID-QUERY-WORDING': (
 'Store A, B voiding A, and C attempting to void B. Request A/B via statementId and voidedStatementId and query their shared namespace after each step.',
 'A is hidden from ordinary queries and explicit voidedStatementId retrieves A. B cannot itself become voided and remains eligible; accepted C remains eligible.',
 'The sentence about retrieving voiding Statements via voidedStatementId conflicts with surrounding rules. Retain that exact discriminator without relabelling B as voided.'),
}

# Expectations derived from the alternatives already stated above. These are
# conditional observation oracles, not adoption of any competing reading.
# A missing determinate cell is recorded explicitly instead of graded PASS.
READING_EXPECTATIONS = {
 'OPEN-EXT': ([], 'External grammar endpoints without pinned provider evidence are source-indeterminate; finite xAPI calling-boundary assertions still grade.'),
 'OPEN-PUT': ([('general batch wording includes PUT', 'An otherwise-valid [S] with its matching statementId is admitted as that reading permits.'), ('single-Statement PUT only', 'The array shape is not admitted by this reading; an exact rejection status needs its applicable error rule.')], 'The choice between the two readings is unresolved; neither changes ordinary S PUT or batch POST.'),
 'OPEN-RELATED': ([('chapter 5 Activity expansion', 'Activity-only Context matches are included with related_activities=true and excluded by the ordinary direct-only false control.'), ('chapter 4 literal Agent wording', 'Activity-only expansion is not established by this sentence; record observed membership without inventing an exclusion duty.')], 'The conflicting scope cannot be selected from the target result.'),
 'OPEN-LASTMOD': ([], 'The empty set has no maximum stored; no exact empty-result Last-Modified value follows from this rule.'),
 'OPEN-RESULT': ([], 'Exact terminal-more representation, relative-link base and lifetime require a disposition; usable continuation and ordinary result contents remain graded.'),
 'OPEN-PRECONDITION': ([], 'Headerless POST/DELETE and ambiguous inherited RFC cells need a method-specific disposition; existing unconditional PUT and an established failing supplied precondition have common oracles.'),
 'OPEN-STATE': ([('dispatch by absence of stateId', 'Without stateId the request lists ids; with since it returns only ids updated strictly after since.'), ('literal Required since list cell', 'A list request omitting since fails that required-parameter condition.')], 'List dispatch and required-since conflict remain unresolved; do not infer a selector exemption for single-document requests.'),
 'OPEN-GROUP': ([('chapter 5 Identified Group selectors', 'Same Group IFI selects the same document scope despite member omission/order/content.'), ('chapter 4 Agent-only selectors', 'Group selectors are outside this reading of the endpoint contract; record the observed response without deriving a universal rejection code.')], 'Neither reading grants Anonymous Group document access; the conflicting admitted domain remains unresolved.'),
 'OPEN-ACTIVITY-LIST': ([('Activity-scoped resource/chapter 5', 'activityId selects the independently populated Activity profile list.'), ('literal chapter 4 agent cell', 'Record agent-selector list behavior against the independently populated scopes; the source does not define enough to invent an Agent-to-Activity scope mapping.')], 'The selector and its scope are in conflict; successful shape alone does not resolve membership.'),
 'OPEN-EQUALITY': ([('general 4.2 comparator exclusions', 'For the actual comparator, changing only a listed excluded component leaves logical equality true; substantive actor/verb/Object identity changes remain false.'), ('output or signature reuse of comparison latitude', 'No output/signature permission follows without its operation-specific reading; evaluate the POL-EQUALITY procedures separately.')], 'Case, nesting, signature applicability and output latitude have different unresolved cells. Only legal placements may be used.'),
 'OPEN-FORMAT': ([('mandatory structural format reading', 'Otherwise isolated malformed IRI/IRL/UUID syntax is rejected under the applicable field/parameter rule.'), ('identifier-validation waiver reading', 'Omitting validation may admit a malformed identifier; record that permission, not a duty to accept every malformed value.')], 'Keep syntactic validation, assignment checks and reachability separate; the conflict does not waive String types.'),
 'OPEN-MAY-NOT': ([('prohibition on order restrictions', 'Otherwise identical valid property orderings are admitted.'), ('negated permission reading', 'The disputed sentence alone supplies no mandatory admission/rejection for changed order; retain observations.')], 'Issuer intent for the modal wording is unresolved; duplicate member names have a separate common rule.'),
 'OPEN-ARRAY': ([('one-or-more relevantTypes', 'A present empty relevantTypes list fails this cardinality reading.'), ('zero-or-more relevantTypes', 'A present empty relevantTypes list is admitted as the generic relation under this reading.')], 'Do not transfer either minimum to other array fields; unrelated empty-array cells need their own source basis.'),
 'OPEN-EXTENSION-DUP': ([('core duplicate-name rule extends into extension values', 'The raw duplicate inside the extension-owned object is rejected under this scope reading.'), ('opaque extension-owned value reading', 'Core vocabulary validation does not settle this nested duplicate; observe parser behavior without imposing acceptance.')], 'No scope reading is adopted; duplicate-preserving raw requests and ordinary opaque controls are required.'),
 'OPEN-LANGUAGE': ([], 'Registry-level rejection and defective producer wording need separate evidence. The token-length floor and String values are independently graded.'),
 'OPEN-IRI-STRENGTH': ([('producer best-practice preamble elevates should', 'A producer breach of the stated IRI governance advice fails that producer reading.'), ('ordinary modal should', 'The same producer observation is advisory under the modal reading.')], 'These readings do not add receiver ownership or network-reachability rejection.'),
 'OPEN-SUBTIME': ([], 'The populated nested instant need not equal root T or recording R without further source evidence; valid UTC population and explicit U normalization have common oracles.'),
 'OPEN-VERSION': ([], 'Absent headers, unlisted patch versions and short body versions have distinct unresolved cells; use explicit supported controls and actual accepted-body retention, not a blanket acceptance choice.'),
 'OPEN-JWS': ([], 'Full external certificate/chain grammar and trust scope require the missing normative evidence; supplied-certificate verification and definite malformed-signature controls remain graded.'),
 'POL-EQUALITY-REPLAY': ([('literal root timestamp comparator exclusion', 'Changing only root timestamp leaves comparator equality true.'), ('stricter compatibility comparator policy', 'The changed timestamp may be unequal under that recorded policy; do not call the policy an issuer correction.')], 'Nested comparator scope is unresolved. Both readings still forbid any stored-state mutation on replay; advisory 409 is not a Boolean comparator oracle.'),
 'POL-EQUALITY-SIGNATURE': ([('4.2 exclusions govern signature equivalence', 'A root excluded-field-only change remains logically equivalent for this stage, with cryptographic/shape checks held valid.'), ('signature equivalence does not import those exclusions', 'Record the field-specific signature decision; absence of the cross-reference alone does not specify every alternative equality rule.')], 'The cross-reference is missing; nested and case cells need a reading rather than unconditional accept/reject.'),
 'POL-EQUALITY-OUTPUT': ([('strict preservation except specific transformations', 'Unexplained changes to supplied fields fail this output reading after explicit defaults/overwrite/normalization are applied.'), ('broader 4.2 output latitude', 'Record field, placement and operation; the permitted transformation set is not established merely by choosing the label broad.')], 'Cells without an explicit permitted transformation are source-indeterminate, never blanket PASS for arbitrary output.'),
 'POL-VERSION-MISSING': ([('ordinary resource default admission policy', 'An otherwise-valid headerless ordinary request succeeds under the recorded default policy.'), ('ordinary resource reject-missing policy', 'That headerless request returns 400 under the recorded rejection policy.')], 'About missing-header scope and competing error causes remain separate, not implied by either ordinary-resource policy.'),
 'POL-ABOUT': ([('closed response table', 'An additional top-level property violates this adopted closure reading.'), ('extensible response table', 'Additional properties alone do not fail that reading; known required fields remain validated.')], 'Duplicate-key scope is a separate unresolved cell; removal of an ADL prohibition does not decide IEEE closure.'),
 'OPEN-VOID-QUERY-WORDING': ([('ordinary voided-target reading', 'voidedStatementId selects actually voided A; B remains retrievable ordinarily and is not relabelled voided.'), ('literal voiding-Statement sentence', 'Record the B lookup via voidedStatementId as the discriminator; preserve the non-voidability of B in other operations.')], 'The contradictory lookup wording cannot be resolved by treating B as voided.'),
}


def interpretation_tests(rules, issues):
    if {i['id'] for i in issues} != set(EXPERIMENTS) or set(READING_EXPECTATIONS) != set(EXPERIMENTS):
        raise ValueError('interpretation experiment inventory drift')
    tests=[]
    for issue in issues:
        probe, common, alternatives=EXPERIMENTS[issue['id']]
        readings, unresolved = READING_EXPECTATIONS[issue['id']]
        tests.append(dict(id='issued-discriminator-' + issue['id'], issues=[issue['id']],
            rules=[r['id'] for r in rules if issue['id'] in r['issues']], level='interpretation-dependent',
            fixture='Isolated endpoint and explicit project interpretation profile; retain raw requests/responses and any independent adapter evidence. No external adjudication is assumed.',
            steps=[probe, 'For field experiments separate comparator, replay and output and use only legal placements; root-only id/stored/authority/version are not valid SubStatement positives. Evaluate the undisputed common assertions separately. Record each disputed observation under every stated reading without selecting an interpretation from target behavior.'],
            assertions=[common, alternatives, 'Only the disputed verdict is INDETERMINATE until a versioned project/issuer disposition is attached. Common mandatory oracles remain graded; missing fixtures are NOT-RUN.'],
            oracle_partition=dict(common=dict(grading='applicable-rule-strength', expectation=common),
                conditional=[dict(reading=r, expectation=e, status='conditional-not-adopted') for r,e in readings],
                unresolved=dict(status='source-indeterminate', reason=unresolved),
                execution='definition-only; requires separate execution receipt')))
    from issued_definition_contracts import apply_interpretation_definitions
    return apply_interpretation_definitions(tests)
