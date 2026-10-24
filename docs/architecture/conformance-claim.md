# xAPI Conformance Claim

This document defines the claim boundary for Kiva's xAPI LRS interface.

The design target is **IEEE Std 9274.1.1-2023**. The merged main ledger supplies the
responsibility set; [the detailed test projection](../../spec/conformance/issued-test-projection.md)
connects each responsibility, condition and exception to concrete test definitions
and exact definition identities. Its static gate detects drift in those bindings.
Source conflicts retained under the project amendment have separate Kiva policy
outcomes; policy PASS is not an IEEE ruling. Definition completion supports a
claim about the design and its test specification. A claim that a particular
product revision conforms additionally requires the execution evidence below.
Official review is the GitHub PR merge. The structural gate does not measure
review acceptance, and no repository adoption record is required.

## Temporary integration policy for PR #3

The user selected ledger-first integration, with implementation completion in
separate follow-up work. The required Nix `memory-conformance` and
`postgres-conformance` checks therefore apply the explicit
[CI execution baseline](../../spec/conformance/ci-execution-baseline.json).
Their success means **no change from the reviewed, known-gap execution baseline**;
it does not mean that the LRS conforms or that all cases were executed.

The baseline contains 117 distinct residual cases: 10 FAIL and 104 NOT-RUN for
memory, 13 FAIL and 104 NOT-RUN for PostgreSQL, out of 17,933 cases per backend.
The 13 FAIL cases expose SHA-384/512 attachment support, comma decimal durations
and PostgreSQL U+0000 persistence gaps. NOT-RUN retains missing signature,
parameter-vocabulary and resource-absence prerequisites, plus numeric and IRL
provider limitations. These are product/provider work items, not IEEE exceptions.

The gate binds the entire case-file inventory and both fixture profiles. Every
case must appear exactly once in the HTTP execution report; cases outside the
baseline must PASS. Known gaps must match the exact case, definition, backend,
outcome and failure/unavailability messages. New failures, new NOT-RUN cases,
missing/duplicate cases, inconsistent totals, changed reasons and infrastructure
errors reject the gate. A formerly failing/unavailable case that now passes also
rejects a stale baseline: remove that residual after reviewing the improvement.
There is no automatic baseline update or general `allow-not-run` flag.

The wrapper creates fresh capture paths and preserves the runner's exit status,
unaltered JSON/JUnit and execution log. The Nix packages
`memory-conformance-evidence` and `postgres-conformance-evidence` save these
artifacts and an input/output receipt even when the runner fails or cannot emit
complete reports. These packages only capture evidence; their successful build
does not accept the execution. The separate required checks validate the capture
receipt and apply the strict regression gate. Missing artifacts, infrastructure
errors and new regressions still fail those checks.

CI builds and copies each evidence package before enforcing the checks. Its
artifact upload runs even after a gate failure, preserving JSON/JUnit when emitted,
execution receipts/logs and Nix build logs as `conformance-regression`. A rejected
gate cannot prevent installation of the already built evidence dependency. The
separate regression decision retains `conformance_complete: false`. Diagnostic
comparison of a supplied report remains `report-comparison-only`; it is not a
fresh execution receipt.

An initial 404 from an awaited single-Statement GET is an observation delay only
when the runner labels it `pending_visibility`, records exactly its initial-delay
reason, and a later attempt of the same step returns 200 with no reasons beyond
that case's exact baseline reasons, including their step IDs. The case's final
outcome and complete reason lists must still match the baseline. A known
NOT-RUN or FAIL remains such; resolving visibility does not establish conformance.
The gate recognizes that resolved delay. It still rejects unlisted header defects,
unresolved waits, loss of previously observed visibility and other-step success.

Strict execution remains available through `run_memory_conformance.sh`,
`run_postgres_conformance.sh` and the runner itself. They still exit nonzero for
any FAIL or NOT-RUN. Release evidence generation continues to use those strict
entry points, not the integration wrapper. Integration success leaves the
implementation gaps open. The normal static gate separately checks ledger/test
correspondence and reproducibility; it is not an adoption or review-acceptance gate.

