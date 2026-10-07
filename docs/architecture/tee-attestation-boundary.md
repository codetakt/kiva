# TEE Attestation Boundary

Kiva CC5 implements `kiva-tee-portable-attestation-v1`: a portable,
deployment-bound TEE attestation evidence gate.

This is not a hardware remote-attestation verifier. Kiva does not parse or
verify AWS Nitro, Intel SGX, AMD SEV-SNP, or other vendor quote formats in the
current implementation. Instead, Kiva validates a structured evidence envelope
that a deployment-specific verifier has produced or admitted.

## Mechanism

- family: `deployment-bound-portable`;
- format: `portable-evidence-v1`;
- digest domain: `kiva-confidential:tee-attestation-evidence:v1`;
- digest: SHA-256 over the profile, family, format, job id, measurement, image
  digest, policy binding, validity window, and monotonic evidence sequence;
- release policy: n-of-n for the current multi-party confidential-computation
  boundary;
- secret provisioning: allowed only after accepted attestation and policy
  approval;
- result release: allowed only after accepted attestation, policy approval, and
  matching attested job identity.

The job contract is `spec/confidential/tee-attestation-job-contract.yaml`.
Deterministic vectors are in `spec/confidential/tee-attestation-vectors.yaml`.

## Admission Rules

The Rust core rejects:

- missing policy approval;
- unsupported family or format;
- job identifier mismatch;
- measurement mismatch;
- image provenance digest mismatch;
- policy binding mismatch;
- not-yet-valid evidence;
- expired evidence;
- evidence sequence below the configured minimum;
- canonical evidence digest mismatch;
- replayed evidence by job/sequence or digest.

Accepted attestation returns metadata only: job id, evidence sequence,
evidence digest, measurement, image digest, policy binding, and acceptance
timestamp. It does not release secrets or computation results by itself.

## Deployment Assumptions

The portable evidence gate assumes the deployment has a separate verifier or
control plane that maps vendor-specific evidence into Kiva's portable
measurement, image digest, and policy binding fields. That external verifier
must define:

- which TEE family or cloud service is trusted;
- how vendor quotes are verified;
- how measurements are built and pinned;
- how image provenance is produced;
- how rollback state is stored durably;
- how clock freshness is anchored;
- which operators can approve policies and rotate measurements.

Kiva records and enforces the resulting boundary facts. It does not prove the
external verifier, cloud service, hypervisor, enclave runtime, or hardware.

## Assurance

Current assurance includes:

- Rust tests for valid evidence, binding mismatches, policy denial, freshness,
  rollback, digest mismatch, replay, secret provisioning, and release gates;
- deterministic vectors validated by
  `scripts/ci/validate_tee_attestation_assurance.py`;
- Kani harnesses for the pure attestation gate and secret/result release gates;
- Tamarin model `proofs/tamarin/kiva_tee_attestation_boundary.spthy`.

The Tamarin model specifies that accepted attestation traces to a configured and approved
policy, that secret provisioning and result release require accepted
attestation, and that stale, replayed, or rollback evidence rejection does not
become accepted or released evidence.

## Non-Claims

CC5 does not claim:

- Nitro, SGX, SEV-SNP, or vendor quote verification;
- hardware side-channel resistance;
- cloud-operator isolation;
- enclave runtime correctness;
- confidentiality of code or data inside a TEE;
- malicious participant robustness;
- production MPC or FHE security.

Those claims require a concrete hardware/cloud profile, quote verifier,
operational key ceremony, side-channel review, and external security review.
