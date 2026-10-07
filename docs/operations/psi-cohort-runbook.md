# PSI Cohort Operations Runbook

This runbook covers the bounded `kiva-psi-cohort-hash-v1` profile.

## Pre-Job Checks

- Select one namespace per deployment cohort.
- Confirm every participant uses the same canonicalization rule.
- Confirm raw identifiers will not be submitted to Kiva.
- Confirm every participant can produce one dataset for the same job id.
- Record whether the identifier namespace is high entropy enough for
  hash-token matching. If not, require a stronger external tokenization layer.

## Submission Rules

- Submit exactly one dataset per participant index.
- Submit only 32-byte identifier digests.
- Do not submit empty datasets.
- Do not submit duplicate digests inside one participant dataset.
- Treat a replayed job id as an incident or idempotency conflict.

## Release Rules

- Release only intersection digests and the intersection count.
- Do not publish participant input sets.
- Do not publish non-matching digests.
- Do not infer raw-record access from a PSI output.
- Preserve PSI result evidence with the release record used by the deployment.
  Do not register PSI outputs as PDS data products until a dedicated PDS source
  kind, HTTP contract, and PostgreSQL persistence path are implemented.

## Evidence

Archive these artifacts with the release evidence bundle:

- `spec/confidential/psi-cohort-job-contract.yaml`
- `spec/confidential/psi-cohort-assurance.yaml`
- `spec/confidential/psi-cohort-vectors.yaml`
- `scripts/ci/validate_psi_cohort_assurance.py` output
- `proofs/tamarin/kiva_psi_cohort_boundary.spthy`
- Kani report containing `psi_cohort_intersection_exposes_only_common_digests`

## Incident Handling

If raw identifiers are submitted to a Kiva PSI job boundary, reject the job and
rotate the job id. If a namespace or canonicalization rule was wrong, retire
the result metadata and rerun with a new job id and corrected job contract.
