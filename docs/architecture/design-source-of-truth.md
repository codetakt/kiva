# Design Source of Truth

Kiva keeps human design documents thin. Review decisions live in canonical
machine-readable ledgers; generated files are reproducible projections; CI
guards enforce drift boundaries.

Editors review source YAML, OpenAPI contracts, and Rust implementation directly.
Generated YAML, JSON, Markdown, and Rust registry files are committed evidence
but are reviewed through generator checks, drift checks, and release evidence
reports. The generated review policy is declared in
`spec/architecture/policies/artifact-class-policy.source.yaml`.

## Canonical Sources

- **Core design model:** `spec/architecture/core-model.source.yaml` is the
  thin review entrypoint for the current design. Human edits start one level
  above it at `spec/architecture/design-universe.source.yaml`. The root delegates
  authoritative rows to `design-source-nodes.source.yaml`,
  `design-package-rows.source.yaml`, and `design-ledger-rows.source.yaml`, then renders
  `spec/architecture/architecture-graph.source.yaml`,
  `spec/architecture/design-packages.source.yaml` and
  `spec/architecture/ledger-catalog.source.yaml` as generated source
  projections. The core model names the primary roots as OpenAPI,
  OperationKernel, AssurancePrimitive, and CapabilityPort. Projection, CI, PDS,
  and verification ledgers are supporting governance, operation, or assurance views over
  those roots.
- **HTTP contracts:** `spec/openapi/kiva-xapi.yaml`,
  `spec/openapi/kiva-pds.yaml`, and `spec/openapi/kiva-confidential.yaml` own
  external request and response shape. OpenAPI remains the HTTP contract source
  of truth.
- **Architecture graph, projection normal form, and policy:**
  `spec/architecture/design-universe.source.yaml` owns the editable metamodel.
  Its three normalized fragments keep source identity, package membership, and
  ledger lifecycle independently reviewable without creating additional roots.
  `spec/architecture/architecture-graph.source.yaml` is its generated source
  identity projection for compiled architecture tooling and human-readable
  source ownership. `spec/architecture/projection-specs.source.yaml` is a
  generated-only compatibility view for source-to-projection behavior, not a
  canonical source classification.
  `spec/architecture/projection-artifacts.source.yaml`,
  `spec/architecture/projection-runtime.source.yaml`, and
  `spec/architecture/projection-check-bindings.source.yaml` are the primary
  ProjectionArtifact, ProjectionRuntime, and ProjectionCheckBinding ledgers.
  `spec/architecture/projection-normal-form-policy.source.yaml` owns that
  transition policy: the normal-form projection ledgers are the compiler input,
  while ProjectionSpec remains a generated-only compatibility sidecar checked
  for drift.
  `spec/architecture/architecture-policy.source.yaml` includes policy fragments
  under `spec/architecture/policies/`, including `source_layers:`,
  `artifact_class_policy:`, `confidential_assurance_source_model:`,
  control-plane/data-plane policy, PDS resource pattern policy, and
  workflow-entrypoint policy. The graph source does not own `source_kind:
  canonical` classification directly; classification is compiled from policy.
  `spec/architecture/source-ledger-schemas.yaml` is the schema-first registry
  for canonical source ledgers that have JSON Schema coverage.
- **OperationKernel entrypoint:** `spec/architecture/operation-kernel.source.yaml`
  is the human review entrypoint for OperationKernel. It groups the split
  operation ownership, surface algebra, generation policy, implementation
  policy, PDS-family, and cross-surface family ledgers. Those fragments remain
  editable, but they are reviewed as OperationKernel fragments rather than as
  separate operation roots.
- **AssurancePrimitive entrypoint:** `spec/evidence/assurance-primitive.source.yaml`
  is the human review entrypoint for claims, obligations, discharges, and
  receipts. `EvidenceSet`, `EvidenceReceipt`, `ClaimGraph`,
  `VerificationObligation`, and executed receipt overlays remain materialized
  inputs to `generated/architecture/assurance-graph.yaml`.
  The same source owns node-type authority levels, so the AssuranceGraph
  generator contains no independent claim-authority policy.
