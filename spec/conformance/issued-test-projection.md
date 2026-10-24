# Issued LRS ledger test projection

The pinned IEEE Std 9274.1.1-2023 catalog and merged detailed ledger in
[the baseline](issued-ledger-baseline.json) define the responsibility set.
The projection connects 673 rules and 103 exceptions to 17,933 HTTP cases and
145 procedures. Its reproducible indexes preserve each rule's actor, normative
strength, conditions, exception branches and interpretation/dependency boundaries.

## Definition sources

- [Duty and assertion mappings](lrs-duty-review.yaml) connect ledger duties to
  concrete observations; [procedure definitions](lrs-review-tests.yaml) and
  [issued procedures](issued-test-procedures.yaml) supply controlled scenarios.
- [Interpretation and dependency contracts](issued-definition-contracts.json)
  specify 57 branches, Kiva policy choices, source interpretations, equality and
  output matrices, version/error-cause combinations and document preconditions.
- [Contract vectors](definition-contract-vectors.yaml) define positive, negative
  and boundary expectations for twelve incorporated primitive contracts.
- [Source anchors](issued-definition-source-anchors.json) preserve derivation
  coordinates. Historical private source locators are provenance, not review
  approval or inputs fetched by the static generator.
- [Source residuals](issued-test-definition-source-residuals.json) retain the
  two unresolved topics and three branches under the
  [source residual policy](issued-test-definition-completion-amendment.md).
  Kiva choices do not resolve source conflicts or replace IEEE allowed sets.

[Reproduction instructions](generated-artifacts.md) materialize the generated
[projection](issued-test-projection.json), [matrix](issued-test-matrix.md),
assertion manifest, cases and procedures. Git tracks their authored inputs,
generators and exact lock, rather than the expanded files.

## What the checks establish

The normal `--require-complete` gate and Nix `conformance-plan` check structural
ledger/test correspondence, concrete conditions and exceptions, reverse case
coverage and artifact identity. They do not consume local review or adoption
verdicts. The `review_binding_sha256` field retains its historical schema name;
it binds a rule, its obligations, case assertions and procedures, not an approval.
`test_definition_projection_complete` describes structural coverage, while
`semantic_coverage_complete` remains false because code cannot infer semantic
approval from hashes.

Official review is the GitHub PR merge. Interim review receipts, progress reports,
acceptance tables and adoption states belong outside the repository. Older records
remain available in Git history; they do not govern the current checks. There is
no separate local adoption step required to make `conformance-plan` pass.

## Oracle and execution boundaries

Timestamp checks distinguish definite format/calendar/UTC violations from legal
forms unsupported by a bounded parser. Preservation validates both operands before
deferring their comparison. A previously retrieved Statement violates the
Consistent-Through disappearance guarantee only when the watermark is strictly
later than its stored instant; equality does not establish that guarantee.
Direct, list, continuation-page and multipart retrievals retain returned `stored`
evidence under the effective credentials and selector. List results establish
ordinary `statementId` visibility, not `voidedStatementId` visibility.

For POSTs with submitted IDs, `statement-ids` checks the response UUID array in
request order, allowing hexadecimal case differences. Readback uses the submitted
IDs. Server-assigned IDs are captured only when absent from the submitted body,
after checking the response array's size and UUID syntax.

`statement-query` requires bounded continuation collection. Each page respects
the requested limit; the observed union preserves captured stored times, query
boundaries, uniqueness and order. Complete membership requires a terminal page.
A missing continuation response is a violation; transport or page-budget limits
remain unavailable, while already observed violations remain FAIL.

Numeric preservation retains exact input information and permits legitimate
binary32 endpoint serializations. A shared rounded bin alone is insufficient.
A representation beyond an adapter's exact-input capability is NOT-RUN rather
than a passing preservation result. The full relation is specified in DC-NUMBER.

HTTP media types preserve token/quoted-string semantics and case-sensitive
parameter values where required. Multipart comparisons also retain framing,
boundary and transfer-encoding constraints. Definite violations survive unavailable
sibling checks. Producer duties, recommendations and permissions keep their own
strength; they are not promoted into mandatory LRS rejection rules.

Signature cases requiring `signature-validation-context` remain NOT-RUN without
that provider. A bundled self-signed certificate does not establish trust.
Other delegated capabilities and environment fixtures must also be supplied and
verified before their execution can be complete.

Execution reporting requires complete bound input/output evidence separately
from structural coverage. The [CI known-gap policy](../../docs/architecture/conformance-claim.md)
checks exact regression outcomes while preserving raw FAIL and NOT-RUN results.
A successful integration check does not establish LRS conformance. Hosted artifact
collection is described in [the CI guide](hosted-conformance.md).
