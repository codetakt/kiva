# DH-OPRF PSI Runbook

## Runtime Selection

Select a maintained DH-OPRF PSI runtime that supports the
`ristretto255-sha512-voprf` suite and can be isolated behind Kiva's Rust
adapter boundary. Record the runtime artifact digest before deployment.

## External Review

Do not enable production release while
`external_crypto_review.status = pending`. Bind the completed review report to
the runtime artifact digest and update the review evidence digest only after
the reviewed artifact matches the deployed artifact.

## Participant Lifecycle

Register PSI participants through the CC10 participant lifecycle. Only active
participants may be admitted to a DH-OPRF PSI job. Suspended, frozen, removed,
missing, or wrong-owner participants must fail closed.

## Abort Handling

Treat any participant abort as terminal for the transcript. Do not compute or
release an intersection for a job with an aborted participant transcript.
Operators must create a replacement job id for a restart.

## Replay Handling

Retain the accepted job id and transcript digest in the deployment's generic
confidential job evidence before release. Reject later submissions with the
same job id or transcript digest. A dedicated PSI transcript PostgreSQL store
is not implemented in the current Kiva boundary.

## Release

Release only after intersection computation, active participant checks,
external cryptographic review completion, policy-authority approval, and
governed release treatment. Released metadata must not contain raw
identifiers, non-matching elements, participant input sets, OPRF private keys,
blinds, or unblinded intermediate values.

Do not register DH-OPRF PSI outputs as PDS data products until a dedicated PDS
source kind, HTTP contract, and PostgreSQL persistence path are implemented.

## Incident Response

Freeze affected participants through CC10 if runtime compromise, key exposure,
or transcript replay is suspected. Revoke the runtime artifact from deployment
selection, preserve transcript/audit metadata, and require a new external
review before reenabling production PSI release.