The baseline comes from the sealed R5 reviewer logs at `fd8d942`; applicability
to `8d96de8` was reviewed as a bounded delta. This provenance is not a fresh
measurement of later code. Both commit IDs belong to superseded history in
`pre-consolidation.bundle`, SHA-256
`1f9a09ef2e900e1c6aebfdb7465aee09baa356e0060aa6af0c7780150c59e6ff`;
they need not be reachable from the current PR or main. Private historical log
locators document provenance; the committed exact definitions, profiles and gap
entries suffice to rerun the gate. New backend checks must validate the chosen policy
on the actual candidate; their evidence is recorded separately.

## Interface Scope

The conformance aid targets the xAPI LRS HTTP interface. PDS, BI, OAuth/OIDC
and confidential-computation workflows have separate contracts. Execution
results and conformance assessments are stored locally under `target/`.

## Non-Claims

Kiva must not claim any of the following unless an authorized program exists
and Kiva has completed it:

- IEEE certification.
- Official IEEE conformance.
- Certified xAPI LRS status.
- Conformance of Kiva-specific PDS, BI, governance, OAuth/OIDC, or
  confidential-computation behavior to the xAPI LRS standard.

## Evidence Boundary

An evidence bundle for a Kiva revision must include:

- Requirement catalog audit for
  `spec/requirements/ieee-39274-1-1-2025.yaml`.
- Source-clause triage for IEEE source sections that do not map one-to-one to
  catalog rows.
- OpenAPI contract validation for `spec/openapi/kiva-xapi.yaml`.
- Discharge plan validation for `spec/conformance/xapi-lrs-discharge.yaml`.
- Fixture schema validation for endpoint and authentication profiles.
- Validate-only execution of the language-neutral YAML conformance cases.
- Live HTTP execution against the memory backend.
- Live HTTP execution against the PostgreSQL backend.
- Case matrix and coverage reports.
- ADL compatibility report.
- Optional full ADL xAPI 2.0 execution report and raw logs when the evidence
  run explicitly enables external compatibility execution.
- Report schema validation for generated JSON reports.
- Schema validation for `metadata/manifest.json`.
- Bundle validation that checks required reports, logs, JUnit output, artifact
  lists, and checksum coverage.
- The exact git revision and dirty-tree status.
- `metadata/checksums.sha256` for the generated evidence files.

Generate a release-candidate local bundle with:

```sh
scripts/ci/generate_conformance_evidence.sh --require-clean-git
```

The default bundle location is `target/conformance-evidence/latest`. The
directory is intentionally ignored by git because it records generated evidence
for a particular local run.

Generate a static Nix-store bundle with:

```sh
nix build .#conformance-evidence-static
```

That package emits the same bundle shape under `result/`, including
`metadata/manifest.json` and `metadata/checksums.sha256`, but intentionally
uses `--skip-live` and therefore does not contain memory HTTP, PostgreSQL HTTP,
or PostgreSQL execution-verification reports.

For ordinary CI-style verification, `nix flake check` runs static contract
validation plus live HTTP regression checks against the explicitly enumerated
integration baseline for both backends. It does not discharge the remaining
product/provider gaps. The release-candidate evidence bundle remains the archival
artifact because it records reports, logs, checksums, and git metadata for a
specific revision.

When both xAPI conformance and formal verification evidence have been
generated for the same release candidate, validate them as one evidence set:

```sh
scripts/ci/validate_release_evidence_set.py \
  --conformance target/conformance-evidence/latest \
  --formal target/formal-verification-evidence/latest
```

The evidence-set validator first runs the conformance and formal bundle
validators, then checks that both bundles name the same clean git commit, were
generated with `--require-clean-git`, include full live xAPI conformance
evidence, include PostgreSQL execution verification, and include Kani
regression evidence. This is still internal release evidence, not IEEE
certification.

## Normative Audit

