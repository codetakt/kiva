# Server Modular Boundaries

Kiva keeps the server as a strict modular monolith. The goal is to preserve one
deployable server while keeping HTTP, application workflows, wire contracts,
repository dispatch, and release evidence separate enough to review and verify.
This document is the human-readable boundary inventory that mirrors the
machine-readable manifests; broader design rationale stays in
`docs/architecture/system-design.md`.
The machine-readable drift guard for these boundaries is
`spec/architecture/boundaries.yaml`, which is validated by
`scripts/ci/validate_architecture_boundaries.py`.
The integrated surface review catalog is
generated as `generated/architecture/surface-catalog.yaml` from
`spec/architecture/operation-ownership.source.yaml`,
`spec/architecture/surface-projections.yaml`,
`spec/architecture/surface-model.source.yaml`,
`spec/architecture/surface-generation.policy.yaml`,
`spec/architecture/surface-implementation.policy.yaml`,
`spec/architecture/pds-resource-families.source.yaml`, and
`spec/architecture/surface-families.source.yaml`. The source ledgers own
canonical per-operation entries, the operation algebra, generated-output policy,
implementation projection policy, PDS route contexts, PDS handler-family
ownership, and cross-surface family ownership.
`scripts/ci/generate_surface_catalog_projections.py` projects the narrower
application-operation, PDS family, and cross-surface family ledgers, while
`scripts/ci/validate_surface_catalog.py` and
`scripts/ci/generate_surface_operation_index.py` check and render the catalog.
Application workflow labels, OpenAPI `operationId` values, and OAuth/OIDC
resource-server scopes are projected through
`generated/architecture/operation-catalog.yaml` from the surface catalog and checked by the same boundary
validator. The projection also enforces unique entry IDs, unique workflow
labels, unique contract-local `operationId` links, and equality with the Rust
`APPLICATION_OPERATION_SPECS` registry under
`crates/kiva-application/src/operation_catalog/generated/registry.rs`. The Rust
projection carries private generated operation specs plus
`ApplicationOperationId`, `application_operation_specs()`, and lookup methods,
making operation entries a first-class application concept instead of a
parallel string table. The generator also writes
`docs/architecture/generated/application-operation-catalog.md` and
`generated/application-operation-catalog.evidence.json` for review and release
evidence.
Public implementation claims are separately tracked in
`spec/architecture/claim-catalog.yaml`, with generated review evidence in
`docs/architecture/generated/claim-catalog.md` and
`generated/claim-catalog.evidence.json`.

## Current Module Boundaries

- `crates/kiva-server/src/lib.rs` is the composition root. It owns router
  assembly while normal implementation bodies live in `core/` and `surfaces/`.
  Compatibility aliases for the extracted server test harness live in
  `crates/kiva-server/src/test_support.rs`.
  Test helper compatibility imports are owned by
  `crates/kiva-server/src/tests/helpers.rs`, with helper bodies split under
  `crates/kiva-server/src/tests/helpers/` by state, confidential workflows,
  PDS workflows, and fixtures.
- `crates/kiva-server/src/core/config.rs` owns server-wide constants and profile
  identifiers.
- `crates/kiva-server/src/core/state.rs` owns `AppState` shape and
  surface-context projection.
  `AppState` is composed from `XapiApp`, `PdsApp`, `ApplicationApp`, and
  shared runtime/authentication configuration. `ApplicationApp` owns
  `PdsApplicationAdapter` and `ConfidentialApplicationAdapter` handles assembled by
  the composition root. It is not the handler state type for public surfaces:
  route assembly projects it into `XapiHttpContext`, `PdsHttpContext`,
  `PdsApplicationHttpContext`, `ConfidentialHttpContext`, and
  `SystemHttpContext` before calling surface-local routers.
  `crates/kiva-server/src/core/state/authorization.rs` owns typed
  context-specific authorization methods. Those methods return
  `kiva_identity::AuthorizedPrincipal` witnesses instead of exposing raw
  resource-operation enums to handlers.
  `crates/kiva-server/src/core/state/generated_surface_profiles.rs` is generated
  from the surface catalog and owns route/profile labels for the bounded PDS
  repository, PDS application, and confidential contexts.
  `crates/kiva-server/src/core/state/surface_operation.rs` binds those generated
  labels to typed surface-operation markers; route assembly checks the generated
  labels against the intended route context before attaching state.
