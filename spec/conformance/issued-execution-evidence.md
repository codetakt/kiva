# Issued-test execution evidence

The test ledger defines what must be checked. Execution completeness is a
separate assertion that needs the exact definition and actual, current
evidence. A target FAIL is an executed observation; it does not imply a missing
test or a conforming product.

`--require-conforming` adds the implementation acceptance gate. It requires
complete execution, zero failing mandatory observations, a structurally valid
HTTP report checked against the entire current suite without gap exceptions,
consistent totals and retrieval attempts, a successful execution wrapper, and
current implementation/build/database input hashes plus server/runner binary
identities. `--require-complete` alone deliberately still permits observed FAILs.

The resulting `test_specification_conformant` flag describes the configured,
finite observation and binding gate. It does not assert semantic equivalence
between the implementation and every requirement. Source-directed review and
counterexample searches remain necessary, including combinations not enumerated
by the fixed suite. A known semantic violation remains a product gap regardless
of this flag. Provider delegation preserves the integration obligations: correct
inputs, configuration, invocation, result handling and admission decisions.

Every procedure also needs `definition_coverage`: an exact map from each leaf
JSON pointer returned by `definition_obligations(spec)` to
`{"sha256": "<current obligation digest>", "checks": ["<check id>"]}`.
The map covers setup, action, expected result, fixture, steps, assertions, rubric,
vectors and association controls when present. Missing/stale obligations and
unknown/duplicate check links fail the gate. A single PASS label cannot replace
the complete rubric. Links remain author evidence; their semantic adequacy needs
review and is not established by hashing.

After generating the locked artifacts and building release binaries, capture a
fresh backend execution with:

```sh
python3 -B scripts/ci/capture_issued_test_execution.py \
  --backend memory --out target/issued-execution/memory \
  --fixture tests/conformance/controls/kiva_procedure_fixture.json
```

Repeat for `postgres` in a fresh directory. The producer records current inputs
before and after the actual run, raw outputs, fixture and binary identities;
failed/incomplete observations are preserved. The local fixture's policy input
hashes must be checked against the implementation before use. The portable runner
executes 25 partial procedures. `run_lrs_native_controls.py` additionally records
the actual Rust/Python test inventory, binaries, toolchains, per-test outcomes
and native HTTP/control observations. `collect_lrs_procedure_evidence.py` combines
these with the full HTTP report through the explicit 145-procedure manifest.
It checks exact rubric leaves, dependency order and every selected witness;
missing executions or incomplete dependencies cannot be replaced by map counts.
Only exact named partial capability gaps can be superseded, retaining their
original observations. Actual failures and unrelated transport gaps remain.

`--workers 4` uses PR #4's isolated HTTP shards and preserves their raw reports.
Procedures run on one additional isolated backend; its one-case setup report
stays separate from the full HTTP report. Release assessment reassembles the
shards and checks their aggregate identities before considering completeness.

`generate_conformance_evidence.sh --issued-execution-root target/issued-execution`
includes both captures in a bundle. The default full release validator reassesses
their original raw evidence, requires both backends to pass this gate, and ignores
a supplied summary's claimed verdict. HTTP-only evidence cannot satisfy that
release scope. `--allow-static-conformance` retains its explicit limited scope.
This gate does not turn project policy choices into IEEE adjudications or replace
independent execution acceptance and provider qualification.

`report_issued_test_results.py --require-complete` requires all of the following:

- Structural ledger/test completeness in the exact supplied projection; semantic
  review remains a separate GitHub PR decision.
- An HTTP execution report with final `passed` or `failed` status, all required
  cases present, and only executed `passed` or `failed` case outcomes, with no
  unresolved checks or unvisited steps within those cases.
- Every declared procedure present with the current effective rubric hash.
- Every procedure and check decided. Empty checks, `not-run`, `unresolved`,
  `incomplete`, missing/unknown labels and unsupported applicability exemptions
  remain incomplete, including when a sibling check already failed.
- A run-time receipt binding the projection, definitions, shared execution
  contract and the actual HTTP, case-map and procedure-report bytes.

`observed` completes a check only when its exact ID is declared `advisory` in
that procedure's `check_levels`. This map is part of the effective rubric and
its hash. Unlisted checks default to `required`, including response-header
checks inside an advisory procedure. There are no prefix or wildcard exemptions.
The executor and collector use the same definition lookup; a report's own
`level` is informational and cannot grant an exemption. Thus a required
procedure can finish with decided mandatory checks and observed HEAD metadata
recommendations, while an unresolved mandatory check still blocks completion.
An overall `observed` procedure outcome is allowed only for a definition whose
procedure-level `level` is `advisory`; per-check completion is still required.

