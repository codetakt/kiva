# Conformance Endpoint Profiles

Kiva's xAPI LRS conformance aid uses the pinned IEEE Std 9274.1.1-2023
requirements baseline. It keeps generic HTTP cases portable by separating
implementation-specific trigger values from the test cases themselves.

Equivalence with the 2025 edition has not been established. Requirement
traceability alone does not establish implementation conformance; see the
[xAPI claim boundary](conformance-claim.md).

## MIME admission policy

MIME admission distinguishes the HTTP envelope grammar from RFC 2045 part
fields. Legal part comments, quoted pairs and CRLF folding are accepted; raw
attachment bytes remain opaque. Input attachments with no transfer-encoding
field follow the binary-default recommendation. Responses always emit binary.
An absent nonfirst Content-Type is retained as absent (the persisted internal
media-type string is empty), so no invented octet-stream type contradicts the
Statement metadata. Responses omit this optional field in that case.

Media comparisons use case-insensitive type and parameter names, text charset
values and message/external-body access-type values; multipart boundary values
remain case-sensitive. Only text/plain has the declared default US-ASCII here.
Identical decoded unknown parameters are accepted. Kiva rejects unsupported
parameter relations and duplicate parameters/fields as a local admission
policy. That rejection does not establish universal parameter inequality or
resolve the test oracle's conditional/unknown branch. Oracle checks still
preserve independently known failures when parameter knowledge is unavailable.

## Metadata and Person policies

The current metadata capability is deliberately limited to submitted information.
`GET /activities` merges first-known Activity properties from accepted Statements,
in recording order, adding missing properties and language entries. Extension
owner values and arrays are atomic. Later spelling or answer changes do not
overwrite earlier values in that resource. Exact Statement output retains each
submission; canonical Statement output selects languages from that Statement's
retained maps. Unauthorized submissions do not contribute information.

Kiva does not consume or host resolvable Activity metadata and has no deferred
metadata fetch worker. Fetch timing, outbound Accept preferences, fetched-field
retention and alternate hosted document handling are conditional optional branches,
not unexecuted mandatory fetch duties. Native controls use a reachable independent
JSON/HTML metadata host, establish this source-bound capability before requests,
and still verify mandatory Activity fallback and retained information. A quiet
network interval alone is not evidence of a permanent no-fetch capability.

The Person resource has no external directory or enrichment store. It projects the
requested Agent's IFI and optional name. Its native fixture establishes an empty
isolated repository before introducing an unrelated Agent. Independent HTTP peer
controls separately verify that the association oracle accepts multiple related
identifiers and names, rejects unrelated values, compares complete Account pairs,
and retains NOT-RUN when association evidence is unavailable. Those peers qualify
the oracle; they are not executions of a Kiva enrichment feature.

## Interaction policy

Interaction validation checks the interactionType vocabulary and presence,
common JSON validity, component/list/String types and Language Maps. It does
not interpret response-pattern grammar, grade answers, or validate characterstring
parameters. Invalid or unknown parameters remain supplied Strings. Whitespace-free
component identifiers are producer advice; the explanatory 250-character fill-in
description is not a receiver length limit. The separately controlled interpreter
in conformance fixtures checks list defaults and item-language positions; it is
not a Kiva grading feature.

## Implementation limits

`KIVA_XAPI_STATEMENT_PAGE_SIZE` optionally sets a positive maximum number of
Statements per response page. Omitted `limit` and `limit=0` use this maximum;
a positive request limit uses the smaller of the two values. Continuations
apply the same configuration after filtering and authorization. An unset
maximum preserves the existing unbounded default. Native graph fixtures set
the maximum to two and traverse every continuation in both directions.

### Statement read permissions

`KIVA_XAPI_STATEMENT_VISIBILITY_JSON` configures additional Statement read
permissions by the authenticated principal's verified subject. For example:

```json
{"default":[],"subjects":{"reader-a":["00000000-0000-4000-8000-000000000001"],"administrator":"all"}}
```

