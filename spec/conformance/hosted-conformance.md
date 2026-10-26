# Hosted conformance evidence

The [xAPI Conformance workflow](../../.github/workflows/conformance.yml)
reproduces the static definitions, runs oracle controls, captures disposable
memory/PostgreSQL executions and enforces their regression checks. Its dispatch
and pull-request triggers are defined in the workflow. A local Nix result does
not establish that the hosted collection or upload ran successfully.

To assess a hosted run, check its head SHA and attempt number against the intended
revision, then inspect all job/step conclusions, including artifact uploads.
Do not transfer a result to another revision without an input-identity comparison.

Memory and PostgreSQL run in separate matrix jobs on isolated runners, with
`fail-fast: false`. Each still executes the complete suite against its own target;
cases are not run concurrently against a shared LRS. The static plan/bundle,
definition reproduction and oracle controls run independently of both backends.
PostgreSQL storage verification remains in the PostgreSQL job.

Each backend copies its evidence before enforcing its gate and uploads
`conformance-regression-memory` or `conformance-regression-postgres` even after
gate failure. Preserve the backend's `build.log`, `gate.log` and each emitted
`reports/xapi-<backend>-http.json`, `-http.xml`, `-execution.log` and
`-execution.json`; PostgreSQL also supplies `verification.log`. Missing reports
and infrastructure errors remain failures; successful artifact capture alone
does not accept execution.

The final `xAPI Conformance Evidence` job preserves the existing check name. It
collects the available backend artifacts into `conformance-regression`, retaining
the `memory/` and `postgres/` directories, including when a dependency fails.
The original per-backend uploads remain available if aggregation fails. This job
requires every dependency to succeed: failed, cancelled or skipped jobs cannot
produce a successful overall check. The static job separately uploads
`conformance-evidence-static`.

Verify each execution receipt's backend, original exit, input identities and raw
artifact hashes. Compare reports with the committed `ci-execution-baseline.json`;
never update the baseline automatically to accommodate unexpected results. A
separate comparison of downloaded reports must be labeled as a replay, not a
fresh target run. Check the uploaded static bundle and lock against the committed
reproducible identities as well.

The [claim boundary](../../docs/architecture/conformance-claim.md) applies to all
these results. Known FAIL/NOT-RUN outcomes remain visible. A successful run does
not prove the failure-upload branch or complete product conformance. GitHub run
logs and artifacts carry execution evidence; no repository review receipt or
adoption record is required.

An automatically skipped workflow supplies no hosted execution evidence. Commit
messages containing `[skip ci]` suppress automatic push/pull-request workflows;
an explicit `workflow_dispatch` can still verify that revision. Run the workflow
on the intended PR revision during formal review and before merging it.
Check the actual branch protection and required-check settings;
do not infer their configuration from the existence of workflow checks.

## Runtime and cache behavior

The backend wrappers and static runner use the workspace release profile, matching
the Nix dependency artifacts. The static package retains the inherited Cargo target
directory. The static bundle verifies its already generated artifacts when invoking
the projection validator. The independent reproduction check still expands twice
with different hash seeds. No outcome, prerequisite, polling window or assertion
is removed to reduce runtime.

Before parallelization, [run 40287271568](https://github.com/codetakt/kiva/actions/runs/40287271568)
took about 131 minutes in the evidence job: memory capture 37m28s, PostgreSQL
capture 53m09s, checks 16m53s and static evidence 21m16s. Its setup logs reported
both FlakeHub Cache and the native GitHub Actions cache disabled. These timings
include builds and are a baseline, not a prediction for subsequent runs.

The shared setup action now explicitly enables the GitHub Actions backend of the
Nix binary cache. No FlakeHub account, secret or OIDC permission is required for
this backend. Nix still validates derivation identities; only matching store
outputs can be reused. GitHub's cache scope and eviction policies apply, so a
first run or a cache miss still performs the build. Parallel cold jobs may each
build a dependency before either has uploaded it. A cache hit on an execution
derivation reuses its existing receipt and reports; it is not a fresh target run.

The backend jobs have independent 100-minute limits, static evidence 60 minutes,
and the aggregation job 10 minutes. The critical path is the slowest parallel job
plus aggregation, rather than the sum of backend and static durations. Confirm
the actual overlap, cache activation/hits, uploads and total duration in hosted
runs; runner availability and cold builds can still affect elapsed time.
