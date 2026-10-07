# Private Set Intersection

Kiva CC4 implements a bounded cross-school cohort matching profile:
`kiva-psi-cohort-hash-v1`.

The profile computes an n-of-n intersection over pre-tokenized identifier
digests. Kiva does not accept raw cohort identifiers in the job contract. Each
participant canonicalizes identifiers, derives a SHA-256 digest using the
profile domain, job id, namespace, and canonical identifier, and submits only
the digest set. Kiva returns only digests present in every participant set.

## Mechanism

- use case: cross-school cohort matching;
- participant model: at least two school or district custodians;
- threshold: n-of-n, all participants must submit exactly one dataset;
- canonicalization: trim and ASCII-lowercase;
- digest domain: `kiva-confidential:psi-cohort-identifier:v1`;
- digest inputs: domain, 32-byte job id, namespace length and bytes,
  canonical identifier length and bytes;
- output: sorted intersection digests and intersection count.

The job contract is `spec/confidential/psi-cohort-job-contract.yaml`. Test
vectors are in `spec/confidential/psi-cohort-vectors.yaml`.

## Admission Rules

The core implementation rejects:

- replayed job ids;
- missing participant datasets;
- duplicate participant indexes;
- participant indexes outside the configured participant count;
- empty participant datasets;
- duplicate identifier digests within one participant dataset;
- empty or control-character identifiers when using the helper canonicalizer.

Empty intersections are valid. A valid empty intersection returns no digest
values, rather than exposing non-matching records.

## PDS Boundary

PSI outputs are aggregate-insight shaped, but the current PDS public surface
does not expose a dedicated `private-set-intersection-result` source kind,
HTTP endpoint, or PostgreSQL result table. The current boundary is the Rust
core result and assurance evidence; a PSI-specific PDS source kind remains
future work. PSI outputs do not grant raw-record read access and do not expose
per-participant inputs.

## Assurance

Current assurance includes Rust tests, deterministic vectors, a Kani harness,
the PSI assurance validator, and a Tamarin boundary model. Tamarin checks that
release requires a configured PSI job, intersection computation requires both
participant submissions for the released digest, and left-only or right-only
non-matches cannot be released.

## Non-Claims

This milestone does not implement:

- OPRF PSI;
- Diffie-Hellman PSI;
- malicious-secure PSI;
- fuzzy matching or general record linkage;
- cryptographic protection if the identifier namespace is low entropy;
- participant-side key ceremony or salting protocol.

Deployments with low-entropy identifiers must add a stronger participant-side
tokenization or OPRF layer before using this profile for public claims.
