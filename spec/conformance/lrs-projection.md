# Public LRS source projection

The input is `9274.1.1 xAPI Base Standard for LRSs.md` at documentation
revision `fb8ecb2c1a258e6d3d397805243fd793d47cec5d`, SHA-256
`2f43d1a0bccbe24c5adaef0daa3519837c6583ba7c3bfbda86ba964924b1ff0b`.
The approved `2.0.0` release points to an earlier revision,
`134afd8c108b4d1b98294a1613db38a6b8fe73d8`.
This suite uses the pinned public source with later corrections. It does not
establish that those bytes are identical to the IEEE or ISO published PDF.
The user-supplied IEEE Std 9274.1.1-2023 PDF has now been read for the full LRS
clause and reconciled with this source. Provenance, printed/PDF page coordinates,
differences and test decisions are recorded in
[lrs-published-source.md](lrs-published-source.md) and
[lrs-published-source.yaml](lrs-published-source.yaml). The chapter 4 PDF retains the document selector conflicts and duplicated
`related_activities` description. The current policy reads Activity Profile
list scope and `related_activities` through the redundant chapter 5 tables.
ISO 2025 equivalence remains unverified.
The diff from the approved release has two changes: the `related_activities`
table now describes Activity filtering instead of repeating the Agent-filter
description, and the Characterstring Parameters example link has moved. The
current suite uses the PDF chapter 5 Activity-filter row; it does not depend on
an external correction or claim that an official erratum has been verified.

The suite covers the LRS role. LRP/LRC application duties, xAPI Profiles,
security policy and optional hosted metadata are classified separately.
The source document remains authoritative; the requirement catalog is
an index, and catalog or heading counts do not establish completeness.

The portable assets are:

- `lrs-duty-review.yaml`: authored source/action/condition judgments for the
  repaired candidate, expanded into concrete case IDs and assertion pointers,
  including each table field and placement. This is an author semantic review,
  not an independent verdict; its audit checks references and drift.
- `lrs-source-projection.yaml`: source byte identity, contiguous hashed source
  units, semantic rule groups, external references and explicit interpretations.
- `lrs-published-source.yaml`: published IEEE PDF/extraction provenance, actual
  reading/visual scope, interpretation findings and coarse published clause
  coordinates for the existing rule groups. This does not replace an atomic
  requirement inventory or discharge remaining review procedures.
- `lrs-interpretation-policy.yaml`: named project interpretations, reasons and
  selectors binding known source issues to actual expectation bodies. See
  [lrs-interpretation-policy.md](lrs-interpretation-policy.md) for result handling.
- `tests/conformance/models/lrs-tables.yaml`: each LRS data table's properties,
  valid values, types, required fields, enumerations, exclusive identifiers and
  legal Statement/SubStatement placements.
- `tests/conformance/http/generated/`: deterministic table and behavioral
  cases, usable independently of Kiva libraries and the Rust reference runner.
- `lrs-table-bindings.yaml`: individual table conditions and their actual
  status assertions, with optional interaction validation kept separate.
- `lrs-assertion-manifest.yaml`: exact case and expectation hashes plus source
  bindings. Removing an assertion or changing a source sentence is detected.
- `lrs-review-tests.yaml`: concrete procedures for conditions that need an
  independent clock, delayed-visibility fixture, policy declaration, external
  RFC vectors, or semantic examination of a response. These procedures are
  neither waivers nor passes inferred from an HTTP run.
- `schemas/conformance-case.schema.json`: the portable request, response,
  capture and relationship assertion contract.

Generate and check the assets from the repository root in the pinned Python,
PyYAML/LibYAML and JSON Schema environment. See the complete
[input/output inventory and reproduction procedure](generated-artifacts.md).
OpenSSL is not used by static expansion; signature cases are fixed inputs.

```sh
python -B scripts/ci/lrs_generated_artifacts.py generate
python -B scripts/ci/lrs_generated_artifacts.py check
python -B scripts/ci/validate_lrs_projection.py --definitions-only --skip-source
```

