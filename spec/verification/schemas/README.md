# Verification Schemas

This directory defines machine-readable contracts for Kiva's internal formal
verification reports.

- `formal-verification-report.schema.json`: JSON emitted by Kani, Tamarin, and
  F* verification runners when report output is enabled.
- `formal-verification-evidence-manifest.schema.json`: manifest emitted by the
  formal verification evidence bundle generator.

Use `scripts/ci/validate_formal_verification_evidence_bundle.py` to validate a
complete bundle, including report schemas, manifest totals, artifact lists, and
checksums.

These reports are internal assurance evidence for selected implementation and
symbolic boundary properties. They are not IEEE conformance certificates and do
not prove full platform correctness.
