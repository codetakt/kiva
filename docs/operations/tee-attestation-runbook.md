# TEE Attestation Operations Runbook

This runbook covers the portable `kiva-tee-portable-attestation-v1` evidence
gate.

## Pre-Deployment Checks

- Select the external TEE verifier or deployment control plane.
- Record the supported hardware or cloud TEE family outside Kiva.
- Pin the expected measurement.
- Pin the expected image provenance digest.
- Pin the expected policy binding.
- Configure a monotonic minimum evidence sequence.
- Configure the freshness window and clock source.
- Obtain explicit policy-authority approval before enabling secret
  provisioning.

## Evidence Admission

- Submit only `portable-evidence-v1` envelopes.
- Bind every envelope to one job id.
- Reject evidence outside the validity window.
- Reject evidence below the minimum sequence.
- Reject reused job/sequence pairs and reused evidence digests.
- Persist accepted evidence metadata before provisioning secrets or releasing
  results.

## Secret Provisioning

- Provision secrets only after `verify_tee_attestation_evidence` succeeds.
- Require policy approval at provisioning time.
- Do not provision secrets from stale, replayed, rollback, or mismatched
  evidence.
- Treat a failed evidence digest check as an integrity incident.

## Result Release

- Release results only for the attested job identity.
- Require accepted attestation metadata.
- Require policy approval at release time.
- Record measurement, image digest, policy binding, evidence digest, evidence
  sequence, and acceptance timestamp in audit metadata.

## Evidence

Archive these artifacts with the release evidence bundle:

- `spec/confidential/tee-attestation-job-contract.yaml`
- `spec/confidential/tee-attestation-assurance.yaml`
- `spec/confidential/tee-attestation-vectors.yaml`
- `scripts/ci/validate_tee_attestation_assurance.py` output
- `proofs/tamarin/kiva_tee_attestation_boundary.spthy`
- Kani report containing
  `tee_attestation_gate_fails_closed_for_policy_freshness_and_replay`
- Kani report containing `tee_secret_and_result_release_gates_fail_closed`

## Incident Handling

If evidence is expired, not yet valid, replayed, below the minimum sequence, or
bound to the wrong job, reject it and do not provision secrets. If a
measurement, image digest, or policy binding mismatch occurs, suspend the job
until the deployment owner confirms whether this is a planned rollout,
misconfiguration, or compromise. Use a new job id after changing any pinned
measurement, image, or policy binding.