With no environment setting, the existing operation-level read permission grants
access to all Statements. When the setting is present, an omitted `default` denies
all IDs. A subject entry replaces that default; `"all"` permits every ID and an
array permits only its UUIDs. Invalid UUIDs, unknown fields and unknown selection
strings cause startup to fail. Anonymous requests use the default when anonymous
operation access is enabled. Configure stable subjects from the token verifier,
not bearer-token strings or client-supplied authority.

The policy applies to normal and voided single-Statement reads, GET/HEAD, list
results, continuation pages and returned raw attachments. An excluded single ID
returns the same 404 as an absent ID. Collection filtering precedes page limits;
each continuation authenticates again and applies its caller's current selection.
Global voiding/reference semantics are evaluated before permission filtering.
The policy does not change operation scopes, Statement write permissions, or the
Activity, Person and Document resource policies. It is a deployment selection
mechanism, not a universal claim about privacy of those separate resources.

Native conformance controls configure disjoint ID sets before ingestion, use an
external verifier with an independent decision log, and follow every continuation
with the original credential. They also try cross-user and unauthenticated cursor
reuse and compare actual multipart bytes against the authorized ID inventory.

Some xAPI LRS requirements specify the response status for implementation
limits, but not the concrete trigger threshold:

- Oversized Statements, documents, or attachments must be rejected with `413`.
- Requests rejected because of rate limiting must be rejected with `429`.

A fixed black-box suite cannot choose one universal payload size or burst count
that is correct for every LRS. Large arbitrary payloads can be slow or unsafe,
and timing-dependent rate-limit probes are flaky.

## Fixture Contract

The runner therefore accepts an endpoint profile with `--endpoint-profile PATH`.
The profile is a local conformance fixture declaration for the target LRS, not
an xAPI extension and not a production API contract.

The current profile schema is intentionally small. Its historical `standard`
value remains a compatibility identifier, not a claim about the reviewed edition:

```yaml
schema_version: 1
standard: IEEE/ISO/IEC 39274-1-1-2025
profiles:
  deterministic-too-large-threshold:
    max_request_body_bytes: 4096
    probe_body_bytes: 8192
  deterministic-rate-limit-trigger:
    request_count: 3
    window_seconds: 1
    path: /__kiva/conformance/rate-limit
```

`deterministic-too-large-threshold` says that the target LRS will reject the
runner's generated valid xAPI Statement body when its body length is at least
`probe_body_bytes`, because that exceeds the target's declared
`max_request_body_bytes`.

`deterministic-rate-limit-trigger` says that the target LRS will return `429`
for the final request when the runner sends `request_count` immediate requests
to `path` within `window_seconds`.

The bundled Kiva example uses a dedicated `__kiva` fixture path so the complete
bundled suite can run against one fresh in-memory server without exhausting the
standard `/about` resource. It also sets the Statement body limit high enough
for normal multipart and signed-Statement fixtures while keeping the generated
oversized probe deterministic.

The profile schema also accepts two declarations based on independent fixture
evidence:

- `declared-parameter-vocabulary`: `contexts` records a method, path, evidence
  and nonempty `unrecognized` parameter list. A rejection response cannot
  itself establish that the parameter is unknown to the target.
- `declared-resource-absence`: `case_evidence` maps each exact case ID to
  independent evidence that all negative selectors are absent for the request
  credentials immediately before and throughout that case. A fresh-server
  label or the response under test is insufficient. The runner checks evidence
  availability; fixture validity and integration are separate later work.

`signature-validation-context` declares independent evidence of the target's
RS256/384/512 policy, verification with supplied `x5c`, and acceptance without
cryptographic verification when no certificate is supplied. It enables the fixed
signature vectors; their actual responses determine the result. Certificate
trust-chain validation is a separate policy.

The Python oracle controls compare the five prerequisite names in the Rust
validator, Python case-matrix reporter and both profile/report schemas. This
guards duplicated vocabularies; it does not certify fixtures.

The example profile describes the disposable Kiva harness. Its vocabulary
declarations refer to the server's request allowlists. Its absence declarations
reserve the exact empty-result Agent and three missing document addresses; the
harness creates a fresh single-writer tenant and the suite never writes those
addresses. Do not reuse these declarations on an existing or shared LRS.

