# Confidential Aggregate Threat Model

## Scope

This document defines Kiva's first confidential-computation workflow target:
cross-institution aggregate learning analytics without analyst access to raw
personal learning records.

The workflow is intentionally narrow:

- one or more data custodians hold raw xAPI learning records;
- a confidential computation participant contributes the computation boundary;
- a policy authority authorizes result release;
- an analyst reads only the released aggregate insight.

This is a boundary and governance threat model. The first concrete mechanism
target is n-of-n additive secure aggregation over a fixed finite field. It does
not claim production MPC, malicious-secure PSI, hardware-vendor TEE quote
verification, production FHE, or full threshold-cryptography deployment yet.
Differential privacy support currently includes a bounded epsilon budget ledger, a checked
integer noise-application evidence boundary, a server-side CSPRNG-backed
integer discrete-Laplace-family sampler, and a fail-closed release gate for
already-noised aggregate results.
It also exposes basic sequential composition summaries for persisted releases
charged to one subject budget. It does not yet implement advanced
privacy-accounting composition across subjects or time windows, or a formal
distributional proof for the sampler. Kiva also implements a Shamir threshold
reconstruction boundary with hash-committed share-opening verification over the
same fixed finite field, plus a bounded hash-token PSI cohort matching
boundary, plus a portable deployment-bound TEE attestation evidence gate. The
Shamir boundary is threshold cryptography with VSS admission; the
two-authority threshold release workflow remains a governance approval gate.
Kiva also implements a deterministic FHE small-sum feasibility boundary for
strict parameter admission, fixture evaluation, and governed release, but not
production homomorphic encryption.
The implemented DP release profile is documented in
`docs/architecture/differential-privacy-assurance.md`.

## Assets

- Raw xAPI Statements and document resources.
- Subject-oriented Personal Data Store views.
- Confidential computation inputs, shares, transcripts, and intermediate
  artifacts.
- Aggregate insight outputs.
- Release decisions and audit evidence.

## Roles

- Data subject: the learner or person associated with records.
- Data custodian: an institution or service that can store and read raw records
  under policy.
- Computation participant: a party or node that contributes to an approved
  confidential workflow.
- Policy authority: the party authorized to release computation results.
- Analyst: a user or service allowed to read released aggregate insight.
- Auditor: a party that reviews evidence without needing raw records.

## Security Goals

The first workflow must maintain these boundary properties:

- Raw record reads are custodian-only.
- Analysts cannot read raw learning records.
- Aggregate computation requires both admitted custodian input and an admitted
  computation participant.
- Aggregate release requires policy-authority approval.
- Threshold-controlled aggregate release requests require a prior persisted
  Kiva aggregate computation result for the same transcript and value.
- Differential privacy aggregate release requires a computed aggregate,
  minimum population, non-zero epsilon budget within policy, and an explicit
  applied-noise flag.
- Server-sampled differential privacy aggregate release samples noise inside
  the server boundary, spends the subject-owned budget atomically, and publishes
  only the noised aggregate and accounting metadata.
- Differential privacy budget spending requires a valid, not-over-spent ledger,
  a non-zero request, and enough remaining epsilon budget.
- Differential privacy budget ledgers are subject-owned and persisted before
  release use.
- Differential privacy composition summaries must be derived from
  subject-owned budget ledgers and same-budget release metadata without
  exposing raw aggregate or noise evidence.
- Shamir threshold reconstruction requires a validated threshold plan, at
  least threshold unique verified share openings, participant indexes inside
  the participant set, successful commitment-opening verification, and
  successful finite-field interpolation.
- TEE-backed execution requires an accepted portable evidence envelope for the
  configured job id, measurement, image provenance digest, policy binding,
  validity window, evidence sequence, replay record, and policy approval before
  secrets or results are admitted.
- FHE feasibility release requires an admitted small-sum profile, n-of-n
  ciphertext fixtures, checked non-wrapping sum evaluation, policy approval,
  governed release treatment, and a clear plaintext/ciphertext
  boundary.
