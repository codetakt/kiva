# Kiva Conformance Planning

This directory maps requirement catalogs to executable or reviewable evidence.
The current xAPI LRS requirements baseline is the pinned IEEE Std 9274.1.1-2023.
Historical `39274-1-1-2025` filenames and machine labels are compatibility
identifiers; equivalence with the 2025 edition has not been established.
Requirement traceability alone does not establish implementation conformance;
see the [xAPI claim boundary](../../docs/architecture/conformance-claim.md).

The source-pinned LRS projection, portable vectors, assertion contract and
review procedures are described in [lrs-projection.md](lrs-projection.md).
Source provenance and project interpretations are described in
[lrs-published-source.md](lrs-published-source.md) and
[lrs-interpretation-policy.md](lrs-interpretation-policy.md).

Execution reports, review decisions and assurance assessments are stored locally
under the Git-ignored `target/` directory. This directory contains test definitions,
source mappings and instructions for producing those local records.

To execute review procedures after a disposable backend's HTTP suite, set
`KIVA_CONFORMANCE_PROCEDURE_REPORT_ROOT` to a fresh directory under `target/`
and `KIVA_CONFORMANCE_PROCEDURE_FIXTURE` to the declared target fixture before
running `run_memory_conformance.sh` or `run_postgres_conformance.sh` in `nix develop`.
Missing target fixtures remain `not-run` and an incomplete run cannot report all-pass.

Bounded design controls can be executed locally with:

```sh
python scripts/ci/run_lrs_evaluation_controls.py --runner target/debug/xapi-lrs-conformance --out target/lrs-evaluation-controls
python scripts/ci/run_lrs_procedure_controls.py --runner target/debug/xapi-lrs-conformance --out target/lrs-procedure-controls
python scripts/ci/run_lrs_person_identifier_controls.py --runner target/debug/xapi-lrs-conformance --server target/debug/kiva-server --out target/lrs-person-controls
```

Use a fresh output directory for each run.

- `xapi-lrs-discharge.yaml`: discharge plan for the IEEE Std 9274.1.1-2023
  xAPI LRS requirement catalog.
- `adl-compatibility-map.yaml`: conservative compatibility map from the ADL
  xAPI 2.0 LRS Conformance Test Suite to Kiva requirement ids.
- `adl-run-expectations.yaml`: revision-pinned expected outcomes for every
  identifiable test emitted by the external ADL xAPI 2.0 suite.
- `ieee-source-clause-triage.yaml`: audit triage for source document clauses
  that do not map one-to-one to requirement catalog rows.
- `endpoint-profile.example.yaml`: example deterministic endpoint fixture
  declaration for otherwise implementation-specific error checks.
- `auth-profile.example.yaml`: example environment-backed authentication
  fixture declaration for credential-sensitive cases.
- `executable-gap-map.yaml`: requirements that intentionally remain outside
  fixed generic HTTP cases.
- `schemas/`: JSON Schema contracts for fixture files and machine-readable
  conformance reports.

The ADL LRS Conformance Test Suite is treated as an external compatibility
gate, not as the normative source for Kiva's conformance claim. The IEEE 2023
published LRS text, pinned public source and explicit interpretations are
distinguished in the source records; the catalog is an index for traceability.

The conformance aid is intended to be donated as language-neutral conformance
assets with a Rust reference runner. Requirement metadata, YAML cases, fixture
contracts, JSON Schemas, and report formats carry the conformance meaning. The
Rust runner is the strict execution engine for those assets, not the sole place
where the suite's specification is encoded.

Coverage can be summarized from the discharge plan:

```sh
python scripts/ci/audit_requirement_catalog.py
python scripts/ci/validate_conformance_fixtures.py --check-schemas
python scripts/ci/report_conformance_case_matrix.py
python scripts/ci/report_conformance_coverage.py \
  spec/conformance/xapi-lrs-discharge.yaml
python scripts/ci/report_adl_compatibility.py \
  spec/conformance/adl-compatibility-map.yaml
```

The report distinguishes requirements with active evidence from requirements
that are only assigned to planned evidence.
The fixture validator checks the endpoint and authentication profile examples
against their JSON Schema contracts and cross-field invariants.
The case matrix report distinguishes requirements referenced by executable YAML
cases from requirements still covered only by other evidence.
The ADL compatibility report distinguishes direct ADL-equivalent coverage,
partial ADL coverage, explicit ADL gaps, and unclassified requirements.
The requirement catalog audit checks stable requirement ids, clause references,
levels, projection classes, and the optional local IEEE source-document
reference. When the local IEEE source checkout is available, it also verifies
that every catalog clause appears in the source document and that every source
clause without a direct catalog row is classified by
`ieee-source-clause-triage.yaml`.

