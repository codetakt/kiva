# Reproducing the LRS test definitions

The IEEE Std 9274.1.1-2023 ledger is the responsibility baseline. Git stores
the definition inputs, generators and exact artifact lock.
Expanded cases and indexes are generated locally at their established paths;
they are ignored by Git. A reproducible bundle supplies the expanded portable
suite for readers who do not want to run the generators.

Reproducibility establishes identity. The normal completion gate checks concrete
ledger/test correspondence, explicit conditions and exceptions, and reverse case
coverage. It does not read review-receipt or adoption records. Official review is
the GitHub PR merge. The generated `test_definition_projection_complete` flag
describes structural correspondence; `semantic_coverage_complete` remains false
because a generator cannot establish semantic approval from counts or hashes.

Execution completion separately requires every expected observation and verified
input/output evidence. It does not depend on a repository review verdict, and
does not imply that the observed results pass. Source residuals and product/
provider conformance remain separate. Static reproduction does not create a new
backend execution receipt.

The former local review-verdict gate has been removed. `conformance-plan`
checks the structural contract described above; passing it does not assert that
pending review comments are resolved. Review progress and acceptance tables are
not repository deliverables. Historical versions remain available through Git.

## Inputs and outputs

[lrs-generated-artifacts.json](lrs-generated-artifacts.json) is the complete
machine-readable input selection, ordered producer recipe and output inventory.
[lrs-generated-artifacts.lock.json](lrs-generated-artifacts.lock.json) records
each effective input/output's SHA-256 and byte length, plus the generation
toolchain. Neither file records semantic acceptance.

| Classification | Contents | Git policy |
| --- | --- | --- |
| Definition inputs | Merged catalog/ledger, table model, handwritten HTTP cases, Python scenario/selection definitions, interpretation/dependency contracts and fixed vectors | Tracked; change intentionally and assess affected meanings |
| Review workflow | PR discussion and merge on GitHub; interim progress notes remain local | Not generation inputs or repository acceptance records |
| CI integration baseline | `ci-execution-baseline.json`, exact known residuals and profile/case identities | Tracked; update only by explicit disposition, never by accepting a new failing run automatically |
| Fixed signing inputs | `tests/conformance/fixtures/signing/cases.json` | Tracked; 42 exact public synthetic cases with original source and canonical selected-case digests; historical commit is a provenance label, not a required ref |
| Case expansion | Five YAML files under `tests/conformance/http/generated/` | Ignored; regenerated from models/scenarios and fixed inputs |
| Procedure expansion | `issued-definition-tests.yaml`, `issued-supplemental-tests.yaml` | Ignored; regenerated from contracts and scenario definitions |
| Derived relations | `lrs-table-bindings.yaml`, `issued-completion-bindings.json`, `issued-readback-origins.json`, `issued-rejection-origins.json` | Ignored; generated with the cases |
| Derived indexes | `issued-test-projection.json`, `issued-test-matrix.md`, `lrs-assertion-manifest.yaml` | Ignored; generated after the cases/procedures, bound to the definition and oracle inputs |
| Unicode provider tables | `vendor/pkix-path/src/rfc4518/unicode32.rs`, `tests/conformance/fixtures/pkix-unicode32.json.gz` | Ignored; reproduced from pinned RFC3454 and Unicode 3.2 sources before Rust compilation |
| Implementation witness bindings | `implementation-procedure-bindings.json` | Tracked author input: explicit rubric selections, semantic rationales and execution witnesses; no static producer infers these judgments |
| Provider reference inputs | `jws-provider.json`, synthetic public keys/certificates, vendored provider sources | Tracked frozen inputs with provenance; author fixture creation is separate from ordinary static expansion |
| Private evidence | Historical sealed packets, receipts, local logs | Preserve their recorded locations and scope; not regenerated or silently replaced |

This inventory concerns the LRS test projection. It does not reclassify the
already merged ledger's published tables or unrelated architecture generators.
The pre-commit index check rejects every declared output, including files added
with `git add -f`. The lock, migration checksums and upstream package metadata
remain tracked because they pin inputs; they are not expansion outputs.
Historical links to the expanded paths resolve after regeneration or inside a
bundle; the Git web view intentionally has no large generated blobs.

