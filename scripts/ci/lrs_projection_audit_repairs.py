"""Concrete SHA-2 partitions added after the issued-ledger cross-review.

These exercise the xAPI boundary, not full cryptographic primitive certification.
The independent standard-library digest is derived from exact fixture bytes.
"""
import copy
import hashlib
import json

from generate_lrs_projection_cases import case, expect, request, step, uuid_for
from lrs_projection_behavior import S
from lrs_projection_issued import storage_case

DATA = b'audit attachment\x00with CRLF\r\nend'


def multipart(body, data, digest, boundary='issued-sha2-boundary'):
    return (b'--' + boundary.encode() + b'\r\nContent-Type: application/json\r\n\r\n'
        + json.dumps(body, separators=(',', ':')).encode() + b'\r\n--' + boundary.encode()
        + b'\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary'
        + b'\r\nX-Experience-API-Hash: ' + digest.encode() + b'\r\n\r\n' + data
        + b'\r\n--' + boundary.encode() + b'--\r\n')


def attachment_digest_cases():
    cases, bindings = [], {}
    for algorithm in ['sha384', 'sha512']:
        digest = hashlib.new(algorithm, DATA).hexdigest()
        metadata = dict(usageType='urn:example:attachment', display={'en':algorithm},
            contentType='application/octet-stream', length=len(DATA), sha2=digest)
        common = ['IP-Attachment-sha2-value', 'IP-Attachment-sha2-type']
        name = 'issued-audit-' + algorithm + '-fileurl'
        body = dict(copy.deepcopy(S), id=uuid_for(name),
            attachments=[dict(metadata, fileUrl='https://example.invalid/' + algorithm)])
        vector = storage_case(name, body, 'put', ['xapi-lrs-4.1.3-attachment-transmission-criteria'])
        # Explicitly inspect the digest even if a broad preservation comparator
        # later grants latitude to another Attachment property.
        vector['steps'][-1]['expect'].setdefault('json', {}).setdefault('pointers', {})['/attachments/0/sha2'] = {'equals':digest}
        cases.append(vector)
        bindings[name] = common + ['IP-json-attachment-admission', 'IP-return-serialization']
        for wrong_data in [False, True]:
            name = 'issued-audit-' + algorithm + ('-raw-mismatch' if wrong_data else '-raw-match')
            body = dict(copy.deepcopy(S), id=uuid_for(name), attachments=[copy.deepcopy(metadata)])
            data = bytes([DATA[0] ^ 1]) + DATA[1:] if wrong_data else DATA
            raw = multipart(body, data, digest)
            req = request('PUT', '/statements', {'statementId':body['id']}, raw_body_hex=raw.hex())
            req['headers']['Content-Type'] = 'multipart/mixed; boundary=issued-sha2-boundary'
            steps = [step('store-digest-mismatch' if wrong_data else 'store-valid', req,
                          expect(400 if wrong_data else 204))]
            if wrong_data:
                steps.append(step('immediate-not-stored', request('GET', '/statements',
                    {'statementId':body['id']}), expect(404)))
            else:
                steps.append(step('metadata-readback', request('GET', '/statements',
                    {'statementId':body['id'], 'format':'exact', 'attachments':'false'},
                    await_retrieval_ms=5000), expect(200, {'/attachments/0/sha2':{'equals':digest}},
                    checks=[{'kind':'json-preservation','expected':body}])))
                steps.append(step('raw-digest-readback', request('GET', '/statements',
                    {'statementId':body['id'], 'attachments':'true'}, await_retrieval_ms=5000),
                    expect(200, checks=[{'kind':'multipart','expected_data':[DATA.decode('utf-8')]}])))
            cases.append(case(name, ['xapi-lrs-4.1.3-multipart-reject-unmatched' if wrong_data
                else 'xapi-lrs-4.1.3-multipart-accept-valid'], steps=steps))
            bindings[name] = common + ['IP-attachment-match','IP-multipart-match-sha2']
            if not wrong_data:
                bindings[name] += ['IP-return-serialization', 'IP-attachment-response']
    return cases, bindings


