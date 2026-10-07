# System Design

Kiva is a strict modular monolith. The deployable unit is one Rust server, but
the implementation is split by dependency direction so that xAPI conformance,
PDS workflows, confidential-computation boundaries, and release evidence can be
reviewed independently.
The compact design entrypoint is
`spec/architecture/design-universe.source.yaml`. Source-node, package-row, and
ledger-row authority is normalized into three declared child fragments, which render the committed
ArchitectureGraph, DesignPackage, and LedgerCatalog source projections. Review
then follows `spec/architecture/core-model.source.yaml`: OpenAPI, OperationKernel,
AssurancePrimitive, and CapabilityPort are the primary roots; ProjectionRuntime,
RegistryDrivenCommand, PdsCorePlane, and BoundaryProperty are supporting models.
The source graph is the generated ArchitectureGraph projection from
DesignUniverse. That graph, ProjectionRuntime normal-form ledgers, and
architecture policy compile into
`generated/architecture/compiled-architecture.yaml`.
`spec/architecture/projection-specs.source.yaml` is retained only as a
drift-checked compatibility sidecar, not as compiled IR input. The executable
review projection is `spec/architecture/design-backbone.yaml`.

The four claim boundaries are the IEEE xAPI LRS surface, the PDS supporting
plane under OperationKernel, the confidential-computation CapabilityPort, and
the evidence-driven assurance layer. Runtime code should follow the narrow path
`HTTP surface -> OpenAPI DTO -> application command/query -> port -> repository -> storage`.
Mechanical Rust boundary inventory remains in
`docs/architecture/server-modular-boundaries.md`; executable source ownership,
dependency anchors, and projection policy live under
`spec/architecture/boundaries.yaml` and `spec/architecture/`. Public
implementation claims are owned by `spec/architecture/claim-catalog.yaml` and
projected into `docs/architecture/generated/claim-catalog.md`.
`spec/ci/projection-graph.yaml` is the machine-readable CI projection graph
generated from the same compiled IR; evidence-check execution reports combine
direct manifest inputs with projection-derived inputs from that graph.
The generated evidence-check manifest annotates ProjectionRuntime-backed checks
with `projection_spec_refs`, so projection provenance is reviewed from the
normal-form source-to-projection ledgers instead of being inferred from a flat
CI list. CI group metadata is owned by `spec/ci/evidence-check-groups.source.yaml`.
`spec/ci/evidence-check-catalog.source.yaml` and
`spec/ci/evidence-gate-plan.source.yaml` are the primary CheckCatalog and
GatePlan ledgers; `spec/ci/evidence-checks.source.yaml` remains a retained
compatibility view checked by
`spec/ci/evidence-check-normal-form-policy.source.yaml` and classified outside
the generated CI manifest runtime source.
ProjectionRuntime behavior is primary in
`spec/architecture/projection-artifacts.source.yaml`,
`spec/architecture/projection-runtime.source.yaml`, and
`spec/architecture/projection-check-bindings.source.yaml`; ProjectionSpec is a
generated-only compatibility view checked by
`spec/architecture/projection-normal-form-policy.source.yaml`.
`generated/architecture/assurance-graph.yaml` is the primary generated evidence
review surface. `generated/evidence/evidence-set.yaml` joins required CI
groups, check metadata, effective inputs, outputs, and ready public claims as a
materialized input to AssuranceGraph. Domain packages such as conformance
evidence, PDS evidence, confidential-computation assurance, and
formal-verification reports remain specialized sources under that common flow.
The assurance vocabulary is intentionally reduced to
`Claim -> Obligation -> Discharge -> Receipt`; AssuranceGraph is the primary
review surface over that flow, while EvidenceSet, ClaimGraph,
VerificationObligation, and receipt overlays are materialized inputs.
Assurance node authority is declared by the AssurancePrimitive source and
projected into the graph; it is not embedded in generator code.
`spec/evidence/review-bundle.source.yaml` fixes the small human-facing review
bundle: SourceRegistry, OperationKernel, AssuranceGraph, and MechanismProfile
index are the routine generated review surfaces. Compatibility-retirement
history is supporting evidence and is inspected when those primary surfaces or
a compatibility change point deeper. Other generated evidence remains committed
and reproducible, but is inspected through those primary surfaces unless a
primary surface points deeper.
The `scripts/ci/kiva-ci explain` command provides a thin CLI over those review
surfaces for operation, claim, mechanism, and source lookups.
`scripts/ci/kiva-ci run generators -- --check` runs the common
dependency-ordered generator drift check. Its order is derived by stable
topological sort from the ProjectionRuntime generator plan.
`generated/evidence/evidence-receipts.yaml` is the generated receipt review
plane. `EvidenceReceipt` rows give CI checks stable receipt identities, and
`VerificationReceipt` rows give verification discharge methods stable receipt
identities. Generated rows are declared receipt requirements; executed release
receipts can refine the same ids with command output hashes.
`generated/evidence/executed-receipt-overlay.yaml` is the release-run overlay:
the committed version records no executed run, and release jobs can regenerate
it from run reports to attach SHA-256 hashes. The release validator enforces
that committed overlays remain no-run declarations while release-mode overlays
must carry at least one ReceiptRun and executed receipt.
`generated/architecture/claim-graph.yaml` is the generated claim/evidence
graph joining claim readiness, EvidenceSet rows, receipt indexes, executed
receipt overlays, capability bindings, and verification obligations.
`generated/architecture/assurance-graph.yaml` is the higher-level review graph
over the same evidence model and the primary human review surface: claims,
declared receipts, confidential capabilities, and verification obligations are
presented as one typed node/edge surface while the specialized source
projections remain intact.
`generated/architecture/compatibility-retirement-plane.yaml` is the generated
compatibility review plane: it joins removed/retained compatibility-surface
history, DTO semantic-compatibility retirement, and deletion-readiness gates,
while making clear that `ready-to-delete` is not deletion authorization.
The compatibility ledger is also checked by
`scripts/ci/validate_compatibility_exit_gates.py`, which requires any retained
surface to carry an explicit migration target and executable forbidden-use gate.
For confidential computation, `generated/confidential/mechanism-profile-index.yaml`
is the generated `MechanismProfile` review plane. It joins through
`generated/confidential/capability-binding-index.yaml`, where
`CapabilityBinding` owns mechanism-to-operation affinity. Default
capability-to-workflow affinity is owned by
`spec/confidential/capability-taxonomy.source.yaml`, keeping per-binding rows
thin. The capability taxonomy validator connects this workflow taxonomy to the
provider capability vocabulary and mechanism catalog `required_provider_capability`
rows.
The confidential-computation boundary should be read as a sealed
`CapabilityPort` model: PDS and application workflows bind to capabilities and
governed release artifacts, while reference primitives and external runtimes
pass through the same admission/evidence/runtime/release gate shape before
release partitioning.
`spec/confidential/mechanism-packages.source.yaml` is the compact edit surface
for that boundary. A `MechanismPackage` row joins the maturity mechanism,
CapabilityBinding, provider kind, sealed provider boundary, assurance registry
entry, and verification family. The generated MechanismProfile and
PackageProfile rows plus the Capability/Policy/Evidence index are projections
of those package facets, not separate roots. PackageProfile covers every
MechanismPackage, including packages without a mechanism-catalog descriptor.
`generated/verification/verification-obligations.yaml` is the generated
tool-neutral `VerificationObligation` plane.
`generated/confidential/capability-policy-evidence.yaml` is the confidential
Capability/Policy/Evidence review plane joining capability bindings, provider
boundary policy, maturity, operation affinity, assurance registry entries, and
verification kernels.
`generated/architecture/source-registry.yaml` is the unified generated source
registry for architecture sources, source-layer classification, schema coverage,
and projection roles.
PDS behavior is reviewed through `spec/pds/core-planes.source.yaml`: event,
inventory, policy, and release. Each plane packages the relevant
`spec/pds/invariants.source.yaml` rows through checked `invariant_refs` and the
PDS transition ledgers in `spec/pds/state-machines.source.yaml` through checked
`state_machine_refs`.
The generated `pds-plane-packages.yaml` view resolves those references together
with resource families, implementation anchors, milestones, and CI checks.
These PDS model sources are Operation-layer sources; their evidence is handled
by Assurance.
Consent, lifecycle, governance, approved data products, and provider bindings
are feature families inside those planes under OperationKernel, not new
top-level design roots.

