# Confidential Key and Participant Lifecycle

Kiva CC10 adds a production-oriented metadata substrate for confidential
computation participants and key lifecycles. This layer is intentionally
separate from the IEEE xAPI LRS interface. It controls who may join Kiva
confidential jobs, who may register additive share-signing keys, and which
participant lifecycle evidence is exported for audit.

## Boundary

The implemented boundary is metadata-only. Kiva records participant identity
bindings, owner subjects, participant indexes, mechanism roles,
authorization-scope labels, key ceremony references, recovery contact
references, evidence references, status, and audit events. It does not expose
raw shares, signatures, private keys, PSI OPRF seeds, TEE quotes, FHE secret
keys, plaintexts, or reconstructed secrets through participant metadata or
audit endpoints.

CC10 does not claim production PSI OPRF ceremonies, production FHE key
management, vendor TEE quote verification, distributed key generation, or
malicious-secure MPC. Those mechanisms can attach their future ceremonies to
the same participant records through `key_ceremony_ref` and
`mechanism_roles`, but those references are not cryptographic proof by
themselves.

## Lifecycle Model

Participants start in `invited` status. Only `active` participants are admitted
to new confidential jobs, additive share-signing key registration, additive
key rotation, additive share submission, and additive aggregate computation.
`suspended`, `frozen`, and `removed` participants fail closed for new work.

The strict transition table is:

- `invited` -> `active`, `suspended`, `removed`
- `active` -> `suspended`, `frozen`, `removed`
- `suspended` -> `active`, `frozen`, `removed`
- `frozen` -> `active`, `removed`
- `removed` has no outgoing transitions

Idempotent same-state transitions are accepted and do not create duplicate
audit events. Invalid rewinds, such as `active` -> `invited`, return conflict.

## API Contract

The CC10 API surface is:

- `GET /confidential/participants`
- `POST /confidential/participants`
- `POST /confidential/participant-transitions`
- `GET /confidential/participant-audit-events`

All write operations require `confidential:write`; metadata reads require
`confidential:read`. The OpenAPI contract is
`spec/openapi/kiva-confidential.yaml`, and
`scripts/ci/validate_confidential_openapi.py` checks that participant
endpoints, scopes, and metadata-only schemas remain present.

## Persistence

PostgreSQL persistence is defined by
`db/migrations/20260619260000_confidential_participant_lifecycle.sql` and
mirrored in `db/schema.sql`.

The primary participant key is tenant, owner subject, participant subject, and
participant index. Registration and its initial audit event are written in one
transaction. Transitions use `SELECT ... FOR UPDATE`, update the participant,
and append an audit event in the same transaction.

The additive signing-key and confidential job paths check participant status
before accepting new keys, share submissions, aggregate computations, or jobs.
Missing, wrong-owner, invited, suspended, frozen, and removed participants
fail closed before those writes occur.

## Verification

Rust tests cover in-memory metadata/audit export, missing/invited/suspended
participant rejection, wrong-owner participant rejection, duplicate metadata
conflicts, and active participant acceptance. PostgreSQL verification includes
an ignored integration test for idempotent participant registration,
transition audit, and active gating.

Kani harnesses in `kiva-confidential` prove the participant transition table
and active-only admission predicate over the finite participant status model.
The Tamarin model
`proofs/tamarin/kiva_confidential_participant_lifecycle_boundary.spthy`
documents the representative protocol boundary: job/key admission requires a
prior activation event, every transition emits matching audit evidence,
removed participants emit removal audit evidence, and raw payload publication
is absent from the model.