- **Claims and maturity:** `spec/architecture/claim-catalog.yaml` and
  `spec/architecture/claim-maturity.yaml` own public implementation claims,
  maturity labels, `readiness_requirement:`, and evidence paths.
- **PDS core planes:** `spec/pds/core-planes.source.yaml` owns the compact PDS
  supporting model under OperationKernel. PDS extension behavior is mapped to
  the event, inventory, policy, and release planes; each plane packages the
  relevant PDS invariant refs from `spec/pds/invariants.source.yaml` and PDS
  state-machine refs from `spec/pds/state-machines.source.yaml`.
  `generated/architecture/pds-plane-packages.yaml` joins those rows with
  resource families, implementation anchors, and checks for routine review. PDS
  model sources belong to the Operation layer; only their claims and evidence
  belong to Assurance. This model
  does not expand the IEEE xAPI LRS conformance claim.
- **Confidential mechanism packages:** `spec/confidential/mechanism-packages.source.yaml`
  is the human review entrypoint for confidential-computation package rows. A
  package binds mechanism, capability, sealed provider boundary, maturity,
  assurance, and verification facets. Capability taxonomy, bindings, provider
  contract, maturity, assurance registry, and verified kernels remain package
  facets instead of independent confidential-computation roots. The generated
  index contains a `PackageProfile` for every package, including package rows
  that intentionally have no standalone mechanism-catalog row.
- **DTO boundary:** `spec/openapi/dto-boundary-policy.yaml` owns the current
  `external_contract_boundary`, `internal_semantic_boundaries`,
  `generated_contract_dto_map_contracts`, and
  `semantic_compatibility_map_contracts` policy. `spec/openapi/semantic-compatibility-retirement.yaml`
  owns the remaining semantic compatibility retirement ledger.
  `spec/openapi/dto-boundary-ledger.yaml` is the active DTO boundary ledger; it
  records `spec/openapi/dto-generation-plan.yaml` only as the retired legacy
  path and must match those split sources.
  `spec/openapi/semantic-compatibility-map.yaml`
  is the hand-maintained compatibility ledger. `spec/openapi/generated-contract-dto-map.yaml`
  is generated from OpenAPI contracts and generated contract crates.
- **Provider assurance:** `spec/confidential/secret-compute-provider-contract.yaml`,
  confidential mechanism ledgers, and the typed evidence registry own provider
  vocabulary, admission evidence, runtime gates, and release gates. The
  `spec/confidential/capability-taxonomy.source.yaml` ledger owns default
  capability-to-workflow affinity; the generated `CapabilityBinding` index owns
  mechanism-to-operation binding rows.
  The generated `MechanismProfile` index joins provider boundaries to
  capability bindings, maturity, assurance, and verification evidence.

## Generated Projections

Generated artifacts are review views, not new design authority. The compact
entrypoints are:

- `generated/architecture/source-registry.yaml`: source ownership, schema
  coverage, projection roles, and generated artifact paths.
- `generated/architecture/operation-kernel.yaml`: the first-class operation IR
  joining HTTP contract identity, `SurfaceOperation` semantics, workflow
  ownership, resource-family assignment, PDS planes, and requirement evidence.
- `generated/architecture/pds-plane-packages.yaml`: the operation-owned PDS
  review view joining each plane to invariants, state machines, resources,
  anchors, milestones, and checks.
- `generated/architecture/assurance-graph.yaml`: the primary AssurancePrimitive
  review surface over Claim -> Obligation -> Discharge -> Receipt.
- `generated/confidential/mechanism-profile-index.yaml`: the primary generated
  confidential MechanismPackage review surface over capability, provider,
  maturity, assurance, and verification facets.
- `generated/architecture/compatibility-retirement-plane.yaml`: supporting
  removed or retained compatibility-surface history plus deletion-readiness
  evidence; `ready-to-delete` remains evidence, not deletion authorization.
- `generated/evidence/evidence-set.yaml`,
  `generated/evidence/evidence-receipts.yaml`, and
  `generated/architecture/claim-graph.yaml`: materialized inputs to the
  AssuranceGraph review surface.

