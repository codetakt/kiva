# Assurance and Confidential Computation Strategy

## Scope

This document describes the architecture and verification strategy for Kiva's
confidential-computation boundaries. Execution results and review decisions are
kept locally under `target/`.

## Summary

Kiva should use the pinned **IEEE Std 9274.1.1-2023** as its current xAPI
requirements baseline and build privacy, Personal Data Store, BI, and confidential
computation capabilities as platform layers around the xAPI interface.

The implementation direction is:

- Use **Rust** for the LRS, API server, storage services, policy engine, job orchestration, and BI-facing services.
- Use a **Project Everest-family stack** selectively for high-assurance cryptographic and parsing boundaries, especially HACL*, EverCrypt, F*, Low*, KaRaMeL, and EverParse where they provide concrete value.
- Use **Tamarin** for protocol-level models of authorization, delegation, consent, multi-party computation workflows, result release, and public-sector separation-of-authority properties.
- Use **Nix flakes** to pin toolchains, verification tools, generated artifacts, and repeatable CI checks.

Project Everest-style components are useful for high-assurance cryptographic implementation boundaries. They should not be treated as a complete solution for secure multi-party computation, fully homomorphic encryption, private set intersection, analytics semantics, or institutional governance.

## xAPI Boundary

Kiva's xAPI surface provides the LRS interface for learning record producers
and consumers. Its requirements are traced to the pinned IEEE 2023 baseline;
that traceability alone does not establish implementation conformance.
Equivalence with the 2025 edition has not been established. See the
[xAPI claim boundary](conformance-claim.md).

The implemented xAPI scope includes the standard LRS resource families:

- `statements`
- `agents/profile`
- `activities`
- `activities/profile`
- `activities/state`
- `about`

The LRS layer should focus on xAPI structural conformance:

- Accept, validate, store, retrieve, and page Statements according to the standard.
- Preserve Statement immutability and idempotency requirements.
- Support document resources with ETag and conditional request behavior.
- Implement attachment handling and multipart behavior as required by the standard.
- Treat Statement validation as structural and syntactic at the LRS boundary.

Semantic validation of verbs, activity types, institutional policy, learner consent, and analytical eligibility should live in explicit higher-level layers. Those layers may use xAPI Profiles and governance policy, but they should not be confused with the core LRS conformance boundary.

Local reference repositories:

- `/home/claude/Codex/opensource.ieee.org/xapi/xapi-base-standard-documentation`
- `/home/claude/Codex/opensource.ieee.org/xapi/xapi-base-standard-examples`
- `/home/claude/Codex/opensource.ieee.org/xapi/xapi-profiles`

## Platform Layers

Kiva is designed as a layered platform:

1. **xAPI LRS layer**
   - Provides the standards-based REST/JSON interface.
   - Owns xAPI structural conformance, Statement persistence, document resources, pagination, and attachment handling.

2. **Identity and authorization layer**
   - Uses OAuth 2.0 and OpenID Connect as the basis for authentication, delegated authorization, client identity, user identity, and token-based access.
   - Models scopes and permissions around data subjects, institutions, applications, analysts, auditors, and public-interest authorities.

3. **Personal Data Store layer**
   - Manages learner-related records with provenance, policy, consent state, revocation, and subject-oriented access boundaries.
   - Separates raw learning records from approved analytical views and derived data products.

4. **Governance and audit layer**
   - Records policy decisions, consent transitions, access requests, computation jobs, result releases, and administrative actions.
   - Supports public-sector deployments where no single operator should have unchecked authority over identity, policy, raw data, computation, and result disclosure.

5. **BI and analytics layer**
   - Provides dashboards, reports, exports, and learning analytics over approved views.
   - Treats analytical eligibility as a policy decision, not as a direct side effect of raw LRS access.

6. **Confidential computation layer**
   - Supports multi-party analytics, privacy-preserving joins, aggregate statistics, result release controls, and auditable computation workflows.
   - May use secure multi-party computation, private set intersection, threshold cryptography, differential privacy, trusted execution environments, or fully homomorphic encryption depending on the use case.

## Separation of Authority

Public-sector and school deployments should avoid designs where one party controls every sensitive function. Kiva should model separate roles even if a small deployment maps several roles to the same organization.

Important roles include:

- **Data subject:** the learner or person associated with the records.
- **Data custodian:** the school, institution, or service operating storage.
- **Identity authority:** the system responsible for user and client identity.
- **Policy authority:** the body defining access, consent, retention, and public-interest rules.
- **Computation coordinator:** the component that schedules approved multi-party jobs.
- **Computation participant:** a party contributing encrypted, secret-shared, or policy-filtered input.
- **Analyst:** a user or system requesting approved insight.
- **Auditor:** a party that inspects policy compliance and evidence without needing raw personal records.

The architecture should make these separations explicit in policy, protocol models, audit records, and deployment documentation.

## Assurance Stack Decision

### Rust

Rust is the preferred implementation language for Kiva's core services because it offers memory safety, strong typing, good async/server ecosystem support, high-quality cryptography bindings, and a practical path to property testing and bounded model checking.

Rust should be used for:

- HTTP services and xAPI endpoints.
- Storage adapters and migrations.
- Policy decision points and policy enforcement points.
- OAuth/OIDC integration.
- Audit and event pipelines.
- Analytics orchestration.
- FFI boundaries to verified C or generated code where needed.

### Project Everest-Family Components

Project Everest-family components should be used selectively, not universally.

Good candidates:

- HACL*/EverCrypt-backed cryptographic primitives.
- F*/Low* specifications for narrow, security-critical functions.
- EverParse for strict parsers or binary/canonical encodings where parsing bugs would be security-relevant.
- KaRaMeL extraction for small verified cores that justify the engineering cost.

Poor candidates:

- Broad business logic that changes often.
- Full BI query engines.
- General application CRUD.
- Exploratory analytics code.
- Entire MPC/FHE stacks unless a concrete library and proof boundary are selected.

The useful claim is not "Kiva is verified because it uses Project Everest." The useful claim is "specific Kiva components have explicit formal boundaries, verified artifacts, and documented assumptions."

### Tamarin

Tamarin should be part of the Kiva assurance strategy.

Tamarin is appropriate for symbolic protocol analysis, including:

- OAuth/OIDC authorization and token flows as used by Kiva.
- Consent delegation and revocation workflows.
- Multi-party computation setup and participant admission.
- Input commitment, result release, and auditability protocols.
- Replay resistance, impersonation resistance, and privilege-escalation checks.
- Collusion assumptions across schools, public bodies, compute nodes, analysts, and operators.

Tamarin is not a substitute for implementation verification. It should not be used to prove:

- Rust memory safety.
- Parser correctness.
- Numeric correctness of analytics.
- MPC library implementation correctness.
- Storage engine behavior.

Those concerns require Rust tests, property tests, Kani, F*/Low*/EverParse, cryptographic library review, and operational controls.

## Confidential Computation Scope

"Confidential computation" in Kiva should be treated as a family of techniques rather than one mechanism.

Mechanism families in scope:

- **Secure multi-party computation:** useful when multiple institutions jointly compute aggregates without sharing raw inputs.
- **Private set intersection:** useful for matching cohorts or records across institutions without exposing non-matches.
- **Threshold cryptography:** useful for splitting signing, decryption, or result-release authority.
- **Differential privacy:** useful for reducing disclosure risk from aggregate outputs.
- **Trusted execution environments:** useful for deployment-constrained confidential processing, but they add hardware and attestation trust assumptions.
- **Fully homomorphic encryption:** useful for narrow computations where its performance and operational cost are acceptable.

Kiva avoids claiming generic secure computation. It picks concrete workflows,
defines threat models, selects bounded mechanisms, and tests or proves those
specific properties.

Current and target workflows:

- Cross-school aggregate learning analytics with no raw record centralization.
- Public-interest reporting where analysts receive only approved aggregate outputs.
- Privacy-preserving cohort intersection for longitudinal analysis.
- Threshold-controlled release of sensitive analytical results.
- Audited separation between operational administrators and personal learning records.