The integrated operation review source is
`spec/architecture/operation-ownership.source.yaml`. It contains sparse
semantic overlays keyed by OpenAPI contract and operation id. HTTP method,
path, and required scopes are projected from OpenAPI, and HTTP-only contracts
such as xAPI can be listed as derived pass-through contracts instead of
maintaining empty ownership rows.
The surface model uses split canonical sources instead of one broad surface
source: the operation overlay, projection review policy, operation algebra,
generation policy, implementation policy, PDS resource-family source, and
cross-surface family source are separately classified in the architecture-policy
ledger. `spec/architecture/surface-projections.yaml` owns the thin review
entrypoint and edit policy. `spec/architecture/surface-model.source.yaml` owns
the `SurfaceOperation` algebra that all operation projections must derive from,
`spec/architecture/surface-generation.policy.yaml` owns generated output and
source-ledger policy, and `spec/architecture/surface-implementation.policy.yaml`
owns route coverage, application-operation, PDS family, and cross-surface
implementation projection policy.
`spec/architecture/operation-kernel.source.yaml` is the higher-level review
entrypoint over those split ledgers. Reviewers should start there and then
follow a referenced fragment only when changing a specific operation, route,
family, or generated-surface policy.
`spec/architecture/pds-resource-families.source.yaml` owns the PDS family
inventory, and `spec/architecture/surface-families.source.yaml` owns
cross-surface family descriptions for xAPI, PDS, and confidential routes.
`scripts/ci/generate_surface_catalog_bundle.py` combines those source ledgers
into `generated/architecture/surface-catalog.yaml`, and
`scripts/ci/validate_surface_catalog.py` checks that the generated bundle
exactly covers the generated HTTP operation catalog and that the algebra's
projection targets match the active generated ledgers.
Application operations that cross the PDS or confidential application boundary
remain recorded in `generated/architecture/operation-catalog.yaml`, the workflow
projection generated from the surface catalog. The architecture boundary check
verifies that each catalog entry links one Rust workflow variant, one stable
workflow label, one OpenAPI `operationId`, and the exact
`x-kiva-required-scopes` value from the OpenAPI contract. Catalog entry IDs,
workflow labels, and contract-local `operationId` values are checked for
uniqueness, and the catalog is mirrored by
`crates/kiva-application/src/operation_catalog/generated/registry.rs` directly
from the generated surface catalog through
the private generated `APPLICATION_OPERATION_SPECS` table, the
`application_operation_specs()` facade, and the generated
`ApplicationOperationId` enum.
Every HTTP operation is also projected from OpenAPI into
`docs/architecture/generated/http-operation-catalog.md` and
`generated/http-operation-catalog.evidence.json` by
`scripts/ci/generate_http_operation_catalog.py`, using the shared OpenAPI
projection helpers in `scripts/ci/kiva_ci/openapi.py`. That generated catalog
is the complete endpoint inventory; `generated/architecture/operation-catalog.yaml`
remains the narrower workflow-operation catalog.
`scripts/ci/generate_surface_operation_index.py` renders the generated surface
bundle from `generated/architecture/surface-catalog.yaml` into
`docs/architecture/generated/surface-operation-index.md` and
`generated/surface-operation-index.evidence.json`. That index is the review
surface for answering whether an endpoint is HTTP-only, enters an application
workflow, belongs to a PDS resource family, or all three.
`scripts/ci/generate_operation_kernel.py` renders
`generated/architecture/operation-kernel.yaml` and
`docs/architecture/generated/operation-kernel.md` as the first-class
`OperationKernel` IR directly from the split surface sources plus the generated
HTTP operation catalog. The kernel joins HTTP contract identity,
`SurfaceOperation` semantics, application workflow ownership, surface-family
assignment, optional PDS resource-family assignment, and xAPI requirement
evidence. The narrower operation, family, route-coverage, and review-index
catalogs remain specialized projections of this kernel.
`scripts/ci/validate_operation_kernel_authority.py` keeps that relationship
executable by checking that HTTP operations, route coverage, and PDS
resource-family assignments all resolve through OperationKernel rows.
`scripts/ci/generate_route_coverage.py` renders the same catalog into
`docs/architecture/generated/route-coverage.md` and
`generated/route-coverage.evidence.json`, checking that every OpenAPI operation
is present in the Axum route files.
The same surface projection step also writes
`crates/kiva-server/src/core/state/generated_surface_profiles.rs`, the generated
route/profile registry used by PDS and confidential route assembly to keep
bounded HTTP contexts aligned with catalog-owned surface labels.
For PDS resource families, `generated/architecture/pds-resource-families.yaml`
projects resource ownership, route context, and implementation-file coverage
from the surface catalog. OpenAPI and the generated HTTP catalog own operation
shape, while the surface catalog owns workflow execution links and route
contexts. The PDS family validator derives operationId, handler, path constant,
schemas, and scope checks from those sources, and it verifies the per-family
server role matrix instead of treating every handler/storage file as independent
design authority.
`generated/architecture/surface-families.yaml` is the sibling cross-surface ledger:
it assigns every xAPI, PDS, and confidential HTTP operation to exactly one
review family by PDS family reference or longest route prefix.

## Layering

The current source of truth is this dependency order:

