# CI Gates

This document records the pre-publication CI and branch-protection policy for
Kiva.

## CI Repository

The current pre-publication CI mirror is:

```text
https://github.com/codetakt/kiva-server-ci
```

The local repository uses the `ci` remote for that mirror:

```bash
git remote add ci https://github.com/codetakt/kiva-server-ci.git
```

## Required Checks

Protected `main` requires the following GitHub Actions job checks:

- `Core Checks & Server Build`
- `xAPI Conformance Evidence`
- `F*, Kani, and Tamarin`
- `Evidence Set Contract`

These checks correspond to the following workflow responsibilities:

- Core Rust formatting, linting, tests, documentation, dependency policy,
  database migrations, OpenAPI contracts including the PDS OpenAPI/Rust DTO
  boundary map and architecture-boundary drift validation,
  confidential-computation assurance
  for differential privacy, Shamir VSS admission, PSI cohort matching,
  review-gated DH-OPRF PSI upgrade, portable TEE attestation, review-gated AWS
  Nitro TEE attestation, review-gated FROST threshold signing, plus the FHE sum
  feasibility boundary and review-gated FHE runtime boundary, the CC7
  confidential-computation assurance package, the CC15 external production
  assurance readiness gate, the CC16 secret/confidential-compute provider
  contract, PDS publication-readiness checks, and server package build
- xAPI conformance planning, memory and PostgreSQL HTTP execution, PostgreSQL
  execution verification, and static conformance evidence packaging
- F*, Kani, and Tamarin formal-boundary checks plus formal smoke evidence
  packaging
- Strict release-evidence fixture validation and combined static/smoke evidence
  set validation, with PDS static evidence collected as a publication artifact

The OpenAPI contract gate is manifest-driven. Nix invokes
`scripts/ci/run_evidence_checks.py` against `spec/ci/evidence-checks.yaml`
using the `openapi-contract` group. The manifest also declares the group's Nix
gate, claim boundary, required check membership, and each check's `check_kind`,
input/output list, repository-mutation flag, network flag, and artifact class,
so group drift and execution metadata are validated from the manifest rather
than from a hardcoded Python list. That group runs the
OpenAPI validators, generated operation and surface projections, route coverage
projection, generated PDS schema artifacts, Rust DTO map checks, the DTO
boundary ledger validator, the generated DTO map projection manifest,
projection-graph validation, claim catalog projection with typed claim maturity
labels, design backbone validator, and
`scripts/ci/validate_architecture_boundaries.py` against
`spec/architecture/boundaries.yaml`. The design backbone is generated from
`spec/architecture/architecture-graph.source.yaml`, and the projection graph
itself is generated from `spec/architecture/architecture-graph.source.yaml#/sources`,
so CI checks source entries that declare `projection_sources`, targets, and
`projection_checks` against the rendered
`spec/ci/projection-graph.yaml`. Checks that set
`derived_input_policy: projection-graph` keep their direct `inputs` list empty
and rely on the projection graph for effective input reporting, avoiding a
second hand-maintained copy of source and target paths.

The architecture-boundary validator ensures contract evidence paths exist,
source boundary anchors exist, design documents mention current surface context
boundaries, retired architecture names are absent from docs and specs, public
HTTP handlers do not regress to full `AppState` extraction, runtime-independent
crates do not acquire server adapter dependencies, the application layer remains
HTTP-free, PDS route ownership remains pinned to the correct bounded HTTP
context, selected source owner files remain within review-size limits, selected
application/core/xAPI request sources stay free of wildcard imports, Cargo
dependency rules hold, selected module declarations stay on their intended
public/private boundary, milestone ledgers remain present, and CI check groups
retain their manifest-declared validator membership. Compatibility-surface
`rust-use-absent` checks use `tools/kiva-rust-use-scan`, an AST-based Rust
`use` scanner, instead of text matching arbitrary Rust source. The included
surface-source model check also pins human-editable surface ledgers, the split
surface model/generation/implementation policy files, and generated surface
outputs from `spec/architecture/surface-projections.yaml`.
The included control-plane boundary fragment also pins workflow entry through
`ApplicationOperationGateway`, PDS domain-event projection, and confidential
control-plane/data-plane separation.

The confidential-computation assurance gate uses the same manifest runner with
the `confidential-computation-assurance` group. That runner executes each
declared validator with its configured arguments and JSON report path and writes
a grouped run report, so adding or removing OpenAPI or assurance validators
happens in one manifest rather than inside the Nix expression.

`checks.x86_64-linux.design-review-suite` is the orchestration gate over the
same manifest. It invokes `scripts/ci/run_design_review_suite.py --suite all`,
which resolves the required evidence groups from `spec/ci/evidence-checks.yaml`
and writes a single design-review run report. The suite runner is deliberately
outside the evidence-check manifest so it cannot recursively validate itself.
Use it as the local and CI entrypoint when the question is whether all
manifest-declared design-review gates pass together.

## Branch Protection

The `main` branch is protected with:

- required status checks and up-to-date branches
- pull request review count set to zero for single-maintainer operation
- no last-pusher approval restriction
- required conversation resolution
- linear history
- admin enforcement
- force-push and branch-deletion denial

Direct pushes to `main` are intentionally not the normal workflow. Pull
requests remain the expected publication path so that required CI evidence,
Copilot review comments, and conversation-resolution gates are visible before
merge.

## Pull Request Flow

Use a topic branch:

```bash
git switch -c <topic>
git push -u ci <topic>
gh pr create --repo codetakt/kiva-server-ci --base main --head <topic>
```

Merge only after the required checks are green and all conversations are
resolved. Human approval is optional in the current single-maintainer setup and
is not a branch-protection blocker.

## Claim Boundary

Green CI supports Kiva's internal release-readiness and conformance-evidence
claims. Confidential-computation assurance evidence supports the selected CC1
through CC16 engineering and release-candidate readiness boundaries, including
review-gated PSI, AWS Nitro TEE, FROST threshold-signing, FHE runtime, and
external production assurance gates plus the provider contract where
production claims remain blocked by external review outcomes.
PDS publication-readiness evidence supports the current Kiva PDS extension
boundary only. It does not create an IEEE certification claim, expand the
IEEE/ISO/IEC 39274-1-1:2025 xAPI LRS conformance surface, or prove complete
end-to-end formal correctness.