- `crates/kiva-server/src/core/request_guards.rs` owns reusable request guard
  behavior for runtime limits, rate limiting, and OAuth/OIDC resource-server
  authorization. Each bounded HTTP context delegates guard checks through this
  helper while exposing only its own repositories or application capabilities.
  Scope checks are reached through `authorize_typed_xapi_request`,
  `authorize_typed_pds_request`, and
  `authorize_typed_confidential_request`.
- `crates/kiva-server/src/core/bootstrap.rs` owns environment-backed
  `AppState::from_env()` construction, including PostgreSQL repository
  assembly and memory-store fallback.
- `crates/kiva-server/src/core/auth.rs` owns bearer-token fixture
  authentication and OAuth/OIDC resource-server scope checks, projecting
  accepted credentials into `kiva_identity::AuthenticatedPrincipal` and typed
  `kiva_identity::AuthorizedPrincipal` operation witnesses.
- `crates/kiva-server/src/core/runtime.rs` owns environment parsing, request body
  limits, and the in-process conformance rate limiter.
- `crates/kiva-server/src/core/extract.rs` owns typed HTTP request extraction
  helpers. Confidential-computation and PDS JSON request parsing should use
  this helper so handlers do not hand-roll identical `serde_json` error paths.
- `crates/kiva-server/src/core/request_util.rs` owns shared query/header parsing
  helpers used across xAPI and confidential-computation endpoints.
- `crates/kiva-server/src/core/response.rs` owns common HTTP response helpers,
  xAPI version/header construction, date/ETag/hex utilities, and the generic
  `HttpError`. It also owns `json_projection_response`, the shared fail-closed
  adapter from fallible DTO projection to HTTP backend errors.
  Surface-specific error-to-response mapping stays in the owning surface.
- `crates/kiva-server/src/core/outcome.rs` owns only backend-neutral shared
  outcomes such as `StoreOutcome` and `InsertOutcome`.
- `crates/kiva-server/src/core/paths.rs` owns route constants for Kiva-specific
  PDS and confidential-computation extensions.
- `crates/kiva-application` owns pure PDS application orchestration for
  inventory, metadata export, approved data-product registration, consent,
  lifecycle, retention, and governance workflows that need xAPI statement
  metadata or confidential release-artifact availability in addition to PDS
  repository state. Application records, snapshots, errors,
  `PdsApplicationCapabilities`, `PdsApplicationPorts`,
  `PdsApplicationPortSet`, `PdsSubjectIdentityMapper`,
  `PdsSubjectProjectionPort`, and `PdsApplicationWorkflow` live under
  `crates/kiva-application/src/pds/`.
  `crates/kiva-application/src/runtime.rs` owns `ApplicationRuntimePort`,
  `ApplicationClock`, `ApplicationIdGenerator`, `ApplicationWorkflowBoundary`,
  and `DefaultApplicationRuntime`, keeping application timestamps, workflow
  IDs, and workflow boundary observation injectable and server-independent.
  `ApplicationWorkflowAuditEvent` and `ApplicationWorkflowAuditStatus` cover
  started, completed, and failed boundary outcomes through
  `complete_workflow_boundary` and `fail_workflow_boundary`. `Admitted<T, W>`
  carries values that have crossed application admission before reaching a
  port; `PrincipalBound` and `WorkflowBound` witnesses record the principal or
  workflow run used for admission. `ApplicationWorkflowContext` is passed to
  operations inside the executor and creates those admitted values through
  context methods. `ApplicationOperationKernel` is the module-private operation
  lifecycle kernel. `ApplicationOperationGateway`,
  `ApplicationOperationPlan`, and
  `ApplicationAdmissionPolicy` are the shared operation kernel used by PDS and
  confidential workflows before they cross repository ports. Runtime boundary
  calls receive `ApplicationWorkflowSpec` values that carry stable labels only;
  PDS and confidential workflow modules map domain-specific workflow enums to
  stable evidence labels while Rust workflow markers remain in generated
  operation-catalog evidence.