## Runner Behavior

### Signature provider configuration and scope

`KIVA_XAPI_CERTIFICATE_POLICY` supplies the path to local, operator-trusted JSON to the
signature validator. The synthetic policy under `tests/conformance/controls/`
is only a disposable test fixture: its private signing key is public. A received
`x5c` chain cannot add trust anchors or status records. Production deployments
provide their own anchors, initial policy set, intended signature service and
authenticated status evidence. Omitting this capability leaves supplied `x5c`
unverifiable and admission fails; it never makes an arbitrary root trusted.

Kiva selects authenticated out-of-band status. Each record binds the complete
certificate SHA-256, its issuer SPKI SHA-256, source provenance, an inclusive
validity interval and `good`, `revoked` or `unknown`. The source string documents
the trusted configuration channel; it does not authenticate data from a remote
request. Missing/wrong bindings, stale/future evidence and unknown status cannot
prove non-revocation. A current revoked result is distinguished from unavailable
or indeterminate evidence. The supplied interval is this provider's contract;
it is not the exclusive `nextUpdate` endpoint of a CRL.

CRL, delta-CRL, wire OCSP and network certificate discovery are not selected.
Their conditional DP, reason-mask, indirect-entry, delta-number and CRL-signature
vectors remain documented alternatives, not executed successes. The selected
out-of-band alternative still checks every path certificate. The anchor is an
external trust input and is excluded from the prospective path. Historical
validation is explicit through `validation_time`; ordinary execution uses the
current clock, never the Statement's timestamp.

The selected certificate signature provider accepts unrestricted RSA with
SHA-256, SHA-384 and SHA-512. SPKI `rsaEncryption` parameters are NULL; these
certificate signature algorithms accept NULL or absent parameters. PSS/OAEP
restricted keys are not converted into unrestricted keys. Payload JWS permits
RS256/RS384/RS512 and requires an actual RSA modulus of at least 2048 bits.
Original protected-header/payload bytes and original TBSCertificate bytes are
verified before logical comparison. No additional certificate algorithm is
silently declared supported. Legacy v1/v2 intermediates have no independent CA
confirmation service in this configuration and are rejected.

Canonical GeneralizedTime fractions are compared as exact decimal digits against
the validation clock, including fractions finer than nanoseconds. Neither the
typed parser's millisecond limit nor the path library's time representation
rounds those original boundaries. A separate typed view accommodates the
libraries; verification still signs the original TBS bytes, and the validated
leaf key supplies the payload verifier directly.

Certificate names use a separate comparison view. DirectoryString preparation
uses frozen Unicode 3.2, RFC3454 B.2, RFC4518 non-substring rules and erratum860;
UTF8String, PrintableString and BMPString are supported. Unsupported necessary
Teletex/Universal comparisons cannot establish a valid path. Unknown attribute
OIDs use typed binary comparison. DNS/domainComponent and mail local/host roles
retain their own rules. URI comparison normalizes syntax and the specified
HTTP/HTTPS/FTP/LDAP scheme defaults; reserved escapes and userinfo/query/fragment
remain significant. Name constraints compare the URI's DNS host separately.

The IDNA2003 provider uses Unicode 3.2 Nameprep, explicit AllowUnassigned=false
and STD3 settings, and only the RFC3492 Punycode primitive from `idna`; it never
calls that package's IDNA2008/UTS46 mapping. Certificate IRI producer conversion
performs NFC and UTF-8 escaping before IDNA host comparison. These producer and
display roles do not become new blanket xAPI HTTP rejection rules. Certificate
normalization never modifies stored Statements or their IRI equality strategy.

No-x5c D-10=A distinguishes unknown key (`not-evaluated`) from independently
configured known key (`verified` or `failed`). A failed known-key signature is
rejected. D-11 selects rejection of duplicate decoded JOSE member names; unique
unknown noncritical members remain permitted. These are Kiva policy choices,
separate from mandatory x5c/path/payload checks and external source adjudication.

