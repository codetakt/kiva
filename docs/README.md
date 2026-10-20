# Kiva Documentation

This directory is the canonical human-readable map for Kiva design and
operations documentation. The repository keeps machine-readable contracts and
evidence under `spec/`; design notes in `docs/architecture/` explain the
boundaries behind those contracts; runbooks in `docs/operations/` describe how
to produce local evidence and operate the bounded mechanisms.

Execution results, review decisions and assurance assessments are kept under the
Git-ignored `target/` directory. Published documentation describes contracts,
design constraints and verification procedures.

## Boundary Map

Kiva is organized around five claim boundaries, but the design model enters
through four primary roots: OpenAPI, OperationKernel, AssurancePrimitive, and
CapabilityPort.

- **IEEE xAPI LRS:** the HTTP/JSON learning-record surface, with requirements
  mapped to the pinned IEEE Std 9274.1.1-2023.
- **PDS supporting plane:** Kiva-specific consent, lifecycle, governance, and
  approved data-product command/query workflows under OperationKernel.
- **Confidential computation provider boundary:** mechanism contracts,
  reference primitives, provider admission, and sealed capability evidence gates.
- **Analytics serving boundary:** governed aggregate reads over approved data
  products and admitted confidential releases, separate from the xAPI LRS.
- **Evidence-driven assurance layer:** conformance, formal-model,
  release-evidence, and CI checks reviewed through AssuranceGraph.

Kiva has four deliberately separate public surfaces:

- **xAPI LRS:** HTTP/JSON interface with the pinned IEEE Std 9274.1.1-2023
  requirements baseline, PostgreSQL-backed persistence, OpenAPI projection,
  conformance aid, and release evidence. The xAPI claim is not an IEEE
  certification claim.
- **PDS extension:** Kiva-specific consent, audit, lifecycle, approved
  data-product, governance-template, and provider-binding registry surface
  around accepted xAPI records and confidential releases. Provider bindings are
  metadata-only control-plane evidence admitted by the confidential provider
  contract, not secret custody or cryptographic payload storage. The PDS
  extension is not part of the IEEE xAPI LRS surface.
- **Confidential computation extension:** bounded additive aggregation,
  differential privacy release, Shamir VSS admission, hash-token PSI,
  review-gated DH-OPRF PSI, portable and Nitro TEE admission, FROST,
  FHE-feasibility, FHE-runtime, job lifecycle, participant/key lifecycle, and
  provider-contract boundaries. These are engineering assurance boundaries for
  external review, not production MPC/FHE/TEE certification claims.
- **Analytics serving:** read-only product listing, product detail, and
  aggregate payload retrieval over approved products and currently admitted
  releases. External reporting tools consume this surface; Kiva does not host
  a BI product, and this surface does not expose raw Statements or
  individual-level data.

The historical label `IEEE/ISO/IEC 39274-1-1-2025` and associated filenames are
compatibility identifiers. Equivalence with the 2025 edition has not been
established, and requirement traceability alone does not establish
implementation conformance.
See the [xAPI claim boundary](architecture/conformance-claim.md).

The main entry points are:

- [Design Source of Truth](architecture/design-source-of-truth.md)
- [System Design](architecture/system-design.md)
- [xAPI Conformance Claim](architecture/conformance-claim.md)
- [PDS Extension Compatibility](architecture/pds-extension-compatibility.md)
- [Personal Data Store Milestones](architecture/pds-milestones.md)
- [Analytics Claim Boundary](architecture/analytics-claim-boundary.md)
- [Reference Reporting Deployments](architecture/reference-reporting-deployments.md)
- [Confidential Computation Milestones](architecture/confidential-computation-milestones.md)
- [Server Modular Boundaries](architecture/server-modular-boundaries.md)
- [Formal Verification Roadmap](architecture/formal-verification-roadmap.md)
- [Claim Strengthening Roadmap](architecture/claim-strengthening-roadmap.md)
- [CI Gates](operations/ci-gates.md)
- [Release Evidence](operations/release-evidence.md)

