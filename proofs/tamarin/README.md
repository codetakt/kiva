# Tamarin Models

This directory contains symbolic protocol and authorization models for Kiva.

Run all current models with:

```sh
nix develop .#verification -c scripts/ci/run_tamarin_verification.sh
```

The models are deliberately small. They capture access-control,
resource-server acceptance, PDS consent/delegation, and representative
confidential-workflow boundaries without trying to prove the full runtime
implementation or the cryptography of a concrete OAuth/OIDC deployment.

Current models:

- `kiva_authorization_boundary.spthy`: seed role/token boundary for raw-record
  and aggregate-read access.
- `kiva_oauth_oidc_boundary.spthy`: resource-server acceptance model for
  trusted issuers, LRS audiences, delegated scopes, revocation, active
  introspection, token binding, refresh-token exchange, and replay resistance.
  It keeps grants and refresh tokens linear so proof search stays finite. The
  Rust M11 implementation enforces the runtime scope gate for already-validated
  bearer tokens; this model remains the symbolic deployment-protocol boundary
  and does not prove a concrete IdP, JWKS validator, or introspection client.
- `kiva_confidential_workflow_boundary.spthy`: PDS/BI and confidential
  aggregate workflow boundary model. It proves that raw reads are
  custodian-only, analysts do not read raw records, aggregate computation
  requires admitted custodian input and computation participation, release
  requires policy authority, analyst aggregate reads require release, and a
  representative two-authority threshold release requires distinct authorities,
  both approvals, prior computation, and a release audit event before analyst
  threshold aggregate reads.
- `kiva_additive_share_transport_boundary.spthy`: representative two-party
  additive-share transport boundary model. It proves that accepted shares trace
  to registered participants, required transcript slots, registered signing
  keys, active subject-bound signing keys, prior commitments,
  signature-verification events, subject-bound submissions, and persisted
  replay records. It also models submitted-share metadata audit as derived from
  subject-bound submission and not publishing share values or signatures,
  signing-key rotation and revocation so a revoked key cannot be accepted after
  revocation, and aggregate computation using accepted shares from both
  transcript participants.
- `kiva_additive_collusion_boundary.spthy`: representative additive-workflow
  collusion and separation boundary model. It proves that the two-party
  workflow uses distinct participants, aggregate computation requires
  scheduling plus both participant shares, coordinators cannot submit shares,
  analysts and auditors cannot release aggregates, release requires policy
  authority and prior computation, and share acceptance does not bypass release
  before aggregate reads.
- `kiva_pds_consent_boundary.spthy`: PDS consent/delegation boundary model. It
  proves that delegated reads require matching, unrevoked consent; raw-record
  reads require raw-read consent or explicit custodial authority; aggregate
  reads require aggregate-read consent; analysts cannot read raw records; and
  raw-read and aggregate-read scopes do not substitute for each other.
- `kiva_pds_data_product_release_boundary.spthy`: PDS data-product release
  admission boundary model. It proves that registered PDS data products trace
  to an available release for the same owner and source kind, that threshold
  sources require completed threshold release, that differential privacy
  sources require a persisted differential privacy release, and that Shamir
  threshold reconstruction sources require persisted reconstruction metadata.
- `kiva_pds_data_product_access_boundary.spthy`: PDS data-product access
  boundary model. It proves that data-product reads require a registered
  product, aggregate-insight consent for the same owner and analyst, unrevoked
  consent, and that raw-read consent does not substitute for product access.
- `kiva_public_sector_separation_boundary.spthy`: public-sector separation of
  duties and audit boundary model. It proves that platform operators and
  analysts cannot admit raw records, operators cannot approve releases,
  release approval requires policy authority, released aggregates require
  prior approval and emit audit events, aggregate reads require release, and
  auditor evidence access does not grant raw-data admission.
- `kiva_threshold_release_failure_boundary.spthy`: finite threshold-release
  failure/replay boundary model. It proves that persisted release requests
  require a computed aggregate, rejected missing-computed requests are not
  persisted under the same request identity, rejected non-authority approvals
  are not accepted, duplicate left approval does not release without a later
  right approval, and release requires both approvals.
- `kiva_differential_privacy_budget_boundary.spthy`: representative
  differential privacy budget boundary model. It proves that budget
  registration traces to a registered owner, accepted spends trace to the
  registered budget owner, foreign-subject spend rejection is not accepted, and
  aggregate release requires a prior budget spend and applied-noise event.
