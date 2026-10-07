# Kiva Requirement Catalogs

This directory contains machine-readable requirement catalogs used to trace Kiva
contracts, tests, implementation code, and formal models back to standards.

- `ieee-39274-1-1-2025.yaml`: clause-indexed xAPI LRS requirements paraphrased
  from IEEE/ISO/IEC 39274-1-1-2025 for traceability.

OpenAPI files should reference catalog entries through `x-kiva-requirements`.
Requirements that OpenAPI cannot fully express remain linked from the contract
and must be discharged by conformance tests, property tests, Kani harnesses,
Tamarin models, or narrow F*/Low* models where appropriate.