A full local evidence bundle, including validate-only and live HTTP evidence
for the memory and PostgreSQL backends, can be generated with:

```sh
scripts/ci/generate_conformance_evidence.sh
```

The default output is `target/conformance-evidence/latest`. The bundle includes
JSON and Markdown reports, JUnit XML for runner executions, validation logs,
checksums, and a manifest with the exact git revision. The full bundle also
includes a PostgreSQL execution verification report generated from a temporary
local PostgreSQL cluster. The live HTTP and PostgreSQL execution portion can be
skipped for static inspection with `--skip-live`. Release-candidate evidence
should be generated from a clean tree:

```sh
scripts/ci/generate_conformance_evidence.sh --require-clean-git
```

Full ADL execution is an explicit external-checkout-dependent option. Install
the suite dependencies in a local checkout, set `KIVA_ADL_SUITE_ROOT`, and add
`--with-adl-run`:

```sh
KIVA_ADL_SUITE_ROOT=/path/to/lrs-conformance-test-suite \
  scripts/ci/generate_conformance_evidence.sh --with-adl-run
```

The runner starts a fresh memory-backed Kiva instance without mandatory
authorization and invokes the ADL suite for xAPI 2.0.0. It archives the raw
hierarchical ADL log and console output, then emits `reports/adl-run.json`.
The normalized report pins both Git revisions and compares every derived test
identity with `adl-run-expectations.yaml`. Unknown or missing identities,
unexpected failures, and unexpectedly repaired divergences or open defects
fail closed. An ADL process failure is acceptable only when every emitted
outcome matches the ledger. Initial unexplained failures are recorded as
`open-defect`, not as standards-version divergences.

This live ADL path is intentionally absent from `nix flake check`; it requires
an external checkout and mutates a disposable target LRS. Release evidence can
make it mandatory with `validate_release_evidence_set.py --require-adl-run`.

Nix can build the static report set without starting Kiva:

```sh
nix build .#conformance-evidence-static
```

`nix flake check` also runs the disposable memory and PostgreSQL HTTP
conformance checks and validates their generated JSON reports. The static
evidence package remains separate so it can be built without live services.

Generic executable HTTP coverage includes endpoint-profile-sensitive cases when
the target LRS declares deterministic fixture behavior. In particular,
`xapi-lrs-4.1.5-too-large` uses a declared maximum request body size and a
generated probe Statement body that exceeds it, and
`xapi-lrs-4.1.5-rate-limit` uses a declared burst count and path that
deterministically returns `429`.

The black-box conformance aid runner lives under
`tools/xapi-lrs-conformance`. It validates YAML cases by default and executes
HTTP checks only when an LRS endpoint is supplied:

```sh
cargo run -p xapi-lrs-conformance -- --validate-only
cargo run -p xapi-lrs-conformance -- \
  --validate-only \
  --requirement xapi-lrs-4.2.4.2-authority-deterministic
cargo run -p xapi-lrs-conformance -- --endpoint http://localhost:8080
cargo run -p xapi-lrs-conformance -- \
  --endpoint http://localhost:8080 \
  --bearer-token-env KIVA_CONFORMANCE_TOKEN \
  --endpoint-profile spec/conformance/endpoint-profile.example.yaml \
  --junit target/xapi-lrs-conformance.xml
cargo run -p xapi-lrs-conformance -- \
  --endpoint http://localhost:8080 \
  --allow-destructive \
  --auth-profile-file spec/conformance/auth-profile.example.yaml
```

Kiva also provides disposable full-suite checks for both storage backends:

```sh
scripts/ci/run_memory_conformance.sh
scripts/ci/run_postgres_conformance.sh
scripts/ci/run_postgres_verification.sh
```

The memory check runs `kiva-server` without `DATABASE_URL`. The PostgreSQL
check starts a temporary local PostgreSQL cluster, applies the versioned
migrations, and runs `kiva-server` with `DATABASE_URL`. Both execute the
bundled HTTP cases with the endpoint and auth fixtures enabled. The PostgreSQL
verification check starts a temporary local PostgreSQL cluster and runs the
ignored execution test for threshold-release idempotence and retry behavior.
The memory and PostgreSQL HTTP checks are wired into `nix flake check`; the
full local evidence bundle additionally preserves their JSON and JUnit reports
with checksums.

For release-candidate evidence, validate the xAPI conformance bundle together
with the formal verification bundle:

```sh
scripts/ci/validate_release_evidence_set.py \
  --conformance target/conformance-evidence/latest \
  --formal target/formal-verification-evidence/latest
```

This confirms that both bundles refer to the same clean revision and that the
conformance bundle contains live memory, live PostgreSQL, and PostgreSQL
execution-verification evidence.