A `not-applicable` outcome with a free-text reason stays incomplete. Selected
alternatives instead need explicit source-conditioned links to actual configured
runtime witnesses. For example, the authenticated out-of-band status provider is
exercised with certificate/issuer/time-bound good, revoked, unknown and missing
evidence; it does not assert execution of the unselected CRL/OCSP mechanisms.
Likewise, producer and display roles do not become invented HTTP rejections.
These author applicability judgments remain visible in each binding's rationale;
hashing or a passing neighboring test does not independently validate them.

HTTP outcomes and completion are independent. A missing mandatory header and an
interrupted body produce a FAIL plus an unresolved body observation. The runner
records `unresolved_checks` alongside the unchanged outcome and failure messages;
it also records the unvisited suffix when a multi-step case stops early. Conditions
on those unvisited steps have not been evaluated and cannot establish completion.
The collector exposes `incomplete_http_cases` and `unresolved_http_checks`, both
globally and through the affected rule references, and rejects incomplete cases
under `--require-complete`. It also recognizes final `ORACLE-UNAVAILABLE` messages
in legacy mixed failure lists; omitting the new field cannot conceal them.
Intermediate polling attempts that have resolved do not themselves make a final
observation incomplete. Raw FAIL counts and known-gap regression decisions remain
separate from this execution-completeness gate.

An attempt stopped before final response evaluation records
`HTTP response checks not evaluated after request stopped before final evaluation`
in `unresolved_checks` when neither a more specific unresolved entry nor an
`ORACLE-UNAVAILABLE` reason already explains the stop; an initial-delay attempt
superseded by a completed retry contributes no final unresolved entry.

Collected StatementResult pages must contain a statements array whose elements
are objects with string IDs before the runner can use the union for membership,
absence or captures. An invalid element fails the page and cannot establish that
an excluded Statement is absent. This prerequisite does not replace the additional
Statement shape, identifier-format and preservation assertions in case definitions.
If collection stops before a valid terminal page, the report retains both the
observed FAIL and unresolved collection/aggregate checks, including when this is
the last step. A terminal aggregate that was evaluated and failed is complete;
an invalid page that prevented that evaluation is not.

## Interrupted procedures and raw response evidence

A failed positive-read prerequisite retains its FAIL and adds a required NOT-RUN
for dependent observations. Replay procedures also record skipped PUT/POST and
comparison items. A failure therefore cannot make an unfinished procedure count
as fully executed.

Authority observations validate returned IFIs independently of deterministic
identity comparison, including members of identified and anonymous Groups.
Identical invalid identifiers cannot establish a PASS. SHA-1 uses 40 hexadecimal
digits; OpenID uses absolute URI syntax; mbox uses the bounded DC-MAILTO adapter;
Account.homePage checks the necessary IRI syntax. Successful identity/shape
checks alone do not establish Account locator semantics. The DC-IRL adapter
now recognizes HTTP(S) access after RFC 3987 mapping; other schemes retain
unavailable locator checks. Unsupported legal mailto forms remain NOT-RUN. An independent invalid member
still fails when another member needs a provider. These output checks do not
change the separate input-acceptance waivers or supply unavailable providers.

In Python `http.jsonl`, request/response `body_hex` is the authoritative body
evidence. Decode those bytes with the member-preserving JSON loader when checking
extension member multiplicity. The convenience `json` field, and structured
values inside `preservation.jsonl`, are last-value map views: repeated property
names are not faithfully represented there. Observations identify the case and
pointer; correlate them with the procedure/ordinal and raw HTTP records. Preserve
the complete HTTP log alongside summaries. `incomplete_body` means the full body
was not captured and cannot support a complete preservation verdict.

## Execution receipt

### Known-gap diagnostic migration

The POST readback correction in `b09d351` binds a successful response to the
submitted Statement ID and uses that known ID for retrieval. It no longer
captures `/0` from a rejected write. Six existing baseline entries therefore
lost the secondary diagnostic `step store: expected JSON pointer /0 for capture issued_id`:

| Cases in `ci-execution-baseline.json` | Retained primary failure | Backends |
| --- | --- | --- |
| `issued-completion-root-duration-comma-batch`, `issued-completion-root-duration-comma-post` | `step store: expected status 200, received 400` | memory, postgres |
| `issued-completion-sub-duration-comma-batch`, `issued-completion-sub-duration-comma-post` | `step store: expected status 200, received 400` | memory, postgres |
| `issued-utf8-valid-boundaries-batch`, `issued-utf8-valid-boundaries-post` | `step store: expected status 200, received 500` | postgres |

