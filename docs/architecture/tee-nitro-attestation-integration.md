# AWS Nitro Attestation Integration

CC12 implements `kiva-tee-aws-nitro-review-gated-v1`: a review-gated AWS Nitro
Enclaves attestation boundary for Kiva confidential jobs.

This boundary selects AWS Nitro Enclaves as the first concrete TEE target. Kiva
still does not parse raw COSE/CBOR Nitro attestation documents in-process. A
deployment-specific verifier adapter must verify the raw document and emit the
normalized claims that Kiva admits.

## Boundary

The verifier adapter is responsible for:

- parsing the raw Nitro attestation document;
- validating the certificate chain and configured trust anchor;
- validating the COSE signature;
- validating the nonce or challenge binding;
- validating PCR measurements against the configured policy;
- rejecting debug-mode documents and all-zero debug PCR values;
- emitting normalized `verified-nitro-attestation-document-v1` claims;
- recording its artifact digest, trust-anchor digest, verifier key epoch, and
  external-review evidence digest.

Kiva is responsible for:

- binding normalized claims to a confidential job id;
- matching PCR0, PCR1, PCR2, PCR3, and PCR8 against policy;
- matching image provenance, policy binding, and nonce digest;
- enforcing freshness windows, monotonic evidence sequence floors, and replay
  records;
- rejecting stale, rollback, replayed, debug, mismatched, or unreviewed
  evidence;
- releasing keys only after accepted attestation, policy approval, external
  review, and current verifier key epoch;
- releasing results only after key provisioning and matching job policy.

## PCR Policy

The CC12 policy pins the PCRs that matter for Kiva's first Nitro boundary:

- PCR0: enclave image measurement;
- PCR1: kernel and bootstrap measurement;
- PCR2: application measurement;
- PCR3: parent IAM role binding;
- PCR8: EIF signing-certificate measurement.

PCR values are modeled as raw bytes rather than `SecureAggregateCommitment`
because Nitro debug-mode attestations can carry all-zero PCR values. Kiva keeps
those values representable so the rejection path is testable.

## Rollout And Rollback

Every image rollout requires:

- a new PCR policy;
- a new image provenance digest;
- verifier policy update;
- policy-authority approval before key release;
- evidence sequence floor review.

Rollback is allowed only as an operator-controlled incident action. The
rollback must update the retired sequence floor and record the previous image
digest. Kiva rejects evidence below the configured sequence floor even if the
attestation document is otherwise fresh.

## Claim Language

Allowed claim:

- Kiva has a review-gated AWS Nitro attestation verifier boundary for
  confidential job key and result release.

Prohibited claims before external security review:

- Kiva provides production hardware-backed confidential execution;
- Kiva itself verifies raw Nitro COSE/CBOR attestation documents;
- Kiva proves cloud-operator isolation;
- Kiva proves hardware side-channel resistance;
- Kiva proves enclave runtime correctness.

## Assurance

Implemented assurance includes:

- Rust core types and tests in `crates/kiva-confidential/src/lib.rs`;
- deterministic vectors in
  `spec/confidential/tee-nitro-attestation-vectors.yaml`;
- job contract in
  `spec/confidential/tee-nitro-attestation-job-contract.yaml`;
- assurance profile in
  `spec/confidential/tee-nitro-attestation-assurance.yaml`;
- validator
  `scripts/ci/validate_tee_nitro_attestation_assurance.py`;
- Kani harnesses for admission and key/result release gates;
- Tamarin model `proofs/tamarin/kiva_tee_nitro_boundary.spthy`;
- operator runbook `docs/operations/tee-nitro-runbook.md`.

The Tamarin boundary model specifies that accepted Nitro
attestation traces to configured policy, adapter validation facts, PCR and
nonce matching, external review, and replay-record creation; it does not prove
Nitro hardware, AWS infrastructure, COSE, X.509, CBOR parsing, or enclave code.
