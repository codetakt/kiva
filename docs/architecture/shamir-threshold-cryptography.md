# Shamir Threshold Cryptography

Kiva now includes a Shamir threshold reconstruction mechanism with
hash-committed share-opening verification for the confidential-computation
extension. This is distinct from the existing two-authority threshold release
workflow, which is a governance approval gate over an already-computed
aggregate.

## Mechanism

The implementation uses Kiva's fixed secure aggregate field:

- modulus: `4_294_967_291`;
- participant indexes: one-based interpolation points;
- reconstruction: Lagrange interpolation at `x = 0`;
- admission: at least `threshold` unique shares, no duplicate participant
  indexes, participant indexes inside the configured participant set, and
  threshold at least two;
- verification profile: `kiva-shamir-hash-vss-v1`;
- share-opening commitment: SHA-256 over the domain
  `kiva-confidential:shamir-share-opening:v1`, transcript id, participant
  index, field value, and 32-byte opening nonce.

The reference vector is:

- polynomial: `f(x) = 1234 + 111*x + 222*x^2 mod p`;
- threshold: 3;
- participants: 5;
- shares: `(1,1567)`, `(2,2344)`, `(3,3565)`, `(4,5230)`, `(5,7339)`;
- expected secret: `1234`.

The vector, opening nonce seeds, and expected commitments are stored in
`spec/confidential/shamir-threshold-vectors.yaml`.

## VSS Admission And Complaints

CC3 uses a narrow VSS-admission profile:

- a share opening must match its published hash commitment before it can be
  used for interpolation;
- commitment mismatches fail closed with no persisted reconstruction;
- a complaint is sustained when the opening fails verification and rejected
  when the opening verifies;
- sustained complaint openings are excluded from reconstruction;
- reconstruction proceeds only when at least `threshold` unique openings have
  verified.

This profile detects tampered or equivocated submitted openings at the Kiva
boundary. It does not prove that a dealer generated all shares from a valid
hidden polynomial; deployments that need malicious-dealer security still need a
stronger Feldman/Pedersen VSS or DKG protocol outside the current Kiva
endpoint.

## API And Persistence

Kiva exposes:

- `POST /confidential/shamir-threshold-reconstructions`
- `GET /confidential/shamir-threshold-reconstructions`

POST verifies share openings, reconstructs, and persists a subject-owned
reconstruction result. GET returns metadata only. Submitted shares, commitments,
and opening nonces are not persisted by this endpoint, and metadata does not
expose shares or reconstructed values.

PostgreSQL persistence is in:

- `kiva.confidential_shamir_threshold_reconstructions`
- `db/migrations/20260619210000_confidential_shamir_threshold_reconstructions.sql`
- `db/migrations/20260619240000_confidential_shamir_vss_metadata.sql`

The PDS extension can register metadata-only data products that reference
persisted reconstruction metadata through source kind
`shamir-threshold-reconstruction`. That PDS integration does not expose the
submitted shares or reconstructed value.

## Verification Targets

Verification targets the VSS-admission boundary:

- Rust test targets include reconstruction vectors, share-opening commitment vectors,
  tampered opening rejection, duplicate-share rejection, complaint outcomes,
  and HTTP metadata behavior.
- Kani targets include threshold-policy admission and fail-closed rejection for
  insufficient or duplicate shares before reconstruction, plus complaint
  resolution.
- F* models the abstract Shamir reconstruction gate and requires verified share
  openings.
- Tamarin models a finite representative boundary where reconstruction requires
  a configured transcript, a distinct threshold share set, and verified share
  openings, and where sustained complaints and rejected openings cannot become
  accepted reconstruction under the same request identity.
- PDS integration checks that Shamir data products require persisted
  reconstruction metadata owned by the registering subject.
- Operator procedures are in `docs/operations/shamir-vss-runbook.md`.

## Non-Claims

This milestone does not implement:

- distributed key generation;
- malicious-dealer-robust VSS;
- proactive share refresh;
- threshold signatures or threshold decryption;
- FHE, PSI, or TEE execution.

Those remain separate mechanism milestones.
