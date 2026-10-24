# Issued-test execution evidence

The test ledger defines what must be checked. Execution completeness is a
separate assertion that needs the exact definition and actual, current
evidence. A target FAIL is an executed observation; it does not imply a missing
test or a conforming product.

`report_issued_test_results.py --require-complete` requires all of the following:

- Structural ledger/test completeness in the exact supplied projection; semantic
  review remains a separate GitHub PR decision.
- An HTTP execution report with final `passed` or `failed` status, all required
  cases present, and only executed `passed` or `failed` case outcomes.
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

The current collector does not verify applicability evidence:
even a `not-applicable` label with a free-text reason stays incomplete. A future
applicability adapter must establish the definition's condition and bind the
supporting evidence before such an exemption can complete this gate. This limit
does not classify the target as failing its IEEE duty.

## Interrupted procedures and raw response evidence

A failed positive-read prerequisite retains its FAIL and adds a required NOT-RUN
for dependent observations. Replay procedures also record skipped PUT/POST and
comparison items. A failure therefore cannot make an unfinished procedure count
as fully executed.

In Python `http.jsonl`, request/response `body_hex` is the authoritative body
evidence. Decode those bytes with the member-preserving JSON loader when checking
extension member multiplicity. The convenience `json` field, and structured
values inside `preservation.jsonl`, are last-value map views: repeated property
names are not faithfully represented there. Observations identify the case and
pointer; correlate them with the procedure/ordinal and raw HTTP records. Preserve
the complete HTTP log alongside summaries. `incomplete_body` means the full body
was not captured and cannot support a complete preservation verdict.

## Execution receipt

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
current files, and requires each report's bytes in the same receipt. Changing
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
