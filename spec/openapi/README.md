# Kiva OpenAPI Contracts

This directory contains machine-readable API contracts for Kiva.

- `kiva-xapi.yaml`: OpenAPI 3.1 contract for the IEEE/ISO/IEC 39274-1-1-2025 aligned xAPI LRS surface.
- `kiva-confidential.yaml`: OpenAPI 3.1 contract for Kiva-specific
  confidential-computation endpoints. This is a platform extension contract,
  not part of the IEEE xAPI LRS surface.
- `kiva-pds.yaml`: OpenAPI 3.1 contract for Kiva-specific Personal Data Store
  consent, approved data-product, governed data-product read, lifecycle
  metadata, public-sector governance profile, policy decision, and audit
  endpoints. This is a platform extension contract, not part of the IEEE xAPI
  LRS surface.

PDS publication examples live in `../../examples/pds`, and generated PDS
component schema artifacts live in `../../generated/pds-openapi-schemas`.
`generated-contract-dto-map.yaml` is the active generated-contract DTO map. It
is generated from OpenAPI contracts and generated contract crates by
`../../scripts/ci/generate_generated_contract_dto_map.py`; it records external
transport DTOs owned by generated contract crates.
`semantic-compatibility-map.yaml` is the active hand-maintained semantic DTO
compatibility map. It records internal semantic DTO compatibility anchors.
Mappings can be presence-only or can opt into strict object/enum shape checks.
For strict mappings,
`scripts/ci/validate_openapi_rust_dto_map.py` compares OpenAPI object
properties and required fields with Rust struct serde field names, and compares
OpenAPI enum values with Rust serde enum labels in the OpenAPI contract gate.
`rust-dto-map.yaml` is a retired OpenAPI/Rust DTO archive retained for migration
history only; active checks and projections read the two split maps directly.
`dto-boundary-policy.yaml` records the active generated-contract HTTP DTO
boundary policy. `semantic-compatibility-retirement.yaml` records the completed
former aggregate-guard retirement and the remaining semantic compatibility
constraints. `dto-boundary-ledger.yaml` is the active DTO boundary ledger kept
at the canonical path and records `dto-generation-plan.yaml` only as a retired
legacy path; its validator expands those split sources while keeping OpenAPI as
the HTTP contract source of truth. Each contract row names the generated
`external_contract_boundary` separately from
`internal_semantic_boundaries`, and separates
`generated_contract_dto_map_contracts` from
`semantic_compatibility_map_contracts`.
The xAPI DTO generation target is
`../../crates/kiva-xapi-openapi-contract`; server xAPI Statement-family request
parsing and response projection now pass through that contract crate before
handwritten xAPI normalization and HTTP semantic helpers. The PDS generated
contract crate covers the DTO-bearing OpenAPI request-body schema closure and
success-response schema closure, while `../../crates/kiva-pds` remains the
internal semantic boundary. The ledger also records completion milestones for
generated request/response parity, server handler adoption, and semantic
compatibility DTO retirement. Each public HTTP contract now declares
`GeneratedContractBoundary` for handlers that use generated contract DTOs in
both directions. A `CompatibilityHttpBoundary` is retained only as a documented
future migration state, and must carry a
`runtime_adoption_gate` whenever it is reintroduced.

The generated HTTP operation catalog in
`../../docs/architecture/generated/http-operation-catalog.md` and
`../../generated/http-operation-catalog.evidence.json` is derived from these
OpenAPI contracts by `../../scripts/ci/generate_http_operation_catalog.py`.
The generated surface catalog in
`../../generated/architecture/surface-catalog.yaml` is generated from the split
surface source ledgers, validated against that HTTP projection, and used to
generate the application workflow ledger, PDS resource-family ledger, and
server route/profile registry. Route coverage evidence is generated from the
same catalog into `../../docs/architecture/generated/route-coverage.md` and
`../../generated/route-coverage.evidence.json`.
`../../spec/architecture/architecture-graph.source.yaml#/sources` owns the
CI-checked source-to-projection graph metadata for these generated and
validated artifacts; `../../spec/architecture/design-backbone.yaml` and
`../../spec/ci/projection-graph.yaml` are generated review projections.
`../../generated/openapi-dto-map-projection.yaml` records the generated Rust
contract crate schema membership derived from the DTO boundary ledger, the split
DTO maps, and OpenAPI schema closures.

The OpenAPI contract captures the HTTP boundary. Constraints that are difficult or impossible to express directly in OpenAPI are recorded in `x-kiva-assertions` and should be covered by conformance tests, property tests, Kani harnesses, Tamarin models, or narrow F*/Low* models where appropriate.

Each operation or schema that projects an IEEE requirement references the
requirement catalog through `x-kiva-requirements`.