The first selected threat-model target is cross-institution aggregate learning
analytics with custodian-only raw record access, admitted computation
participation, policy-authority result release, and analyst access only to
released aggregate insight. Kiva now keeps the implemented mechanism claims in
the dedicated confidential-computation milestone ledger: n-of-n additive
aggregation, differential privacy release assurance, hash-committed Shamir
VSS admission, bounded hash-token PSI cohort matching, review-gated DH-OPRF
PSI runtime-boundary work, portable deployment-bound TEE attestation evidence
gating, and review-gated AWS Nitro verifier-adapter attestation. It also
includes a review-gated FROST threshold-signing runtime boundary and a narrow
FHE small-sum feasibility boundary plus a review-gated FHE runtime integration
boundary; production PSI before external
cryptographic review, production hardware-backed TEE execution before external
security review, production malicious-robust threshold cryptography before
external cryptographic review, and production FHE remain outside the current
claim.
CC7 packages these selected boundaries for external
engineering review through
`spec/confidential/confidential-computation-assurance-package.yaml`; that
package does not create legal certification, IEEE xAPI certification, full
formal verification, production MPC, malicious-secure PSI, production OPRF PSI
before external cryptographic review, production hardware-backed TEE execution,
production threshold cryptography, production FHE, or raw vendor quote parsing
claims. CC15 adds a repository-side release-candidate assurance readiness gate
through `spec/confidential/external-production-assurance-readiness.yaml`, but
external review outcomes and production security claims still remain outside
the repository until independently obtained. CC16 adds
`spec/confidential/secret-compute-provider-contract.yaml`, which fixes the
`kiva-reference-primitives-v1` provider for tests and verification while
requiring external provider bindings and review evidence for production
claims. See
`docs/architecture/confidential-aggregate-threat-model.md`,
`docs/architecture/additive-secure-aggregate-mechanism.md`, and
`docs/architecture/confidential-computation-milestones.md`.

## Verification and Evidence Plan

Kiva should maintain a requirement and evidence model from the beginning.

Recommended evidence tracks:

- **xAPI conformance:** requirement matrix, standard examples, request/response tests, pagination tests, document resource tests, attachment tests, and negative tests.
- **Authorization and consent:** integration tests, policy tests, token-flow tests, revocation tests, and Tamarin models for critical protocols.
- **Rust implementation safety:** unit tests, property tests, Kani harnesses for bounded state-machine checks, fuzzing for parsers and request admission.
- **Cryptographic boundaries:** explicit algorithm allowlists, HACL*/EverCrypt usage where practical, constant-time review where secrets are processed.
- **Parsing boundaries:** strict JSON admission rules for xAPI resources and policy documents; EverParse/F* only where the boundary is stable and security-critical.
- **Confidential computation:** threat models, protocol diagrams, Tamarin models, library selection notes, result-release policies, and reproducible test vectors.
- **Source-level evidence registry:** typed entries in
  `spec/confidential/assurance-evidence-registry.yaml` bind Rust symbols, Rust
  tests, Kani harnesses, and Tamarin lemmas to source files so assurance
  validators resolve evidence through one catalog instead of ad hoc source
  scans.
- **Supply chain:** pinned Nix toolchains, cargo deny/audit/vet or equivalent checks, SBOM generation, and vulnerability response workflow.

## Non-Goals

Kiva does not claim:

- Full formal verification of the entire Kiva platform.
- Generic conformance to every future xAPI Profile.
- That xAPI itself provides consent, privacy, or confidential computation semantics.
- That Project Everest components prove the security of complete multi-party analytics workflows.
- That Tamarin proves implementation correctness.
- That BI users can safely query raw personal learning records directly.

## Integration Boundaries

The integration plan uses the following boundaries:

- Kiva behaves as an OAuth/OIDC-style resource server for already-validated
  bearer tokens and exact scopes; production JWKS, JWT validation,
  introspection, and tenant-specific claim transformation remain deployment
  adapters.
- PostgreSQL with Atlas-managed migrations backs the LRS, PDS, audit, and
  confidential metadata state; future event-log architecture remains
  deployment-specific.
- Additive aggregation, differential privacy, Shamir VSS admission, PSI, TEE,
  Nitro, FROST, FHE, and provider boundaries are tracked in the confidential
  computation milestone ledger with explicit non-claims.
- The representative threshold-release model has a concrete HTTP API,
  PostgreSQL persistence, PDS data-product linkage, Kani coverage, and Tamarin
  boundary evidence.
- Learner consent, institutional policy, public-interest authority, revocation,
  retention metadata, lifecycle cases, and public-sector governance profiles
  are expressed in the PDS policy and persistence model.
- Public conformance and assurance claim language is defined in the xAPI
  conformance claim, PDS milestone ledger, confidential computation milestone
  ledger, and release evidence-set validators.