All six remain FAIL. No gap was removed or relabeled, and the other baseline
outcomes and reason strings were preserved. Their case hashes and enclosing
file hashes were rebound to the corrected definitions. This is a change in
diagnostic collection, not a resolution of the comma-Duration or U+0000 product
defects. The gate still requires the exact recorded failure sets and rejects
any subsequent drift, including an unexpected PASS.

The later authority-identifier correction strengthens the generated readback
shapes and the dedicated authority case. Its baseline update changes only five
enclosing case-file hashes and 46 existing gap case hashes. All 117 gap entries
retain their identities, backend scopes, outcomes and exact diagnostic strings.
The added authority checks must pass the same regression gate; this rebinding
does not exempt any new failure or unavailable result.

### Receipt fields

Supply `--evidence-receipt run-receipt.json` in addition to `--case-receipt`.
The former is an execution input/output receipt; the latter identifies case
definitions and cannot establish which comparator produced an observation.

The execution producer must preserve these fields:

| Field | Required meaning |
| --- | --- |
| `schema_version` | `1` |
| `kind` | `issued-test-execution-receipt` |
| `projection_sha256` | SHA-256 of the exact projection file used for the run |
| `input_sha256` | Repository-relative paths and SHA-256 for every `execution_contract` entry and every case/procedure definition file |
| `artifacts_sha256` | Artifact paths and SHA-256 for the HTTP report, case receipt and every supplied procedure report |

Hashes concern file bytes. Rubric hashes continue to use compact insertion-order
JSON of the effective procedure, including expanded common expectations. The
collector compares recorded input hashes with both the projection contract and
the projection's `definition_files` map and current files, and requires each
report's bytes in the same receipt. `definition_files` fixes the exact bytes of
every indexed HTTP/procedure definition when the projection is generated. Its
path inventory must exactly match those indexes. A freshly bound run of edited
YAML cannot reuse a projection generated from the previous definitions. Changing
only a comparator therefore invalidates old execution evidence even when case
definitions are unchanged. All supplied artifacts must remain unchanged during
assessment.

The producer must capture inputs around the authorized execution and bind its
outputs afterwards; stamping current source hashes onto old results is not
evidence. Producer trust, runner binary/build provenance, target configuration
and collection capabilities require separate execution acceptance. File identity
checks do not establish these facts. This correction supplies receipt validation;
it does not implement or authorize that later execution pipeline.

Historical reports without this receipt can still be summarized, with
`execution_evidence_verified: false`. They cannot pass `--require-complete`.
Stale or mismatched supplied receipts are rejected. The collector never creates
a replacement receipt from the current tree.

Collected StatementResult queries apply membership and `/statements` array
shapes to the complete union. Required membership and minimum counts wait for
the terminal page; invalid observed items and exceeded maximum counts remain
failures even if later collection is unavailable. The five SHA-1 Agent query
witnesses collect up to 100 pages, including optional cross-IFI results, before
asserting the final membership or allowed ID/count set. An unfinished collection
retains NOT-RUN rather than establishing those final predicates.

## UUID identity and exact JSON comparison

`statement-query`, `statement-result-ids`, visibility evidence and Python
procedure ID membership compare standard UUIDs without hexadecimal case
sensitivity. Authority bootstrap and released-visibility reads also validate the
returned ID's standard UUID syntax before comparing its identity. These checks
do not rewrite the captured response or establish exact-value preservation.

Generic JSON pointer `equals`/`not_equals` and object-subset
`contains`/`not_contains` retain exact string comparison, including members named
`id`. Such members can belong to Activities or opaque extension values and do
not acquire UUID semantics from their names. The legacy strict-value absence
witnesses are used alongside separate exact-preservation checks; a raw
`not_contains: {id: ...}` is not an independent UUID-identity absence oracle.
Use an explicitly typed ID check or the procedure's canonical UUID membership
comparison when identity, rather than the returned JSON spelling, is the duty.

## Shared procedure text

`lrs-review-tests.yaml` declares common response expectations once and references
them with `{{common:name}}`. `lrs_procedure_definitions.load_procedures` expands
these references before projection, rubric hashing and execution. Unknown,
empty, nested or malformed references are rejected. Factoring text preserves
the effective procedure; a substantive change to a common obligation still
affects every referencing procedure and needs the corresponding impact review.

The receipt/completion controls and the other Python oracle controls are wired
to the separate `lrs-oracle-controls` Nix check. These controls verify the evidence
mechanism; they are not LRS target runs or provider certification. Definition
correspondence, execution completion and product conformance remain distinct.
Official review is the GitHub PR merge, not a local adoption record.