The detailed requirement catalog paraphrases the pinned IEEE Std 9274.1.1-2023
xAPI LRS source for traceability. It is not a replacement for the standard text.
The historical `ieee-39274-1-1-2025.yaml` filename and machine `standard` labels
are compatibility identifiers. Equivalence with a separate ISO/2025 edition
has not been established. The ledger's source reading, review and adoption
state are distinct from test execution and implementation conformance; consult
the [ledger review guide](../../spec/requirements/lrs-requirements-ledger-review.md)
for the current candidate and its historical adoption boundary.

The machine audit checks that the catalog has stable requirement ids, valid
clause references, valid requirement levels, valid OpenAPI/assertion projection
classes, the local source-document reference, the source checkout revision, and
the local source document section headings. Each catalog clause must be present
in the local IEEE source document. Each local IEEE source section without a
direct catalog row must be classified in
`spec/conformance/ieee-source-clause-triage.yaml`.

This is a release-candidate implementation audit. It is not an external legal
certification or an IEEE-issued conformance decision.

The local IEEE Open Source reference expected by the audit is:

```text
/home/claude/Codex/opensource.ieee.org/xapi/xapi-base-standard-documentation/9274.1.1 xAPI Base Standard for LRSs.md
```

Set `KIVA_IEEE_XAPI_REFERENCE_ROOT` when the reference checkout is elsewhere.
Use `--no-require-ieee-reference` on the evidence script only for portable
static checks that cannot access the local reference checkout.

## Profile Split

Kiva keeps two separate profiles:

- **xAPI conformance profile:** the generic, language-neutral black-box LRS
  target used by the conformance aid. It permits anonymous cases where the
  standard behavior is not credential-sensitive and uses explicit fixture
  credentials only for authorization and authority cases.
- **Kiva production profile:** the platform deployment profile. It uses
  OAuth 2.1-style bearer-token resource-server behavior, OIDC-derived identity,
  tenant policy, consent, audit, and Kiva-specific access-control decisions.

The production profile may be stricter than the generic xAPI conformance
profile. For example, `KIVA_REQUIRE_AUTHORIZATION=true` rejects anonymous
access to ordinary xAPI resources while leaving `/about` discoverable. That is
appropriate for Kiva deployments, but it is not the default generic xAPI
conformance fixture because several black-box cases intentionally exercise
anonymous behavior.

## ADL Compatibility

The ADL xAPI LRS Conformance Test Suite is useful compatibility evidence for
the older xAPI ecosystem. It is not the normative source for the pinned
IEEE Std 9274.1.1-2023 requirements.

Kiva records ADL compatibility through
`spec/conformance/adl-compatibility-map.yaml` and
`scripts/ci/report_adl_compatibility.py`. When the local ADL checkout is
available, the report can verify that mapped ADL source files exist:

```sh
python scripts/ci/report_adl_compatibility.py \
  spec/conformance/adl-compatibility-map.yaml \
  --adl-root /home/claude/Codex/github.com/adlnet/lrs-conformance-test-suite
```

The result should be presented as compatibility or gap evidence. It does not
prove that Kiva satisfies the pinned IEEE source or a separate ISO/2025 edition.

The ADL compatibility report records the local ADL checkout path, git commit,
dirty status, and source-file verification status when `--adl-root` is
provided. Kiva does not treat direct ADL execution as the normative gate because
the ADL suite is an independently maintained external compatibility source,
while Kiva's normative traceability remains anchored to the IEEE requirement
catalog.

For stronger compatibility evidence, a local ADL checkout can be executed in
full against a disposable memory-backed Kiva instance:

```sh
KIVA_ADL_SUITE_ROOT=/path/to/lrs-conformance-test-suite \
  scripts/ci/generate_conformance_evidence.sh --with-adl-run
```

The normalized `reports/adl-run.json` records the ADL and Kiva revisions and
requires every emitted test to match `adl-run-expectations.yaml`. The gate
rejects unknown or missing test identities, unexpected failures, and stale
expected divergences or open defects that now pass. A recorded open defect is
an explicit compatibility gap requiring separate disposition; it is not an
accepted IEEE divergence and does not strengthen the normative conformance
claim. This external live run is opt-in and is not part of `nix flake check`.
