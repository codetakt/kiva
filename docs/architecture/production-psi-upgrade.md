# Production PSI Upgrade

Kiva CC11 upgrades the bounded hash-token PSI boundary with a selected
DH-OPRF PSI runtime boundary. The selected protocol family is
`ristretto255-sha512-voprf` behind an isolated Rust adapter. Kiva itself does
not implement OPRF private-key handling, blinding, evaluation, or finalization
in the LRS/server process; it accepts opaque evaluated elements and transcript
metadata from the reviewed runtime boundary.

## Boundary

The implemented boundary is review-gated. Kiva validates the runtime artifact
digest, external review evidence digest, participant indexes, participant
transcript status, opaque evaluated OPRF elements, transcript digest, replay
record shape, and governed release facts.

The current repository does not expose a dedicated DH-OPRF PSI HTTP API,
PostgreSQL transcript/result table, or PDS data-product source kind. PSI/OPRF
release evidence is carried through the generic confidential job lifecycle
boundary until a dedicated PDS source kind is implemented.

Kiva does not accept raw learner identifiers, OPRF private keys, blinds,
unblinded intermediate values, non-matching elements, or per-participant
element sets across the release boundary. The release shape contains only the
job id, participant count, intersection elements, and transcript digest.

Production PSI remains blocked until external cryptographic review is complete
and the review evidence digest is bound to the deployed runtime artifact.

## Semantics

CC11 keeps PSI as an n-of-n mechanism. Every configured participant must submit
exactly one transcript and every transcript must be in `submitted` status.
Any `aborted` participant transcript blocks intersection and release.

Replay prevention is keyed by both job id and transcript digest. Duplicate job
ids or duplicate transcript digests fail closed before release. Release also
requires active participants from the CC10 lifecycle, policy approval,
governed release treatment, and completed external cryptographic review.

## Evidence

The machine-readable profile is
`spec/confidential/production-psi-upgrade-assurance.yaml`. The job contract is
`spec/confidential/psi-oprf-dh-voprf-job-contract.yaml`, deterministic boundary
vectors are in `spec/confidential/psi-oprf-dh-voprf-vectors.yaml`, benchmark
requirements are in `spec/confidential/psi-oprf-dh-voprf-benchmarks.yaml`, and
interop expectations are in `spec/confidential/psi-oprf-dh-voprf-interop.yaml`.

Rust tests cover positive intersection, duplicate element rejection, abort
rejection, replay rejection, review-pending release rejection, and vector
validation. Kani checks opaque intersection suppression and the fail-closed
release gate. Tamarin models participant admission, non-match suppression,
abort handling, replay-record evidence, release authorization, external review
gating, and absence of raw identifier publication.