def audit_scenarios():
    tests = []

    def add(name, rules, fixture, steps, assertions):
        tests.append(dict(id='issued-' + name, rules=['IP-' + r for r in rules.split()],
            level='clause-specific', fixture=fixture, steps=steps, assertions=assertions))

    add('sha2-attachment-boundary',
        'Attachment-sha2-value Attachment-sha2-type json-attachment-admission attachment-match multipart-match-sha2',
        'Independent SHA-224/256/384/512 implementation (FIPS180-2 Change Notice 1), exact bytes 61 75 64 69 74 20 61 74 74 61 63 68 6d 65 6e 74 00 77 69 74 68 20 43 52 4c 46 0d 0a 65 6e 64 and independent MIME parser. Record algorithm, bytes and digest before submission.', [
        'For SHA-224 use the separately transcribed DC-MIME rrf07-sha224-full-output vector (raw abc, 3 octets, published full 56-hex digest). Its independent wrong-raw, wrong-hash and wrong-length branches have one cause each; no digest or crypto fixture is generated during definition work.',
        'For SHA-224, SHA-384 and SHA-512 separately, PUT a fresh otherwise-valid Statement with fileUrl, correct length, contentType and the independently calculated sha2. Require 204, then GET exact with attachments=false and check the submitted digest and metadata. No fetch of fileUrl is required to validate its syntax or preserve the supplied digest.',
        'For each algorithm repeat without fileUrl using multipart: first JSON section, raw bytes, matching X-Experience-API-Hash and binary transfer encoding. Require 204. Read JSON metadata and attachments=true; independently hash returned bytes and require matching metadata/hash header and original bytes.',
        'Change only the first raw byte, retaining byte count, contentType, original metadata digest and hash header. Require 400. Also exercise an independently changed same-length declared hash with the original bytes. Keep digest correspondence distinct from a length/media-type fault.',
        'Use SHA-256 as the existing baseline; SHA-256-only or SHA-384-only acceptance cannot replace the other permitted algorithm partition. Apply issued-rejected-batch-settlement when testing rejection within a batch.'
        ], ['Valid SHA-224, SHA-384 and SHA-512 cannot be rejected merely because their digest strings are not 64 hex digits.',
            'Metadata preservation, raw digest matching and output byte integrity are separate assertions.',
            'Strict submitted-metadata preservation is the recorded OPEN-EQUALITY/POL-EQUALITY-OUTPUT reading. Admission of valid algorithms and consistency between returned raw bytes, digest and metadata remain independently checked.',
            'These are xAPI integration partitions, not full FIPS certification or a requirement to retrieve fileUrl.'])

    add('rejected-batch-settlement', 'batch-atomic-rejection batch-duplicate-id',
        'Isolated LRS with an independent ingestion fixture. It supplies a request-correlated completion receipt proving all commit work and visibility queues for that request have drained (including rejected-request tasks), or a durable-state snapshot at an equally proven quiescent point. A release signal alone, an immediate 404, and an arbitrary sleep are not completion evidence. A harness timeout bounds the experiment, not IEEE visibility latency.', [
        'Capture baseline durable state and choose fresh ids A/B/C. Submit an all-valid three-Statement POST control. Release and drain the fixture; after the correlated completion receipt, require success and retrieval of all three, or confirm them in the independent durable snapshot. An ineffective/unavailable fixture makes the settlement branch NOT-RUN.',
        'In a new isolated namespace POST [valid A, invalid B with verb.id=42, valid C]. Require 400. Record immediate GETs as provisional observations only. Release and drain all request-associated work, record the completion receipt, and then require all three ids absent and no batch-related mutation of the baseline durable state.',
        'Repeat with [S,S] sharing both id and body, and with the same id but differing bodies. Require 400 and, after the same settlement evidence, absence of every submitted id. Include the multipart missing-data middle from issued-multipart-contract step 6.',
        'Use negative fixtures that commit a valid sibling synchronously and that queue it until release after the 400. Both must FAIL on the settled absence or durable snapshot comparison. A correct atomic rejection passes. The accepted control prevents a no-op barrier or always-empty query fixture from masquerading as proof.',
        'If completion cannot be established before the harness budget, record NOT-RUN for settled absence, retaining any already observed mandatory status or storage violation as FAIL. Never turn a missing fixture, timeout or a sequence of 404 responses into PASS.'
        ], ['The whole-batch rejection assertion includes valid siblings before and after the invalid member.',
            'A completion receipt binds the batch request and covers asynchronous commit as well as visibility; a timestamp or release acknowledgement alone is insufficient.',
            'The same explicit settlement condition governs JSON and multipart batches. No fixed protocol deadline is invented.'])

    add('about-error-cross-product', 'about-version-exemption',
        'Isolated About GET/HEAD endpoint; independently declared supported and unsupported version values; credentials accepted for About; a fixture declaration that parameter auditUnknown is unrecognized by this deployment. Retain that declaration separately from responses. Optionally provide independently refused credentials. If a non-version cause cannot be established, its branch is NOT-RUN.', [
        'For GET and HEAD, send otherwise-valid About requests with a supported header and a declared unsupported header. Require 200 for both; version alone is not a rejection cause. GET validates required About metadata; HEAD requires independently mandatory xAPI headers and no body; corresponding entity metadata is a SHOULD observation.',
        'Repeat the supported and unsupported header requests with auditUnknown=1, whose nonrecognition was independently declared. Require 400 for the unknown parameter in both cells. The version exemption must not disable this independently applicable validation. Capture the cause declaration, request bytes, status, headers and body.',
        'Remove the version header and repeat both otherwise-valid and declared-unknown-parameter requests. Retain missing-header disposition and cause observations. If absence itself is rejected, do not infer whether parameter validation ran from the shared 400. Its exact missing-header/competing-cause oracle is source-indeterminate under OPEN-VERSION/POL-VERSION-MISSING until a disposition is recorded.',
        'If refused credentials are available, establish the actual refusal independently and compare supported/unsupported/absent headers. Enforce the applicable authentication rule when refusal is established; record unresolved competing-cause priority separately rather than requiring an invented universal order.',
        'Use controls that reject valid About solely for the unsupported header and that exempt unknown parameters only under that header. The first fails step 1; the second fails step 2. Required response headers and HEAD body rules remain graded independently in every cell.'
        ], ['The version exception changes only version-based rejection; an independently established non-version requirement is not waived.',
            'Common oracles are graded; missing-header and unestablished cause-priority cells remain explicitly INDETERMINATE. Unknown fixture cause is NOT-RUN, not an interpretation choice.',
            'POL-ABOUT response-property questions remain separate from version and request-validation causes.'])

    add('actor-order-exception', 'preserve-actor equivalence-ignore-group-member-order',
        'An otherwise-valid Statement with a Group actor containing two distinct member IFIs A/B, and the actual comparator adapter from issued-logical-comparator.', [
        'Apply table-driven-response-preservation rubric 2 to supplied and returned actor values: [A,B] and [B,A] preserve the member multiset. Removing A or B, adding a member or changing an IFI is a substantive change and must fail that preservation comparison.',
        'Apply issued-logical-comparator step 2 to pairs differing only in Group member order; the general comparator returns true. Retain substantive negative controls from its step 1. If no actual comparator adapter is available this operation is NOT-RUN, not inferred from a replay status.',
        'For stored-id replays follow issued-replay-state-snapshot: comparator order latitude is not permission to mutate stored state. Keep HTTP status advice separate from the required state invariant.'
        ], ['Member order and member identity/multiplicity are separate comparisons.',
            'Output preservation, logical comparison and replay state each use the referenced operation-specific oracle.'])

    add('object-preservation-exception', 'preserve-object equivalence-ignore-referenced-activity-definitions',
        'Complete Statement pairs with an Activity Object id and two different valid definitions; independent maintained Activity metadata; an actual comparator adapter when available. Only legal Object/SubStatement placements are used.', [
        'For logical comparison use issued-logical-comparator steps 1–2: changing Activity id is false; changing only referenced Activity definition is true under general 4.2 comparison. This does not adjudicate signature applicability (POL-EQUALITY-SIGNATURE).',
        'For storage/readback use table-driven-response-preservation rubrics 1–4 and issued-metadata-language-inventory step 2. Check exact/default submitted values, ids-format identifying projection, and canonical definitions against the independent maintained inventory. Explicit normalization rules remain enforced.',
        'Record broader exact-output latitude separately under OPEN-EQUALITY/POL-EQUALITY-OUTPUT. Known specific transformations have their own oracles; a cell whose permitted latitude has not been determined is source-indeterminate, not permission to omit or rewrite arbitrary Object content.',
        'For replays use issued-replay-state-snapshot and require unchanged stored state. A comparison exclusion alone cannot authorize replay mutation or a change of Object identity. Repeat nested operations only where the field is legal; root-only id/stored/authority/version do not become positive SubStatement fields.'
        ], ['Activity identity, comparison-only definition exclusion, rendering format and replay immutability are distinct.',
            'Unresolved output latitude and signature scope remain explicit; no blanket output omission is introduced.'])
    return tests
