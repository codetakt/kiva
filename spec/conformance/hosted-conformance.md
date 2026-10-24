# Hosted conformance evidence

The [xAPI Conformance workflow](../../.github/workflows/conformance.yml)
reproduces the static definitions, runs oracle controls, captures disposable
memory/PostgreSQL executions and enforces their regression checks. Its dispatch
and pull-request triggers are defined in the workflow. A local Nix result does
not establish that the hosted collection or upload ran successfully.

To assess a hosted run, check its head SHA and attempt number against the intended
revision, then inspect all job/step conclusions, including artifact uploads.
Do not transfer a result to another revision without an input-identity comparison.

The workflow copies backend evidence before enforcing the gates and uploads
`conformance-regression` even after gate failure. Preserve both backend build logs,
`gates.log`, and each emitted `reports/xapi-<backend>-http.json`, `-http.xml`,
`-execution.log` and `-execution.json`. Missing reports and infrastructure errors
remain failures; successful artifact capture alone does not accept execution.

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

Commits marked `[skip ci]` have no hosted execution evidence. While hosted
execution is held, the first authorized workflow run (including a first run on
main after merge) is the first verification of these collection/upload steps.
Keep that distinction from the local Nix results and check the run as above.
