# Secret and Confidential-Compute Provider Integration

## Scope

Kiva now treats secret custody, encryption providers, and confidential-compute
runtimes as explicit building components. The contract separates
`SecretCustodyProvider` from `ComputeMechanismProvider` so custody-only
bindings do not appear as confidential computation mechanisms. The contract is
defined in
`spec/confidential/secret-compute-provider-contract.yaml`.

This does not move production cryptographic primitives into Kiva by default.
Kiva owns the control plane: provider identity, artifact binding, parameter-set
binding, review-evidence binding, key epoch and custody metadata, replay
prevention, release policy, PDS data-product governance, and audit export.
Specialized providers own the data plane: KMS/HSM secret custody, TEE quote
verification, FHE encryption/evaluation/decryption, malicious-secure PSI, or
threshold-signing arithmetic.

The Rust boundary mirrors this split with
`kiva_confidential_core::ControlPlaneEvidence<T>` and
`kiva_confidential_core::ExternalDataPlaneBoundary<T>`.
`SecretComputeProviderKind::is_secret_custody_provider()` and
`SecretComputeProviderKind::is_compute_mechanism_provider()` expose the same
boundary split to callers. The shared manifest enum is further narrowed by
`ReferenceProviderKind`, `SecretCustodyProviderKind`, and
`ComputeMechanismProviderKind`; production runtime helpers accept the compute
kind, not the shared enum, so reference fixtures and custody-only providers
cannot enter that path by construction. All provider-kind enums implement
`SecretComputeProviderKindContract`, which fixes the shared label and
manifest-kind projection used by contract validation and digest-bearing
metadata.
`SecretComputeProviderRuntimeBoundary` stores admission and runtime-claim facts
as Kiva control-plane evidence, while the coarse runtime class is tagged as an
external data-plane boundary.
The provider-kind registry is generated from the provider contract by
`scripts/ci/generate_secret_compute_provider_kind_registry.py` into both
`spec/confidential/secret-compute-provider-kind-registry.yaml` and
`crates/kiva-secret-provider-contract/src/kind_registry.rs`. The
`kiva-secret-provider-contract` crate owns provider vocabulary and the generated
registry; `kiva-confidential::mechanism::provider` re-exports those items only
as an admission/runtime compatibility surface. Rust callers read the generated
registry instead of maintaining a parallel provider-kind table.

## Reference Primitive Provider

The fixed reference provider is `kiva-reference-primitives-v1`. It is used for
deterministic fixtures, local tests, formal-boundary models, and examples.
It includes Kiva's implemented field arithmetic, Shamir reconstruction,
hash-committed VSS admission, deterministic PSI/FHE fixtures, normalized TEE
fixture metadata, and threshold-signing metadata fixtures.

The reference provider is deliberately not production cryptography. It cannot
support a production security claim, cannot act as a production secret-custody
provider, and must not be used as evidence of malicious-secure MPC, production
FHE, production TEE isolation, or production threshold cryptography.

## External Provider Contract

Production-oriented providers must bind at least:

- provider identity digest;
- provider artifact digest;
- capability;
- contract profile;
- parameter-set digest when the runtime has cryptographic parameters;
- review-evidence digest before production claims;
- external review completion status before production claims.

The current Kiva repository defines and validates those metadata fields at the
contract and Rust boundary. New Rust call sites construct bindings through
`SecretComputeProviderEvidence`, `SecretComputeProviderReviewEvidence`, and
`SecretComputeProviderUsageClaim`, so review state and production use are
typed facts before they become persisted binding metadata. Runtime provider
kinds with cryptographic parameter sets, such as FHE, PSI, and threshold
cryptography, are rejected when the binding omits `parameter_set_digest`.
Provider-kind labels, Rust marker families, mechanism-catalog participation, and
production-claim eligibility are projected into the generated provider-kind
registry, so additions to the contract must update the generated review ledger
and Rust registry together.

Kiva also exposes a metadata-only PDS provider-binding registry at
`/pds/provider-bindings`. The registry accepts generated OpenAPI DTOs,
normalizes them into `kiva-pds` command inputs, admits them through the
confidential provider contract, and persists only admitted control-plane
metadata in memory or PostgreSQL. The table
`kiva.pds_provider_bindings` stores provider class, mechanism, digest,
evidence-gate, release-gate, runtime-class, and production-readiness metadata.
It is not a secret-custody store or cryptographic data plane.

Kiva should not store raw provider secrets, FHE ciphertext payloads, TEE secret
material, OPRF keys, threshold private shares, signatures, signed envelopes,
provider payloads, or decryption keys in ordinary PDS evidence.

## Capability Mapping

Provider capabilities are intentionally narrow:

- `secret-custody` for KMS/HSM/envelope-encryption custody;
- `tee-attestation` for TEE attestation and attested execution evidence;
- `fhe-evaluation` for FHE runtime evidence;
- `psi-evaluation` for PSI runtime evidence;
- `threshold-signing` for threshold key ceremony or signing evidence;
- `reference-fixtures` for Kiva's deterministic reference provider.