Historical commit labels such as `b505ffc` and checkpoint `cd0773e` belong
to superseded local histories and need not be reachable from main. They are
provenance annotations, not reproduction dependencies. The signing fixture
contains all selected case objects and their canonical content digest; the
ledger authority is the merged revision and exact file digests in
`issued-ledger-baseline.json`. The historical checkpoint is not an additional
authority or required acceptance artifact.

The signing file is now an explicit fixed fixture input, including request and
expectation definitions. Normal expansion does not sign, generate keys or
certificates, verify cryptography, call a server, download references, or provision
a provider. Editing a signed vector means editing its fixed input with provenance
and separate review; there is no automatic crypto-generation fallback.

## Reproduce from a fresh checkout

Nix builds materialize the two Unicode provider outputs in their build source.
The default and verification development shells also materialize missing or
stale copies and verify the locked identities before Cargo is used. When using
the pinned Python/Cargo tools directly, prepare these build prerequisites with:

```sh
python3 -B scripts/ci/generate_pkix_unicode32.py --if-needed
```

This performs only the Unicode expansion. The complete command below reproduces
all 16 declared outputs. Generated bytes remain identical to the locked corpus.

Use the `conformance-definitions` Nix shell (Python dependencies only), pinned by
`flake.lock`, or an existing environment matching the toolchain in the artifact
lock. The recorded pinned environment is CPython 3.13.13, PyYAML 6.0.3 with
LibYAML 0.2.5; the full static validator also requires jsonschema 4.26.0.
The tool refuses a generation-version mismatch. An intentional environment
upgrade requires a new lock and a comparison of the resulting outputs.

```sh
nix develop .#conformance-definitions
python -B scripts/ci/lrs_generated_artifacts.py generate
python -B scripts/ci/lrs_generated_artifacts.py check
python -B scripts/ci/validate_lrs_projection.py --definitions-only --skip-source
```

`generate` verifies the input set and hashes, copies only declared inputs into a
fresh temporary directory, expands all three stages, compares every output with
the committed lock, and then materializes the matching outputs in this checkout.
No old generated file is copied into that directory. Missing/extra inputs,
undeclared producer outputs, changed inputs or output drift fail the command.
`check` repeats expansion in another temporary directory with a different Python
hash seed and checks any outputs already materialized in the checkout. It writes
neither the lock nor the checkout's generated files. `verify` checks existing
inputs and outputs against the lock without regenerating them.

An evidence entry point that has just generated the artifacts can call
`validate_lrs_projection.py --verify-artifacts` to verify their complete input/output
identities before running the same projection, schema and source-binding checks.
Missing or changed artifacts are rejected; this mode does not repair them or skip
validation. The default standalone validator still generates first. The separate
reproduction check continues to expand both hash seeds from empty temporary trees.

`--allow-review-pending` is retained as a compatibility option; it does not change
the checks. No mode requires a review receipt. `--skip-source` explicitly leaves external source bytes unverified; the
merged ledger remains pinned. No licensed IEEE PDF or private historical review
packet is needed to reproduce the static outputs.

The validator and CI/evidence entry points materialize locked artifacts before
consuming them. The Rust runner rejects a missing generated case file when the
standard case root is selected; deliberately selected custom subsets remain
possible. Before invoking standalone reporting tools, run `generate` as above.
The two authorized local backend checks preserve raw FAIL/NOT-RUN results and
produce a separate regression decision; see [the claim boundary](../../docs/architecture/conformance-claim.md).
Full implementation claims additionally require both complete backend captures
with all 145 procedures; the HTTP integration gate alone is insufficient.

For the two authorized local backend checks, evidence capture and gate enforcement
are separate. For example:

```sh
nix build .#memory-conformance-evidence --out-link result-memory-evidence
nix build .#checks.x86_64-linux.memory-conformance
```