- `crates/kiva-server/src/app/` owns cross-surface server adapter assembly.
  `app/pds.rs` exposes `PdsApplicationAdapter` as the server-local PDS
  application composition handle; `app/pds/server_adapters.rs` owns
  adapter composition; concrete `XapiStatementReadPort`, `PdsRecordPort`,
  `ReleaseArtifactAvailabilityPort`, server repository/service bindings, and
  record projection live under `app/pds/server_adapters/`. The
  `crates/kiva-server/src/app/pds/server_adapters/records.rs` review unit owns
  server-state to application record/error projection. Metadata export uses
  grouped `PdsMetadataRecords` on `PdsRecordPort` before the subject snapshot
  is projected into inventory or export results. `PdsSubjectId` marks the
  application subject boundary before wire DTO projection.
  PDS write intent is admitted through server-independent
  `crates/kiva-pds/src/pds/commands.rs` and
  `crates/kiva-pds/src/pds/events.rs` before a persistence adapter is called;
  concrete command and event bodies are split under
  `crates/kiva-pds/src/pds/commands/` and
  `crates/kiva-pds/src/pds/events/`. PDS application command writes use the
  same command/event admission boundary for consent, lifecycle, retention, and
  governance intent projection. The application layer checks that each
  admitted `PdsDomainEvent` belongs to the authenticated principal through
  `PdsDomainEvent::controlling_subject()` before the event reaches the server
  adapter. The application write port accepts
  `Admitted<PdsDomainEvent, PrincipalBound>` and returns
  `ApplicationWriteOutcome`; adapter projection is split by consent,
  data-product, lifecycle, and governance resource family.
  `app/confidential.rs` owns `ConfidentialApplicationAdapter`, the server
  composition handle used by `ConfidentialHttpContext`. Confidential handlers
  call `confidential_application()` and enter resource-family workflow modules
  under `app/confidential/` without handing route code a repository facade
  directly. Confidential workflow admission projects admitted outcomes into a
  `WorkflowAdmitted` event carrying `ConfidentialWorkflowFamily`; the
  application workflow creates the workflow-bound admission, records admission
  through the separate `ConfidentialWorkflowAdmissionRecorder` port, and
  executes repository work inside one operation lifecycle. The capability
  aggregate owns repository and admission-recorder ports separately, so
  resource repository traits remain focused on resource operations.
- `crates/kiva-server/src/surfaces/xapi/` owns the xAPI route, handler,
  service, repository, storage, wire, model, and request parsing modules.
  `surfaces/xapi/handlers.rs` is a module owner over resource-family handlers
  under `surfaces/xapi/handlers/` for `/about`, Statement, document,
  Agent, and Activity endpoints.
- `crates/kiva-server/src/surfaces/pds/` owns the PDS route, handler, service,
  repository, storage, wire, model, and release-artifact reference modules.
  All PDS HTTP operations run their repository or application work through
  `ApplicationOperationGateway` with catalog-backed `ApplicationOperationId`
  values after rate-limit, authorization, and request parsing have succeeded.
  Mutating service methods construct `kiva-pds` commands and admitted domain
  events before calling the repository facade. The repository facade converts
  admitted events into `surfaces/pds/model/projection.rs` projection values,
  and PostgreSQL storage applies those projections to durable state plus the
  lifecycle ledger inside one transaction. Provider-binding registry handlers,
  service admission, model state, repository dispatch, wire projection, memory
  storage, and PostgreSQL storage are split under their matching PDS resource
  units; this registry persists only admitted provider metadata and contract
  gates, not secrets or cryptographic runtime payloads.
- `crates/kiva-server/src/surfaces/confidential/` owns the
  confidential-computation route, handler, repository, storage, wire, and
  model modules. Use-case orchestration is owned by `app/confidential.rs` and
  `app/confidential/`.
- `surfaces/*/routes.rs` owns route composition for its public surface.
  Each router takes its own surface-specific context and calls `.with_state()`
  with that context only.
