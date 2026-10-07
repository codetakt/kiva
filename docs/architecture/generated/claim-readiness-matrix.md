# Generated Claim Readiness Matrix

This file is generated from the claim catalog, claim maturity catalog, CI
projection graph, and evidence-check manifest by
`scripts/ci/generate_claim_readiness_matrix.py`.

It maps contract boundaries to artifact paths, projection edges and check
definitions. Run-specific readiness assessments are kept locally under `target/`.

| Contract ID | Boundary | Evidence Mode | Artifact Paths | Projections | Check Definitions |
| --- | --- | --- | ---: | ---: | ---: |
| `reference-reporting-deployments` | `analytics-serving` | `projection-backed` | 7 | 4 | 9 |
| `governed-aggregate-analytics-serving` | `analytics-serving` | `projection-backed` | 8 | 2 | 5 |
| `governed-analytics-serving-boundary` | `analytics-serving` | `projection-backed` | 4 | 2 | 2 |
| `ieee-xapi-lrs-aligned-interface` | `xapi-lrs` | `projection-backed` | 5 | 3 | 8 |
| `pds-governed-metadata-platform` | `personal-data-store` | `projection-backed` | 6 | 4 | 7 |
| `learner-principal-consent-agency` | `personal-data-store` | `projection-backed` | 6 | 1 | 1 |
| `learner-initiated-data-portability` | `personal-data-store` | `projection-backed` | 8 | 3 | 6 |
| `production-oauth-oidc-idp-connectivity` | `identity-provider-deployment-adapter` | `projection-backed` | 6 | 1 | 2 |
| `confidential-provider-contract-boundary` | `confidential-computation` | `projection-backed` | 7 | 3 | 4 |
| `reference-provider-experiment-boundary` | `confidential-reference-primitives` | `path-only` | 6 | 1 | 1 |
| `evidence-gated-release-posture` | `verification-and-evidence` | `projection-backed` | 6 | 8 | 10 |