- Analyst aggregate reads require a prior aggregate release.
- Analyst aggregate reads require analyst registration.
- For threshold-controlled result release, a representative two-authority
  release path requires distinct policy authorities, both approvals after
  computation, and a release audit record before analyst threshold aggregate
  reads. Kiva now maps this representative boundary to a runtime approval API
  for releasing an already-computed aggregate value. This is a governance
  approval boundary, not a threshold-cryptography claim.

## Trust Assumptions

- The xAPI LRS layer correctly enforces the REST/JSON contract before records
  enter higher-level workflows.
- The policy layer classifies raw records, PDS views, aggregate insights,
  confidential-computation resources, and audit evidence consistently.
- The selected confidential-computation mechanism will later provide its own
  cryptographic security assumptions and implementation evidence. The current
  additive mechanism validates transcript structure, Ed25519 share-envelope
  signatures against active server-side signing-key registrations, Bearer
  subject binding at share submission, replay windows, and modular
  reconstruction. The current Shamir boundary validates threshold structure,
  unique participant indexes, finite-field membership, hash-committed
  share-opening verification, complaint outcomes, and interpolation, but does
  not prove malicious-dealer robustness or distributed key generation. The
  current TEE boundary assumes a deployment-specific verifier maps vendor
  evidence into Kiva's portable measurement, image digest, and policy binding
  fields; Kiva does not prove the vendor quote, cloud service, enclave runtime,
  or side-channel posture. The current FHE boundary is a deterministic fixture
  for feasibility and governance checks; it does not provide cryptographic
  confidentiality, key management, ciphertext serialization, or production
  homomorphic evaluation.
- Deployment documentation will state when multiple logical roles are operated
  by the same organization.

## Non-Goals

This threat model does not prove:

- cryptographic privacy of a selected MPC, malicious-secure PSI, hardware TEE,
  production FHE, or malicious-robust threshold scheme;
- correctness of aggregate analytics;
- differential privacy bounds;
- resistance to all collusion patterns;
- database, runtime, or operating-system isolation;
- full public-sector legal compliance.

## Evidence

The verification plan references these structural boundaries:

- `spec/confidential/confidential-computation-assurance-package.yaml` packages
  the selected CC1 through CC6 boundaries for external engineering review and
  records the CC7 non-claim language for legal certification, IEEE xAPI
  certification, full formal verification, production MPC, malicious-secure
  PSI, production FHE, vendor TEE quote verification, and
  malicious-dealer-robust VSS.
- `kiva-policy` encodes the baseline policy distinction between raw learning
  records, PDS views, aggregate insight, audit evidence, and confidential
  computation resources.
- `kiva-confidential` encodes release-policy structure for multi-party
  workflows, the first additive secure-aggregate transcript combiner, and the
  signed authenticated-share envelope and replay-window boundary. It also
  encodes the pure differential privacy budget ledger, basic sequential
  composition summary, integer noise evidence, deterministic sampler core, and
  release gate for already-noised aggregate results. It also encodes mechanism
  selection rules, Shamir threshold reconstruction over the fixed field, and
  bounded hash-token PSI cohort intersection over pre-tokenized identifier
  digests. It also encodes the portable TEE attestation evidence gate,
  including measurement, image digest, policy binding, freshness, rollback,
  replay, secret-provisioning, and result-release checks. It also encodes the
  deterministic FHE small-sum feasibility parameter gate, fixture evaluation,
  and release gate.
- `db/schema.sql` defines the durable replay-record table that enforces one
  accepted additive share per tenant, transcript, and participant at the
  database boundary, plus the active signing-key registration table, the
  submitted-share table used before envelope verification, the computed
  aggregate result table, the two-authority threshold-release request and
  approval tables, the subject-owned differential privacy budget ledger table,
  the Shamir reconstruction metadata table, and the PDS data-product
  release-reference columns that enforce source-kind references to existing
  confidential release metadata.