- `surfaces/*/handlers.rs` owns HTTP handler bodies. Handlers authenticate,
  parse requests, select HTTP status codes, and call application services; they
  do not choose memory versus PostgreSQL backends directly and do not extract
  full `AppState`.
- `surfaces/*/outcome.rs` owns surface-specific repository outcomes and storage
  errors.
- `surfaces/*/response.rs` owns surface-specific error-to-response mapping and
  HTTP response construction helpers.
- `surfaces/confidential/handlers.rs` is split into resource-family files under
  `surfaces/confidential/handlers/` for additive shares and keys, threshold
  releases, Shamir reconstructions, differential privacy, jobs, and
  participants. The handler owner also centralizes confidential endpoint entry
  helpers for rate-limit checks, typed OAuth/OIDC scope authorization, and JSON
  body parsing.
- `kiva-application::confidential` owns application-layer workflow port traits
  for additive aggregation, differential privacy, jobs, participants, Shamir,
  and threshold release. Workflow admission uses
  `ConfidentialWorkflowKind`, so evidence labels are derived from typed
  variants through generated operation-catalog metadata rather than direct
  string literals.
  `crates/kiva-server/src/app/confidential/` implements those ports as
  repository adapters; `ConfidentialApplicationAdapter` does not expose a
  direct repository field.
- `surfaces/confidential/wire.rs` and
  `surfaces/confidential/response.rs` are small module owners. The actual
  confidential wire projection and error-response bodies live under
  `surfaces/confidential/wire/` and `surfaces/confidential/response/` by
  additive, differential-privacy, job, participant, Shamir, and threshold
  resource family.
- `surfaces/pds/handlers.rs` is split into resource-family files under
  `surfaces/pds/handlers/` for consent, data products, provider bindings,
  access, lifecycle, and governance handlers. Shared imports live in
  `handlers/prelude.rs`; shared generated-OpenAPI request parsing lives in
  `handlers/request.rs`.
- `surfaces/pds/wire.rs` is a small owner over `surfaces/pds/wire/` projection
  modules for access, consent, data products, provider bindings, governance,
  inventory/export, lifecycle, and shared DTO helpers.
- `surfaces/pds/repository.rs` is a small owner over
  `surfaces/pds/repository/` dispatch units for access, consent, data
  products, provider bindings, governance, and lifecycle.
- `surfaces/pds/storage/postgres.rs` is a small PostgreSQL backend owner over
  `surfaces/pds/storage/postgres/` persistence units for access, consent, data
  products, provider bindings, governance, and lifecycle. Shared SQL helpers
  are split under `surfaces/pds/storage/postgres_queries/` behind
  `surfaces/pds/storage/postgres_queries.rs`.
- `surfaces/xapi/request.rs` is split under `surfaces/xapi/request/` into
  duplicate-key JSON parsing, Statement/document content parsing, multipart
  attachment validation, Statement normalization, Person projection,
  Statement/document query helpers, and authority/response-format projection
  helpers. Statement shape validation is owned by
  `crates/kiva-xapi/src/xapi/validation.rs` and called from server
  normalization.
- `surfaces/confidential/model.rs` is split under
  `surfaces/confidential/model/` into request DTOs, filters, durable state,
  storage keys, validation helpers, and differential-privacy helper mapping.
- `surfaces/pds/model.rs` is split under `surfaces/pds/model/` into durable
  state and keys, query filters, request construction, and the PDS-to-policy
  adapter boundary. Provider-binding registry state is a PDS server-local
  state family, while provider-binding request/response DTOs remain in
  `crates/kiva-pds`.
- `surfaces/pds/service.rs` owns PDS-only use-case orchestration behind a
  narrow service handle.
- `app/confidential.rs` and `app/confidential/` own confidential-computation
  use-case orchestration behind `ConfidentialApplicationAdapter`.
- `surfaces/*/repository.rs` owns repository facades and dispatches to memory
  or PostgreSQL storage.
- `surfaces/confidential/repository.rs` groups repository facade dispatch by
  resource family under `surfaces/confidential/repository/`.
- `surfaces/pds/model/state.rs` owns only PDS state module declarations and
  compatibility re-exports. Concrete server-local state types are split under
  `surfaces/pds/model/state/` by access, consent, data products, governance,
  keys, ledger, and lifecycle resource family.
