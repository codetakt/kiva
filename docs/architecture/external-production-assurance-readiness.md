# External Production Assurance Readiness

## Scope

CC15 makes Kiva's selected confidential computation boundary review-ready for
an exact deployment profile. It does not create a production security claim by
itself. The machine-readable profile is
`spec/confidential/external-production-assurance-readiness.yaml`.

The profile binds the CC8 multi-agency aggregate deployment selection to the
CC7 assurance package, the CC9 job lifecycle, the CC10 participant and key
lifecycle, and the review-gated CC11 through CC14 mechanism-promotion
boundaries.

## Release-Candidate Boundary

The first release-candidate scope is
`kiva-cc8-multi-agency-aggregate-v1` using the
`multi-agency-research-aggregate-v1` deployment template.

The enabled mechanisms for the first practical workload are:

- structural additive secure aggregation;
- differential privacy release;
- Shamir VSS-admission reconstruction.

The bounded but not production-promoted mechanisms are hash-token PSI and the
portable TEE evidence gate. DH-OPRF PSI, AWS Nitro TEE attestation, FROST
threshold signing, and FHE runtime integration remain review-gated and disabled
for the first practical workload unless a deployment binds external review
evidence to the runtime artifact.

## Evidence Gate

The CC15 gate requires the release candidate to collect and validate:

- clean-git evidence for the release candidate;
- live PostgreSQL xAPI conformance and PostgreSQL execution evidence;
- F*, Kani, and Tamarin formal-boundary evidence with Kani regression scope
  for release claims;
- PDS publication evidence;
- release evidence-set validation across conformance and formal bundles;
- database migration state, OpenAPI contracts, toolchain versions, checksums,
  and review outcomes.

The in-repository fixture check tests whether the release evidence contract can
reject incomplete evidence. A real production claim still requires externally
generated evidence from a clean release-candidate run and external review
outcomes.

## Claim Language

Kiva may claim that CC15 defines a release-candidate assurance gate for the
selected deployment profile and mechanism set.

Kiva must not claim from CC15 alone:

- completed external production security review;
- legal or regulatory privacy certification;
- IEEE xAPI certification;
- full formal verification;
- production MPC;
- malicious-secure PSI;
- production FHE;
- production hardware-backed TEE confidential execution;
- production threshold cryptography.

## Review Inputs

External reviewers should receive the assurance package manifest, this CC15
profile, the operational runbook, OpenAPI contracts, migration state, Rust
source, formal-boundary artifacts, deployment templates, the CC16 provider
contract, and release evidence bundles. Review outcomes must be bound to the
exact deployment profile, selected mechanism set, provider contract, runtime
artifacts, verifier artifacts, and release candidate commit before Kiva
changes public claim language.
