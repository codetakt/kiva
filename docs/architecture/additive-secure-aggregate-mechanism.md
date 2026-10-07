# Additive Secure Aggregate Mechanism

## Scope

This is Kiva's first concrete confidential-computation mechanism target. It is
an n-of-n additive secure aggregation transcript over a fixed finite field.

The mechanism is intentionally narrow. It gives Kiva a strict implementation
core for validating participant counts, one-share-per-participant transcripts,
field membership, and aggregate reconstruction. It does not yet claim a
production MPC deployment.

## Mechanism

The current field modulus is:

```text
p = 4,294,967,291
```

This is the largest 32-bit prime. Each share is a field element `s_i` where
`0 <= s_i < p`. The released aggregate is:

```text
aggregate = sum(s_i) mod p
```

The current release policy is n-of-n:

- `participant_count >= 2`
- `release_threshold == participant_count`
- each participant index is one-based and within the participant set
- each participant index appears exactly once
- each accepted authenticated envelope must match a registered commitment slot
  by transcript identifier, participant index, commitment, and signing-key
  identifier
- the signed production path verifies an Ed25519 signature over the canonical,
  domain-separated additive-share envelope payload before acceptance
- the Kiva server does not trust verification keys supplied by an aggregate
  request; it resolves each slot's signing-key identifier against active
  server-side registrations
- participants can submit signed additive-share envelopes through
  `POST /confidential/additive-share-submissions`; the server accepts a
  submission only when the claimed signing key is active, bound to the Bearer
  subject, bound to the registered participant index, and verifies the envelope
  signature
- each accepted authenticated envelope must carry a non-zero sequence inside
  the configured replay window
- each accepted envelope produces a replay record that must be persisted before
  later envelopes for the same transcript participant are accepted
- the Postgres schema stores replay records with a
  `tenant_id, transcript_id, participant_index` primary key, so sequence changes
  cannot bypass single-use participant acceptance
- the Kiva server endpoint `POST /confidential/additive-aggregates` reads
  existing replay records, verifies signed envelopes, computes the aggregate,
  and persists accepted replay records and the computed aggregate result through
  one repository transaction
- the Kiva server exposes non-share audit metadata through
  `GET /confidential/additive-share-signing-keys` and
  `GET /confidential/additive-share-submissions?transcriptId=...` as
  subject-scoped metadata, and through
  `GET /confidential/additive-share-replay-records?transcriptId=...` as
  transcript-scoped metadata, so operators can inspect registered-key state,
  submitted-share state, and accepted replay records without returning
  additive share values or signed envelopes

Threshold release, Shamir sharing, differential privacy, PSI, and TEE-backed
execution are separate mechanisms. They must not be represented as this
additive n-of-n transcript. Kiva now implements Shamir threshold
reconstruction with hash-committed VSS admission as a separate mechanism
documented in
`shamir-threshold-cryptography.md`. Kiva's threshold-release HTTP path remains
a two-authority governance gate for releasing an already-computed aggregate
value, not a threshold-cryptography mechanism.

## Security Boundary

This mechanism helps enforce structural transcript integrity:

- malformed field elements are rejected;
- partial transcripts are rejected;
- duplicate participant shares are rejected;
- out-of-range participant indexes are rejected;
- the aggregate is reconstructed with deterministic modular arithmetic.
- the representative share-transport boundary requires accepted shares to
  trace to registered participants, required transcript slots, registered
  signing keys, and prior commitments.
- the representative collusion boundary requires distinct two-party
  participants, workflow scheduling, both accepted participant shares, policy
  authority release, and release before analyst reads. It also keeps
  coordinators from submitting shares and keeps analyst/auditor roles from
  releasing aggregate results.
- the Rust boundary rejects invalid field elements, invalid release policies,
  non-n-of-n additive plans, and empty transcript identifiers, commitments,
  signing-key identifiers, and transport proof tags, including through serde
  deserialization.