- `surfaces/confidential/model/state.rs` owns only confidential state module
  declarations and compatibility re-exports. Concrete server-local state types
  are split under `surfaces/confidential/model/state/` by additive,
  differential privacy, jobs, keys, participants, Shamir, and threshold
  release resource family.
- `surfaces/*/storage.rs` owns storage module composition and shared storage
  types while concrete memory, PostgreSQL, query, and mapping bodies are split
  under matching `surfaces/*/storage/` directories where the surface is large
  enough to need separate review units.
- PDS storage mirrors successful mutating state changes into the
  `pds_ledger_events` projection. Memory storage records the same ledger event
  shape as PostgreSQL, and the table is owned by
  `db/migrations/20260619270000_pds_ledger_events.sql`.
  Provider-binding registry metadata is stored separately in
  `pds_provider_bindings`, owned by
  `db/migrations/20260619280000_pds_provider_bindings.sql`; it is not a
  lifecycle event stream and does not store provider secrets or runtime
  payloads.
- `surfaces/confidential/storage.rs` is a normal module owner over storage
  bodies. `surfaces/confidential/storage/memory.rs` and
  `surfaces/confidential/storage/postgres.rs` own backend-specific
  resource-family implementations under
  `surfaces/confidential/storage/{memory,postgres}/`, while common validation
  and typed PostgreSQL row shapes stay as sibling review units without changing
  repository APIs. The memory repository itself is composed from
  resource-family stores such as `MemoryAdditiveStore`,
  `MemoryDifferentialPrivacyStore`, `MemoryJobStore`,
  `MemoryParticipantStore`, `MemoryShamirStore`, and `MemoryThresholdStore`.
- `surfaces/*/wire.rs` owns JSON response projection from internal state to
  externally documented wire shapes. Stable DTO definitions belong to boundary
  crates such as `crates/kiva-pds` and `crates/kiva-confidential-contract`.
  PDS and confidential wire projection returns
  `Result<Value, serde_json::Error>` and is sent through
  `json_projection_response`, so serialization failures are backend errors
  rather than silent `null` responses. PDS wire projection is split under
  `surfaces/pds/wire/` and shares DTO construction between server state and
  application records through private projection traits instead of maintaining
  parallel mapping functions. PDS inventory/export additionally pass through
  `RecordInventoryView` and `MetadataExportView` so HTTP handlers do not
  assemble DTO fragments directly.
- `crates/kiva-application/src/pds/capabilities.rs` owns the PDS capability
  module boundary, with focused implementation units under
  `crates/kiva-application/src/pds/capabilities/`. `ports.rs` owns
  `PdsApplicationPorts` and `PdsApplicationPortSet`; `projection.rs` owns
  subject-scoped PDS metadata projection through `PdsSubjectProjectionPort` and
  `PdsSubjectIdentityMapper`; `records.rs` owns xAPI, PDS read/write, and
  release-artifact port traits. Projection consumes `StoredStatementMetadata`
  from `XapiStatementReadPort::list_metadata_by_actor_ifi` instead of scanning
  raw Statement JSON, and returns a `PdsSubjectSnapshot` that is projected into
  either record inventory or metadata export output.
- `crates/kiva-application/src/pds/model.rs` owns PDS application records,
  snapshots, and errors under `crates/kiva-application/src/pds/model/`.
  `crates/kiva-application/src/pds/capabilities.rs` owns the narrow
  cross-surface capability aggregate. `PdsApplicationAdapter` wraps
  `PdsApplicationCapabilities`; `XapiStatementReadPort`, `PdsRecordPort`, and
  `ReleaseArtifactAvailabilityPort` are concrete owned handles assembled by
  the server adapter layer. Metadata export obtains related PDS records as
  `PdsMetadataRecords`, which keeps the application port stable while server
  adapters continue to call resource-family services internally.
  `PdsApplicationAdapter::from_server_adapters` is the composition-root adapter
  assembly helper; `PdsApplicationCapabilities::new` remains the explicit constructor
  for tests and future non-server composition.
