# FHE Runtime Runbook

This runbook covers the CC14 review-gated FHE sum runtime boundary. It is for
operators integrating an externally reviewed FHE runtime with Kiva metadata
and release governance. It does not make a production FHE claim by itself.

## Runtime Selection

1. Select a runtime for the `bfv-integer-sum` / `sum` profile.
2. Record the runtime artifact digest and parameter-set digest.
3. Record the external cryptographic review evidence digest.
4. Keep runtime binaries, parameter manifests, and review evidence immutable
   for audit.

## Key Epoch

1. Create a new key epoch before admitting participant ciphertexts.
2. Ensure the external runtime owns key generation and decryption keys.
3. Store only key-custody evidence digests in Kiva.
4. Rotate the key epoch when participants are replaced or runtime parameters
   change.

## Ciphertext Admission

1. Confirm the job uses `fhe-sum-runtime` and an n-of-n release policy.
2. Confirm external review is complete before production-facing claims.
3. Submit exactly one metadata record per active participant.
4. Reject missing custody evidence, parameter-set mismatches, duplicate
   participants, removed participants, aborted participants, digest mismatch,
   and replay.

## Evaluation

1. Evaluate only from an accepted ciphertext transcript.
2. Record the evaluated ciphertext digest and evaluation-proof digest.
3. Reject evaluation digest mismatch and replay.
4. Preserve runtime logs outside Kiva as external evidence.

## Release

1. Confirm the ciphertext transcript and evaluation are accepted.
2. Confirm the key epoch is active.
3. Require policy-authority approval.
4. Preserve governed release evidence through the generic confidential job
   lifecycle.
5. Keep raw plaintexts, decryption keys, and ciphertext payloads outside Kiva
   metadata exports.
6. Do not register FHE runtime outputs as PDS data products until a dedicated
   PDS source kind, HTTP contract, and PostgreSQL persistence path are
   implemented.

## Incident Handling

- Review pending: block release and public production FHE claims.
- Parameter mismatch: reject the transcript and open a runtime evidence review.
- Custody missing: suspend the participant submission until evidence is
  replaced.
- Replay: reject the candidate transcript or evaluation and preserve both
  record digests for audit.
- Runtime compromise: retire the key epoch, disable `fhe-sum-runtime`, and
  require new review evidence before re-enabling.
