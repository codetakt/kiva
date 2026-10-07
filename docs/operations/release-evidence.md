# Release Evidence

Kiva uses release evidence bundles to keep xAPI conformance, formal
verification, PDS publication, and confidential-computation assurance claims
traceable to a concrete git revision. These bundles are internal engineering
evidence. They are not IEEE certification, legal certification, production
MPC/FHE/TEE certification, or external security-review outcomes.

## xAPI Conformance Evidence

Generate the local evidence bundle with:

```sh
scripts/ci/generate_conformance_evidence.sh --require-clean-git
```

The full bundle includes validate-only reports, memory and PostgreSQL HTTP
conformance runs, the PostgreSQL execution verification report, a
schema-validated manifest, and checksum validation.

Generate a static, no-server conformance evidence bundle from Nix with:

```sh
nix build .#conformance-evidence-static
```

The static bundle uses the same manifest and checksum validation, but skips
live HTTP and PostgreSQL execution evidence.

## Formal Verification Evidence

Generate the formal verification evidence bundle with:

```sh
nix develop .#verification -c \
  scripts/ci/generate_formal_verification_evidence.sh --require-clean-git
```

The formal bundle includes Kani, Tamarin, and F* boundary reports plus a
schema-validated manifest. It is internal assurance evidence, not a
certification claim.

## PDS Publication Evidence

Generate the PDS publication evidence bundle with:

```sh
scripts/ci/generate_pds_evidence.sh --require-clean-git
```

The PDS bundle validates the Kiva PDS extension contract, examples, generated
schemas, PDS policy tests, PDS server tests, optional PostgreSQL/Kani/Tamarin
evidence, a schema-validated manifest, and checksums. It supports publication
readiness for the current PDS extension boundary; it is not an IEEE xAPI
certification claim and does not expand the IEEE/ISO/IEC 39274-1-1:2025 LRS
conformance surface.

Generate a static PDS evidence bundle from Nix with:

```sh
nix build .#pds-evidence-static
```

## Release Evidence Set

Validate both release-candidate bundles together with:

```sh
scripts/ci/validate_release_evidence_set.py \
  --conformance target/conformance-evidence/latest \
  --formal target/formal-verification-evidence/latest
```

The release evidence-set validator requires both bundles to describe the same
clean git revision, full live xAPI conformance evidence, PostgreSQL execution
verification, and Kani regression evidence.

## Confidential Computation Assurance

Validate the confidential-computation assurance package with:

```sh
scripts/ci/validate_confidential_computation_assurance_package.py
```

The CC7 package and CC15 readiness gate tie the selected additive aggregation,
differential privacy, Shamir VSS admission, PSI cohort, portable TEE evidence,
review-gated DH-OPRF PSI, review-gated Nitro TEE, review-gated FROST, FHE
feasibility, and review-gated FHE runtime boundaries to their contracts,
vectors, proof artifacts, runbooks, deployment templates, release-candidate
evidence requirements, and non-claim language. This is engineering assurance
for external review, not legal certification or a production MPC/FHE/TEE
security claim.

Kiva also defines a secret/confidential-compute provider contract with a fixed
reference primitive provider, `kiva-reference-primitives-v1`, for deterministic
tests and verification. Admitted provider-binding metadata can be registered in
the PDS provider-binding registry, but the registry is not a secret-custody
service or cryptographic runtime. The reference provider is not production
cryptographic security evidence; production use requires an external provider
binding and review evidence.