The routine human review bundle is declared in
`spec/evidence/review-bundle.source.yaml`. Lower-level generated YAML and
Markdown files remain committed evidence, but they are supporting views unless
the review bundle promotes them.

ProjectionRuntime normal-form ledgers under `spec/architecture/projection-*.source.yaml`
are now the compiler input. `ProjectionSpec` remains a generated-only
compatibility view and is checked against those ledgers by
`validate_projection_normal_form.py` and
`validate_projection_transition_cleanup.py`.

## Executable Guards

- `scripts/ci/validate_architecture_core_model.py` checks the thin core model
  against existing operation, assurance, projection, CI, confidential, PDS, and
  verification ledgers.
- `scripts/ci/validate_ci_command_registry.py` checks that `kiva-ci` commands
  are derived from the generated evidence-check manifest. The same entrypoint
  exposes `kiva-ci explain operation|claim|mechanism|source <id>` so routine
  review can start from primary generated surfaces instead of opening every
  supporting projection directly.
- `scripts/ci/run_generators.py` is the common generator runner. It is exposed
  as `scripts/ci/kiva-ci run generators -- --check` for dependency-ordered
  generator drift checks. The order is a stable topological sort of
  `ProjectionRuntime.generator_plan`; Python code does not carry a parallel
  hard-coded generator order.
- `scripts/ci/validate_pds_core_planes.py` checks that PDS milestones and
  invariants are covered by the event, inventory, policy, and release planes
  and that each plane references existing implementation anchors and CI checks.
- `scripts/ci/validate_pds_state_machines.py` checks the PDS consent,
  lifecycle, release-artifact, and provider-binding state machines against PDS
  planes, invariants, CI checks, and implementation anchors.
- `scripts/ci/validate_design_backbone.py` checks the compiled design backbone,
  including `validate_architecture_graph_source`,
  `validate_surface_operation_algebra`, generated schema strategy, DTO
  boundary policy, and workflow entrypoint policy.
- `scripts/ci/validate_projection_graph.py` checks `render_projection_graph`,
  projection inputs, generated targets, referenced CI check IDs, and
  `derived_input_policy: projection-graph` checks whose effective inputs come
  from the projection graph instead of repeated manifest lists.
- `scripts/ci/validate_surface_catalog.py`,
  `scripts/ci/validate_pds_resource_families.py`, and
  `scripts/ci/validate_surface_families.py` check generated surface,
  resource-family, and family ledgers. `def validate_resource_families(` and
  `validate_surface_family_ledger` are the executable review points for those
  generated ledgers.
- `spec/architecture/boundaries.yaml` and
  `scripts/ci/validate_architecture_boundaries.py` enforce docs, source groups,
  dependency rules, module ownership, source size, hygiene rules, compatibility
  surfaces, and route ownership.
  Compatibility-surface `rust-use-absent` checks use the AST scanner in
  `tools/kiva-rust-use-scan` so negative import checks inspect Rust `use`
  items rather than arbitrary text.
- `spec/ci/evidence-check-groups.source.yaml` owns required CI group order,
  Nix gates, and claim boundaries. `spec/ci/evidence-checks.source.yaml` owns
  per-check execution rows and the `RegistryDrivenCommand` execution model.
  `spec/ci/evidence-checks.yaml` is the generated manifest; every check row
  carries a `ci_command`, and `scripts/ci/run_evidence_checks.py` executes
  checks through `scripts/ci/kiva-ci`. `spec/ci/evidence-check-catalog.source.yaml`
  and `spec/ci/evidence-gate-plan.source.yaml` are the primary CI normal-form
  ledgers; `spec/ci/evidence-checks.source.yaml` is the compatibility source
  view and is classified separately from the generated CI manifest runtime.
  `NoAliasDumper`,
  `dump_yaml`, `classified_source_map`, `artifact_catalog_from_sources`, and
  `artifact_class_policy_from_architecture_policy` keep generator output stable.
  The generated `EvidenceSet` index gives those gates a single review model
  without changing the underlying domain-specific reports.
  The generated manifest annotates ProjectionSpec-backed checks with
  `projection_spec_refs`, so CI check provenance is derived from the
  ProjectionRuntime normal-form ledgers and compared against the generated
  ProjectionSpec compatibility view instead of being reviewed as an isolated
  manifest row.
