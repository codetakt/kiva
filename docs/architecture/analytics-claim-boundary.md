# Analytics Claim Boundary

## Purpose

Kiva analytics serving is a governed read boundary over approved PDS data
products and confidential releases. It is a distinct application surface that
may expose aggregate outputs after PDS policy, confidential-computation, and
release controls have admitted them.

BI0 defined that boundary and its milestones. BI1 implements the bounded
serving capability described below without broadening the xAPI LRS surface.
BI2 adds validated, vendor-neutral deployment guidance for external reporting
consumers without adding a hosted dashboard or runtime capability.

## Boundary

The analytics boundary may read only products that are already represented as
approved PDS data products or admitted confidential releases. It must preserve
tenant and subject scope, require an exact authorization scope, use bounded
cursor pagination, and emit audit evidence for every read. The boundary does
not derive authority from the shape of an xAPI Statement and does not bypass
the existing PDS or confidential release controls.

The following are explicit non-claims:

- Analytics serving is not part of the IEEE/ISO/IEC 39274-1-1:2025 xAPI LRS
  conformance surface and does not extend Kiva's xAPI conformance claim.
- The boundary does not provide raw xAPI Statements or individual-level data.
- Kiva does not host or claim a BI dashboard or reporting product. Such tools
  are external consumers of the analytics API.
- BI1 makes no production analytics, production confidential-computation, or
  end-to-end privacy guarantee. Confidential mechanisms retain the maturity
  labels and provider responsibilities recorded by their existing ledgers.

## Milestones

- **BI0: Analytics claim boundary.** Define the serving boundary, non-claims,
  architecture placement, and conservative claim maturity. No implementation
  is included.
- **BI1: Governed aggregate serving API.** Add a dedicated HTTP surface for
  approved aggregate products and confidential releases, with tenant and
  subject scope, cursor pagination, exact authorization, and audit evidence.
- **BI2: Reference reporting deployment.** Provide schema-checked school and
  corporate-training configurations for external BI1 consumers. The reporting
  tool remains outside Kiva; the templates are not legal advice.

The machine-readable milestone authority is
`spec/analytics/publication-milestones.yaml`.

BI2 guidance and its machine-readable catalog are documented in
`docs/architecture/reference-reporting-deployments.md` and
`spec/analytics/reporting-templates.yaml`.

## BI1 Surface Placement

BI1 uses **Option A: a dedicated analytics surface**. Its composition is
`surfaces/analytics`, an analytics-specific `HttpContext`, and the separate
`spec/openapi/kiva-analytics.yaml` contract. The surface calls HTTP-independent
application ports that expose only admitted product metadata and aggregate
release payloads.

Option A keeps analytics visibly separate from the xAPI conformance surface and
from the personal governance operations in the PDS contract. It also gives
pagination, authorization, and read-audit semantics a single owner.

**Option B**, adding serving operations under the PDS surface and
`kiva-pds.yaml`, is rejected. It would mix personal-data governance operations
with aggregate-product consumption, make scope intent less precise, and couple
external reporting clients to the PDS administration contract.

BI1 introduces the exact, non-hierarchical `analytics:read` scope. Reusing
`pds:read` would blur access to PDS metadata with permission to consume an
approved aggregate product. No other scope implies `analytics:read`.

## Evidence Registration

The analytics milestone ledger is registered as a canonical DesignUniverse
source and is schema checked by the existing source-ledger validator. BI1 adds
a dedicated analytics OpenAPI validator to the OpenAPI contract gate. Contract,
surface, operation, route, DTO-boundary, test, and claim projections are checked
through the existing evidence registry.
