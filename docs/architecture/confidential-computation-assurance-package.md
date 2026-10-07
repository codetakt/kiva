# Confidential Computation Assurance Package

## Scope

CC7 packages Kiva's selected confidential computation boundaries for external
engineering review. The package is defined in
`spec/confidential/confidential-computation-assurance-package.yaml`.

The package does not add a new cryptographic mechanism. It ties the implemented
and review-gated CC1 through CC16 confidential computation and provider-contract
boundaries to their contracts, vectors, formal-boundary models, CI gates,
runbooks, deployment templates, and public claim language.
CC8 adds the first practical deployment and workload selection profile:
`spec/confidential/practical-deployment-workload-selection.yaml`.
CC11 adds a review-gated DH-OPRF PSI upgrade profile; it is included as an
internal runtime-boundary and review-gate artifact, not as a production PSI
claim before external cryptographic review.
CC12 adds a review-gated AWS Nitro Enclaves verifier-adapter profile; it is
included as a vendor TEE attestation boundary, not as a production
hardware-backed confidential execution claim before external security review.
CC13 adds a review-gated FROST threshold-signing runtime boundary; it is
included as threshold-cryptography engineering evidence, not as a production
or malicious-robust threshold-cryptography claim before external cryptographic
review.
CC14 adds a review-gated FHE runtime integration boundary; it is included as
FHE runtime engineering evidence, not as a production FHE claim before
external cryptographic and performance review.
CC15 adds a repository-side external production assurance readiness gate for
the exact CC8 deployment profile, release-candidate evidence requirements,
operational controls, and public claim language. It is not evidence that
external review or certification has already been completed.
CC16 adds a secret/confidential-compute provider contract and fixed reference
primitive provider. It lets Kiva treat external KMS/HSM, TEE, FHE, PSI, and
threshold runtimes as metadata-bound providers while keeping the reference
provider non-production.

## Selected Mechanism Set

The review package includes these bounded mechanisms:

- structural n-of-n additive secure aggregation with signed share transport and
  replay persistence;
- bounded differential privacy release profile for persisted aggregate results;
- hash-committed Shamir VSS-admission reconstruction;
- bounded hash-token PSI cohort job contract;
- review-gated DH-OPRF PSI runtime boundary for production-upgrade review;
- portable deployment-bound TEE attestation evidence gate;
- review-gated AWS Nitro Enclaves verifier-adapter boundary;
- review-gated FROST threshold-signing runtime boundary;
- deterministic FHE small-sum feasibility boundary;
- review-gated FHE sum runtime integration boundary.

The FHE feasibility entry is included as feasibility evidence only. The portable
TEE entry is an evidence gate only. The Nitro entry is a review-gated
verifier-adapter boundary only. The FROST entry is a review-gated
external-runtime boundary only. The FHE runtime entry is a review-gated
external-runtime boundary only. The hash-token PSI entry is a bounded
job-contract boundary only.
None of these entries imply production MPC, malicious-secure PSI, production
OPRF PSI before external cryptographic review, production hardware-backed TEE
execution before external security review, production FHE, Kiva in-process raw
vendor quote parsing, malicious-dealer-robust VSS, or production
malicious-robust threshold cryptography.

## Evidence Model

The package validator requires:

- every selected mechanism to identify a milestone, profile, claim status,
  non-claims, and existing evidence paths;
- individual assurance profiles for DP, Shamir VSS, PSI, TEE, and FHE to remain
  published or review-gated and validator-backed;
- the CC11 production PSI upgrade profile to remain review-gated and
  validator-backed until external cryptographic review is complete;
- the CC12 AWS Nitro attestation profile to remain review-gated and
  validator-backed until external security review is complete;
- the CC13 FROST threshold-signing profile to remain review-gated and
  validator-backed until external cryptographic review is complete;
- the CC14 FHE runtime profile to remain review-gated and validator-backed
  until external cryptographic and performance review is complete;
- the CC15 external production assurance readiness profile to require clean
  release-candidate evidence, live PostgreSQL conformance evidence, formal
  verification regression evidence, PDS evidence, release evidence-set
  validation, operational controls, and explicit non-claim language;
- the CC16 provider contract to fix the reference primitive provider, validate
  provider binding vectors, require capability/mechanism alignment, and reject
  reference-provider production claims;
- OpenAPI contracts, PDS generated schema index, PDS examples, database schema,
  and Atlas migration sum to exist;
- CI workflows to reference the required Nix checks for Rust, PostgreSQL,
  OpenAPI, confidential assurance, PDS publication, F*, Kani, Tamarin, and
  release evidence;
- deployment templates to state enabled and disabled confidential mechanisms;
- the operator runbook to cover key lifecycle, participant lifecycle, incident
  handling, emergency access, audit export, and retirement;
- the compliance matrix to contain the CC7 through CC16 traceability
  requirements.

This gives reviewers one stable manifest to inspect while keeping mechanism
claims anchored in the narrower CC1 through CC15 artifacts and provider-contract
claims anchored in the CC16 artifacts.
The CC8 profile gives reviewers a concrete multi-agency workload target for
CC9 and CC10 without turning the package into a production security claim.

The assurance source model is intentionally one-way:

`assurance package -> selected mechanism profiles -> verified kernel ledger -> typed evidence registry -> source anchors`

`spec/architecture/design-backbone.yaml` owns this source model. The assurance
package stores only `assurance_source_model_ref` back to that backbone entry
and owns public claim language, mechanism selection, non-claims, and review
packaging. `spec/verification/confidential-verified-kernels.yaml` owns
verification-boundary grouping over implementation anchors, Rust tests, Kani,
Tamarin, and selected F* artifacts. `spec/confidential/assurance-evidence-registry.yaml`
owns typed source-level evidence entries. Validators should not make the package
itself a second typed-evidence registry.

## Claim Language

Kiva may claim that it has a CC7 confidential computation assurance package for
the selected bounded mechanism set. That is an engineering assurance and
review-readiness claim.

Kiva must not claim from CC7 alone:

- IEEE xAPI certification;
- legal privacy certification;
- full formal verification of the platform;
- production MPC;
- malicious-secure PSI;
- production OPRF PSI before external cryptographic review;
- production hardware-backed TEE execution before external security review;
- production threshold cryptography before external cryptographic review;
- malicious-robust threshold cryptography before external cryptographic review;
- production FHE;
- production FHE runtime integration before external cryptographic review;
- completed external production security review before review outcomes are
  bound to the release candidate;
- reference primitive provider evidence as production cryptographic security;
- Kiva in-process raw vendor TEE quote parsing;
- malicious-dealer-robust VSS or DKG.

## Review Procedure

Before external review, run the CI gates named in
`docs/operations/ci-gates.md`. The review input is the package manifest, the
JSON report generated by
`scripts/ci/validate_confidential_computation_assurance_package.py`, the
source-level assurance evidence registry in
`spec/confidential/assurance-evidence-registry.yaml`, and the CI evidence
artifacts for conformance, formal verification, PDS publication, and
confidential-computation assurance. The registry is typed: each registered Rust
symbol, Rust test, Kani harness, and Tamarin lemma names the source file that
the validator must resolve before the package is considered internally
consistent.