Additional common headers can be supplied with repeated `--header NAME:VALUE`
arguments. Case-specific headers override common headers. Requests can select a
named credential fixture with `request.auth_profile`; the runner can read those
fixtures from repeated `--auth-profile NAME=ENV` bearer-token arguments or from
an `--auth-profile-file` YAML document. Test cases that may mutate or delete
LRS state must set `safety.destructive: true` and
`safety.requires_isolated_lrs: true`; the runner refuses to execute them unless
`--allow-destructive` is supplied.

An auth profile file declares environment-backed request headers. A profile may
use `bearer_token_env` to inject `Authorization: Bearer ...`, and may also use
`headers` to map explicit HTTP header names to environment variables containing
the full header value.

Endpoint-profile-sensitive cases declare `requires_endpoint_profiles`. The
runner validates those cases without a profile, but HTTP execution fails the
case unless `--endpoint-profile PATH` supplies the required deterministic
fixture. The current endpoint profile contract supports:

- `deterministic-too-large-threshold`: declares `max_request_body_bytes` and a
  larger `probe_body_bytes`; the runner generates a valid xAPI Statement body
  with at least the probe size and expects `413`.
- `deterministic-rate-limit-trigger`: declares `request_count`,
  `window_seconds`, and `path`; the runner sends that many immediate requests
  to the declared path and evaluates the final response as `429`.

Use `--json` for the full machine-readable conformance report. Use
`--junit PATH` when CI needs a JUnit XML pass/fail artifact in addition to the
normal text or JSON output. JSON reports include a `schema_version` field so CI
consumers can pin report parsing behavior.

Machine-readable reports can be checked against the pinned JSON Schema
contracts:

```sh
cargo run -p xapi-lrs-conformance -- --validate-only --json > target/xapi-run.json
python scripts/ci/validate_conformance_fixtures.py --check-schemas
python scripts/ci/audit_requirement_catalog.py --json \
  > target/requirement-catalog-audit.json
python scripts/ci/report_conformance_coverage.py \
  spec/conformance/xapi-lrs-discharge.yaml --json > target/xapi-coverage.json
python scripts/ci/report_conformance_case_matrix.py \
  --json > target/xapi-case-matrix.json
python scripts/ci/report_adl_compatibility.py \
  spec/conformance/adl-compatibility-map.yaml --json > target/adl-compatibility.json
python scripts/ci/validate_conformance_reports.py \
  --check-schemas \
  --run-report target/xapi-run.json \
  --case-matrix-report target/xapi-case-matrix.json \
  --coverage-report target/xapi-coverage.json \
  --adl-report target/adl-compatibility.json \
  --requirement-audit-report target/requirement-catalog-audit.json
```

Use repeated `--suite ID`, `--case-id ID`, and `--requirement ID` filters to
run a validated subset of the loaded cases. The runner still validates all
loaded case files before filtering, then reports only the selected cases. A
filter that selects no cases fails the run.

A case can use a single `request`/`expect` pair or an ordered `steps` list. Use
`steps` when a requirement needs a write followed by a read or another
state-dependent assertion. Step execution stops at the first failed step.

A step may declare `when_previous_status: 200` to run only after the immediately
preceding unconditional step returns 200. The guard must select an allowed,
required status of that step. For the disputed unknown-offset case, 400 is an
allowed rejection and has no readback; 200 requires the UTC readback. Unexpected
statuses and failed checks still fail the case. This conditional follow-up does
not exempt missing target fixtures from `not-run`.

Step expectations may capture scalar JSON response values for later steps:

```yaml
expect:
  status: 200
  capture:
    generated_statement_id:
      pointer: /0
```

Later request values and string expectations can reference captures with
`{{generated_statement_id}}`. Captures are scoped to a single case and must be
defined by an earlier step.

Endpoint profiles expose runner-provided captures such as
`{{kiva_endpoint_profile_too_large_statement_body}}`,
`{{kiva_endpoint_profile_rate_limit_request_count}}`, and
`{{kiva_endpoint_profile_rate_limit_path}}`. These captures are only valid in
cases that declare the corresponding `requires_endpoint_profiles` entry.

Credential-sensitive authorization and authority tests require the target LRS
to expose the fixture profiles documented in
`docs/architecture/conformance-auth-fixtures.md`.

ADL compatibility is recorded as external compatibility evidence. When
`--adl-root` points to a local ADL checkout, Kiva verifies mapped ADL source
paths and records the ADL checkout commit and dirty status. Direct ADL runner
execution remains non-normative because it is an independently maintained
compatibility suite and is destructive against a target LRS. The current suite
also supports anonymous execution, which Kiva uses only for this disposable
compatibility fixture; Kiva's production boundary remains OAuth/OIDC based.