The first package saves raw JSON/JUnit when produced, the execution log, original
exit and a bound capture receipt. It does not claim success of the regression gate
or of LRS conformance. The second checks that receipt and the exact residual
baseline; its failure leaves the first package available for inspection. Substitute
`postgres` for the other backend. CI copies both evidence outputs before the gates
and uploads them even after failure. The runner's strict FAIL/NOT-RUN policy is
unchanged; no baseline entry is added automatically from a failing run.

## Bounded runner controls

After generating definitions, build the pinned runner and exercise the selected
46 cases against eight local finite responders:

```sh
nix develop -c cargo build --locked -p xapi-lrs-conformance
nix develop .#conformance-definitions -c python -B scripts/ci/run_issued_revision_controls.py \
  --runner target/debug/xapi-lrs-conformance \
  --out target/lrs-revision-controls
```

The output directory must not already exist. `summary.json` records exact expected
FAIL sets, mutation-specific admission/readback diagnostics and the one reason-bound
signature NOT-RUN per mode. Unrelated failures and unavailable observations are
rejected, including on otherwise passing cases. After an intended violation stops
a case, only the exact unvisited suffix of that definition is permitted; this does
not count the skipped checks as executed. A successful control run validates those
finite witnesses; it does not supply a signature provider or execute an LRS product.
These controls are manual; the Python unit controls run
under `lrs-oracle-controls`. Neither static artifact generation nor that Nix check
implicitly runs these responders.

The two raw-wire numeric-string controls use the same pinned runner:

```sh
nix develop .#conformance-definitions -c python -B scripts/ci/run_issued_wire_controls.py \
  --runner target/debug/xapi-lrs-conformance \
  --out target/lrs-wire-controls
```

The strict peer must pass both cases; the coercing peer must fail at the declared
admission check with only the expected unvisited readback. Unrelated failures or
unavailable observations cannot establish a mutation witness. The Person controls
likewise permit only the exact IRL observation for the valid Account response;
their malformed-OpenID witnesses must contain no unresolved observations.

## Intentional definition changes

1. Edit the authored inputs or generator. Preserve the previous lock and exact
   output identity as the comparison baseline.
2. Assess affected per-rule bindings and shared execution contracts. Compare the
   actual requirement, condition, exception and assertion changes; hashes alone
   do not demonstrate that the new definitions are correct.
3. Run `python -B scripts/ci/lrs_generated_artifacts.py refresh`. It expands from
   the new inputs and updates the artifact lock and local outputs. It does not
   create or update review-receipt/adoption records.
4. Run `check`, compare case/procedure identities and rule bindings, and perform
   the static validation above. Review the authored diff, compact lock diff and
   affected expanded definitions together.
5. Commit the inputs and inventory/lock, and submit the changes through the PR.
   Keep intermediate review administration outside the repository. Preserve an
   exact bundle when needed to reproduce or distribute the definitions.

The lower-level `audit_lrs_source_projection.py --skip-source --render-manifest`
only renders an index from already pinned ledger identities. It cannot update
the lock or grant acceptance. The existing `--update-bindings` path still
requires the pinned external source bytes. The ordinary generation/validation
path verifies the rendered bytes against the committed artifact lock.

## Portable review/distribution bundle

```sh
python -B scripts/ci/lrs_generated_artifacts.py bundle \
  --output target/lrs-generated/lrs-static.tar.gz
sha256sum target/lrs-generated/lrs-static.tar.gz
```

Run `generate`/`check` first. `bundle` verifies all current input/output identities
and includes them and the lock at their repository-relative paths. Archive order,
ownership, modes and timestamps are normalized, including the gzip header. The
same locked files produce the same archive bytes. The bundle is a static review
and distribution artifact, not a live test report or a buildable full checkout.
Its embedded generators can reproduce the outputs in the documented environment.

Retain the bundle with its digest and source commit in the review/release store.
An expiring CI upload is a convenience copy, not the sole historical record.
Even without a download service, a checkout of the corresponding source commit
and its lock can reconstruct the outputs. Previous reviewed/generated bytes remain
available in their original commits and sealed packets; this migration does not
rewrite those historical identities.
