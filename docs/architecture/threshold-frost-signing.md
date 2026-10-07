# Threshold FROST Signing Boundary

CC13 implements `kiva-threshold-frost-ed25519-review-gated-v1`: a
review-gated FROST threshold-signing runtime boundary for Kiva confidential
jobs.

This is not an in-process FROST implementation. Kiva delegates DKG, share
custody, proactive refresh, complaint handling, equivocation detection, and
signature generation to a deployment-selected runtime. Kiva admits normalized
metadata from that runtime and enforces the surrounding policy, replay,
release, and audit gates.

## Boundary

The external runtime is responsible for:

- running FROST DKG or importing a reviewed key ceremony;
- protecting participant private shares outside Kiva persistence;
- producing public key-ceremony commitment metadata;
- producing share-custody evidence metadata;
- reporting complaint outcomes and equivocation evidence;
- performing proactive refresh when participant replacement or key rotation
  requires a new key epoch;
- producing threshold signature-share metadata and a final signature digest;
- binding its artifact digest and external review evidence digest.

Kiva is responsible for:

- requiring a threshold of at least two;
- requiring exactly one key-ceremony transcript per participant;
- rejecting removed, inactive, aborted, or out-of-range participants;
- rejecting unresolved complaints;
- rejecting detected equivocation;
- requiring share-custody metadata before ceremony admission;
- deriving a domain-separated key-ceremony digest;
- rejecting key-ceremony replay;
- accepting signing transcripts only after an accepted key ceremony;
- requiring at least threshold unique signature shares;
- deriving and checking a domain-separated signing transcript digest;
- requiring active key epoch, policy approval, governed key use, and external
  cryptographic review before release.

## Record Boundary

Kiva defines and validates only metadata record shapes:

- runtime artifact digest;
- external review evidence digest;
- key epoch;
- public commitment digest;
- share-custody evidence digest;
- key-ceremony digest;
- message digest;
- signature digest;
- signing transcript digest;
- replay records.

The current repository carries FROST release and audit evidence through the
generic confidential job lifecycle. It does not add a dedicated FROST transcript
HTTP API or PostgreSQL FROST transcript table.

Kiva does not persist FROST private shares, raw DKG messages, raw signature
shares, or signing nonces in ordinary Kiva persistence.

## Claim Language

Allowed claim:

- Kiva has a review-gated FROST threshold-signing boundary for governed
  metadata-only key use.

Prohibited claims before external cryptographic review:

- Kiva implements FROST in-process;
- Kiva provides production threshold cryptography;
- Kiva provides malicious-robust threshold cryptography;
- Kiva proves DKG cryptographic soundness;
- Kiva proves proactive refresh cryptographic soundness;
- Kiva persists FROST transcripts in a dedicated HTTP/PostgreSQL store;
- Kiva proves side-channel resistance of the selected runtime.

## Assurance

Implemented assurance includes:

- Rust core types and tests in `crates/kiva-confidential/src/lib.rs`;
- deterministic vectors in `spec/confidential/threshold-frost-vectors.yaml`;
- job contract in `spec/confidential/threshold-frost-job-contract.yaml`;
- assurance profile in `spec/confidential/threshold-frost-assurance.yaml`;
- validator `scripts/ci/validate_threshold_frost_assurance.py`;
- Kani harnesses for key-ceremony and release gates;
- Tamarin model `proofs/tamarin/kiva_threshold_frost_boundary.spthy`;
- operator runbook `docs/operations/threshold-frost-runbook.md`.

The Tamarin boundary model specifies that accepted ceremonies and
signatures trace to the expected configured facts and that release requires
accepted ceremony, accepted signature, active key epoch, governed key use, and
policy approval. It does not prove FROST group arithmetic, DKG soundness,
runtime memory safety, or malicious-participant robustness.