- the Rust combiner accepts only envelopes that match the expected commitment
  slot before reconstruction.
- the Rust signed-envelope combiner verifies Ed25519 signatures against the
  registered verification key before replay-window acceptance and aggregate
  reconstruction.
- the Rust replay-window boundary rejects zero sequence values, inverted replay
  windows, out-of-window envelopes, and envelopes whose replay record already
  exists.
- the database schema provides a durable replay-record table with byte-width
  checks for transcript and signing-key identifiers, positive participant
  indexes, and unsigned 64-bit sequence bounds.
- the database schema provides a signing-key registration table with byte-width
  checks for signing-key identifiers and Ed25519 public keys, positive
  participant indexes, OAuth/OIDC subject binding, and an active/revoked state
  boundary.
- the database schema provides a submitted-share table keyed by tenant,
  transcript, and participant; repeated identical submissions are idempotent,
  while different submissions for the same participant conflict.
- the Kiva server endpoint
  `POST /confidential/additive-share-signing-key-revocations` lets the
  registering OAuth/OIDC subject revoke a signing key; revoked keys are not used
  for aggregate verification.
- the Kiva server endpoint
  `POST /confidential/additive-share-signing-key-rotations` lets the registering
  OAuth/OIDC subject atomically register a replacement key for the same
  participant and revoke the old key.
- the server repository inserts accepted replay records in the same transaction
  used to read active registered signing keys, read submitted envelopes when
  the release request omits inline envelopes, read prior replay records, and
  compute and persist the release result; database conflicts are returned as
  replay detection.
- the audit read endpoints expose only signing-key metadata, submitted-share
  metadata, and replay-record metadata. They do not expose submitted share
  values, signed envelopes, or aggregate input payloads.
- `POST /confidential/threshold-release-requests`,
  `POST /confidential/threshold-release-approvals`, and
  `POST /confidential/threshold-releases` add a distinct two-authority
  governance release boundary for an already-computed aggregate, while
  `GET /confidential/threshold-release-requests` exposes request and approval
  metadata without returning the aggregate value. A threshold-release request
  is accepted only when its transcript and aggregate value match a persisted
  additive aggregate result.
- `POST /confidential/differential-privacy-releases` adds a distinct
  noised-output release boundary for an already-computed aggregate when the
  caller supplies explicit aggregate and noise evidence. The server requires a
  subject-owned differential privacy budget, a matching persisted aggregate
  result, checked signed integer noise application, population and epsilon
  release-gate validation, and an atomic budget spend before persisting the
  release evidence.
- `POST /confidential/differential-privacy-sampled-releases` uses the same
  release, validation, idempotency, budget-spend, and metadata-hiding boundary,
  but samples integer discrete-Laplace-family noise inside the server from OS
  randomness instead of accepting a caller-supplied noise value. Public
  responses and metadata lists return the noised value and accounting
  parameters, not the raw aggregate or recorded noise sample.
- `GET /confidential/differential-privacy-composition-summaries` adds a basic
  sequential-composition audit view for subject-owned budgets. It reports
  ledger spend, release-attributed epsilon spend, unattributed manual spend,
  remaining budget, release count, and max single-release epsilon without
  returning raw aggregate or noise evidence.

The following concerns remain outside this core:

- random share generation;
- operator lifecycle procedures and external key-provisioning ceremonies;
- Shamir VSS admission, DKG, or threshold-decryption semantics;
- malicious participant robustness beyond this n-of-n additive transcript;
- collusion resistance beyond the n-of-n transcript assumption;
- deployment-specific privacy review beyond the implemented CC2 DP release
  profile, including concrete BI-query sensitivity and cross-system
  composition policy.

Those concerns need separate protocol assumptions, tests, and proof artifacts
before Kiva can claim production-grade confidential aggregation.

## Test Vector

The canonical vector is recorded in
`spec/confidential/additive-secure-aggregate-vectors.yaml`.

