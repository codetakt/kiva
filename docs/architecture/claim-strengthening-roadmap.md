# Claim Strengthening Roadmap

## Scope

This roadmap describes proposed boundaries for learner agency, analytics serving
and externally reviewed confidential-computation deployments.

This document is planning guidance, not a claim. Per repository convention,
each milestone enters the machine-readable ledgers
(`spec/pds/publication-milestones.yaml`,
`spec/confidential/computation-milestones.yaml`, or a new ledger for new
families) when its boundary work lands, together with plane coverage,
invariants, CI checks, implementation anchors, and a
`spec/architecture/claim-catalog.yaml` entry for any new public wording.
Milestone IDs below are reserved proposals, not ledger rows.

## Track A: Learner Agency (PDS family, reserved M15-M17)

### M15 Learner-principal deployment profile

- Boundary: a validated deployment profile in which the data subject is the
  OAuth/OIDC token subject, exercising existing `pds:read`, `pds:write`,
  `pds:delegate`, and `pds:revoke` scopes over their own consent grants,
  revocations, inventory, and lifecycle cases. Builds directly on the M11
  resource-server boundary; no new authorization semantics.
- Exit criteria: deployment template under `spec/pds/deployment-templates.yaml`
  plus tests proving subject-bound token flows for consent grant, revocation,
  inventory read, and metadata export; template validation wired into
  `pds-publication-readiness`.
- Unlocked claim language: "learners can directly exercise consent and
  revocation over their own records."
- Estimated scope: small. The scope vocabulary and handlers already exist;
  the milestone is a profile, fixtures, and evidence.

### M16 Production IdP integration adapter

- Boundary: JWKS signature validation, token introspection, and tenant claim
  transformation as a deployment adapter crate outside the resource-server
  core, preserving the M11 fail-closed boundary. The resource-server core
  continues to consume validated subjects and scopes.
- Exit criteria: adapter crate with contract tests against a fixture IdP,
  boundary rules keeping the adapter out of runtime-independent crates, and
  documentation of the deployment seam.
- Unlocked claim language: "connects to production OAuth/OIDC identity
  providers."

### M17 Subject data portability export

- Boundary: learner-initiated export of accepted xAPI Statements plus PDS
  metadata for the requesting subject, admitted as a governed operation with
  consent and audit evidence. Extends the M4 metadata-only export to include
  statement payloads under the subject's own authority.
- Exit criteria: documented export format, OpenAPI operation with exact
  scopes, PostgreSQL execution tests, and audit-event coverage; explicit
  non-expansion of the xAPI LRS conformance claim.
- Unlocked claim language: "learner-initiated data portability."
- Depends on: M15.

## Track B: Externally Reviewed Confidential Computation (CC family, reserved CC17-CC19)

### CC17 External production review execution

- Boundary: execute the CC15 gate for real. Externally generated evidence
  from a clean release-candidate run, plus external review outcomes, bound to
  the exact deployment profile (`multi-agency-research-aggregate-v1`),
  mechanism set, provider contract, and release-candidate commit.
- Exit criteria: the CC15 evidence-set validation passing on externally
  generated bundles; review outcomes recorded in the assurance evidence
  registry; claim-catalog wording updated through the readiness gate.
- Unlocked claim language: "externally reviewed confidential-computation
  configuration for production-grade cross-organizational aggregation."
- Depends on: an external reviewer engagement; no new code boundary.

### CC18 First production provider binding

- Boundary: one real external secret/confidential-compute provider admitted
  through the CC16 contract with runtime gates enabled in a reference
  deployment, exercising provider-binding registry, admission evidence, and
  runtime gate ownership end to end.
- Exit criteria: provider TCK passing for the admitted provider; binding and
  runtime-gate evidence in the registry; deployment runbook.
- Unlocked claim language: "production provider integration track record."
- Depends on: CC17 for production wording; can start earlier as evaluation.

### CC19 Review-gated mechanism promotion (per mechanism)

- Boundary: promote one review-gated mechanism at a time (DH-OPRF PSI, AWS
  Nitro TEE attestation, FROST threshold signing, FHE runtime) by binding
  external review evidence to the runtime artifact, as CC11-CC14 already
  require.
- Exit criteria: per-mechanism external review evidence bound in the
  assurance registry; maturity ledger row updated.
- Unlocked claim language: mechanism-specific production wording, one
  mechanism per promotion.
- Depends on: CC17; mechanism order is a deployment decision.

## Track C: Analytics Serving Layer (new BI family, reserved BI0-BI2)

This track defines governed aggregate serving with dashboards as external
consumers.

### BI0 Analytics claim boundary

- Boundary: define the analytics surface as governed reads over approved
  data products and confidential releases, separate from the xAPI LRS
  conformance surface and from raw-statement access.
- Exit criteria: a new milestone ledger and claim-boundary document mirroring
  the PDS M0 pattern; claim-catalog entry with a conservative maturity label.

### BI1 Governed aggregate query surface

- Boundary: a read API over approved data products with exact
  OAuth/OIDC scopes, cursoring, and audit evidence, sufficient for external
  BI tools to consume without touching raw records.
- Exit criteria: OpenAPI contract, surface-catalog and operation-kernel
  registration, memory and PostgreSQL tests, scope enforcement tests.
- Unlocked claim language: "analytics-ready governed aggregate serving for
  institutional reporting."
- Depends on: BI0.

### BI2 Reference reporting deployment template

- Boundary: validated deployment guidance plus a reference dashboard
  configuration (external tooling) for school and corporate reporting over
  the BI1 surface; guidance, not a hosted BI product claim.
- Exit criteria: template validation in CI mirroring the M13 pattern.
- Unlocked claim language: "reference reporting deployments for schools and
  corporate training."
- Depends on: BI1.

## Track D: External Conformance Recognition (reserved X1)

### X1 Full ADL LRS Test Suite compatibility evidence

- Boundary: run the complete ADL xAPI LRS conformance test suite against a
  release candidate and publish the compatibility report inside the
  conformance evidence bundle. ADL results remain compatibility evidence.
  The requirements baseline is the pinned IEEE Std 9274.1.1-2023;
  [xAPI claim limits](conformance-claim.md) remain in force, including
  unestablished 2025 equivalence and separate implementation evidence.
- Exit criteria: full-suite run recorded in the evidence bundle with the
  existing ADL compatibility report tooling; bundle validator updated to
  require it for release candidates.
- Unlocked claim language: "full ADL LRS Test Suite compatibility evidence
  for the release candidate," which procurement reviewers widely recognize.

## Sequencing by Claim Value

Recommended order, optimizing unlocked claim strength per unit of work:

1. **M15** - cheapest milestone with the largest positioning gain
   (learner-exercised agency substantiates the data-sovereignty orientation).
2. **X1** - external recognition for the already-strong xAPI core.
3. **CC17** - unlocks the word "production" for the confidential layer;
   long lead time because it needs an external reviewer, so start early.
4. **M16** - removes the largest practical deployment objection.
5. **BI0 + BI1** - makes the BI positioning honest.
6. **M17, BI2, CC18, CC19** - deepen each track after the headline claim
   upgrades land.

Each completed milestone updates the claim catalog through the normal
readiness gate; public wording changes only after the corresponding evidence
is bound.
