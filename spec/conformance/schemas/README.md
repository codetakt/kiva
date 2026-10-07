# Conformance Schemas

This directory defines the machine-readable contracts for the Kiva xAPI LRS
conformance aid.

- `conformance-run-report.schema.json`: JSON emitted by
  `xapi-lrs-conformance --json`.
- `conformance-case-matrix-report.schema.json`: JSON emitted by
  `scripts/ci/report_conformance_case_matrix.py --json`.
- `conformance-coverage-report.schema.json`: JSON emitted by
  `scripts/ci/report_conformance_coverage.py --json`.
- `adl-compatibility-report.schema.json`: JSON emitted by
  `scripts/ci/report_adl_compatibility.py --json`.
- `requirement-catalog-audit-report.schema.json`: JSON emitted by
  `scripts/ci/audit_requirement_catalog.py --json`, including local IEEE
  source-clause and triage evidence when a reference checkout is available.
- `postgres-verification-report.schema.json`: JSON emitted by
  `scripts/ci/run_postgres_verification.sh` when
  `KIVA_POSTGRES_VERIFICATION_JSON_REPORT` is set.
- `conformance-evidence-manifest.schema.json`: manifest emitted by
  `scripts/ci/generate_conformance_evidence.sh` for release-candidate xAPI
  conformance evidence bundles.
- `endpoint-profile.schema.json`: YAML/JSON fixture contract for deterministic
  endpoint behavior required by otherwise implementation-specific HTTP cases.
- `auth-profile.schema.json`: YAML/JSON fixture contract for environment-backed
  authentication profiles used by credential-sensitive HTTP cases.

All schemas currently use `schema_version: 1`. Increment the schema version for
breaking changes to field names, required fields, enum values, or value
semantics.