1. **Domain and contract crates**
   - `crates/kiva-domain` owns canonical server-independent newtypes such as
     `Iri`, `StatementId`, `Actor`, `Activity`, and the compact immutable
     `Statement` value. It is not an HTTP DTO crate and has no Axum, SQLx,
     process-environment, policy, or confidential-runtime dependencies.
   - `crates/kiva-xapi` owns server-independent xAPI admission,
     canonicalization, metadata extraction, statement equivalence, and query
     filtering. `src/lib.rs` stays a thin re-export owner over
     `src/xapi/mod.rs`, which owns
     `src/xapi/{constants,errors,admission,authority,language,filtering,comparison,actor,validation}.rs`.
   - `crates/kiva-pds` owns PDS vocabulary, internal command/event projection,
     validation, normalization, and retained compatibility DTOs that do not
     depend on Axum, SQLx, or runtime state. Its stable boundary files live under
     `src/pds/` behind `src/pds/mod.rs`, including
     `crates/kiva-pds/src/pds/commands.rs` and
     `crates/kiva-pds/src/pds/events.rs`, with resource-family command and
     event bodies under `crates/kiva-pds/src/pds/commands/` and
     `crates/kiva-pds/src/pds/events/`. The command/event model covers
     consent, approved data products, lifecycle events and cases, retention
     policies, and governance profiles.
   - `crates/kiva-release-artifacts` owns normalized confidential
     release-artifact kinds, PDS data-product compatibility source kinds,
     `ReleaseArtifactRef` values, and fixed-width lowercase artifact-id
     parsing shared by PDS and policy.
   - `crates/kiva-identity` owns OAuth/OIDC resource-server scope labels,
     authenticated principals, and typed operation witnesses.
   - `crates/kiva-policy` owns authorization decisions, delegation,
     lifecycle, data-product, and public-sector governance-policy facts split
     under `src/policy/` behind `src/policy/mod.rs`. It re-exports identity
     types for compatibility, but identity remains the owning crate.
  - `crates/kiva-pds-openapi-contract` owns the generated PDS OpenAPI request
     and success-response DTO slice checked by the generated-contract DTO map.
     `crates/kiva-pds` remains the internal semantic boundary for PDS command,
     query, event, and response vocabulary, and its remaining OpenAPI parity
     anchors live in `spec/openapi/semantic-compatibility-map.yaml`;
     `spec/openapi/dto-boundary-ledger.yaml` records that split as
     `external_contract_boundary` plus `internal_semantic_boundaries`.
   - `crates/kiva-xapi-openapi-contract` owns the xAPI OpenAPI DTO contract
     slice used by xAPI Statement-family request parsing and response
     projection before xAPI normalization and HTTP semantic helpers.
   - `crates/kiva-http-core` owns xAPI HTTP version/header, ETag, document,
     and Statement query contracts under `src/http/` behind `src/http/mod.rs`.
     `spec/openapi/dto-boundary-ledger.yaml` records the adopted generated DTO
     boundary and the remaining compatibility-guard retirement target: every
     external HTTP DTO comes from an OpenAPI-generated contract crate, while
     handwritten domain and application types remain internal. Each contract row
     records its generated target, target boundary type, handler-adoption state,
     generated-contract DTO map contracts, and semantic compatibility map
     contracts. `active_guards` names only the active generated-contract and
     semantic compatibility maps, while `retired_archives` records
     `spec/openapi/rust-dto-map.yaml` as a closed migration archive.
     Every public HTTP contract now uses `GeneratedContractBoundary`;
     `CompatibilityHttpBoundary` is retained only as a guarded transitional
     state for future migrations.
     `crates/kiva-http-core` remains the pure xAPI HTTP semantics crate for
     version/header, ETag, document preconditions, and Statement query
     decisions, not the external DTO boundary. `remaining_compatibility_work_queue`
     is empty after generated-boundary adoption.
   - `crates/kiva-confidential-contract` owns confidential-computation HTTP
     request and response DTOs. Confidential handlers parse external payloads
     through that contract crate, convert them into internal request models under
     `crates/kiva-server/src/surfaces/confidential/wire/requests/`, and project
     responses through generated contract DTOs instead of defining wire schema
     inline.
   - `crates/kiva-confidential-core` owns the small confidential boundary
     types needed by pure application workflows, such as confidential job
     status, transcript identifiers, and control/data-plane boundary tags.
   - `crates/kiva-confidential` owns confidential-computation primitive types,
     mechanism gates, job lifecycle, differential privacy, provider contracts,
     descriptor registry, generated mechanism catalog source, and review-gated
     external-runtime boundaries. Provider kinds are split into typed reference,
     custody, and compute-mechanism enums so production runtime helpers cannot
     accept reference fixtures or custody-only providers. The provider-kind
     registry is generated from
     `spec/confidential/secret-compute-provider-contract.yaml` into
     `spec/confidential/secret-compute-provider-kind-registry.yaml` and
     `crates/kiva-secret-provider-contract/src/kind_registry.rs`, with
     `kiva-confidential::mechanism::provider` retaining compatibility
     re-exports for callers.
2. **Application orchestration**
   - `crates/kiva-application` owns pure application use cases that combine
     contract/domain crates without importing HTTP, SQLx, or server state. Its
     PDS boundary coordinates record inventory, metadata export, approved
     data-product registration, consent, lifecycle, retention, and governance
     writes over explicit xAPI, PDS, and release-artifact availability ports.
     `crates/kiva-application/src/runtime.rs` owns
     `ApplicationRuntimePort`, `ApplicationClock`, `ApplicationIdGenerator`,
     `ApplicationWorkflowBoundary`, and `DefaultApplicationRuntime`, giving
     application workflows deterministic time, workflow identifiers, and
     workflow boundary observation without importing server adapters.
     `ApplicationWorkflowAuditEvent` records started, completed, and failed
     boundary outcomes through the runtime port. `Admitted<T, W>` carries
     application-admitted values together with typed admission witnesses before
     those values cross a port. `ApplicationWorkflowContext` is the scoped
     executor value that creates workflow-bound and principal-bound admitted
     values through `admit_workflow` and `admit_principal`.
     `ApplicationOperationKernel` is the module-private operation kernel over the
     workflow lifecycle, `ApplicationOperationGateway` is the public entrypoint
     for operation lifecycle, catalog-backed operation lookup, admission, and
     execution, `ApplicationOperationPlan`
     binds operation metadata to admission policy, and
     `ApplicationAdmissionPolicy` is the small sync contract used by PDS and
     confidential workflows before port crossing.
     Workflow admission uses
     `ApplicationWorkflowSpec`, a small runtime value around a typed
     `ApplicationWorkflowKind` label. PDS and confidential workflows expose
     their own workflow kind enums, while generated metadata from
     `generated/architecture/operation-catalog.yaml` owns their stable evidence
     labels, Rust markers, OpenAPI links, scopes, and confidential workflow
     families. PDS additionally exposes `PdsWorkflowBoundary` so commands,
     queries, and direct-access checks are explicit in runtime code; command
     workflows assert the command boundary and read-model workflows assert the
     query boundary before entering the application gateway.
   - `crates/kiva-server/src/app/` owns server adapter assembly for
     cross-surface use cases that need more than one public surface. `app/pds.rs`
     binds server repositories and services to the pure PDS application ports.
     PDS command writes enter through pure command/event projection before the
     server writes persistent state. The application workflow admits each
     `PdsDomainEvent` through `ApplicationAdmissionPolicy` against the
     authenticated principal using the domain-owned
     `PdsDomainEvent::controlling_subject()` semantic before it reaches the
     adapter. The PDS write port accepts
     `Admitted<PdsDomainEvent, PrincipalBound>` and projects only admitted events into
     `ApplicationWriteOutcome`, with adapter implementation bodies split by
     consent, data-product, lifecycle, and governance resource family. PDS
     surface services are retained for single-surface reads and direct
     access-audit operations.
   - Surface services remain single-surface capability providers. Cross-surface workflows
     should move into `app/` instead of widening a surface service or handler.
