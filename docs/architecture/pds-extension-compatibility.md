# PDS Extension Compatibility

Kiva's Personal Data Store (PDS) API is a Kiva platform extension layered
around the xAPI Learning Record Store. It does not change the
[xAPI LRS interoperability surface](conformance-claim.md) and should not be
described as part of the IEEE xAPI conformance claim.

## Compatibility Boundary

The standards-facing xAPI contract is `spec/openapi/kiva-xapi.yaml`. It
defines the LRS HTTP boundary for xAPI
Statements, state, activity profile, agent profile, and related resources.
Its requirement references use the pinned IEEE Std 9274.1.1-2023 baseline.
Equivalence with the 2025 edition has not been established.

The PDS contract is `spec/openapi/kiva-pds.yaml`. It defines Kiva-specific
endpoints for consent, delegation, metadata lifecycle, governed data-product
access, and deployment governance. These endpoints are separately tagged with
`x-kiva-contract.area: personal-data-store` and are not projected into the xAPI
LRS contract.

Kiva can therefore make two separate statements:

- the xAPI LRS interface has requirement traceability to the pinned IEEE 2023
  baseline, with implementation conformance assessed separately;
- the PDS API is a Kiva extension that governs personal-data and analytics
  workflows around accepted xAPI records.

Neither statement is an IEEE certification claim; source traceability alone
does not establish implementation conformance.

## Current PDS Extension Surface

The current publication boundary covers:

- subject-scoped consent grants and revocations;
- access checks and access-audit metadata;
- approved data-product registration from governed `ReleaseArtifactRef`
  metadata, currently backed by threshold-release, differential-privacy-release,
  or Shamir threshold-reconstruction artifacts. The PDS registration path checks
  those artifacts through the PDS application
  `ReleaseArtifactAvailabilityPort` rather than depending on each confidential
  mechanism directly;
- governed metadata-only data-product reads;
- subject-facing record inventory and metadata export;
- lifecycle metadata for export, retention review, correction request,
  dispute, and tombstone events;
- operational retention policies and correction/dispute/tombstone case
  metadata;
- public-sector governance profiles for separated custodian, operator,
  policy-authority, analyst, auditor, and optional identity-authority subjects;
- metadata-only references to external secret, encryption, and
  confidential-compute provider contracts through the PDS provider-binding
  registry, without raw-secret custody, ciphertext payload storage, key-share
  storage, or runtime payload custody;
- deployment templates, legal-policy examples, and operator runbooks for
  school-district and multi-agency PDS deployments.

PDS responses are intentionally metadata-oriented. They must not expose raw
xAPI Statement JSON, xAPI result payloads, aggregate payloads, differential
privacy noise values, reconstructed Shamir secrets, secret shares, signatures,
or signed envelopes.

The server keeps PDS behavior in strict modular-monolith boundaries under
`crates/kiva-server/src/surfaces/pds/`:

- `routes.rs` registers PDS routes, while `handlers.rs` is a module owner over
  resource-family handler files for authentication/authorization, request
  parsing, and HTTP status selection.
- `wire.rs` maps internal PDS state into response JSON and owns the
  metadata-only export shape.
- `service.rs` owns read and direct-access PDS surface services. Handlers pass
  a `PdsQueryGateway` value for PDS-only reads and direct access checks instead
  of the full `AppState`.
- `crates/kiva-application/src/pds.rs` owns the server-independent PDS
  application command/query workflows: record inventory, metadata export,
  approved data-product registration, consent, lifecycle, retention, and
  governance commands.
- `crates/kiva-server/src/app/pds.rs` owns server adapter assembly for those
  workflows.
- `crates/kiva-application/src/pds/capabilities.rs` owns the cross-surface
  PDS application capability boundary. `PdsApplicationPorts` groups the
  concrete server adapters, while `PdsSubjectProjectionPort` projects
  subject-scoped xAPI statement metadata before PDS inventory/export response
  construction and `PdsSubjectIdentityMapper` owns subject-to-actor-IFI
  mapping. Raw xAPI Statement JSON does not pass through PDS wire mapping.
  `crates/kiva-server/src/app/pds/server_adapters.rs` binds its concrete
  ports to server repositories, including confidential release-artifact
  availability checks for approved data products.
- `repository.rs` owns the PDS repository facade that dispatches to memory or
  PostgreSQL implementations. `storage.rs` owns the memory/PostgreSQL storage
  implementation bodies and remains a private server internal.
- `crates/kiva-release-artifacts/src/lib.rs` owns normalized
  `ReleaseArtifactRef` values and fixed-width artifact-id parsing.
  `crates/kiva-pds/src/pds/release_artifacts.rs` adapts PDS data-product
  requests into those normalized references.
  `surfaces/pds/release_artifacts.rs` adapts those references into policy
  authorization facts. PDS data-product registration does not depend directly
  on mechanism-specific confidential-computation repositories.

## Publication Artifacts

The publication artifacts for the current PDS boundary are:

- contract: `spec/openapi/kiva-pds.yaml`;
- examples: `examples/pds/`;
- generated JSON Schema artifacts: `generated/pds-openapi-schemas/`, using
  `components.schemas.json` as the definition bundle and individual schema
  files as thin references to that bundle;
- publication milestone mirror: `spec/pds/publication-milestones.yaml`;
- deployment templates: `spec/pds/deployment-templates.yaml`;
- deployment template schema:
  `spec/pds/schemas/pds-deployment-template-catalog.schema.json`;
- evidence manifest schema: `spec/pds/schemas/pds-evidence-manifest.schema.json`;
- evidence generator: `scripts/ci/generate_pds_evidence.sh`;
- evidence validator: `scripts/ci/validate_pds_evidence_bundle.py`;
- publication validator: `scripts/ci/validate_pds_publication.py`.

Generate a local PDS publication evidence bundle with:

```sh
scripts/ci/generate_pds_evidence.sh --require-clean-git
```

For a lightweight bundle that skips PostgreSQL, Kani, and Tamarin execution:

```sh
scripts/ci/generate_pds_evidence.sh \
  --skip-postgres-verification \
  --skip-kani \
  --skip-tamarin
```

The lightweight bundle still validates the OpenAPI contract, generated schema
artifacts, examples, PDS policy tests, PDS server tests, bundle manifest, and
checksums.

## Non-Goals

The current PDS publication boundary is not:

- a complete production Personal Data Store certification;
- a legal compliance opinion for a specific jurisdiction or public body;
- an implementation of production OAuth/OIDC role-claim binding;
- a complete emergency-access workflow implementation;
- a complete secure multi-party computation or confidential-computation
  security proof;
- an expansion of the [xAPI LRS conformance claim](conformance-claim.md).

Physical deletion/redaction execution, legal-hold automation, administrative
UX, production identity-provider bindings, and deployment-specific legal review
remain deployment and product work beyond the current publication-readiness
boundary.
