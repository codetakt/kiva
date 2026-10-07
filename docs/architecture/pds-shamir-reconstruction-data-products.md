# PDS Shamir Reconstruction Data Products

Kiva can register PDS data products that reference persisted Shamir threshold
reconstruction metadata. This completes the current integration between the
M9 Shamir reconstruction boundary, CC3 Shamir VSS-admission boundary, and the
PDS approved data-product registry.

## Boundary

The PDS source kind is:

- `shamir-threshold-reconstruction`

The referenced identifier is the Shamir `reconstruction_id`. The source is
available only when the reconstruction metadata exists and is owned by the
authenticated subject registering the PDS data product. The referenced
metadata includes the Shamir verification profile and verified share count,
but not submitted share values or opening nonces.

PDS data-product responses remain metadata-only. They expose the product
identifier, owner, source kind, source identifier, title, purpose, legal basis,
declared data classes, and creation time. They do not expose submitted Shamir
shares, participant share values, or the reconstructed secret.

## Implementation

- policy admission:
  `crates/kiva-policy/src/lib.rs`
- PDS runtime admission and metadata-only responses:
  `crates/kiva-server/src/surfaces/pds/service.rs`,
  `crates/kiva-server/src/surfaces/pds/repository.rs`,
  `crates/kiva-server/src/surfaces/pds/storage.rs`, and
  `crates/kiva-server/src/surfaces/pds/wire.rs`
- confidential release availability adapter:
  `crates/kiva-server/src/app/pds/server_adapters.rs`
- PDS release-artifact reference normalization:
  `crates/kiva-server/src/surfaces/pds/release_artifacts.rs`
- OpenAPI contract:
  `spec/openapi/kiva-pds.yaml`
- PostgreSQL persistence:
  `db/migrations/20260619220000_pds_shamir_reconstruction_data_products.sql`
- publication example:
  `examples/pds/shamir-reconstruction-data-product.request.json`

## Assurance

- Rust tests cover missing reconstruction rejection, owner-scoped admission,
  foreign-owner rejection, and metadata-only PDS listing.
- Kani covers the source-kind-specific data-product admission rule in the pure
  policy layer.
- Tamarin covers the symbolic boundary that Shamir data products require a
  prior persisted Shamir reconstruction for the same owner and source kind.
- PostgreSQL verification covers the same admission path through migrated
  durable storage when the optional Postgres test runner is configured.

## Non-Claims

This milestone does not make the reconstructed value a BI payload. It also
does not implement distributed key generation, proactive share refresh,
threshold signatures, threshold decryption, PSI, FHE, TEE execution, or
malicious-dealer-robust threshold cryptography.