3. **Surface services and repositories**
   - `crates/kiva-server/src/surfaces/*/service.rs` exposes narrow
     single-surface services for xAPI and PDS read or direct repository
     use cases. Confidential-computation workflows are assembled through
     `crates/kiva-server/src/app/confidential/` so handlers do not receive a
     direct repository capability. Confidential workflow admission emits one
     admitted domain event carrying `ConfidentialWorkflowFamily` for additive
     secure aggregation, differential privacy, confidential jobs,
     participants, Shamir reconstruction, and threshold release. The
     application workflow creates the workflow-bound admission; the capability
     aggregate holds repository execution and `ConfidentialWorkflowAdmissionRecorder`
     ports separately, and the server adapter no longer constructs `Admitted`
     values while executing repository work.
   - `crates/kiva-server/src/surfaces/*/repository.rs` exposes
     memory/PostgreSQL facades. Backend dispatch stays behind these facades and
     is shared through a small enum-dispatch macro, not trait objects.
4. **Adapters**
   - `crates/kiva-server/src/surfaces/*/routes.rs` owns route registration.
   - `crates/kiva-server/src/surfaces/*/handlers.rs` owns HTTP authentication,
     request parsing, status selection, and application-service calls.
   - `crates/kiva-server/src/surfaces/*/wire.rs` owns JSON response
     projection from internal state into contract DTOs.
   - `crates/kiva-server/src/surfaces/*/storage.rs` owns memory and PostgreSQL
     implementation bodies.
5. **Composition root**
   - `crates/kiva-server/src/lib.rs` builds the router and retains only
     the production composition surface.
   - `core/state.rs` projects `AppState` into bounded HTTP contexts:
     `XapiHttpContext`, `PdsHttpContext`, `PdsApplicationHttpContext`,
     `ConfidentialHttpContext`, and `SystemHttpContext`. Public-surface routes
     and handlers receive these contexts instead of the full `AppState`.
     Pure PDS routes use `PdsHttpContext`; PDS routes that combine xAPI
     statement metadata or confidential release-artifact availability use
     `PdsApplicationHttpContext`.
     Shared authentication, authorization, runtime-limit, and rate-limit
     checks are delegated through `core/request_guards.rs` so context structs
     stay as capability projections rather than duplicating guard logic.
     `core/state/authorization.rs` owns the context-specific authorization
     methods, and those methods return `kiva_identity::AuthorizedPrincipal`
     witnesses produced through `authorize_typed_xapi_request`,
     `authorize_typed_pds_request`, and
     `authorize_typed_confidential_request`.
     PDS and confidential route assembly also checks generated surface-profile
     metadata for each bounded context, keeping server context labels aligned
     with the generated surface catalog.
   - `core/bootstrap.rs` owns process-environment and PostgreSQL repository
     construction for `AppState::from_env()`, keeping `core/state.rs` focused
     on state shape and HTTP-context projection.
   - `crates/kiva-server/src/test_support.rs` owns compatibility re-exports
     for the existing in-crate test harness, keeping test-only aliases out of
     normal server composition.
   - `crates/kiva-server/src/tests/helpers.rs` is only a test helper module
     owner. Concrete helper bodies are split under
     `crates/kiva-server/src/tests/helpers/` by state, confidential workflows,
     PDS workflows, and fixtures.
   - PDS and confidential memory tests keep
     `crates/kiva-server/src/tests/{pds_memory,confidential_memory}.rs` as
     compatibility module owners, with concrete workflow tests split under
     matching directories by resource family and lifecycle concern.

## Server Support Modules

The server keeps cross-cutting concerns under `crates/kiva-server/src/core/`:

- `core/config.rs` contains server-wide constants and profile identifiers.
- `core/state.rs` contains `AppState` shape and context projection. `AppState` is composed from
  `XapiApp`, `PdsApp`, `ApplicationApp`, and shared runtime/authentication
  configuration. `ApplicationApp` holds the owned application handles assembled
  by the composition root, including `PdsApplicationAdapter` and
  `ConfidentialApplicationAdapter`. `AppState` is a composition-root object only;
  route registration calls
  `xapi_http_context()`, `pds_http_context()`,
  `pds_application_http_context()`, `confidential_http_context()`, and
  `system_http_context()` so HTTP surfaces can only access the repositories,
  capabilities, limits, and authentication configuration they need.
- `core/bootstrap.rs` contains environment-backed bootstrap, including
  `DATABASE_URL` handling, tenant selection, PostgreSQL pool construction, and
  memory-store fallback for local conformance smoke tests.
- `core/request_guards.rs` owns shared request guard behavior for runtime
  limits, rate limiting, and OAuth/OIDC scope authorization. HTTP contexts hold
  `RequestGuards` plus their own repository or application adapter, keeping the
  repeated guard logic out of surface handlers while preserving narrow
  capabilities. Typed scope checks are exposed through
  `authorize_typed_xapi_request`, `authorize_typed_pds_request`, and
  `authorize_typed_confidential_request`.
- `core/runtime.rs` contains environment parsing, body limits, and rate
  limiting.
- `core/auth.rs` contains bearer-token fixture authentication and OAuth/OIDC
  resource-server scope checks, projecting accepted credentials into
  `kiva_identity::AuthenticatedPrincipal` and then into
  `kiva_identity::AuthorizedPrincipal` operation witnesses.
- `core/extract.rs` contains typed JSON request extraction.
- `core/request_util.rs` contains shared query/header parsing helpers.
- `core/response.rs` contains only shared HTTP response helpers, version
  headers, date/ETag/hex utilities, the generic `HttpError`, and
  `json_projection_response` for fail-closed conversion of fallible wire DTO
  projection into backend error responses.