The assertion manifest is a derived index; its expected digest is tracked in the
artifact lock. CI regenerates and compares exact outputs, and then checks review
bindings. Intentional changes require an explicit lock refresh and review impact
assessment; neither operation silently rebases source identities or grants
acceptance. Use `--source PATH` to check the same pinned public source bytes.
`--skip-source` explicitly reports that source bytes have not been verified.
The audit also binds the bytes of the published-source record. Its
`source_bytes_verified` field concerns the public Markdown; raw PDF verification
is recorded separately in the private reconciliation evidence. A distributable
suite does not require the licensed PDF to be present to validate its cases.

Live tests require a disposable isolated LRS, read access to all fixture
Statements, and the documented credential/limit fixtures. Pass an endpoint and
`--allow-destructive`; the suite creates, overwrites and deletes documents.
Fresh targets are required between runs because Statement ids are deterministic.
The synthetic signing key and certificate are public test data; all three
permitted RSA algorithms use the same reproducible fixture.

The reference runner adds bounded response checks rather than depending on
Kiva's implementation:

- `header-time` compares HTTP dates with a returned timestamp or the maximum
  `stored` in a StatementResult, at HTTP-date second precision.
- `json-time` checks ISO timestamps, UTC conversion, and timestamp equality.
  With `allow_truncation_or_rounding` and a literal `expected`, it accepts
  truncation or rounding at decimal precisions of at least milliseconds. It
  rejects a loss of millisecond precision and a change outside those rules.
- `json-order`, `numeric-precision` and `json-shape` check ordered stored times,
  the binary32 precision floor and table-shaped responses. The shape vocabulary
  is intentionally bounded; it is not a full JSON Schema implementation.
- `multipart` checks framing, binary parts, SHA-2 matching, lengths, metadata
  consistency and required attachment bytes. `bytes` checks arbitrary binary
  document contents without UTF-8 replacement.
  `expected_data_hex` verifies arbitrary binary attachment fixtures, including
  octets outside UTF-8. MIME transport padding and delimiter-like payload bytes
  are distinguished from actual boundary lines.
- `json-capture-equal` compares complete captured JSON, including authorities.
  `status-body` enforces a body condition only for its stated response status.
- `status_in` lists additional permitted statuses alongside `status`.
  Duplicate Statement requests permit both 204 and 409 while still checking
  preservation. Optional interaction validation permits rejection or acceptance.

JSON `equals` compares complete values, independent of object member order and
equivalent JSON number spellings. `contains` on an object or array matches
partial objects recursively, but identifying strings must match completely.
Only a direct string `contains` assertion means a substring search. The
generated recursive format vectors cover all four IFIs, identified/anonymous
Groups, SubStatements and each Context relationship in default, exact, ids and
canonical responses. Explicit default `objectType` markers are accepted in ids
responses; names, displays, definitions and unnecessary identified-Group members
are not accepted there.
Multiple-member vectors accept Group member permutations while rejecting loss
or duplication. All four IFIs are exercised in Agent/Group filter placements.
Closing braces outside an opened capture are literal data, allowing nested JSON
query strings ending in `}}`; opened `{{capture}}` references are still checked.

`request.await_retrieval_ms` is a fixture observation window, from 1 to 60000
milliseconds, for GET only. It allows asynchronously indexed writes to become
visible and commits captures only from a checked response. Its expiration is
not a standards-defined latency limit. Increase it for the target's declared
visibility behavior before interpreting a timeout as a conformance result.
`request.follow_more` follows a captured continuation IRL without appending
original filters. The reference runner currently supports same-origin IRLs.
A target with a different pagination topology needs a suitable runner/fixture.

SHOULD and MAY behavior must not turn into unconditional mandatory failures.
The interpretation ledger records the State list
table, Activity Profile list parameter, identifier-value validation permission,
single PUT/batch wording, optional interaction validation and comparison rules.
The remaining source ambiguities also occur in the received IEEE PDF.
`related_activities` is in scope under the chapter 5 PDF table; see the current
[lrs-interpretation-policy.yaml](lrs-interpretation-policy.yaml).
Resolve material disagreements before asserting donation-ready completeness.

A complete conformance assessment requires both the HTTP results and applicable
review results, with source and target configuration identities. Passing the
drift audit proves that the pinned mappings and assertions are intact; it does
not prove their semantic completeness. Passing a finite vector suite also
does not prove behavior for every possible input or establish IEEE certification.
