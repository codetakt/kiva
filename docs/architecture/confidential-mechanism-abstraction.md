# Confidential Mechanism Abstraction

Kiva separates the [xAPI LRS surface](conformance-claim.md) from Kiva-specific
confidential-computation mechanisms. xAPI remains the learning
record interface. Confidential mechanisms are platform extensions used by PDS
and BI workflows.

## Implemented Mechanisms

`crates/kiva-confidential` exposes a validated mechanism abstraction:

- `AdditiveNOfN` for the existing additive secure aggregate transcript.
- `PrivateSetIntersection` for bounded hash-token cohort PSI.
- `PrivateSetIntersectionOprfDh` for the review-gated DH-OPRF PSI runtime
  boundary.
- `ShamirThreshold` for VSS-admitted threshold reconstruction over the same
  fixed finite field.
- `ThresholdFrostSigning` for the review-gated FROST threshold-signing runtime
  boundary.
- `TrustedExecutionEnvironment` for the portable deployment-bound attestation
  evidence gate.
- `TrustedExecutionEnvironmentAwsNitro` for the review-gated AWS Nitro
  attestation verifier boundary.
- `FheSumFeasibility` for the deterministic small-sum FHE feasibility profile.
- `FheSumRuntime` for the review-gated FHE sum runtime boundary.

The stable mechanism-oriented import surface is
`kiva_confidential::mechanism::*`. The current modules are:

- `mechanism::additive` for additive secure aggregation.
- `mechanism::differential_privacy` for privacy-budget accounting, composition,
  noise, and release gates.
- `mechanism::shamir` for hash-committed Shamir threshold reconstruction.
- `mechanism::threshold` for threshold-release approval and completion rules.
- `mechanism::workflow` for confidential mechanism plans, release policy,
  technique labels, and participant lifecycle rules.
- `mechanism::psi` for bounded cohort PSI and review-gated DH-OPRF PSI.
- `mechanism::tee` for portable TEE and AWS Nitro attestation gates.
- `mechanism::frost` for review-gated threshold FROST signing boundaries.
- `mechanism::fhe` for FHE feasibility and external-runtime FHE boundaries.
- `mechanism::job` for confidential job workflow and release-policy facts.
- `mechanism::provider` for secret/confidential-compute provider contracts.
- `mechanism::registry` for the stable descriptor table that binds mechanism
  kinds to technique families, required provider capabilities, all-participant
  policy expectations, and production review gates.
- `mechanism::core` for the remaining tightly coupled additive, Shamir,
  threshold-release, workflow, PSI, TEE, FROST, and FHE implementation bodies
  and their bundled tests. `mechanism::core::ConfidentialKernelBoundary` names
  the pure kernel boundary rows reviewed by
  `spec/verification/confidential-verified-kernels.yaml`; Kani harnesses live
  under `mechanism::verification`, and provider adapters remain outside the
  pure kernel namespace. The former `mechanism::integrated` compatibility
  namespace is removed and recorded only in the compatibility-retirement ledger.

`primitive.rs` owns the fixed-width finite-field and artifact wrappers used
across additive aggregation, Shamir, PSI, TEE, FROST, FHE, and provider
contracts. `mechanism::job`, `mechanism::differential_privacy`, and
`mechanism::provider` own their implementation bodies directly.
`kiva-confidential-core` owns the shared control/data-plane tags used by
provider runtime boundaries, keeping Kiva-owned evidence separate from external
data-plane execution claims.
`mechanism::registry` owns `CONFIDENTIAL_MECHANISM_REGISTRY` and
`descriptor_for_mechanism`, so technique, provider-capability, and production
review-gate metadata have one reviewable source. The crate root is
intentionally narrow: it exposes the `mechanism` and `primitive` modules, keeps
primitive wrapper re-exports for compatibility, and leaves only
`SecureAggregateError` as a root-level mechanism compatibility export.
Application and server code import mechanism types through
`kiva_confidential::mechanism::{...}`.
`mechanism::core` owns remaining coupled implementation bodies behind the stable
mechanism namespaces. Future implementation moves from core review units must
preserve the mechanism namespace and carry tests, vectors, and assurance
evidence with the moved code.