For three participants:

```text
s_1 = 1,000,000,000
s_2 = 2,000,000,000
s_3 = 1,294,967,333
```

The sum is `4,294,967,333`, so the released aggregate is `42` modulo
`4,294,967,291`.

## Evidence

- `crates/kiva-confidential/src/lib.rs` implements the field element, share,
  n-of-n plan, commitment slot, authenticated envelope, envelope acceptance,
  Ed25519 signed-envelope verification boundary, replay-window check, accepted
  replay record, and share combiner.
- `spec/openapi/kiva-confidential.yaml` records the Kiva-specific
  confidential-computation HTTP contract separately from the IEEE xAPI LRS
  OpenAPI contract.
- `db/schema.sql` and
  `db/migrations/20260619100000_confidential_additive_share_signing_keys.sql`
  define the active signing-key registration table used before envelope
  verification.
- `db/schema.sql` and
  `db/migrations/20260619110000_confidential_additive_share_submissions.sql`
  define the submitted-share table used by the per-participant transport
  endpoint.
- `db/schema.sql` and
  `db/migrations/20260619090000_confidential_additive_share_replay_records.sql`
  define the durable replay-record persistence table.
- `db/schema.sql` and
  `db/migrations/20260619130000_confidential_additive_aggregate_results.sql`
  define the computed aggregate result table used to bind governance release
  requests to prior Kiva computation.
- `db/schema.sql` and
  `db/migrations/20260619120000_confidential_threshold_releases.sql` define
  the two-authority threshold-release request and approval tables.
- `db/schema.sql` and
  `db/migrations/20260619150000_confidential_differential_privacy_releases.sql`
  define noised differential privacy aggregate release evidence.
- `crates/kiva-server/src/surfaces/confidential/routes.rs` registers the
  confidential routes, `surfaces/confidential/handlers.rs` performs
  authentication, parsing, and HTTP status selection,
  `app/confidential.rs` plus `app/confidential/` own use-case orchestration, and
  `surfaces/confidential/repository.rs` plus
  `surfaces/confidential/storage.rs` own memory/PostgreSQL dispatch and
  implementation bodies. Together they wire
  `POST /confidential/additive-share-signing-keys` to
  OAuth/OIDC-subject-bound signing-key registration,
  `POST /confidential/additive-share-submissions` to active-key and
  subject-bound share submission,
  `POST /confidential/additive-share-signing-key-rotations` to subject-owned
  signing-key rotation,
  `POST /confidential/additive-share-signing-key-revocations` to subject-owned
  signing-key revocation, and `POST /confidential/additive-aggregates` to
  active registered-key lookup, submitted or inline signed aggregate
  verification, replay-record loading, aggregate computation, and replay-record
  persistence. They also expose `GET /confidential/additive-share-signing-keys`,
  `GET /confidential/additive-share-submissions`, and
  `GET /confidential/additive-share-replay-records` for non-share audit
  metadata; persist computed aggregate results; and expose threshold-release
  and differential privacy release endpoints that require matching prior
  computed aggregate evidence before governed release metadata is persisted.
- `proofs/kani/README.md` tracks bounded verification targets for the pure
  Rust invariants.
- `proofs/tamarin/kiva_confidential_workflow_boundary.spthy` models the
  workflow authorization boundary around aggregate computation and release.
- `proofs/tamarin/kiva_additive_share_transport_boundary.spthy` models the
  representative authenticated-share and commitment boundary before aggregate
  computation, including active subject-bound signing keys, signing-key
  rotation and revocation, subject-bound submissions, signature-verification,
  replay-record persistence events, and submitted-share metadata audit that
  does not publish share values or signatures.
- `proofs/tamarin/kiva_additive_collusion_boundary.spthy` models the
  representative additive-workflow collusion and separation boundary around
  scheduling, two-party share contribution, release authority, audit metadata,
  and aggregate reads.