Implementation boundary anchors are intentionally not duplicated here. The
reviewable source of truth is intentionally narrow:

- OpenAPI contracts own external HTTP request and response shape.
- The architecture graph and policy ledgers own source, projection, and
  invariant ownership; `spec/architecture/core-model.source.yaml` fixes the
  four primary roots.
- `spec/architecture/operation-kernel.source.yaml`,
  `spec/evidence/assurance-primitive.source.yaml`,
  `spec/confidential/mechanism-packages.source.yaml`, and
  `spec/evidence/review-bundle.source.yaml` are the compact design review
  entrypoints over the split operation, assurance, confidential, and generated
  evidence ledgers.
- `scripts/ci/kiva-ci explain operation|claim|mechanism|source <id>` is the
  compact CLI entrypoint for following the primary generated review surfaces.
- The claim catalog and maturity ledger own public claim readiness.
- The secret-compute provider contract owns provider vocabulary, admission
  evidence, and runtime gate ownership.

Generated catalogs, projection graphs, evidence matrices, and rendered review
documents are derived from those sources. The main projections are:

- `spec/architecture/architecture-graph.source.yaml` for source/projection/check
  ownership across HTTP contracts, the generated surface catalog, public
  claims, milestone ledgers, architecture guards, and generated schema
  artifacts. `spec/architecture/architecture-policy.source.yaml` is a thin
  include root for policy fragments under `spec/architecture/policies/`, which
  own source-layer classification, generated-artifact classification, and
  architecture invariants such as control/data-plane boundaries.
  `generated/architecture/compiled-architecture.yaml` is the persistent compiled
  architecture IR. `spec/architecture/design-backbone.yaml` and
  `spec/ci/projection-graph.yaml` are generated review projections from that IR,
  so policy-owned classification and the artifact catalog are not duplicated in
  the graph source.
- `spec/architecture/boundaries.yaml` and included source-group manifests for
  executable drift checks over source anchors, dependency rules, module
  ownership, route contexts, source-size limits, hygiene rules, and retained
  compatibility surfaces.
- `spec/architecture/operation-kernel.source.yaml` for the OperationKernel
  review entrypoint. It groups the split operation ownership, surface algebra,
  generation policy, implementation policy, PDS-family, and cross-surface
  family ledgers that generate `generated/architecture/surface-catalog.yaml`
  and `generated/architecture/operation-kernel.yaml`.
- `generated/architecture/pds-resource-families.yaml` and
  `generated/architecture/surface-families.yaml`, and
  `generated/architecture/operation-catalog.yaml` for the PDS family, cross-surface
  family, and application workflow projections generated from the surface
  catalog.
- `spec/architecture/claim-catalog.yaml` for public implementation claims and
  evidence paths, with `spec/architecture/claim-maturity.yaml` owning claim
  maturity labels. `generated/claim-readiness-matrix.yaml` and
  `docs/architecture/generated/claim-readiness-matrix.md` join those claims to
  projection edges and CI checks.
- `spec/openapi/generated-contract-dto-map.yaml` and
  `generated/openapi-dto-map-projection.yaml` for generated contract DTO
  membership derived from OpenAPI, generated contract crates, the DTO boundary
  ledger, and the split Rust DTO maps.
- `spec/ci/evidence-checks.yaml` for grouped assurance validators and per-check
  `check_kind`, input/output, mutation, network, and artifact-class metadata.
  It is generated from the primary CheckCatalog/GatePlan ledgers
  `spec/ci/evidence-check-catalog.source.yaml` and
  `spec/ci/evidence-gate-plan.source.yaml`; `spec/ci/evidence-checks.source.yaml`
  remains the compatibility source view over those normal-form ledgers.
  Projection-derived inputs are supplied by `spec/ci/projection-graph.yaml` at
  evidence-run time.
  `scripts/ci/run_design_review_suite.py` is the non-recursive suite runner over
  the required groups; the Nix entrypoint is
  `nix build .#checks.x86_64-linux.design-review-suite`.
