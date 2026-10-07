# AWS Nitro TEE Operations Runbook

This runbook covers `kiva-tee-aws-nitro-review-gated-v1`.

## Pre-Deployment Checks

- Select AWS Nitro Enclaves as the deployment TEE family.
- Build and pin the verifier adapter artifact digest.
- Pin the AWS Nitro trust-anchor digest accepted by the verifier adapter.
- Record the verifier signing-key epoch and minimum accepted epoch.
- Record the external security-review evidence digest.
- Pin PCR0, PCR1, PCR2, PCR3, and PCR8 for the target image and parent role.
- Pin the image provenance digest and policy binding digest.
- Configure the nonce/challenge digest source.
- Configure the evidence validity window and minimum evidence sequence.
- Disable debug-mode secret provisioning and result release.

## Evidence Admission

- Accept only `verified-nitro-attestation-document-v1` normalized claims.
- Require the adapter to verify the raw Nitro attestation document before Kiva
  receives the claims.
- Reject evidence if certificate-chain validation failed.
- Reject evidence if COSE signature validation failed.
- Reject evidence if the trust anchor differs from policy.
- Reject evidence if any pinned PCR differs from policy.
- Reject evidence if debug mode is reported or any admitted PCR is all zero.
- Reject evidence if the nonce digest differs from policy.
- Reject evidence outside the validity window.
- Reject evidence below the configured sequence floor.
- Persist the accepted `(job id, sequence, document digest)` replay record
  before key release.

## Key Release

- Release keys only after `verify_tee_aws_nitro_attestation_evidence` succeeds.
- Require policy approval at key-release time.
- Require external security review completion.
- Require verifier key epoch at or above the configured minimum.
- Never release keys from debug-mode, stale, replayed, rollback, or mismatched
  evidence.

## Result Release

- Release results only after key provisioning has completed.
- Require the accepted attestation job id to match the release policy.
- Require policy approval at result-release time.
- Record PCRs, image digest, policy binding, nonce digest, verifier artifact,
  trust anchor, verifier key epoch, document digest, evidence sequence, and
  acceptance timestamp as audit metadata.

## Image Rollout

- Generate a new image provenance digest.
- Generate or record the new PCR policy.
- Update verifier adapter policy before accepting the new image.
- Increase or confirm the evidence sequence floor.
- Obtain policy-authority approval before enabling key release.
- Keep previous image metadata for audit and rollback analysis.

## Rollback

- Treat rollback as an incident workflow.
- Suspend new key release before changing PCR policy.
- Record the previous and target image digest.
- Update the retired sequence floor so old evidence remains rejected.
- Re-enable key release only after policy approval and verifier policy update.

## Verifier Key Rotation

- Publish the new verifier signing key through the deployment trust process.
- Increment the verifier key epoch.
- Update the minimum accepted verifier key epoch in Kiva policy.
- Reject evidence signed by adapter keys below the minimum epoch.
- Archive the rotation evidence with the release evidence bundle.

## Evidence

Archive these artifacts with the release evidence bundle:

- `spec/confidential/tee-nitro-attestation-assurance.yaml`
- `spec/confidential/tee-nitro-attestation-job-contract.yaml`
- `spec/confidential/tee-nitro-attestation-vectors.yaml`
- `scripts/ci/validate_tee_nitro_attestation_assurance.py` output
- `proofs/tamarin/kiva_tee_nitro_boundary.spthy`
- Kani report containing `tee_aws_nitro_attestation_gate_fails_closed`
- Kani report containing
  `tee_aws_nitro_key_and_result_release_gates_fail_closed`

## Incident Handling

If evidence fails certificate-chain, COSE signature, trust-anchor, PCR, nonce,
freshness, sequence, replay, or digest checks, reject it and do not release
keys. If debug mode is reported, suspend the job and treat the evidence as
ineligible for secret provisioning and result release. If verifier artifact or
verifier key epoch checks fail, rotate or roll back the verifier through the
deployment trust process before resubmitting evidence.
