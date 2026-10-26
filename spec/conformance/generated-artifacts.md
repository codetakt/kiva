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
| Fixed signing inputs | `tests/conformance/fixtures/signing/cases.json` | Tracked; 42 exact public synthetic cases with original source and canonical selected-case digests; historical commit is a provenance label, not a required ref |
| Case expansion | Five YAML files under `tests/conformance/http/generated/` | Ignored; regenerated from models/scenarios and fixed inputs |
| Procedure expansion | `issued-definition-tests.yaml`, `issued-supplemental-tests.yaml` | Ignored; regenerated from contracts and scenario definitions |
| Derived relations | `lrs-table-bindings.yaml`, `issued-completion-bindings.json`, `issued-readback-origins.json`, `issued-rejection-origins.json` | Ignored; generated with the cases |
| Derived indexes | `issued-test-projection.json`, `issued-test-matrix.md`, `lrs-assertion-manifest.yaml` | Ignored; generated after the cases/procedures, bound to the definition and oracle inputs |
| Private evidence | Historical sealed packets, receipts, local logs | Preserve their recorded locations and scope; not regenerated or silently replaced |

This inventory concerns the LRS test projection. It does not reclassify the
already merged ledger's published tables or unrelated architecture generators.
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

`--allow-review-pending` is retained as a compatibility option; it does not change
the checks. No mode requires a review receipt. `--skip-source` explicitly leaves external source bytes unverified; the
merged ledger remains pinned. No licensed IEEE PDF or private historical review
packet is needed to reproduce the static outputs.

The validator and CI/evidence entry points materialize locked artifacts before
consuming them. The Rust runner rejects a missing generated case file when the
standard case root is selected; deliberately selected custom subsets remain
possible. Before invoking standalone reporting tools, run `generate` as above.
Live execution remains separate from structural reproduction. The strict runner
and backend scripts return nonzero for any FAIL or NOT-RUN. Product/provider
gaps remain open; a successful static check does not establish LRS conformance.

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
FAIL sets and the one reason-bound signature NOT-RUN per mode. A successful control
run validates those finite witnesses; it does not supply a signature provider or
execute an LRS product. These controls are manual; the Python unit controls run
under `lrs-oracle-controls`. Neither static artifact generation nor that Nix check
implicitly runs these responders.

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
