# Generated Route Coverage

This file is generated from `generated/architecture/surface-catalog.yaml` by
`scripts/ci/generate_route_coverage.py`.

The route coverage projection checks that every OpenAPI operation in the
surface catalog has a matching Axum route registration. `HEAD` operations are
covered through the corresponding `GET` route registration.

The surface catalog is validated against `generated/http-operation-catalog.evidence.json` before
route coverage is rendered.

| OpenAPI Contract | Method | Path | Operation ID | Route File | Route Anchor | Method Binding |
| --- | --- | --- | --- | --- | --- | --- |
| `spec/openapi/kiva-analytics.yaml` | `GET` | `/analytics/products` | `getAnalyticsProducts` | `crates/kiva-server/src/surfaces/analytics/routes.rs` | `ANALYTICS_PRODUCTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-analytics.yaml` | `GET` | `/analytics/products/{dataProductId}` | `getAnalyticsProduct` | `crates/kiva-server/src/surfaces/analytics/routes.rs` | `ANALYTICS_PRODUCT_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-analytics.yaml` | `GET` | `/analytics/products/{dataProductId}/aggregate` | `getAnalyticsProductAggregate` | `crates/kiva-server/src/surfaces/analytics/routes.rs` | `ANALYTICS_PRODUCT_AGGREGATE_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/about` | `getAbout` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/about"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/about` | `headAbout` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/about"` | `implicit-head-via-get` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/activities` | `getActivity` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/activities"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/activities` | `headActivity` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/activities"` | `implicit-head-via-get` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/activities/profile` | `getActivityProfiles` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/activities/profile"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `PUT` | `/activities/profile` | `putActivityProfile` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/activities/profile"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `POST` | `/activities/profile` | `postActivityProfile` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/activities/profile"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `DELETE` | `/activities/profile` | `deleteActivityProfile` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/activities/profile"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/activities/profile` | `headActivityProfile` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/activities/profile"` | `implicit-head-via-get` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/activities/state` | `getStateDocuments` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/activities/state"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `PUT` | `/activities/state` | `putStateDocument` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/activities/state"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `POST` | `/activities/state` | `postStateDocument` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/activities/state"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `DELETE` | `/activities/state` | `deleteStateDocuments` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/activities/state"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/activities/state` | `headStateDocuments` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/activities/state"` | `implicit-head-via-get` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/agents` | `getAgentPerson` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/agents"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/agents` | `headAgentPerson` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/agents"` | `implicit-head-via-get` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/agents/profile` | `getAgentProfiles` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/agents/profile"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `PUT` | `/agents/profile` | `putAgentProfile` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/agents/profile"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `POST` | `/agents/profile` | `postAgentProfile` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/agents/profile"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `DELETE` | `/agents/profile` | `deleteAgentProfile` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/agents/profile"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/agents/profile` | `headAgentProfile` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/agents/profile"` | `implicit-head-via-get` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/statements` | `getStatements` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/statements"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `PUT` | `/statements` | `putStatement` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/statements"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `POST` | `/statements` | `postStatements` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/statements"` | `explicit-method-router` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/statements` | `headStatements` | `crates/kiva-server/src/surfaces/xapi/routes.rs` | `"/statements"` | `implicit-head-via-get` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/access-audit-events` | `getPdsAccessAuditEvents` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_ACCESS_AUDIT_EVENTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/access-checks` | `postPdsAccessCheck` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_ACCESS_CHECKS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/consent-grants` | `getPdsConsentGrants` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_CONSENT_GRANTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/consent-grants` | `postPdsConsentGrant` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_CONSENT_GRANTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/consent-revocations` | `postPdsConsentRevocation` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_CONSENT_REVOCATIONS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/data-portability-exports` | `getPdsDataPortabilityExports` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_DATA_PORTABILITY_EXPORTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/data-product-reads` | `postPdsDataProductRead` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_DATA_PRODUCT_READS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/data-products` | `getPdsDataProducts` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_DATA_PRODUCTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/data-products` | `postPdsDataProduct` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_DATA_PRODUCTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/governance-profiles` | `getPdsGovernanceProfiles` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_GOVERNANCE_PROFILES_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/governance-profiles` | `postPdsGovernanceProfile` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_GOVERNANCE_PROFILES_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/lifecycle-cases` | `getPdsLifecycleCases` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_LIFECYCLE_CASES_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/lifecycle-cases` | `postPdsLifecycleCase` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_LIFECYCLE_CASES_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/lifecycle-events` | `getPdsLifecycleEvents` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_LIFECYCLE_EVENTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/lifecycle-events` | `postPdsLifecycleEvent` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_LIFECYCLE_EVENTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/metadata-exports` | `getPdsMetadataExports` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_METADATA_EXPORTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/provider-bindings` | `getPdsProviderBindings` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_PROVIDER_BINDINGS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/provider-bindings` | `postPdsProviderBinding` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_PROVIDER_BINDINGS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/record-inventory` | `getPdsRecordInventory` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_RECORD_INVENTORY_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/retention-policies` | `getPdsRetentionPolicies` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_RETENTION_POLICIES_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/retention-policies` | `postPdsRetentionPolicy` | `crates/kiva-server/src/surfaces/pds/routes.rs` | `PDS_RETENTION_POLICIES_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-aggregates` | `postConfidentialAdditiveAggregate` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_ADDITIVE_AGGREGATES_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/additive-share-replay-records` | `getConfidentialAdditiveShareReplayRecords` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_ADDITIVE_SHARE_REPLAY_RECORDS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-share-signing-key-revocations` | `postConfidentialShareSigningKeyRevocation` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_SHARE_SIGNING_KEY_REVOCATIONS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-share-signing-key-rotations` | `postConfidentialShareSigningKeyRotation` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_SHARE_SIGNING_KEY_ROTATIONS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/additive-share-signing-keys` | `getConfidentialShareSigningKeys` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_SHARE_SIGNING_KEYS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-share-signing-keys` | `postConfidentialShareSigningKey` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_SHARE_SIGNING_KEYS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/additive-share-submissions` | `getConfidentialAdditiveShareSubmissions` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_ADDITIVE_SHARE_SUBMISSIONS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-share-submissions` | `postConfidentialAdditiveShareSubmission` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_ADDITIVE_SHARE_SUBMISSIONS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/differential-privacy-budget-spends` | `postConfidentialDifferentialPrivacyBudgetSpend` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_DIFFERENTIAL_PRIVACY_BUDGET_SPENDS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/differential-privacy-budgets` | `getConfidentialDifferentialPrivacyBudgets` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_DIFFERENTIAL_PRIVACY_BUDGETS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/differential-privacy-budgets` | `postConfidentialDifferentialPrivacyBudget` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_DIFFERENTIAL_PRIVACY_BUDGETS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/differential-privacy-composition-summaries` | `getConfidentialDifferentialPrivacyCompositionSummaries` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_DIFFERENTIAL_PRIVACY_COMPOSITION_SUMMARIES_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/differential-privacy-releases` | `getConfidentialDifferentialPrivacyReleases` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_DIFFERENTIAL_PRIVACY_RELEASES_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/differential-privacy-releases` | `postConfidentialDifferentialPrivacyRelease` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_DIFFERENTIAL_PRIVACY_RELEASES_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/differential-privacy-sampled-releases` | `postConfidentialDifferentialPrivacySampledRelease` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_DIFFERENTIAL_PRIVACY_SAMPLED_RELEASES_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/job-audit-events` | `getConfidentialJobAuditEvents` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_JOB_AUDIT_EVENTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/job-transitions` | `postConfidentialJobTransition` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_JOB_TRANSITIONS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/jobs` | `getConfidentialJobs` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_JOBS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/jobs` | `postConfidentialJob` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_JOBS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/participant-audit-events` | `getConfidentialParticipantAuditEvents` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_PARTICIPANT_AUDIT_EVENTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/participant-transitions` | `postConfidentialParticipantTransition` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_PARTICIPANT_TRANSITIONS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/participants` | `getConfidentialParticipants` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_PARTICIPANTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/participants` | `postConfidentialParticipant` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_PARTICIPANTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/shamir-threshold-reconstructions` | `getConfidentialShamirThresholdReconstructions` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_SHAMIR_THRESHOLD_RECONSTRUCTIONS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/shamir-threshold-reconstructions` | `postConfidentialShamirThresholdReconstruction` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_SHAMIR_THRESHOLD_RECONSTRUCTIONS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/threshold-release-approvals` | `postConfidentialThresholdReleaseApproval` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_THRESHOLD_RELEASE_APPROVALS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/threshold-release-requests` | `getConfidentialThresholdReleaseRequests` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_THRESHOLD_RELEASE_REQUESTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/threshold-release-requests` | `postConfidentialThresholdReleaseRequest` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_THRESHOLD_RELEASE_REQUESTS_PATH` | `explicit-method-router` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/threshold-releases` | `postConfidentialThresholdRelease` | `crates/kiva-server/src/surfaces/confidential/routes.rs` | `CONFIDENTIAL_THRESHOLD_RELEASES_PATH` | `explicit-method-router` |