Each confidential mechanism must use a matching capability. For example, an
FHE runtime job cannot be admitted against a secret-custody-only provider, and
a reference provider cannot be used for a production FHE claim.
The mechanism-to-capability table is derived from
`spec/confidential/mechanism-catalog.source.yaml` and only covers
`ComputeMechanismProvider` kinds plus the deterministic reference provider.
`SecretCustodyProvider` is excluded from that compute mechanism catalog. The
provider contract owns provider kinds, binding evidence, runtime/release gates,
and digest vectors instead of maintaining an independent capability map.

## Provider Algebra

The provider contract fixes the sealed admission algebra as
`ProviderCapability -> EvidenceGate -> RuntimeBoundary -> ReleaseGate`.
Reference primitive providers and external providers pass through the same
capability, admission, evidence, runtime-boundary, runtime-binding, and release
gate sequence before release gates partition into fixture-only,
non-production, or production. This is the control-plane composition Kiva
validates before any PDS registry metadata or confidential job can claim
provider support.

The algebra is intentionally narrow:

- capability matching happens first and is mechanism-specific;
- evidence gates distinguish reference fixtures, external non-production, and
  externally reviewed providers;
- runtime boundaries record whether Kiva is looking at a fixture,
  non-production external runtime, or reviewed production runtime;
- release gates derive from those admitted facts and do not assert external
  cryptographic security by themselves.
- reference providers terminate at fixture-only release, while external
  providers can reach production only after reviewed evidence and runtime
  boundary admission.

The Rust API exposes this path as `SecretComputeProviderContractPipeline`.
It is constructed from `ConfidentialMechanismDescriptor`, derives the
`SecretComputeProviderContract`, admits a provider binding, and returns both
the normalized `SecretComputeProviderAdmission` and the
`SecretComputeProviderRuntimeBoundary`. This keeps provider adapters from
building a parallel mapping from mechanism labels to capabilities, review
requirements, or runtime classes.

## Admission Evidence Model

`crates/kiva-confidential::mechanism::provider` exposes
`SecretComputeProviderContract::for_mechanism(...)` as the public admission
object for one confidential mechanism. The contract normalizes a validated
binding into `SecretComputeProviderAdmission`. That value records:

- the selected confidential mechanism;
- the mechanism's required non-reference capability;
- the canonical provider-binding digest;
- the evidence gate: `reference-fixture`, `external-non-production`, or
  `external-reviewed`;
- the release gate: `fixture-only`, `non-production`, or `production`.

This gives Kiva one internal shape for the path
`capability -> admission gate -> evidence -> runtime boundary -> runtime binding -> release`.
Reference primitives can satisfy mechanism tests and fixtures while remaining
`fixture-only`. External providers can reach `production` only when the
binding requests a production claim and supplies completed review evidence.
`SecretComputeProviderRuntimeBoundary` records whether the admitted provider
boundary is fixture-only, external non-production, or external reviewed
production before a mechanism-specific runtime binding is accepted.
`SecretComputeProviderRuntimeClass` is the coarse deployment class exposed to
evidence tooling: `reference`, `external-non-production`, or
`external-reviewed-production`. The class is an external data-plane boundary
tag, not a claim that Kiva executes the provider runtime. New call sites can
construct the two primary paths directly through
`admit_reference_primitive_runtime_boundary(...)` for experiments and
`admit_reviewed_production_runtime_boundary(...)` for reviewed external
compute providers, while both paths still pass through the same provider
contract admission checks.
`ConfidentialMechanismDescriptor` is the public mechanism algebra for provider
capability and review-gate metadata. `SecretComputeProviderCapabilityProfile`
and `SecretComputeProviderContractProfile` are implemented for descriptors,
mechanism kinds, concrete provider contracts, and
`SecretComputeProviderContractPipeline`; mechanism kinds delegate through
descriptors so future provider adapters do not invent a parallel mapping from
mechanism labels to capability, review gates, runtime classes, or release
claims.

## PDS Registry Boundary

The PDS registry is a control-plane boundary:

- route ownership stays under pure `PdsHttpContext`;
- write access uses the `pds:write` OAuth/OIDC scope and read access uses
  `pds:read`;
- request parsing enters through the OpenAPI contract crate before conversion
  into internal PDS command inputs;
- provider metadata is admitted by
  `admit_secret_compute_provider_runtime_boundary(...)` before persistence;
- memory and PostgreSQL stores retain only admitted metadata and computed
  binding evidence.

The registry deliberately does not execute TEE, FHE, PSI, threshold, or
secret-custody runtimes. Runtime execution, key custody, ciphertext handling,
remote attestation, malicious-secure protocol security, and external provider
review correctness remain provider responsibilities and external review
subjects.

## Claim Boundary

Kiva may claim that it has a provider integration contract and a fixed
reference primitive provider for tests and verification. Kiva may also claim
that admitted provider-binding metadata can be managed through a PDS API and
persisted in memory or PostgreSQL as control-plane evidence.

Kiva must not claim from this contract alone:

- production cryptographic security of any external provider;
- production FHE;
- malicious-secure PSI;
- hardware-backed TEE security;
- production threshold cryptography;
- legal privacy certification;
- that Kiva retains or controls raw secret material by default.