- `scripts/ci/validate_source_ledger_schemas.py` validates source ledgers
  listed in `spec/architecture/source-ledger-schemas.yaml` against their JSON
  Schemas. New source ledgers should be added to that registry before they
  become release evidence. Major design ledgers now use dedicated schemas where
  they carry stable domain structure; policy fragments remain on the generic
  source-ledger schema until their row models stabilize.
- `scripts/ci/generate_source_registry.py` renders
  `generated/architecture/source-registry.yaml` and
  `docs/architecture/generated/source-registry.md`, joining generated
  ArchitectureGraph source nodes, policy-derived source-layer classification,
  schema coverage, and projection roles into one review surface.
- `scripts/ci/validate_projection_normal_form.py` and
  `scripts/ci/validate_projection_transition_cleanup.py` keep ProjectionRuntime
  normal form primary and reject stale generated-only ProjectionSpec
  compatibility fields.
  `scripts/ci/validate_evidence_check_normal_form.py` keeps the CI catalog and
  gate-plan primary while checking the compatibility view.
- `scripts/ci/validate_compatibility_exit_gates.py` checks that every retained
  compatibility surface has a migration target, an owner-anchor retirement
  check, a forbidden call-site check, and `next_status: deletion-ready`.

The active source layers are **Contract**, **Operation**, **Assurance**, and
**Governance**. Governance owns the architecture metamodel, executable
projection machinery, CI runtime manifests, and compatibility lifecycle
overlays; Operation owns PDS planes, invariants, state transitions, and resource
families; Assurance owns claims, verification packages, and receipt indexes.
Compatibility retirement history is governance-owned evidence, not a fifth
top-level design domain.

## PDS Resource Pattern

Every PDS resource family should follow the same shape:

`OpenAPI DTO -> kiva-pds command/query -> admission/policy -> service or application workflow -> repository facade -> memory/PostgreSQL storage -> wire projection -> evidence anchors`

This is the pattern for consent, lifecycle, governance, data products, access
checks, and provider bindings. New resources should not introduce a parallel
path unless they need a genuinely different dependency boundary.

The executable family ledger is `generated/architecture/pds-resource-families.yaml`.
It is checked by `scripts/ci/validate_pds_resource_families.py`, which verifies
that every `/pds/...` OpenAPI operation is owned exactly once, its declared
request and success response schemas match the OpenAPI contract, and its Rust
handler appears in the declared route context.
`generated/architecture/operation-kernel.yaml` is the higher-level operation IR
that links the same operation to its HTTP requirements, workflow ownership, PDS
family when present, and surface family. Reviewers should use it as the first
answer to "what does this endpoint execute and what evidence owns it?"
The same validator checks every PDS operation against
`generated/http-operation-catalog.evidence.json`, which is generated directly
from OpenAPI. The generated surface catalog is validated against
that HTTP projection, and `scripts/ci/generate_surface_catalog_bundle.py` plus
`scripts/ci/generate_surface_catalog_projections.py` keep the bundle,
application-operation ledger, PDS family ledger, cross-surface family ledger,
and server surface-profile registry current projections of the same source
entries. PDS read, direct access, and command operations therefore share one
review point for
`ApplicationOperationGateway` lifecycle evidence.
`spec/architecture/surface-projections.yaml.edit_policy` fixes the human
editable ledgers, including the split surface model, generation, and
implementation policy files, so the surface catalog cannot silently become
another manual source of truth.
`spec/architecture/surface-model.source.yaml.surface_operation_algebra` fixes
`SurfaceOperation` as the sole operation algebra joining OpenAPI-derived
`HttpOperation` identity with a sparse `SemanticOperationOverlay`. OpenAPI
contract and operationId key the overlay; method, path, and required scopes are
projected from OpenAPI through
`generated/http-operation-catalog.evidence.json`. The surface catalog validator
checks that this algebra is carried into the generated bundle and that its
projection targets match the active application-operation, PDS family,
route-coverage, surface-index, and server surface-profile outputs.
In runtime code, `PdsWorkflowKind::boundary()` classifies PDS workflows as
`Command`, `Query`, or `DirectAccess`; write paths assert command boundaries and
read projections assert query boundaries so handler shape and application
workflow shape stay aligned with the generated PDS family ledger.

