# Learner Data Portability Export

## Boundary

`GET /pds/data-portability-exports` is a governed PDS operation for
learner-initiated data portability. It is separate from the xAPI Statement
Resource. The format and operation do not expand the IEEE/ISO/IEC
39274-1-1:2025 xAPI LRS conformance claim.

The exact resource-server scope is `pds:read`. No other scope implies it.

## Subject To Actor Mapping

The authenticated token subject is mapped through the existing
`PdsSubjectAccountNamespace` rule. The canonical xAPI IFI is the account IFI
formed from the configured PDS account home page and the exact token subject.
Only Statements indexed by that exact IFI are eligible. The request cannot
name another subject. Missing, malformed, foreign, or otherwise ambiguous
identity associations are excluded fail-closed; Kiva does not infer identity
equivalence from names, email addresses, or other Actor fields.

## Page Format

Each response contains:

- `dataSubject`, `actor`, and `actorIfi`, identifying the governed partition;
- `generatedAt` and a durable `auditReference`;
- at most 100 `statements`, each containing `statementId`, `storedAt`, a
  `voided` marker, and the stored Statement JSON;
- optional `nextCursor` for the next stable storage-order page;
- subject-scoped PDS consent, approved data-product, lifecycle, retention,
  lifecycle-case, and governance metadata.

The cursor is opaque to clients. A cursor is valid only inside the same exact
subject partition. Foreign and unknown cursors are rejected.

## Statement Preservation

The export reads the accepted `serde_json::Value` or PostgreSQL `jsonb` value
directly from the Statement repository and inserts it into the response. It
does not deserialize and reserialize through an OpenAPI Statement DTO. This
preserves JSON values according to the export contract;
PostgreSQL `jsonb` does not preserve insignificant source whitespace or object
member byte ordering.

## Voided Statements

An owned target Statement remains in the portability record and is marked
`voided: true` when an accepted voiding Statement targets it. A voiding
Statement is included as a body only when its own Actor IFI also maps to the
requesting subject. A voiding Statement owned by another Actor is not exposed;
its effect is represented only by the target marker. This rule prevents a
voiding relationship from becoming a cross-subject raw Statement disclosure.
It does not change xAPI query or voiding semantics.

## Audit

Every authenticated export workflow records either
`portability-export-completed` or `portability-export-rejected` in the durable
PDS ledger. The completed event is committed before the response is released,
and its identifier is returned as `auditReference`. A malformed or foreign
cursor reaches the same workflow and records a rejected event. Requests that
cannot establish an authenticated subject are rejected by the resource-server
boundary and cannot create a subject-owned PDS audit event.
