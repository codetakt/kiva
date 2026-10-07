# Confidential Job Lifecycle

Kiva's CC9 boundary turns the selected CC8 confidential mechanisms into
durable, auditable jobs. The lifecycle is metadata-only: it records job
identity, owner, mechanism profile, participant set, policy binding, retry
counter, deadline, scheduler/admission state, and evidence references, but it
does not store raw xAPI payloads, submitted shares, plaintexts, secrets,
signatures, or differential privacy noise samples in the job ledger.

## Model

Each job is keyed by `(tenant_id, job_id)` and starts in `draft`. The persisted
model contains:

- `owner_subject`, derived from the authenticated OAuth/OIDC principal;
- `mechanism` and `mechanism_profile`;
- `workload_id`, usually from the CC8 selected workload profile;
- sorted unique `participant_subjects`;
- `policy_binding_id`;
- strict `status`;
- `retry_count` and `max_retry_count`;
- `deadline_at`;
- `admission_quota_key`;
- `evidence_refs`;
- `created_at` and `updated_at`.

The public API returns only this metadata. Job audit events contain event ID,
job ID, previous/new status, actor subject, reason, evidence references, and
timestamp.

## State Machine

The strict status set is:

- `draft`
- `admitted`
- `running`
- `awaitingRelease`
- `released`
- `failed`
- `cancelled`
- `expired`
- `retired`

Allowed forward transitions are:

- `draft -> admitted`
- `draft -> cancelled`
- `draft|admitted|running|awaitingRelease -> expired`
- `admitted -> running`
- `admitted -> cancelled`
- `running -> awaitingRelease`
- `running -> failed`
- `running -> cancelled`
- `awaitingRelease -> released`
- `awaitingRelease -> failed`
- `awaitingRelease -> cancelled`
- `released|failed|cancelled|expired -> retired`

Same-state transitions are idempotent no-ops. Reopening `released`, `failed`,
`cancelled`, `expired`, or `retired` jobs is rejected.

## Admission And Scheduling Boundary

Kiva currently implements a bounded scheduler admission profile rather than a
distributed worker queue. The HTTP transition to `admitted` enforces:

- at least two participant subjects;
- non-empty policy binding;
- future deadline;
- no more than 64 active jobs for the same `admission_quota_key`;
- retry budget availability.

The transition to `running` additionally enforces a per-tenant bound of 16
running jobs. Deadline expiry is handled by rejecting non-expiry transitions
after `deadline_at` and admitting `expired` only after the deadline has elapsed.

## Persistence

PostgreSQL persists:

- `kiva.confidential_jobs`
- `kiva.confidential_job_audit_events`

Job creation and its initial audit event happen in one transaction. Job
transition uses `SELECT ... FOR UPDATE`, updates the job row, and inserts the
audit event in the same transaction. Duplicate creates are idempotent when the
metadata is identical and conflict when it differs. Duplicate transitions to
the same status are idempotent no-ops.

## Verification

The lifecycle has:

- Rust tests for pure state-machine rules, admission fail-closed behavior,
  memory repository behavior, and ignored PostgreSQL transaction tests;
- Kani harnesses for the pure lifecycle transition table and admission gate;
- a Tamarin boundary model for job creation, transition ordering, release
  ordering, audit-event generation, and raw-payload non-publication;
- OpenAPI contract validation for paths, scopes, request/response schemas, and
  metadata-only schema fields;
- CC9 assurance profile validation in
  `scripts/ci/validate_confidential_job_lifecycle_assurance.py`.

## Non-Claims

CC9 does not claim a production distributed queue, malicious-secure MPC,
production PSI, vendor TEE quote verification, production FHE, DKG, proactive
share refresh, or external certification. CC10 now supplies only the
metadata-only participant and key lifecycle substrate; DKG, proactive refresh,
and stronger production PSI/FHE/TEE/MPC claims remain outside the CC9/CC10
implementation boundary unless a later deployment binds the required provider
and external-review evidence.