- `core/outcome.rs` contains only shared backend-neutral outcomes such as
  `StoreOutcome` and `InsertOutcome`.
- `core/paths.rs` contains route constants for Kiva-specific extension paths.

Surface-local modules live under `crates/kiva-server/src/surfaces/`:

- `surfaces/xapi/` contains the IEEE xAPI LRS route, handler, service,
  repository, storage, wire, response, outcome, model, and request parsing
  boundary. `surfaces/xapi/handlers.rs` owns resource-family handler modules
  under `surfaces/xapi/handlers/`; `surfaces/xapi/request.rs` owns parser,
  query, and projection helper modules under `surfaces/xapi/request/`. Its
  routes and handlers are typed with `XapiHttpContext`.
- `surfaces/pds/` contains the PDS route, handler, service, repository,
  storage, wire, response, outcome, model, and release-artifact reference
  normalization boundary. PDS workflows that need xAPI or confidential state
  are owned by `app/pds.rs`. Pure PDS routes and handlers are typed with
  `PdsHttpContext`; cross-surface PDS application routes are typed with
  `PdsApplicationHttpContext`.
  The provider-binding registry is a pure PDS metadata surface: OpenAPI
  request DTOs become `ProviderBindingRegistrationCommandInput`, the service
  admits them through the confidential provider contract, and memory/PostgreSQL
  storage retains admitted registry metadata under `pds_provider_bindings`.
  `surfaces/pds/repository.rs` is a dispatch owner over
  `surfaces/pds/repository/`, and `surfaces/pds/storage/postgres.rs` is a
  backend owner over `surfaces/pds/storage/postgres/`. Mutating PDS service
  methods construct `kiva-pds` command types and admitted domain events before
  calling the repository facade. Repository methods turn those admitted events
  into PDS projection values, and storage applies the projections to durable
  state plus the lifecycle ledger. Provider-binding storage persists only
  control-plane metadata, evidence gates, release gates, runtime-class labels,
  and digest bindings, not provider secrets or cryptographic payloads.
- `surfaces/confidential/` contains the confidential-computation route,
  handler, service, repository, storage, wire, response, outcome, and model
  boundary. Its routes and handlers are typed with `ConfidentialHttpContext`.
  `surfaces/confidential/wire.rs` and
  `surfaces/confidential/response.rs` are module owners over
  `surfaces/confidential/wire/` and `surfaces/confidential/response/`,
  where projection and error mapping bodies are split by additive,
  differential-privacy, job, participant, Shamir, and threshold resource
  family.
- `surfaces/xapi/storage.rs` is a storage module owner over separate Statement
  memory, Statement PostgreSQL, Statement SQL helper, document memory, document
  PostgreSQL, and document mapping modules.
- `surfaces/xapi/request.rs` is a request module owner over duplicate-key JSON
  parsing, Statement/document content parsing, multipart attachment parsing,
  Statement normalization, Person projection, query validation, and authority/
  response-format projection. Statement shape validation lives in
  `crates/kiva-xapi/src/xapi/validation.rs` so it can be reviewed and tested
  as a pure xAPI boundary rather than an HTTP adapter concern.
- `surfaces/pds/storage.rs` is a storage module owner over separate PDS memory,
  PostgreSQL, PostgreSQL query, row-shape, and storage mapping modules. The
  memory repository holds resource-family stores such as
  `MemoryPdsConsentStore`, `MemoryPdsDataProductStore`, `MemoryPdsAccessStore`,
  `MemoryPdsLifecycleStore`, and `MemoryPdsGovernanceStore` instead of one flat
  set of lock maps.
- `surfaces/pds/handlers.rs` is a handler module owner over consent,
  data-product, access, lifecycle, and governance resource-family handlers.
- `surfaces/confidential/handlers.rs` is a small handler module owner; concrete
  handler bodies are split by confidential resource family under
  `surfaces/confidential/handlers/`. Shared endpoint helpers in the module
  owner perform rate-limit checks, typed OAuth/OIDC scope authorization, and
  JSON body parsing so resource-family handlers focus on the selected use case
  and response mapping.
- `surfaces/confidential/model.rs` and `repository.rs` are small module
  owners; request/filter/state/key/validation model code and resource-family
  repository methods live under matching `surfaces/confidential/{model,repository}/`
  directories. Server adapter methods for confidential application workflows
  live under `app/confidential/`.
- `app/confidential.rs` owns `ConfidentialApplicationAdapter`, the composition
  handle used by `ConfidentialHttpContext` to expose confidential services
  without giving handlers a direct repository field.
- `surfaces/confidential/storage.rs` is a storage module owner.
  `surfaces/confidential/storage/memory.rs` and
  `surfaces/confidential/storage/postgres.rs` own backend-specific
  resource-family bodies under `surfaces/confidential/storage/{memory,postgres}/`.
  The memory repository holds resource-family stores such as
  `MemoryAdditiveStore`, `MemoryDifferentialPrivacyStore`, `MemoryJobStore`,
  `MemoryParticipantStore`, `MemoryShamirStore`, and `MemoryThresholdStore`
  instead of one flat set of lock maps. Common validation and typed PostgreSQL
  row shapes stay as sibling review units without changing repository facade
  APIs.
- `surfaces/pds/model.rs` is a model module owner over state definitions, query
  filters, request construction, and the PDS-to-policy adapter boundary under
  `surfaces/pds/model/`. Server-local PDS state is split by resource family
  under `surfaces/pds/model/state/`, with `state.rs` only owning module
  declarations and compatibility re-exports.
- `surfaces/confidential/model/state.rs` is a state module owner over
  confidential server-local state split under
  `surfaces/confidential/model/state/` by additive, differential privacy, jobs,
  keys, participants, Shamir, and threshold release resource family.
- `crates/kiva-application/src/pds/model.rs` is a module owner over PDS
  application records, snapshots, and errors under
  `crates/kiva-application/src/pds/model/`.