- `app/pds/server_adapters.rs` is a module owner over
  `app/pds/server_adapters/`. Those are the only PDS application adapter files
  that import server repository/service/state types. Other PDS application
  use-case files stay free of `crate::surfaces::*`.
  `crates/kiva-application/src/pds/commands.rs` and
  `crates/kiva-application/src/pds/queries.rs` expose public command/query
  entry points. `crates/kiva-application/src/pds/workflow.rs` owns the
  `PdsApplicationWorkflow` invocation boundary, with focused command and query
  workflows under `crates/kiva-application/src/pds/workflow/`.
- `crates/kiva-release-artifacts/src/lib.rs` owns normalized PDS
  release-artifact references, lowercase artifact-id parsing, and compatibility
  mapping from data-product source kinds. `crates/kiva-pds/src/pds/release_artifacts.rs`
  adapts PDS requests into those normalized references, and
  `surfaces/pds/release_artifacts.rs` owns only policy-adapter conversion from
  that normalized reference.
- `app/pds/server_adapters/release_artifacts.rs` owns the confidential
  release-artifact availability port used by PDS approved data products.
- `crates/kiva-domain` owns canonical domain newtypes shared by the other
  crates. It is not a wire DTO crate and must stay free of server runtime,
  storage, policy, and confidential-computation dependencies.
- `crates/kiva-xapi` owns server-independent xAPI boundary logic: admission,
  authority construction, language-map canonicalization, Statement equivalence,
  promoted Statement metadata, stored timestamp extraction, and Statement
  collection filtering/voided-target ordering.
- `crates/kiva-pds` owns stable PDS boundary types, including consent scope and
  state, legal basis, metadata class, lifecycle vocabulary, governance profile
  kind, approved data-product source kinds, domain command-input/intent types,
  compatibility request aliases, and release artifact references. These types
  must not depend on Axum, SQLx, or server runtime state.
- `crates/kiva-confidential-core` owns the small confidential boundary types
  used by pure application workflows. `crates/kiva-confidential::primitive`
  owns mechanism primitive wrappers while the crate root is a thin stable API
  re-export layer with explicit exports only.
- `crates/kiva-confidential::mechanism` exposes stable mechanism-oriented
  namespaces. `job`, `differential_privacy`, and `provider` own their
  implementation bodies. `provider` also owns `SecretComputeProviderContract`
  as the mechanism-specific admission object plus the normalized
  `SecretComputeProviderAdmission` shape that records required capability,
  evidence gate, binding digest, and release gate for a mechanism. `additive`,
  `shamir`, `threshold`, and `workflow` are stable facades over currently
  integrated bodies; `registry` owns `CONFIDENTIAL_MECHANISM_REGISTRY`, the
  mechanism descriptor table in
  `crates/kiva-confidential/src/mechanism/registry.rs` for technique, provider
  capability, all-participant policy, and production review-gate metadata;
  `core` owns the remaining tightly coupled additive, Shamir,
  threshold-release, workflow, PSI, TEE, FROST, and FHE implementation bodies.
  The former public `integrated` compatibility namespace has been removed and
  is tracked only in the compatibility-retirement ledger.
  Additive secure
  aggregation is split under
  `mechanism/core/additive/` by envelope, replay, plan, and combine
  logic. Lifecycle/workflow state is split under
  `mechanism/core/lifecycle/` by key lifecycle, participant
  lifecycle, threshold-release approval, and mechanism plan. FHE is split into
  feasibility and runtime review units; TEE is split into portable and Nitro
  review units. FROST is split under
  `mechanism/core/frost/` by profile, key ceremony, signature, and
  release gate, with key ceremony split again by gate, transcript, record, and
  admission logic. PSI is split under `mechanism/core/psi/` by
  digest, bounded cohort, DH-OPRF runtime boundary, and release gate, with the
  OPRF boundary split by profile, element, transcript, and intersection logic.
  Integrated profile identifiers, mechanism enums, mechanism-specific boundary
  errors, and the shared `SecureAggregateError` compatibility facade are split
  under `mechanism/core/profiles/` and
  `mechanism/core/profiles/errors/` by mechanism and role. The
  legacy `SecureAggregateError` name is exported from
  `mechanism/core/profiles/errors/compat.rs`, while typed
  mechanism errors remain the primary review units. Server code imports
  mechanism APIs from explicit
  `kiva_confidential::mechanism::{...}` namespaces. The crate root remains a
  compatibility surface for primitive wrappers plus `SecureAggregateError`, not
  a general mechanism export point.