- `kiva-server` wires the first confidential signing-key registration,
  rotation, and revocation endpoints to OAuth/OIDC-derived subjects, wires the
  share submission endpoint to active registered-key and subject checks, and
  wires the confidential aggregate endpoint to active registered-key lookup, the
  signed envelope verifier, submitted-share loading, and the replay-record
  repository transaction boundary. The aggregate endpoint also persists the
  computed result used as evidence for threshold release requests. It also exposes
  `POST /confidential/threshold-release-requests`,
  `POST /confidential/threshold-release-approvals`,
  `POST /confidential/threshold-releases`, and
  `GET /confidential/threshold-release-requests` for the representative
  two-authority aggregate release workflow. It also exposes
  `POST /confidential/differential-privacy-budgets`,
  `POST /confidential/differential-privacy-budget-spends`,
  `GET /confidential/differential-privacy-composition-summaries`,
  `POST /confidential/differential-privacy-releases`,
  `POST /confidential/differential-privacy-sampled-releases`, and
  `GET /confidential/differential-privacy-budgets` for the bounded epsilon
  budget ledger, basic sequential composition summary, and noised aggregate
  release boundary. It also exposes
  `POST /confidential/shamir-threshold-reconstructions` and
  `GET /confidential/shamir-threshold-reconstructions` for owner-scoped
  Shamir reconstruction and metadata-only listing. PDS data-product
  registration now verifies that a referenced
  threshold release is already released by the authenticated subject, or that a
  referenced differential privacy release exists for that subject, before
  storing release metadata for BI use.
- `proofs/tamarin/kiva_confidential_workflow_boundary.spthy` models the
  workflow boundary and specifies the security goals listed above, including a
  representative two-authority threshold release and auditability path.
- `proofs/tamarin/kiva_differential_privacy_budget_boundary.spthy` models the
  representative budget ownership, spend, and release boundary for differential
  privacy aggregate release.
- `proofs/tamarin/kiva_shamir_threshold_boundary.spthy` models the
  representative Shamir threshold boundary: reconstruction requires a
  configured transcript, a distinct threshold share set, and verified share
  openings; duplicate-share rejection, sustained complaints, and rejected
  openings are not treated as accepted reconstruction inputs.
- `proofs/tamarin/kiva_pds_data_product_release_boundary.spthy` models the PDS
  data-product admission boundary and specifies that registered products trace to
  available same-owner, source-kind-specific confidential release metadata.
- `proofs/tamarin/kiva_pds_data_product_access_boundary.spthy` models the PDS
  data-product access boundary and specifies that product reads require a
  registered product and aggregate-insight consent scoped to the same owner.
- `proofs/tamarin/kiva_tee_attestation_boundary.spthy` models the portable
  TEE evidence boundary and specifies that accepted attestation traces to a
  configured and approved policy, secret provisioning and result release
  require accepted attestation, result release remains bound to the attested
  job, and stale, replayed, or rollback evidence rejection is not accepted or
  released.
- `proofs/tamarin/kiva_fhe_feasibility_boundary.spthy` models the FHE
  feasibility boundary and specifies that accepted ciphertext fixtures trace to a
  configured job, sum evaluation requires both participant ciphertexts, result
  release requires prior evaluation and policy approval, released output is
  governed as release metadata, and plaintext publication is absent.
- `proofs/tamarin/kiva_additive_share_transport_boundary.spthy` models the
  representative additive-share transport boundary and specifies that accepted
  shares trace to participants, required transcript slots, signing keys, and
  prior commitments before aggregate computation. It also requires accepted
  shares to pass signature-verification and replay-record persistence events.
- `proofs/tamarin/kiva_additive_collusion_boundary.spthy` models the
  representative additive-workflow collusion and separation boundary. It specifies
  that aggregate computation requires a scheduled two-party workflow with both
  participant shares, and that coordinators, analysts, and auditors do not gain
  share-submission or release authority from their operational or evidence
  roles.
- `spec/confidential/additive-secure-aggregate-vectors.yaml` records canonical
  additive aggregate vectors used by the Rust tests.
- `spec/confidential/shamir-threshold-vectors.yaml` records canonical Shamir
  reconstruction and share-opening commitment vectors used by the Rust tests.
- `spec/confidential/psi-cohort-vectors.yaml` records canonical PSI cohort
  digest vectors used by the Rust tests and assurance validator.
- `spec/confidential/tee-attestation-vectors.yaml` records canonical portable
  TEE attestation digest vectors used by the Rust tests and assurance
  validator.

The next confidential-computation step is to decide whether stronger
malicious-dealer-robust VSS/DKG, malicious-secure PSI, concrete vendor TEE
quote verification, or FHE should complement the current implementation
boundary for a concrete deployment.