Cases that need these fixtures declare `requires_endpoint_profiles`. Validation
does not require a profile file, so the suite remains statically checkable in
CI. A case with an unavailable required profile is NOT-RUN before sending any
requests; it does not become a failed LRS observation.

The runner exposes profile values as case-local captures:

- `kiva_endpoint_profile_too_large_statement_body`
- `kiva_endpoint_profile_rate_limit_request_count`
- `kiva_endpoint_profile_rate_limit_window_seconds`
- `kiva_endpoint_profile_rate_limit_path`

Profile captures are only valid in cases that declare the corresponding
`requires_endpoint_profiles` entry.

## Isolation

### Document conditional requests

Kiva selects D-04=A for the combinations that the incorporated RFC 2616 leaves
undefined: If-Match with If-None-Match or If-Modified-Since, and
If-Unmodified-Since with If-None-Match or If-Modified-Since. Otherwise valid
requests receive `400` with `x-kiva-error: undefined-precondition-combination`.
The diagnostic also accompanies HEAD, whose response has no body. This is
Kiva's selected policy, not a claim that IEEE mandates that status.

Ordinary ETag lists, repeated list-valued fields and the defined combination of
If-None-Match with If-Modified-Since retain RFC 2616 processing. Reads permit weak
If-None-Match comparison; writes and If-Match require strong comparison. Date
conditions compare at HTTP-date second precision. Cache reads can return `304`.
Authentication, request syntax and invalid JSON merge inputs retain their own
errors before the mixed-condition policy. POST checks the existing JSON shape
and its conditional mutation under the same repository lock or transaction.

The native control records all three Document resources, both existence states,
methods, candidate lists, date interactions and unchanged bytes, media type,
ETag and precise update time after rejection.

### Kiva production limits used by the native adapter

`KIVA_XAPI_LIMITS_JSON` optionally configures limits for the authenticated subject.
For example:

```json
{
  "default": {
    "statement_body_bytes": 4096,
    "document_body_bytes": 64,
    "attachment_bytes": 32,
    "rate": {"accepted": 2, "window_seconds": 60}
  },
  "subjects": {
    "larger-writer": {
      "statement_body_bytes": 8192,
      "document_body_bytes": 96,
      "attachment_bytes": 48,
      "rate": {"accepted": 3, "window_seconds": 60}
    }
  }
}
```

These small values are disposable verification fixtures. Omitted settings impose
no additional budget. A subject override inherits omitted fields from `default`;
subjects are resolved by the configured authentication provider, not by trusting
a client-supplied name. Tokens for the same subject share its budget. Anonymous
requests have a separate default bucket. All numeric settings must be positive;
unknown configuration fields and empty subject identifiers are rejected.

Statement limits count the entire HTTP entity, including multipart framing.
Document limits count the HTTP entity for each of the three Document resources.
Attachment limits count each raw MIME part body. The existing 2 MiB transport
extractor ceiling and `KIVA_MAX_STATEMENT_BODY_BYTES` also apply; satisfying one
limit does not bypass another. Every size-caused rejection returns 413, including
extractor rejections, with required xAPI response headers.

The subject rate budget spans authorized xAPI requests across routes and methods.
The first admitted attempt starts a fixed window. `accepted` attempts are allowed;
further attempts receive 429 until the window expires. Later payload errors can
consume an admitted attempt. Refused credentials and rate-denied attempts do not
consume another slot or extend the window. This opt-in setting is separate from
the historical `KIVA_RATE_LIMIT_REQUEST_COUNT` fixture knob, whose count includes
the final denied request. The native adapter controls the same production clock
dependency in a test build and restores an isolated configuration between probes.

Endpoint-profile-sensitive cases should run against disposable or explicitly
configured conformance targets. They are not expected to mutate accepted xAPI
state, but they deliberately exercise resource limits and rate-limit controls.

The standalone conformance aid should keep this contract generic. Kiva-specific
deployment values, throttling knobs, or server configuration should stay in
Kiva CI adapters and should not become part of the portable suite.
