# Threshold FROST Operations Runbook

This runbook covers `kiva-threshold-frost-ed25519-review-gated-v1`.

## Pre-Deployment Checks

- Select the external FROST runtime and suite `ed25519-sha512-frost`.
- Build and pin the runtime artifact digest.
- Record the external cryptographic review evidence digest.
- Define the participant roster and one-based participant indexes.
- Configure the threshold with a minimum value of two.
- Define share-custody evidence requirements for every participant.
- Define complaint resolution and equivocation reporting procedures.
- Define key-epoch rotation rules for participant replacement and proactive
  refresh.
- Disable production threshold-cryptography claims until external review is
  complete and bound to the runtime artifact.

## Key Ceremony Admission

- Accept only normalized metadata from the reviewed runtime.
- Require exactly one ceremony transcript per configured participant.
- Reject removed, inactive, out-of-range, or aborted participants.
- Reject the ceremony when share-custody evidence is missing.
- Reject unresolved complaints before writing a ceremony replay record.
- Reject detected equivocation before writing a ceremony replay record.
- Persist the accepted `(job id, key epoch, ceremony digest)` replay record.
- Do not persist private shares, raw DKG messages, or runtime secret material.

## Signing Admission

- Accept signing transcripts only for an accepted key ceremony.
- Require at least threshold unique signature-share participants.
- Reject duplicate or out-of-range signature-share participants.
- Derive and compare the signing transcript digest before accepting the
  signature metadata.
- Persist message digest, signature digest, signing transcript digest, and
  replay metadata.
- Do not persist raw signature shares or signing nonces.

## Release

- Release threshold-signing outputs only after a completed key ceremony.
- Require a computed signature transcript.
- Require external cryptographic review completion.
- Require an active key epoch.
- Require policy-authority approval.
- Require governed key-use metadata and audit references.

## Participant Replacement And Refresh

- Treat participant replacement as a new key-epoch event.
- Suspend release for the affected key epoch during replacement.
- Run the runtime's reviewed proactive refresh or DKG procedure.
- Record new public commitment and share-custody metadata for every active
  participant.
- Retire old key epochs only after replay records and audit metadata are
  preserved.

## Evidence

Archive these artifacts with the release evidence bundle:

- `spec/confidential/threshold-frost-assurance.yaml`
- `spec/confidential/threshold-frost-job-contract.yaml`
- `spec/confidential/threshold-frost-vectors.yaml`
- `scripts/ci/validate_threshold_frost_assurance.py` output
- `proofs/tamarin/kiva_threshold_frost_boundary.spthy`
- Kani report containing `threshold_frost_key_ceremony_gate_fails_closed`
- Kani report containing `threshold_frost_release_gate_fails_closed`

## Incident Handling

If a ceremony has unresolved complaints, equivocation evidence, missing custody
metadata, participant mismatch, or replay conflict, reject it and do not accept
signing transcripts for that key epoch. If a signing transcript has too few
shares, duplicate participants, digest mismatch, or replay conflict, reject it
and keep the prior accepted key ceremony available only for future valid
signing attempts. If external review evidence is withdrawn or superseded,
suspend release until a new reviewed runtime artifact is bound.