The abstraction binds a concrete mechanism to a `ReleasePolicy`. Mechanism
construction fails closed when the policy is structurally incompatible:

- all mechanisms require at least two participants through `ReleasePolicy`;
- thresholds cannot exceed participant count;
- `AdditiveNOfN` requires threshold equal to participant count;
- PSI, DH-OPRF PSI, portable TEE, Nitro TEE, FHE feasibility, and FHE runtime
  currently require n-of-n release policies;
- `ShamirThreshold` requires threshold at least two and participant indexes
  that fit in the fixed finite field. The HTTP reconstruction path additionally
  requires hash-committed share openings before interpolation.
- `ThresholdFrostSigning` uses the threshold-family release-policy boundary and
  remains review-gated.

PDS does not need to understand every mechanism directly. Mechanisms that
produce governed PDS-visible outputs do so through a normalized
`ReleaseArtifactRef`; the current artifact kinds are threshold release,
differential-privacy release, and Shamir threshold reconstruction. Other
mechanisms remain confidential job, assurance, or external-runtime boundaries
until a deployment adds a reviewed release artifact contract.

The server implementation reflects this boundary with a
`ReleaseArtifactAvailabilityPort` in `crates/kiva-server/src/app/pds/`.
PDS data-product registration resolves artifact availability through that
application port, while the confidential store remains responsible for the
mechanism-specific facts such as threshold release completion, differential
privacy release ownership, and Shamir reconstruction ownership.

## Non-Claims

The abstraction does not make Kiva a production MPC, malicious-secure PSI,
production FHE, production hardware-backed TEE platform, or production
threshold-cryptography system. It provides a strict dispatch boundary for the
current bounded mechanisms and keeps stronger production claims behind external
runtime bindings, review evidence, and deployment-specific acceptance.

## Evidence

- Rust root API compatibility layer: `crates/kiva-confidential/src/lib.rs`
- fixed-width primitive wrappers: `crates/kiva-confidential/src/primitive.rs`
- mechanism namespaces: `crates/kiva-confidential/src/mechanism/`
- additive facade: `crates/kiva-confidential/src/mechanism/additive.rs`
- Shamir facade: `crates/kiva-confidential/src/mechanism/shamir.rs`
- threshold-release facade:
  `crates/kiva-confidential/src/mechanism/threshold.rs`
- workflow facade: `crates/kiva-confidential/src/mechanism/workflow.rs`
- core mechanism owner:
  `crates/kiva-confidential/src/mechanism/core.rs`
- pure kernel boundary marker:
  `crates/kiva-confidential/src/mechanism/core/kernel.rs`
- confidential job implementation:
  `crates/kiva-confidential/src/mechanism/job.rs`
- differential privacy module owner:
  `crates/kiva-confidential/src/mechanism/differential_privacy.rs`
- differential privacy review units:
  `crates/kiva-confidential/src/mechanism/differential_privacy/`
- provider contract module owner:
  `crates/kiva-confidential/src/mechanism/provider.rs`
- provider binding implementation:
  `crates/kiva-confidential/src/mechanism/provider/binding.rs`
- provider admission implementation:
  `crates/kiva-confidential/src/mechanism/provider/admission.rs`
- provider contract object:
  `crates/kiva-confidential/src/mechanism/provider/contract.rs`
- provider binding digest:
  `crates/kiva-confidential/src/mechanism/provider/digest.rs`
- provider runtime boundary:
  `crates/kiva-confidential/src/mechanism/provider/runtime.rs`
- shared control/data-plane boundary tags:
  `crates/kiva-confidential-core/src/lib.rs`
- mechanism descriptor registry:
  `crates/kiva-confidential/src/mechanism/registry.rs`
- HTTP contract: `spec/openapi/kiva-confidential.yaml`
- mechanism maturity registry: `spec/confidential/mechanism-maturity.yaml`
- Kani harnesses:
  - `confidential_mechanism_plan_enforces_additive_and_shamir_threshold_rules`
- Regression runner: `scripts/ci/run_kani_verification.sh`