- `kiva_shamir_threshold_boundary.spthy`: representative Shamir threshold
  reconstruction and VSS-admission boundary model. It proves that
  reconstruction traces to a configured transcript, a distinct threshold share
  set, and verified share openings; that duplicate-share and sustained
  complaint rejections are not accepted as shares for the same request; and
  that rejected openings cannot be reconstructed as either side of the
  threshold pair. It does not model finite-field arithmetic, SHA-256 internals,
  distributed key generation, proactive refresh, or malicious dealer security.
- `kiva_psi_cohort_boundary.spthy`: representative bounded PSI cohort
  boundary model. It proves that released intersection digests require a
  configured PSI job and both participant submissions for the digest, while
  left-only and right-only non-matches cannot be released. It does not model
  SHA-256 internals, OPRF/DH PSI, malicious-secure PSI, or low-entropy
  identifier preimage resistance.
- `kiva_psi_oprf_boundary.spthy`: representative CC11 review-gated DH-OPRF
  PSI boundary model. It proves that release requires a reviewed runtime,
  intersection requires both participant submissions, accepted transcripts emit
  replay-record evidence, release requires policy authorization, left-only and
  right-only elements are not released, aborted transcripts are not released,
  and no raw identifier publication event exists. It does not model OPRF group
  arithmetic, runtime memory safety, malicious-secure PSI, or the external
  cryptographic review itself.
- `kiva_tee_attestation_boundary.spthy`: representative portable TEE
  attestation boundary model. It proves that accepted attestation traces to a
  configured and approved policy, that secret provisioning and result release
  require accepted attestation, that release remains bound to the attested job,
  and that stale, replayed, or rollback evidence rejection does not become
  accepted or released evidence. It does not model Nitro, SGX, SEV-SNP, vendor
  quote validation, side-channel resistance, or cloud-operator isolation.
- `kiva_tee_nitro_boundary.spthy`: representative CC12 AWS Nitro verifier
  boundary model. It proves that accepted Nitro attestation traces to
  configured policy, adapter validation facts, PCR and nonce matching, external
  security review, and replay-record creation, and that key/result release
  requires accepted attestation and prior secret provisioning. It does not
  model Nitro hardware, AWS infrastructure, COSE, X.509, CBOR parsing,
  side-channel resistance, or enclave runtime correctness.
- `kiva_threshold_frost_boundary.spthy`: representative CC13 FROST
  threshold-signing boundary model. It proves that accepted key ceremonies
  trace to configured reviewed runtime facts, threshold policy, active
  participants, custody evidence, complaint resolution, no equivocation, and
  replay-record creation; accepted signatures require an accepted ceremony,
  threshold signature shares, digest matching, and signing replay evidence;
  release requires accepted ceremony, accepted signature, active key epoch,
  governed key use, and policy approval. It does not model FROST group
  arithmetic, DKG soundness, proactive refresh cryptography, runtime memory
  safety, or malicious-participant robustness.
- `kiva_fhe_feasibility_boundary.spthy`: representative FHE sum feasibility
  boundary model. It proves that accepted ciphertext fixtures trace to a
  configured job, evaluation requires left and right ciphertext submissions,
  release requires prior evaluation and policy approval, released outputs are
  governed release outputs, and no public plaintext publication event exists. It
  does not model BFV/CKKS cryptography, key management, ciphertext
  serialization, bootstrapping, or production FHE security.
- `kiva_fhe_runtime_boundary.spthy`: representative CC14 review-gated FHE
  runtime boundary model. It proves that accepted ciphertext transcripts trace
  to a configured reviewed runtime, key-custody evidence, parameter-set
  binding, digest matching, and replay evidence; accepted evaluations require
  an accepted ciphertext transcript and evaluation proof digest; release
  requires accepted ciphertext metadata, accepted evaluation, active key epoch,
  governed release treatment, and policy approval. It does not model FHE
  arithmetic, parameter security, ciphertext serialization, runtime memory
  safety, decryption, or malicious-participant robustness.
- `kiva_confidential_job_lifecycle_boundary.spthy`: representative CC9 durable
  confidential job lifecycle model. It proves that release requires prior
  awaiting-release state, every lifecycle transition emits an audit event,
  released jobs do not return to running, and no raw-payload publication event
  exists. It does not model the distributed worker queue, mechanism-specific
  cryptography, or external operational certification.
- `kiva_confidential_participant_lifecycle_boundary.spthy`: representative
  CC10 confidential participant lifecycle model. It proves that job/key
  admission requires prior activation, every participant transition emits an
  audit event, removed participants emit removal audit evidence, and no
  raw-payload publication event exists. It does not model production
  PSI/FHE/TEE key ceremonies, DKG, or malicious-secure MPC.