## Design Rules

- Do not make PDS, confidential computation, or xAPI separate services until a
  deployment constraint requires it.
- Keep OpenAPI contracts and generated schemas as the external API source of
  truth. `spec/openapi/dto-boundary-ledger.yaml` records the adopted generated
  Rust contract-crate boundary, active DTO guards, and the retired
  `rust-dto-map.yaml` migration archive.
- Keep HTTP handlers free of business orchestration beyond authentication,
  parsing, and response status selection.
- Keep route and handler state bounded by surface context:
  `XapiHttpContext`, `PdsHttpContext`, `PdsApplicationHttpContext`,
  `ConfidentialHttpContext`, or `SystemHttpContext`, never full `AppState`.
- Keep pure PDS handlers on `PdsHttpContext`; use `PdsApplicationHttpContext`
  only for PDS routes that invoke cross-surface application workflows.
- Keep PDS route ownership drift-checked in
  `spec/architecture/boundaries.yaml`: pure PDS route blocks must stay on
  `PdsHttpContext`, and inventory/export/approved data-product registration
  route blocks must stay on `PdsApplicationHttpContext`.
- Keep application use cases free of Axum and HTTP types, and place
  cross-surface workflows in `crates/kiva-server/src/app/` rather than widening
  a public-surface service.
- Keep PDS application use cases free of direct `crate::surfaces::*` imports;
  server-local state and repository/service adapters belong under
  `app/pds/server_adapters*`.
- Keep wire mapping free of repository access and authorization decisions.
  Treat DTO serialization as fallible at the handler boundary through
  `json_projection_response`; do not mask projection failures as JSON `null`.
- Keep repository facades responsible for memory/PostgreSQL dispatch; do not let
  handlers choose a backend.
- Keep domain boundary crates such as `kiva-pds` free of server runtime
  dependencies.
- Keep `spec/architecture/boundaries.yaml` as the executable boundary manifest
  for source anchors, Cargo dependency rules, module-declaration rules, route
  ownership rules, source-size rules, wildcard-import rules, and
  milestone-ledger anchors.
- Keep server test helpers split under `crates/kiva-server/src/tests/helpers/`;
  `helpers.rs` should remain a compatibility owner rather than accumulating
  workflow helper bodies.
- Keep PDS and confidential memory tests split under
  `crates/kiva-server/src/tests/{pds_memory,confidential_memory}/`; the
  sibling `.rs` files should remain compatibility module owners only.
- Keep retained compatibility surfaces listed in
  `spec/architecture/compatibility-surfaces.yaml`; do not add new root
  re-exports or compatibility owners without a migration target.
- Keep milestone status in machine-readable ledgers such as
  `spec/pds/publication-milestones.yaml` and
  `spec/confidential/computation-milestones.yaml`; architecture docs should
  explain claim boundaries rather than duplicate every milestone definition.
- Keep xAPI JSON read-model helpers in `kiva-xapi` when they can be expressed
  without Axum, SQLx, or server state.
- Add mechanism modules in `kiva-confidential::mechanism` as facades first; move
  implementation only when the target module can own tests and evidence without
  widening behavior.
- Add a new boundary only when it removes a real dependency cycle or makes a
  verification/release artifact easier to audit.

## Refactoring Status

- Route registration, HTTP handlers, use-case services, repository facades,
  storage implementations, wire mapping, and server-local models are split by
  public surface under `surfaces/{xapi,pds,confidential}/`.
- Authentication, runtime environment parsing, server state, shared response
  mapping, route constants, request helpers, and common outcomes are split out
  of `lib.rs` under `core/`.
- Environment-backed bootstrap now lives in `core/bootstrap.rs`; `core/state.rs`
  owns state shape and context projection only.
