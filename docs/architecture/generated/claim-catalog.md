# Generated Claim Catalog

This file is generated from `spec/architecture/claim-catalog.yaml` by
`scripts/ci/generate_claim_catalog.py`.

This index lists contract boundaries and their referenced artifacts.
Execution results and assurance assessments are kept locally under `target/`.

| Contract ID | Boundary | Referenced Artifacts |
| --- | --- | ---: |
| `reference-reporting-deployments` | `analytics-serving` | 7 |
| `governed-aggregate-analytics-serving` | `analytics-serving` | 8 |
| `governed-analytics-serving-boundary` | `analytics-serving` | 4 |
| `ieee-xapi-lrs-aligned-interface` | `xapi-lrs` | 5 |
| `pds-governed-metadata-platform` | `personal-data-store` | 6 |
| `learner-principal-consent-agency` | `personal-data-store` | 6 |
| `learner-initiated-data-portability` | `personal-data-store` | 8 |
| `production-oauth-oidc-idp-connectivity` | `identity-provider-deployment-adapter` | 6 |
| `confidential-provider-contract-boundary` | `confidential-computation` | 7 |
| `reference-provider-experiment-boundary` | `confidential-reference-primitives` | 6 |
| `evidence-gated-release-posture` | `verification-and-evidence` | 6 |
