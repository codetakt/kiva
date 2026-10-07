# Confidential Computation Milestones

This document is the human-readable guide for Kiva's confidential-computation
milestone boundaries. The machine-readable source of truth is
`spec/confidential/computation-milestones.yaml`; mechanism-level maturity is
tracked in `spec/confidential/mechanism-maturity.yaml`.

## Claim Boundary

Kiva supports a bounded confidential analytics substrate: additive secure
aggregation, differential privacy release metadata, Shamir VSS-admission
reconstruction, bounded PSI, portable and Nitro TEE admission gates, FROST and
FHE runtime boundaries, confidential job lifecycle, participant/key lifecycle,
and provider-contract metadata.

These are engineering assurance boundaries. Kiva does not claim production MPC,
production FHE, hardware-backed TEE certification, malicious-secure PSI,
malicious-participant robustness, distributed key generation, or external
certification without the required external review and deployment evidence.

## Source of Truth

- Milestone ledger: `spec/confidential/computation-milestones.yaml`
- Mechanism maturity registry: `spec/confidential/mechanism-maturity.yaml`
- Confidential OpenAPI contract: `spec/openapi/kiva-confidential.yaml`
- Assurance package: `spec/confidential/confidential-computation-assurance-package.yaml`
- Evidence registry: `spec/confidential/assurance-evidence-registry.yaml`
- Architecture drift guard: `spec/architecture/boundaries.yaml`

## Boundary Catalog

| Milestone | Boundary |
| --- | --- |
| CC0 | Confidential-computation claim boundary. |
| CC1 | Structural n-of-n additive secure aggregation. |
| CC2 | Differential privacy release-profile constraints. |
| CC3 | Hash-committed Shamir VSS-admission reconstruction. |
| CC4 | Bounded hash-token PSI cohort matching. |
| CC5 | Portable TEE attestation evidence gate. |
| CC6 | Deterministic small-sum FHE feasibility profile. |
| CC7 | Selected mechanism assurance package. |
| CC8 | Practical multi-agency workload selection. |
| CC9 | Durable confidential job lifecycle. |
| CC10 | Metadata-only participant and key lifecycle. |
| CC11 | Review-gated DH-OPRF PSI runtime boundary. |
| CC12 | Review-gated AWS Nitro verifier boundary. |
| CC13 | Review-gated FROST threshold-signing runtime boundary. |
| CC14 | Review-gated FHE runtime boundary. |
| CC15 | Repository-side external production assurance readiness. |
| CC16 | Metadata-only provider contract and reference primitive provider. |

## Maintenance Rule

Update `spec/confidential/computation-milestones.yaml` first when a
confidential-computation milestone changes. Keep this document short: it should
explain the claim boundary and point reviewers to the ledger, contracts,
validators, proof artifacts, and evidence generators.