- PDS, xAPI, and confidential-computation persistence dispatch have dedicated
  surface-local repository facades that share one enum-dispatch macro and keep
  backend selection out of handlers and application services.
- PDS repository dispatch is split under `surfaces/pds/repository/`, and PDS
  PostgreSQL storage bodies are split under
  `surfaces/pds/storage/postgres/`. PDS and xAPI storage bodies stay separated
  by backend and resource family under `surfaces/{pds,xapi}/storage/`,
  matching the confidential-computation storage split without changing facade
  APIs.
- Public HTTP routes now receive bounded contexts:
  `XapiHttpContext` exposes only xAPI repositories plus limits/auth;
  `PdsHttpContext` exposes only PDS services plus limits/auth;
  `PdsApplicationHttpContext` exposes the PDS application workflow capabilities used
  by inventory, metadata export, and approved data-product registration through
  `PdsApplicationAdapter`;
  `ConfidentialHttpContext` exposes confidential services through
  `ConfidentialApplicationAdapter` plus limits/auth; `SystemHttpContext` exposes
  runtime limits for system endpoints.
- Surface-specific response and outcome modules have moved out of `core` so
  shared server support no longer owns PDS, xAPI, or confidential-computation
  error enums.
- PDS data-product release checks go through `PdsApplication` and
  `ReleaseArtifactAvailabilityPort`, which map server request metadata through
  `kiva-pds` boundary references and policy authorization facts before asking
  confidential repositories for artifact availability.
- PDS inventory and metadata export go through `PdsApplication` and
  `PdsApplicationPorts`. `PdsSubjectProjectionPort` uses
  `PdsSubjectIdentityMapper`, `XapiStatementReadPort`, and `PdsRecordPort` to
  project subject-scoped statement metadata before PDS wire mapping. The
  mapper delegates canonical account IFI construction to
  `kiva_pds::PdsSubjectAccountNamespace`, so server adapters do not duplicate
  PDS subject identity formatting or pass raw xAPI Statement JSON to PDS
  response construction.
- xAPI handler bodies are split under `surfaces/xapi/handlers/` by resource
  family while routes keep the same public function names through module
  re-exports.
- xAPI request parsing is split under `surfaces/xapi/request/` so payload
  parsing, attachment validation, normalization, query validation,
  authority/projection formatting, and Person projection have separate review
  units. Pure Statement shape validation has moved to `kiva-xapi`.
- xAPI Statement collection filtering, voided target exclusion, stored-time
  ordering, and `stored` timestamp extraction have moved from the server root
  into `kiva-xapi`.
- PDS wire vocabularies that are not tied to storage or authorization now live
  in `kiva-pds`; PDS request/response DTOs, request shape validation, and
  request normalization live there as well. The server keeps only policy
  mappings such as consent delegation, data-product release artifact
  resolution, access-check vocabulary parsing, governance evidence projection,
  and lifecycle-case transition status conversion under the PDS adapter
  boundary.
- Confidential-computation mechanisms have stable facade modules under
  `kiva_confidential::mechanism::{additive,differential_privacy,shamir,threshold,workflow,psi,tee,frost,fhe,job,provider,registry}`.
- Fixed-width confidential primitive wrappers now live in
  `kiva_confidential::primitive` and remain available through crate-root
  re-exports.
- `mechanism::job`, `mechanism::differential_privacy`, and
  `mechanism::provider` own their implementation bodies directly.
- `mechanism::core` owns the remaining implementation modules under
  `mechanism/core/{profiles,lifecycle,additive,shamir,psi,tee,frost,fhe}.rs`,
  with additive split under `mechanism/core/additive/`, lifecycle
  split under `mechanism/core/lifecycle/`, FHE split under
  `mechanism/core/fhe/`, TEE split under
  `mechanism/core/tee/`, FROST split under
  `mechanism/core/frost/`, and PSI split under
  `mechanism/core/psi/`,
  with bundled tests and Kani harnesses split by mechanism under
  `mechanism/core/tests/` and
  `mechanism/verification/kani_harnesses/`. Future extraction from the core review
  units should happen only with behavior-neutral tests and evidence carried
  with the moved code.
