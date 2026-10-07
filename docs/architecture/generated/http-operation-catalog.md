# Generated HTTP Operation Catalog

This file is generated from `spec/openapi/kiva-analytics.yaml`, `spec/openapi/kiva-xapi.yaml`, `spec/openapi/kiva-pds.yaml`, `spec/openapi/kiva-confidential.yaml` by
`scripts/ci/generate_http_operation_catalog.py`.

It is review evidence for the complete HTTP operation projection from OpenAPI.
Application workflow and PDS family ownership are projected from the generated
surface catalog bundle at `generated/architecture/surface-catalog.yaml`.

| OpenAPI Contract | Method | Path | Operation ID | Required Scopes | Tags |
| --- | --- | --- | --- | --- | --- |
| `spec/openapi/kiva-analytics.yaml` | `GET` | `/analytics/products` | `getAnalyticsProducts` | `analytics:read` | `Analytics` |
| `spec/openapi/kiva-analytics.yaml` | `GET` | `/analytics/products/{dataProductId}` | `getAnalyticsProduct` | `analytics:read` | `Analytics` |
| `spec/openapi/kiva-analytics.yaml` | `GET` | `/analytics/products/{dataProductId}/aggregate` | `getAnalyticsProductAggregate` | `analytics:read` | `Analytics` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/about` | `getAbout` |  | `About` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/about` | `headAbout` |  | `About` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/activities` | `getActivity` | `xapi:read` | `Activities` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/activities` | `headActivity` | `xapi:read` | `Activities` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/activities/profile` | `getActivityProfiles` | `xapi:read` | `Activities`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `PUT` | `/activities/profile` | `putActivityProfile` | `xapi:write` | `Activities`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `POST` | `/activities/profile` | `postActivityProfile` | `xapi:write` | `Activities`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `DELETE` | `/activities/profile` | `deleteActivityProfile` | `xapi:write` | `Activities`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/activities/profile` | `headActivityProfile` | `xapi:read` | `Activities`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/activities/state` | `getStateDocuments` | `xapi:read` | `State`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `PUT` | `/activities/state` | `putStateDocument` | `xapi:write` | `State`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `POST` | `/activities/state` | `postStateDocument` | `xapi:write` | `State`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `DELETE` | `/activities/state` | `deleteStateDocuments` | `xapi:write` | `State`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/activities/state` | `headStateDocuments` | `xapi:read` | `State`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/agents` | `getAgentPerson` | `xapi:read` | `Agents` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/agents` | `headAgentPerson` | `xapi:read` | `Agents` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/agents/profile` | `getAgentProfiles` | `xapi:read` | `Agents`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `PUT` | `/agents/profile` | `putAgentProfile` | `xapi:write` | `Agents`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `POST` | `/agents/profile` | `postAgentProfile` | `xapi:write` | `Agents`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `DELETE` | `/agents/profile` | `deleteAgentProfile` | `xapi:write` | `Agents`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/agents/profile` | `headAgentProfile` | `xapi:read` | `Agents`, `Documents` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/statements` | `getStatements` | `xapi:read` | `Statements` |
| `spec/openapi/kiva-xapi.yaml` | `PUT` | `/statements` | `putStatement` | `xapi:write` | `Statements` |
| `spec/openapi/kiva-xapi.yaml` | `POST` | `/statements` | `postStatements` | `xapi:write` | `Statements` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/statements` | `headStatements` | `xapi:read` | `Statements` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/access-audit-events` | `getPdsAccessAuditEvents` | `pds:audit` | `Access Decisions` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/access-checks` | `postPdsAccessCheck` | `pds:read` | `Access Decisions` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/consent-grants` | `getPdsConsentGrants` | `pds:read` | `Consent Grants` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/consent-grants` | `postPdsConsentGrant` | `pds:delegate` | `Consent Grants` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/consent-revocations` | `postPdsConsentRevocation` | `pds:revoke` | `Consent Grants` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/data-portability-exports` | `getPdsDataPortabilityExports` | `pds:read` | `Lifecycle` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/data-product-reads` | `postPdsDataProductRead` | `pds:read` | `Data Products` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/data-products` | `getPdsDataProducts` | `pds:read` | `Data Products` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/data-products` | `postPdsDataProduct` | `pds:write` | `Data Products` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/governance-profiles` | `getPdsGovernanceProfiles` | `pds:read` | `Governance` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/governance-profiles` | `postPdsGovernanceProfile` | `pds:write` | `Governance` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/lifecycle-cases` | `getPdsLifecycleCases` | `pds:read` | `Lifecycle` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/lifecycle-cases` | `postPdsLifecycleCase` | `pds:write` | `Lifecycle` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/lifecycle-events` | `getPdsLifecycleEvents` | `pds:read` | `Lifecycle` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/lifecycle-events` | `postPdsLifecycleEvent` | `pds:write` | `Lifecycle` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/metadata-exports` | `getPdsMetadataExports` | `pds:read` | `Lifecycle` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/provider-bindings` | `getPdsProviderBindings` | `pds:read` | `Provider Bindings` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/provider-bindings` | `postPdsProviderBinding` | `pds:write` | `Provider Bindings` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/record-inventory` | `getPdsRecordInventory` | `pds:read` | `Lifecycle` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/retention-policies` | `getPdsRetentionPolicies` | `pds:read` | `Lifecycle` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/retention-policies` | `postPdsRetentionPolicy` | `pds:write` | `Lifecycle` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-aggregates` | `postConfidentialAdditiveAggregate` | `confidential:compute` | `Confidential Aggregates` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/additive-share-replay-records` | `getConfidentialAdditiveShareReplayRecords` | `confidential:read` | `Share Submissions` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-share-signing-key-revocations` | `postConfidentialShareSigningKeyRevocation` | `confidential:write` | `Share Signing Keys` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-share-signing-key-rotations` | `postConfidentialShareSigningKeyRotation` | `confidential:write` | `Share Signing Keys` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/additive-share-signing-keys` | `getConfidentialShareSigningKeys` | `confidential:read` | `Share Signing Keys` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-share-signing-keys` | `postConfidentialShareSigningKey` | `confidential:write` | `Share Signing Keys` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/additive-share-submissions` | `getConfidentialAdditiveShareSubmissions` | `confidential:read` | `Share Submissions` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-share-submissions` | `postConfidentialAdditiveShareSubmission` | `confidential:write` | `Share Submissions` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/differential-privacy-budget-spends` | `postConfidentialDifferentialPrivacyBudgetSpend` | `confidential:write` | `Differential Privacy Budgets` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/differential-privacy-budgets` | `getConfidentialDifferentialPrivacyBudgets` | `confidential:read` | `Differential Privacy Budgets` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/differential-privacy-budgets` | `postConfidentialDifferentialPrivacyBudget` | `confidential:write` | `Differential Privacy Budgets` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/differential-privacy-composition-summaries` | `getConfidentialDifferentialPrivacyCompositionSummaries` | `confidential:read` | `Differential Privacy Budgets` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/differential-privacy-releases` | `getConfidentialDifferentialPrivacyReleases` | `confidential:read` | `Differential Privacy Releases` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/differential-privacy-releases` | `postConfidentialDifferentialPrivacyRelease` | `confidential:release` | `Differential Privacy Releases` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/differential-privacy-sampled-releases` | `postConfidentialDifferentialPrivacySampledRelease` | `confidential:release` | `Differential Privacy Releases` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/job-audit-events` | `getConfidentialJobAuditEvents` | `confidential:read` | `Confidential Jobs` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/job-transitions` | `postConfidentialJobTransition` | `confidential:write` | `Confidential Jobs` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/jobs` | `getConfidentialJobs` | `confidential:read` | `Confidential Jobs` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/jobs` | `postConfidentialJob` | `confidential:write` | `Confidential Jobs` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/participant-audit-events` | `getConfidentialParticipantAuditEvents` | `confidential:read` | `Confidential Participants` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/participant-transitions` | `postConfidentialParticipantTransition` | `confidential:write` | `Confidential Participants` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/participants` | `getConfidentialParticipants` | `confidential:read` | `Confidential Participants` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/participants` | `postConfidentialParticipant` | `confidential:write` | `Confidential Participants` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/shamir-threshold-reconstructions` | `getConfidentialShamirThresholdReconstructions` | `confidential:read` | `Shamir Threshold Reconstructions` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/shamir-threshold-reconstructions` | `postConfidentialShamirThresholdReconstruction` | `confidential:compute` | `Shamir Threshold Reconstructions` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/threshold-release-approvals` | `postConfidentialThresholdReleaseApproval` | `confidential:release` | `Threshold Releases` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/threshold-release-requests` | `getConfidentialThresholdReleaseRequests` | `confidential:read` | `Threshold Releases` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/threshold-release-requests` | `postConfidentialThresholdReleaseRequest` | `confidential:write` | `Threshold Releases` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/threshold-releases` | `postConfidentialThresholdRelease` | `confidential:release` | `Threshold Releases` |
