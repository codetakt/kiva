# Personal Data Store Milestones

This document is the human-readable guide for Kiva's PDS milestone boundaries.
The machine-readable source of truth is
`spec/pds/publication-milestones.yaml`.

## Claim Boundary

The PDS API is a Kiva platform extension around the
[xAPI LRS surface](conformance-claim.md). It governs subject-oriented consent,
derived analytical products, access decisions, public-sector governance
metadata, and audit evidence around accepted xAPI records and confidential
release metadata. It does not expand the IEEE xAPI LRS conformance surface.

## Source of Truth

- Milestone ledger: `spec/pds/publication-milestones.yaml`
- PDS OpenAPI contract: `spec/openapi/kiva-pds.yaml`
- PDS publication schemas: `spec/pds/`
- PDS deployment templates: `spec/pds/deployment-templates.yaml`
- Requirement traceability: `spec/compliance-matrix.yaml`
- Architecture drift guard: `spec/architecture/boundaries.yaml`

## Boundary Catalog

| Milestone | Boundary |
| --- | --- |
| M0 PDS boundary | PDS extension boundary separated from xAPI LRS conformance. |
| M1 Consent and audit substrate | Consent, revocation, access decision, and audit metadata substrate. |
| M2 Approved data products | Approved data-product metadata from approved confidential releases. |
| M3 Policy assurance | Policy invariants and their Rust, Kani and Tamarin validation methods. |
| M4 Personal-data lifecycle | Metadata-only personal-data lifecycle boundary. |
| M5 Confidential analytics integration | Current confidential analytics integration boundary. |
| M6 Public-sector governance | Public-sector governance metadata boundary. |
| M7 Publication readiness | Publication-readiness boundary. |
| M8 Confidential mechanism abstraction | Confidential mechanism abstraction boundary. |
| M9 Shamir threshold cryptography | Shamir reconstruction boundary. |
| M10 PDS Shamir data products | Metadata-only PDS Shamir data-product boundary. |
| M11 OAuth/OIDC resource-server binding | OAuth/OIDC resource-server scope boundary. |
| M12 Operational lifecycle workflows | Metadata-only operational lifecycle workflow boundary. |
| M13 Public-sector deployment templates | Public-sector deployment-guidance boundary. |
| M14 Secret and confidential-compute provider integration | Secret and confidential-compute provider contract boundary. |
| M15 Learner-principal deployment profile | Subject-bound learner token flows over the existing PDS scope boundary. |
| M17 Learner-initiated data portability | Subject-bound Statement and PDS metadata export with durable audit evidence. |

## Maintenance Rule

Update `spec/pds/publication-milestones.yaml` first when a PDS milestone
changes. Keep this document short: it should explain the claim boundary and
point reviewers to the ledger, contracts, validators, and evidence generators.