- `generated/architecture/assurance-graph.yaml` for the primary
  claim/evidence review surface. `generated/evidence/evidence-set.yaml`,
  `generated/evidence/evidence-receipts.yaml`,
  `generated/evidence/executed-receipt-overlay.yaml`, and
  `generated/architecture/claim-graph.yaml` are materialized inputs to that
  surface. Executed receipt overlays are no-run by default and can be
  regenerated from release run reports.
- `spec/confidential/mechanism-packages.source.yaml` and
  `generated/confidential/mechanism-profile-index.yaml` for the confidential
  MechanismPackage review path. Packages bind capability, provider boundary,
  maturity, assurance, and verification facets before generated indexes project
  them.
- `spec/pds/core-planes.source.yaml` for the PDS event, inventory, policy, and
  release plane packages. Each plane carries checked invariant refs back to
  `spec/pds/invariants.source.yaml`; the generated
  `generated/architecture/pds-plane-packages.yaml` resolves invariants, state
  machines, resource families, implementation anchors, and checks per plane.
- `spec/architecture/architecture-graph.source.yaml#/sources` for CI-checked
  source-to-projection relationships. The compiled architecture IR, design
  backbone, and generated `spec/ci/projection-graph.yaml` are review projections
  of that graph and policy join.

Human-readable design documents should explain intent and claim boundaries;
mechanical inventories should stay in `spec/` and generated evidence artifacts.

## Architecture

### xAPI, Persistence, and Conformance

- [System Design](architecture/system-design.md)
- [Server Modular Boundaries](architecture/server-modular-boundaries.md)
- [Compatibility Surfaces](architecture/compatibility-surfaces.md)
- [xAPI Conformance Claim](architecture/conformance-claim.md)
- [Conformance Authentication Fixtures](architecture/conformance-auth-fixtures.md)
- [Conformance Endpoint Profiles](architecture/conformance-endpoint-profiles.md)
- [Conformance Suite Contribution Strategy](architecture/conformance-suite-contribution.md)
- [OAuth/OIDC Resource-Server Binding](architecture/oauth-oidc-resource-server-binding.md)
- [PostgreSQL Persistence Strategy](architecture/postgres-persistence.md)

### Personal Data Store

- [PDS Extension Compatibility](architecture/pds-extension-compatibility.md)
- [Personal Data Store Milestones](architecture/pds-milestones.md)
- [PDS Shamir Reconstruction Data Products](architecture/pds-shamir-reconstruction-data-products.md)
- [PDS Operational Lifecycle Workflows](architecture/pds-operational-lifecycle-workflows.md)
- [PDS Public-Sector Deployment Templates](architecture/pds-public-sector-deployment-templates.md)

### Confidential Computation

- [Assurance and Confidential Computation Strategy](architecture/assurance-and-confidential-computation.md)
- [Confidential Computation Milestones](architecture/confidential-computation-milestones.md)
- [Confidential Aggregate Threat Model](architecture/confidential-aggregate-threat-model.md)
- [Additive Secure Aggregate Mechanism](architecture/additive-secure-aggregate-mechanism.md)
- [Differential Privacy Assurance](architecture/differential-privacy-assurance.md)
- [Confidential Mechanism Abstraction](architecture/confidential-mechanism-abstraction.md)
- [Shamir Threshold Cryptography](architecture/shamir-threshold-cryptography.md)
- [Threshold FROST Signing Boundary](architecture/threshold-frost-signing.md)
- [Private Set Intersection](architecture/private-set-intersection.md)
- [Production PSI Upgrade](architecture/production-psi-upgrade.md)
- [TEE Attestation Boundary](architecture/tee-attestation-boundary.md)
- [AWS Nitro Attestation Integration](architecture/tee-nitro-attestation-integration.md)
- [FHE Sum Feasibility Boundary](architecture/fhe-feasibility-boundary.md)
- [FHE Runtime Integration Boundary](architecture/fhe-runtime-integration.md)
- [Confidential Computation Assurance Package](architecture/confidential-computation-assurance-package.md)
- [External Production Assurance Readiness](architecture/external-production-assurance-readiness.md)
- [Secret and Confidential-Compute Provider Integration](architecture/secret-compute-provider-integration.md)
- [Practical Confidential Deployment Selection](architecture/practical-confidential-deployment-selection.md)
- [Confidential Key and Participant Lifecycle](architecture/confidential-key-participant-lifecycle.md)