- `crates/kiva-application/src/pds/capabilities.rs` is a module owner over
  `capabilities/{ports,projection,records}.rs`. It keeps
  `PdsApplicationCapabilities`, `PdsApplicationPorts`, and
  `PdsApplicationPortSet` separate from subject projection and record-port
  traits.
  `app/pds/server_adapters.rs` owns server adapter composition. Concrete PDS
  application port handles are split under `app/pds/server_adapters/` by xAPI
  metadata reads, PDS record access, subject projection, release-artifact
  availability, and server-state record projection.
  `app/pds/server_adapters/records.rs` is the PDS application record projection
  file that imports server-local PDS/xAPI/confidential state to convert it
  into application-local records and errors. Inventory and metadata export
  receive a subject-scoped `PdsSubjectSnapshot`, with PDS resource-family
  records grouped as `PdsMetadataRecords` before final projection.
  `PdsSubjectProjectionPort` owns that read-side snapshot assembly, while
  `PdsSubjectIdentityMapper` owns the subject-to-actor-IFI mapping and
  delegates the canonical account IFI construction to
  `kiva_pds::PdsSubjectAccountNamespace`. `PdsSubjectId` marks the application
  subject boundary before wire DTO projection.
  `surfaces/pds/wire.rs` is a module owner over `surfaces/pds/wire/`; those
  family modules project application records and server-local state through
  shared projection traits, and inventory/export use `RecordInventoryView` and
  `MetadataExportView` before DTO serialization.
  `PdsApplicationAdapter` wraps the server-local `PdsApplicationCapabilities`
  and exposes `PdsApplication::new` only through HTTP context projection, so
  application workflow construction is driven by the composition layer. The
  application workflow sees
  `PdsApplicationPorts`, which contain `XapiStatementReadPort`,
  `PdsRecordPort`, and `ReleaseArtifactAvailabilityPort`, instead of direct
  repository access.
  Composition code uses `PdsApplicationAdapter::from_server_adapters` as the
  only server-facing adapter assembly helper; tests and future application workflows can
  still pass explicit ports through `PdsApplicationCapabilities::new`.
- `crates/kiva-application/src/pds/commands.rs` and
  `crates/kiva-application/src/pds/queries.rs` own the public PDS application
  command/query entry points. `crates/kiva-application/src/pds/workflow.rs`
  owns the per-invocation `PdsApplicationWorkflow` boundary, while
  `crates/kiva-application/src/pds/workflow/commands.rs` and
  `crates/kiva-application/src/pds/workflow/queries.rs` hold focused command
  and query review units. Command workflows begin through
  `ApplicationRuntimePort`, so PDS command writes obtain their timestamp,
  workflow ID, and workflow boundary marker through `DefaultApplicationRuntime`
  or an injected test runtime.
- `crates/kiva-application/src/pds/capabilities/projection.rs` owns
  `PdsSubjectProjectionPort`, which projects subject-scoped xAPI statement
  metadata before PDS wire mapping so PDS inventory/export paths do not pass
  raw Statement JSON through the response layer. `PdsSubjectIdentityMapper`
  is the only component that maps an authenticated PDS subject to the xAPI
  actor IFI used by `XapiStatementReadPort::list_metadata_by_actor_ifi`; the
  canonical account IFI format is centralized in `kiva-pds` through
  `PdsSubjectAccountNamespace`.
  The projection is backed by `StoredStatementMetadata` and returns a
  `PdsSubjectSnapshot`.
- `crates/kiva-release-artifacts/src/lib.rs` owns normalized
  `ReleaseArtifactRef` values, lowercase 32-byte artifact identifier parsing,
  and compatibility mapping from legacy data-product source kinds.
  `crates/kiva-pds/src/pds/release_artifacts.rs` adapts PDS data-product
  request validation into those normalized references.
  `surfaces/pds/release_artifacts.rs` only adapts the normalized PDS reference
  into policy authorization facts. `app/pds/server_adapters/release_artifacts.rs`
  owns the availability port that asks confidential repositories whether those
  artifacts are admissible for PDS approved data products.

## Concept Ownership

The implementation keeps each concept owned by one contract or adapter layer:

| Concept | Owning boundary | Notes |
| --- | --- | --- |
| xAPI HTTP contract | `spec/openapi/kiva-xapi.yaml`, `crates/kiva-xapi-openapi-contract`, `crates/kiva-http-core`, `crates/kiva-server/src/surfaces/xapi/{routes,handlers,request,wire,response}.rs`, `spec/openapi/dto-boundary-ledger.yaml` | OpenAPI owns the external transport shape. `crates/kiva-xapi-openapi-contract` is the generated DTO boundary for Statement-family request parsing and response projection; `crates/kiva-http-core` owns pure HTTP semantics such as version/header, ETag, document preconditions, and Statement query decisions. |
| HTTP surface contexts | `crates/kiva-server/src/core/state.rs`, `crates/kiva-server/src/core/state/http_context.rs` | `AppState` stays at the composition root and is projected into `XapiHttpContext`, `PdsHttpContext`, `PdsApplicationHttpContext`, `ConfidentialHttpContext`, and `SystemHttpContext` before route registration. |
| HTTP request guards | `crates/kiva-server/src/core/request_guards.rs`, `crates/kiva-server/src/core/state/authorization.rs`, `crates/kiva-server/src/core/auth.rs` | Shared rate-limit, runtime-limit, and OAuth/OIDC scope checks are delegated from bounded contexts without giving handlers full `AppState`; typed authorization returns `AuthorizedPrincipal` witnesses through `authorize_typed_xapi_request`, `authorize_typed_pds_request`, and `authorize_typed_confidential_request`. |
| Application runtime | `crates/kiva-application/src/runtime.rs` | `ApplicationRuntimePort` combines `ApplicationClock`, `ApplicationIdGenerator`, and `ApplicationWorkflowBoundary`; `DefaultApplicationRuntime` is the production in-process runtime while tests can inject deterministic ports. |
| xAPI semantics | `crates/kiva-xapi` | Admission, canonicalization, metadata extraction, statement equivalence, and query filtering stay server-independent. |
| Canonical xAPI values | `crates/kiva-domain` | Durable value objects and canonical newtypes stay free of HTTP, SQLx, policy, and confidential-computation dependencies. |
| PDS vocabulary and DTOs | `crates/kiva-pds`, `crates/kiva-pds-openapi-contract`, `spec/openapi/kiva-pds.yaml` | OpenAPI remains the HTTP contract source of truth. The active generated slice parses PDS request bodies through `PdsOpenApi*Request` DTOs; `surfaces/pds/wire/openapi.rs` converts those wire contracts into internal `kiva-pds` domain `*CommandInput` or `*Intent` types. Legacy `*Request` aliases remain only as deprecated compatibility names. |
| PDS server state and persistence | `crates/kiva-server/src/surfaces/pds/{model,storage,repository,service}.rs`, `crates/kiva-server/src/surfaces/pds/model/state/`, `crates/kiva-server/src/surfaces/pds/storage/{rows,postgres_queries}/`, `db/migrations/20260619270000_pds_ledger_events.sql`, `db/migrations/20260619280000_pds_provider_bindings.sql` | Server-local state is split by resource family, while query filters, memory/PostgreSQL persistence, lifecycle-ledger projection, provider-binding registry metadata, and direct access-audit checks stay in the PDS surface. PostgreSQL write paths open a transaction and call `_in_transaction` helpers split by resource family so resource mutations, access-audit rows, and lifecycle-ledger rows commit atomically. Provider-binding writes are metadata-only admissions through the confidential provider contract; approved data products store normalized `kiva_release_artifacts::ReleaseArtifactRef` values and expose legacy `source_kind`/`release_id` fields only as compatibility projections. |
| PDS cross-surface application workflows | `crates/kiva-application/src/pds.rs`, `crates/kiva-application/src/pds/{commands,queries,capabilities,model,workflow}.rs`, `crates/kiva-application/src/pds/{capabilities,model,workflow}/`, `crates/kiva-server/src/app/pds.rs`, `crates/kiva-server/src/app/pds/server_adapters/`, `crates/kiva-pds/src/pds/{commands,events}.rs`, `crates/kiva-pds/src/pds/{commands,events}/`, `crates/kiva-server/src/surfaces/pds/{model,repository}/` | Inventory, metadata export, approved data-product registration, consent, lifecycle, retention, and governance writes combine xAPI `StoredStatementMetadata`, PDS state, and confidential release availability through `PdsApplicationPorts`. The port aggregate contains concrete server adapters for `XapiStatementReader`, separate `PdsRecordReadPort` and `PdsRecordWritePort` capabilities, `PdsSubjectProjectionPort`, and `ReleaseArtifactAvailability`; `PdsSubjectIdentityMapper` owns the subject-to-actor-IFI mapping. Inventory/export read a `PdsSubjectSnapshot` keyed by `PdsSubjectId`; PDS metadata remains grouped as `PdsMetadataRecords`. Adapter projection converts server-local state into application-local records so pure use cases do not import `crate::surfaces::*`. PDS writes are admitted through pure command/event types, repository facades create projection values, and storage applies those projections to durable state plus lifecycle-ledger rows. The composition root assembles an owned `PdsApplicationAdapter` before HTTP context projection. |
| PDS provider-binding registry | `spec/openapi/kiva-pds.yaml`, `crates/kiva-pds/src/pds/{requests,responses}.rs`, `crates/kiva-server/src/surfaces/pds/handlers/provider_bindings.rs`, `crates/kiva-server/src/surfaces/pds/service/provider_bindings.rs`, `crates/kiva-server/src/surfaces/pds/{repository,wire}/provider_bindings.rs`, `crates/kiva-server/src/surfaces/pds/storage/postgres/provider_bindings.rs`, `db/migrations/20260619280000_pds_provider_bindings.sql` | The registry is a metadata-only PDS control plane for admitted secret/confidential-compute provider bindings. It validates OpenAPI request DTOs, admits them through `kiva_confidential::mechanism::provider`, persists digest and gate metadata, and rejects unsupported or unreviewed production claims. It must not store provider secrets, key shares, ciphertext payloads, signatures, signed envelopes, or runtime-specific opaque payloads. |
| PDS-policy translation | `crates/kiva-server/src/surfaces/pds/model/policy_adapter.rs`, `crates/kiva-policy` | The adapter maps PDS vocabulary into policy roles, resources, actions, consent delegations, lifecycle status, and governance evidence. |
| PDS response projection | `crates/kiva-server/src/surfaces/pds/wire.rs`, `crates/kiva-server/src/surfaces/pds/wire/` | Wire projection uses typed DTOs from `kiva-pds`; it must not perform storage access or policy decisions. Surface state and application records share projection traits so DTO construction remains single-sourced, and inventory/export use dedicated view models before serialization. Fallible DTO serialization returns `Result<Value, serde_json::Error>` and is converted by `json_projection_response` rather than falling back to `null`. |
| Confidential boundary types | `crates/kiva-confidential-core` | Pure application workflows depend on small boundary types such as `SecureAggregateTranscriptId` and `ConfidentialJobStatus`, not on confidential mechanism implementations. |
| Confidential primitive and mechanism contracts | `crates/kiva-secret-provider-contract`, `crates/kiva-confidential`, `crates/kiva-confidential/src/mechanism/registry.rs`, `crates/kiva-confidential/src/mechanism/provider.rs`, `crates/kiva-confidential/src/mechanism/provider/` | Provider kind/capability/profile vocabulary lives in the runtime-independent contract crate. Primitive wrappers, additive aggregation, Shamir, PSI, TEE, FROST, FHE, differential privacy, job lifecycle mechanics, provider admission/runtime-boundary evidence, provider capability/contract profile traits, and `CONFIDENTIAL_MECHANISM_REGISTRY` live in `kiva-confidential`. `provider.rs` is the public compatibility owner; `provider/{binding,admission,contract,digest,runtime}.rs` hold the focused provider implementation units. |
| Confidential application workflows | `crates/kiva-application/src/confidential.rs`, `crates/kiva-application/src/confidential/`, `crates/kiva-server/src/app/confidential.rs`, `crates/kiva-server/src/app/confidential/`, `crates/kiva-server/src/core/state.rs`, `crates/kiva-server/src/core/state/http_context.rs`, `crates/kiva-server/src/surfaces/confidential/handlers.rs` | `ConfidentialApplicationAdapter` is assembled by the composition root and wraps the pure `ConfidentialApplicationCapabilities` handle projected into `ConfidentialHttpContext`. Confidential handlers enter resource-family workflow methods for additive aggregation, differential privacy, jobs, participants, Shamir, and threshold release operations through application-layer port traits after shared endpoint helpers perform rate-limit, typed scope authorization, and JSON parsing. The server app modules implement repository adapters behind `ConfidentialWorkflowPort`; application code owns workflow-bound admission, admission-ledger recording, and repository execution through the shared `ApplicationOperationGateway` lifecycle, while repository adapters only record or accept admitted outcomes. |
| Confidential HTTP DTO contract | `spec/openapi/kiva-confidential.yaml`, `crates/kiva-confidential-contract` | Response DTO definitions are checked against the OpenAPI shape map. |
| Confidential HTTP projection | `crates/kiva-server/src/surfaces/confidential/wire.rs`, `crates/kiva-server/src/surfaces/confidential/wire/`, `crates/kiva-server/src/surfaces/confidential/response.rs`, `crates/kiva-server/src/surfaces/confidential/response/` | Server-local projection maps confidential state into the contract DTOs; it must not own the DTO schema. Projection and error-response bodies are split by resource family and fail closed through `json_projection_response`. |
| Confidential server state and commands | `crates/kiva-server/src/surfaces/confidential/{model,repository,storage}.rs`, `crates/kiva-server/src/surfaces/confidential/model/state/` | Server-local state is split by resource family, application workflow modules own use-case orchestration, repository facades own persistence dispatch, and PostgreSQL command modules own transactional scripts. |
| Release artifact admission | `crates/kiva-release-artifacts/src/lib.rs`, `crates/kiva-pds/src/pds/release_artifacts.rs`, `crates/kiva-server/src/app/pds/server_adapters/release_artifacts.rs`, `crates/kiva-server/src/surfaces/pds/release_artifacts.rs`, confidential metadata repositories | PDS data products reference metadata-only confidential release artifacts through `ReleaseArtifactAvailabilityPort` and normalized `ReleaseArtifactRef` values owned by the release-artifact boundary crate. |
| Evidence and assurance | `spec/confidential/`, `spec/verification/`, `generated/evidence/evidence-set.yaml`, `generated/evidence/evidence-receipts.yaml`, `generated/confidential/capability-binding-index.yaml`, `generated/confidential/mechanism-profile-index.yaml`, `generated/verification/verification-obligations.yaml`, `spec/ci/evidence-checks.yaml`, `spec/architecture/architecture-graph.source.yaml`, `spec/architecture/design-backbone.yaml`, `spec/ci/projection-graph.yaml`, `proofs/`, `scripts/ci/`, `docs/operations/` | Machine-readable evidence, the CI evidence-check manifest, architecture graph source, generated review projections, proof artifacts, validators, and runbooks support claim boundaries without expanding runtime ownership. `CapabilityBinding` owns confidential mechanism-to-operation affinity, `MechanismProfile` joins confidential mechanism/provider/capability evidence, `VerificationObligation` treats Kani, Tamarin, F*, Rust tests, and anchors as discharge methods for boundary properties, and receipt indexes bind those rows to stable evidence identities. |

