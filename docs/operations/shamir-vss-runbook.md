# Shamir VSS Operations Runbook

## Scope

This runbook covers Kiva's `kiva-shamir-hash-vss-v1` reconstruction profile.
It is an operational boundary for hash-committed Shamir share openings. It does
not operate a distributed key-generation ceremony, proactive share refresh,
threshold signing, or threshold decryption protocol.

## Participant Onboarding

1. Assign each participant one stable one-based participant index inside the
   configured participant count.
2. Record the transcript identifier, participant count, threshold, and
   verification profile before shares are distributed.
3. Publish one commitment for each participant share opening using the
   `kiva-confidential:shamir-share-opening:v1` domain.
4. Deliver share values and opening nonces over an authenticated channel that is
   separate from public metadata publication.
5. Confirm that every participant can locally recompute its commitment before
   the reconstruction window opens.

## Reconstruction

1. Accept only openings that carry the configured transcript identifier,
   participant index, field value, commitment, and opening nonce.
2. Verify each opening before it is admitted to the reconstruction set.
3. Reject duplicate participant indexes and openings whose commitment does not
   match.
4. Reconstruct only after at least `threshold` unique openings have verified.
5. Persist only reconstruction metadata: profile, participant count, threshold,
   share count, verified share count, complaint count, and timestamps.

## Complaints

1. Sustain a complaint when the opened share does not match the published
   commitment.
2. Reject a complaint when the opening verifies against the published
   commitment.
3. Exclude sustained complaint openings from reconstruction.
4. Escalate repeated sustained complaints to participant rotation before
   starting a new transcript.

## Rotation And Recovery

1. Rotate a participant by assigning a new transcript and redistributing shares;
   do not reuse commitments from an old transcript.
2. If fewer than `threshold` verified openings remain, close the reconstruction
   attempt as failed and create a new transcript after policy approval.
3. For lost opening nonces, treat the participant as unavailable for that
   transcript. Do not bypass commitment verification.
4. For suspected secret compromise, retire the transcript and generate a fresh
   Shamir sharing outside Kiva's metadata endpoint.

## Evidence

The release evidence bundle should include the OpenAPI contract, Shamir VSS
profile, deterministic vectors, Rust/Kani/Tamarin/F* reports, and this runbook.