### Analytics

- [Analytics Claim Boundary](architecture/analytics-claim-boundary.md)
- [Reference Reporting Deployments](architecture/reference-reporting-deployments.md)

### Verification

- [Formal Verification Roadmap](architecture/formal-verification-roadmap.md)

## API Contracts

- [Kiva xAPI OpenAPI contract](../spec/openapi/kiva-xapi.yaml)
- [Kiva PDS OpenAPI contract](../spec/openapi/kiva-pds.yaml)
- [Kiva confidential-computation OpenAPI contract](../spec/openapi/kiva-confidential.yaml)
- [Kiva analytics OpenAPI contract](../spec/openapi/kiva-analytics.yaml)
- [Generated contract DTO map](../spec/openapi/generated-contract-dto-map.yaml)
- [Semantic compatibility DTO map](../spec/openapi/semantic-compatibility-map.yaml)
- [Retired OpenAPI/Rust DTO archive](../spec/openapi/rust-dto-map.yaml)
- [OpenAPI DTO boundary ledger](../spec/openapi/dto-boundary-ledger.yaml)
- [CI projection graph](../spec/ci/projection-graph.yaml)
- [PDS publication artifact schemas](../spec/pds)
- [Compiled architecture IR](../generated/architecture/compiled-architecture.yaml)
- [Architecture design backbone](../spec/architecture/design-backbone.yaml)
- [Generated surface catalog](../generated/architecture/surface-catalog.yaml)
- [Platform architecture boundary manifest](../spec/architecture/boundaries.yaml)
- [Generated HTTP operation catalog](architecture/generated/http-operation-catalog.md)
- [Generated application operation catalog](architecture/generated/application-operation-catalog.md)
- [Generated surface operation index](architecture/generated/surface-operation-index.md)
- [Generated route coverage](architecture/generated/route-coverage.md)
- [Generated implementation claim catalog](architecture/generated/claim-catalog.md)
- [Generated claim readiness matrix](architecture/generated/claim-readiness-matrix.md)
- [CI evidence-check manifest](../spec/ci/evidence-checks.yaml)
- [Confidential computation milestone ledger](../spec/confidential/computation-milestones.yaml)
- [Secret-compute provider-kind registry](../spec/confidential/secret-compute-provider-kind-registry.yaml)

## Operations

- [CI Gates](operations/ci-gates.md)
- [Release Evidence](operations/release-evidence.md)
- [Shamir VSS Operations Runbook](operations/shamir-vss-runbook.md)
- [Threshold FROST Operations Runbook](operations/threshold-frost-runbook.md)
- [PSI Cohort Operations Runbook](operations/psi-cohort-runbook.md)
- [DH-OPRF PSI Runbook](operations/psi-oprf-runbook.md)
- [TEE Attestation Operations Runbook](operations/tee-attestation-runbook.md)
- [AWS Nitro TEE Operations Runbook](operations/tee-nitro-runbook.md)
- [FHE Feasibility Runbook](operations/fhe-feasibility-runbook.md)
- [FHE Runtime Runbook](operations/fhe-runtime-runbook.md)
- [Confidential Computation Assurance Runbook](operations/confidential-computation-assurance-runbook.md)
- [External Production Assurance Runbook](operations/external-production-assurance-runbook.md)

## Requirement Traceability

- [IEEE Std 9274.1.1-2023 requirement catalog](../spec/requirements/ieee-39274-1-1-2025.yaml)
- [xAPI LRS conformance discharge plan](../spec/conformance/xapi-lrs-discharge.yaml)
- [ADL compatibility map](../spec/conformance/adl-compatibility-map.yaml)
