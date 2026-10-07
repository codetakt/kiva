# PDS Operational Lifecycle Workflows

## Scope

M12 defines metadata-only operational workflows for retention schedules,
tombstone publication rules, and correction/dispute/tombstone cases in Kiva's
Personal Data Store extension.

This is not part of the IEEE/ISO/IEC 39274-1-1:2025 xAPI LRS conformance
surface. Accepted xAPI Statements remain immutable LRS records. Kiva records
operational lifecycle metadata around those records and PDS resources instead
of silently rewriting or deleting accepted Statement JSON.

## Implemented Boundary

M12 adds:

- `GET /pds/retention-policies`
- `POST /pds/retention-policies`
- `GET /pds/lifecycle-cases`
- `POST /pds/lifecycle-cases`
- PostgreSQL tables `kiva.pds_retention_policies` and
  `kiva.pds_lifecycle_cases`
- subject-owned metadata export fields `retentionPolicies` and
  `lifecycleCases`
- Kani target for the pure lifecycle-case status transition helper

Retention policies describe the resource kind, data class, retention period,
review interval, optional tombstone timing, publication rule, legal basis, and
human-readable policy description.

Lifecycle cases describe operational correction, dispute, and tombstone
workflows. A case is subject-owned, linked to a resource identifier, and may
reference a retention policy owned by the same subject.

## State Rules

New lifecycle cases must start in `open`.

Allowed status transitions are:

- `open` to `open` or `under-review`
- `under-review` to `under-review`, `resolved`, `rejected`, or `tombstoned`
- `resolved`, `rejected`, and `tombstoned` only to the same terminal status

The server additionally treats terminal case metadata as immutable. Repeating
the exact same terminal registration is idempotent, but changing terminal
resolution or workflow identity is rejected as a conflict.

Terminal statuses require a non-empty resolution. `tombstoned` requires
`case_type: tombstone`, and tombstone cases require an explicit
`tombstone_publication` rule.

## Claim Boundary

M12 makes Kiva able to claim an internal, metadata-only operational lifecycle
workflow boundary for the current PDS extension:

- retention schedules are represented and subject-scoped;
- tombstone publication rules are explicit metadata;
- correction, dispute, and tombstone cases have a strict state machine;
- metadata export includes operational workflow records;
- PDS responses continue to exclude raw Statement JSON, result payloads,
  aggregate payloads, noise values, shares, signatures, and signed envelopes.

M12 does not implement physical deletion, redaction execution, public
publication channels, legal hold automation, data-subject notification
delivery, or deployment-specific retention-law templates.