## Confidential Crate Structure

`crates/kiva-confidential-core` contains the small boundary types that pure
application workflows need without importing confidential mechanism
implementations.

`crates/kiva-confidential/src/lib.rs` is a thin compatibility layer. It exports
the `mechanism` and `primitive` modules, re-exports fixed-width primitive
wrappers for existing callers, and keeps only `SecureAggregateError` at the
crate root. Mechanism types, plans, gates, and external-runtime boundaries are
accessed through explicit `kiva_confidential::mechanism::{...}` namespaces
instead of crate-root wildcard exposure.
The implementation is split as follows:

- `primitive.rs` owns fixed-width primitive wrappers and constants such as
  `SecureAggregateFieldElement`, `SecureAggregateTranscriptId`,
  `SecureAggregateCommitment`, signing-key identifiers, signatures, transport
  proofs, and envelope sequences.
- `mechanism::job` owns confidential job admission and status transitions.
- `mechanism::differential_privacy` owns privacy budget ledgers, composition,
  noise, policy, and release gates.
- `mechanism::provider` owns the `SecretComputeProviderContract` admission
  object, secret/confidential-compute provider bindings, capability matching,
  parameter-set evidence gates, normalized `SecretComputeProviderAdmission`
  evidence, `SecretComputeProviderContractPipeline`, and provider
  runtime-boundary claims.
- `mechanism::registry` owns `CONFIDENTIAL_MECHANISM_REGISTRY`, the descriptor
  table that maps each `ConfidentialMechanismKind` to its technique,
  `SecretComputeProviderCapability`, all-participant policy expectation, and
  production provider-review gate.
- `mechanism::{additive,shamir,threshold,workflow,psi,tee,frost,fhe}` expose stable
  mechanism-oriented namespaces.
- `mechanism::core` is the implementation owner for the remaining coupled
  mechanisms. The former `mechanism::integrated` public compatibility
  namespace has been removed; its retirement is tracked as compatibility
  history.
  `mechanism/core.rs` is the physical core review unit over
  `mechanism/core/{kernel,profiles,lifecycle,additive,shamir,psi,tee,frost,fhe}.rs`.
  `mechanism/core/kernel.rs` names the pure `ConfidentialKernelBoundary` rows
  covered by the verified-kernel ledger; provider adapters, HTTP handlers,
  storage, and external execution evidence remain outside that pure kernel
  boundary.
  Additive secure aggregation is split under
  `mechanism/core/additive/` by envelope, replay, plan, and combine
  logic. Lifecycle/workflow state is split under
  `mechanism/core/lifecycle/` by key lifecycle, participant
  lifecycle, threshold-release approval, and mechanism plan.
  FHE and TEE have additional review units under
  `mechanism/core/fhe/{feasibility,runtime}.rs` and
  `mechanism/core/tee/{portable,nitro}.rs`. FROST is split under
  `mechanism/core/frost/`, and PSI is split under
  `mechanism/core/psi/`. Integrated profile identifiers are split
  by mechanism under `mechanism/core/profiles/`; mechanism enums,
  mechanism-specific boundary errors, and the compatibility
  `SecureAggregateError` facade live in `profiles/mechanisms.rs`,
  `profiles/errors.rs`, and `profiles/errors/`. The legacy error name is
  exported from `profiles/errors/compat.rs`; mechanism-specific typed errors
  remain the primary review units.
  Behavioral tests and Kani harnesses are split by mechanism under
  `mechanism/core/tests/` and
  `mechanism/verification/kani_harnesses/`.

Compatibility surfaces that remain for existing callers and evidence references
are tracked in `spec/architecture/compatibility-surfaces.yaml`; new root
re-exports require an explicit ledger entry and migration target.

## Design Constraints

- Keep xAPI, PDS, and confidential-computation surfaces in one server until a
  deployment constraint requires a split.
- Keep OpenAPI and generated schemas as the external HTTP contract source of
  truth.
- Keep domain crates free of Axum, SQLx, and process environment concerns.
- Keep handlers thin: authentication, parsing, status selection, and delegation.
- Keep HTTP routes and handlers typed with their surface-specific context rather
  than `AppState`.
- Keep pure PDS handlers on `PdsHttpContext`; use `PdsApplicationHttpContext`
  only for PDS routes that invoke cross-surface application workflows.
- Keep PDS route ownership explicit in `spec/architecture/boundaries.yaml` so
  pure PDS routes and cross-surface PDS application routes cannot silently swap
  HTTP context.
- Keep application services free of HTTP types.
- Keep source-owner size, selected wildcard-import boundaries, Cargo
  dependency rules, and module-declaration rules enforced by
  `spec/architecture/boundaries.yaml`.
- Keep milestone status in machine-readable ledgers such as
  `spec/pds/publication-milestones.yaml` and
  `spec/confidential/computation-milestones.yaml`; docs should remain short
  guides to those ledgers.
- Keep single-surface services scoped to their own repositories; put
  cross-surface orchestration in `crates/kiva-server/src/app/`.
- Keep wire mapping free of storage access and authorization decisions.
  Fallible wire DTO serialization should return `Result<Value,
  serde_json::Error>` and be handled through `json_projection_response`, not
  by silently emitting JSON `null`.
- Keep repository facades responsible for backend dispatch.
- Keep surface-specific outcomes and error-to-response mappings in the owning
  surface, not in `core`.
- Move code out of a root module only when the target module can own the
  dependency boundary without widening behavior.

## Claim Boundary

This design supports an IEEE/ISO/IEC 39274-1-1:2025-aligned xAPI LRS
implementation claim, plus Kiva-specific PDS and confidential-computation
engineering assurance claims. It does not assert IEEE certification, production
MPC security, production FHE security, or hardware-backed TEE certification
without external conformance and security review evidence.