PDS resource-family entries are also checked against a per-family server role
matrix: handler, repository, memory storage, PostgreSQL storage, wire
projection, and OpenAPI DTO projection. This keeps the repeated resource-family
shape reviewable without making each handler or storage module a new design
source.

`scripts/ci/validate_design_backbone.py` validates the design-backbone itself:
source ownership entries, generated schema strategy, PDS resource path,
control/data-plane boundary, projection-graph ownership, and workflow
entrypoint policy.

## Control Plane

Kiva owns control-plane evidence: HTTP contracts, metadata registries,
provider identity and artifact digests, evidence gates, release gates,
lifecycle state, access audit, and release evidence.

Kiva does not own production data-plane security by default. Secret custody,
raw provider secrets, ciphertext payload custody, TEE runtime isolation,
malicious-secure MPC/PSI execution, and production FHE security remain external
provider responsibilities unless a future milestone explicitly moves them into
Kiva.

## Workflow Entry

Application workflows should enter through `ApplicationOperationGateway` and
catalog-backed operation metadata. Direct workflow and operation executors are
internal kernel types. This keeps lifecycle audit, admission, runtime error
mapping, and operation evidence in one shape.

## DTO Boundary

`spec/openapi/dto-boundary-ledger.yaml` records the adopted generated-contract
boundary, active DTO guards, and retired DTO archives. Every external HTTP
DTO is generated into a contract crate from OpenAPI, while internal domain and
application types stay semantic and server-independent. Each `contracts[]` row
names the generated `external_contract_boundary` separately from
`internal_semantic_boundaries`; it also splits
`generated_contract_dto_map_contracts` from
`semantic_compatibility_map_contracts`. The generated map is reproduced by
`scripts/ci/generate_generated_contract_dto_map.py`, while the semantic
compatibility map remains hand-maintained until its final retirement gate.

The xAPI target crate is `crates/kiva-xapi-openapi-contract`; xAPI
Statement-family request parsing and response projection pass through that
contract crate before xAPI normalization and pure HTTP semantic helpers in
`crates/kiva-http-core`. PDS and confidential-computation likewise parse
requests and project responses through their generated contract DTO boundaries.
`GeneratedContractBoundary` means both request parsing and response projection
use generated contract DTOs. `CompatibilityHttpBoundary` is retained only as a
guarded future migration state and must carry a CI-validated
`runtime_adoption_gate` if reintroduced. The
`remaining_compatibility_work_queue` is empty after generated-boundary adoption,
and the validator checks it against unresolved handler-adoption target flags.

## Confidential Assurance Sources

The confidential-computation assurance model is deliberately one-way:

`assurance package -> MechanismProfile index -> VerificationObligation index -> typed evidence registry -> source anchors`

The assurance package owns public claim language and review packaging. The
generated `MechanismProfile` index owns the mechanism-by-mechanism review join
across provider capability, HTTP operation, maturity, assurance, and verified
kernel evidence. The generated `VerificationObligation` index owns the
property-first view of verification boundaries; the typed evidence registry
owns source-level evidence entries.

Provider capability mapping is derived from
`spec/confidential/mechanism-catalog.source.yaml`. The provider contract owns
provider kinds, binding evidence, runtime/release gates, and digest vectors; it
does not maintain an independent mechanism-to-capability table. The provider
contract separates `ReferencePrimitiveProvider`, `SecretCustodyProvider`, and
`ComputeMechanismProvider`. Reference primitives are fixture coverage only;
secret custody is excluded from mechanism execution; only compute mechanism
provider kinds participate in the production compute capability map.
`scripts/ci/generate_secret_compute_provider_kind_registry.py` projects the
provider-kind section of that contract into
`spec/confidential/secret-compute-provider-kind-registry.yaml` and
`crates/kiva-secret-provider-contract/src/kind_registry.rs`, keeping the Rust
registry and review ledger generated from the same contract source.
