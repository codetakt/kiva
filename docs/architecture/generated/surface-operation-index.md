# Generated Surface Operation Index

This file is generated from `generated/architecture/surface-catalog.yaml` by
`scripts/ci/generate_surface_operation_index.py`.

The surface catalog is the canonical review surface for public HTTP operations,
application workflow ownership, OAuth/OIDC scopes, PDS route contexts, and PDS
resource-family ownership. It is validated against
`generated/http-operation-catalog.evidence.json` before this index is rendered.

| OpenAPI Contract | Method | Path | Operation ID | Application Operation | Workflow | PDS Family | Route Context | Required Scopes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `spec/openapi/kiva-analytics.yaml` | `GET` | `/analytics/products` | `getAnalyticsProducts` | `analytics-list-products` | `analytics.list_products` |  |  | `analytics:read` |
| `spec/openapi/kiva-analytics.yaml` | `GET` | `/analytics/products/{dataProductId}` | `getAnalyticsProduct` | `analytics-read-product` | `analytics.read_product` |  |  | `analytics:read` |
| `spec/openapi/kiva-analytics.yaml` | `GET` | `/analytics/products/{dataProductId}/aggregate` | `getAnalyticsProductAggregate` | `analytics-read-aggregate` | `analytics.read_aggregate` |  |  | `analytics:read` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/about` | `getAbout` |  |  |  |  |  |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/about` | `headAbout` |  |  |  |  |  |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/activities` | `getActivity` |  |  |  |  | `xapi:read` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/activities` | `headActivity` |  |  |  |  | `xapi:read` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/activities/profile` | `getActivityProfiles` |  |  |  |  | `xapi:read` |
| `spec/openapi/kiva-xapi.yaml` | `PUT` | `/activities/profile` | `putActivityProfile` |  |  |  |  | `xapi:write` |
| `spec/openapi/kiva-xapi.yaml` | `POST` | `/activities/profile` | `postActivityProfile` |  |  |  |  | `xapi:write` |
| `spec/openapi/kiva-xapi.yaml` | `DELETE` | `/activities/profile` | `deleteActivityProfile` |  |  |  |  | `xapi:write` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/activities/profile` | `headActivityProfile` |  |  |  |  | `xapi:read` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/activities/state` | `getStateDocuments` |  |  |  |  | `xapi:read` |
| `spec/openapi/kiva-xapi.yaml` | `PUT` | `/activities/state` | `putStateDocument` |  |  |  |  | `xapi:write` |
| `spec/openapi/kiva-xapi.yaml` | `POST` | `/activities/state` | `postStateDocument` |  |  |  |  | `xapi:write` |
| `spec/openapi/kiva-xapi.yaml` | `DELETE` | `/activities/state` | `deleteStateDocuments` |  |  |  |  | `xapi:write` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/activities/state` | `headStateDocuments` |  |  |  |  | `xapi:read` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/agents` | `getAgentPerson` |  |  |  |  | `xapi:read` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/agents` | `headAgentPerson` |  |  |  |  | `xapi:read` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/agents/profile` | `getAgentProfiles` |  |  |  |  | `xapi:read` |
| `spec/openapi/kiva-xapi.yaml` | `PUT` | `/agents/profile` | `putAgentProfile` |  |  |  |  | `xapi:write` |
| `spec/openapi/kiva-xapi.yaml` | `POST` | `/agents/profile` | `postAgentProfile` |  |  |  |  | `xapi:write` |
| `spec/openapi/kiva-xapi.yaml` | `DELETE` | `/agents/profile` | `deleteAgentProfile` |  |  |  |  | `xapi:write` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/agents/profile` | `headAgentProfile` |  |  |  |  | `xapi:read` |
| `spec/openapi/kiva-xapi.yaml` | `GET` | `/statements` | `getStatements` |  |  |  |  | `xapi:read` |
| `spec/openapi/kiva-xapi.yaml` | `PUT` | `/statements` | `putStatement` |  |  |  |  | `xapi:write` |
| `spec/openapi/kiva-xapi.yaml` | `POST` | `/statements` | `postStatements` |  |  |  |  | `xapi:write` |
| `spec/openapi/kiva-xapi.yaml` | `HEAD` | `/statements` | `headStatements` |  |  |  |  | `xapi:read` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/access-audit-events` | `getPdsAccessAuditEvents` | `pds-list-access-audit-events` | `pds.list_access_audit_events` | `access` | `pds` | `pds:audit` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/access-checks` | `postPdsAccessCheck` | `pds-evaluate-access-check` | `pds.evaluate_access_check` | `access` | `pds` | `pds:read` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/consent-grants` | `getPdsConsentGrants` | `pds-list-consent-grants` | `pds.list_consent_grants` | `consent` | `pds` | `pds:read` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/consent-grants` | `postPdsConsentGrant` | `pds-register-consent-grant` | `pds.register_consent_grant` | `consent` | `pds-application` | `pds:delegate` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/consent-revocations` | `postPdsConsentRevocation` | `pds-revoke-consent-grant` | `pds.revoke_consent_grant` | `consent` | `pds-application` | `pds:revoke` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/data-portability-exports` | `getPdsDataPortabilityExports` | `pds-export-portability` | `pds.export_portability` | `inventory-export` | `pds-application` | `pds:read` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/data-product-reads` | `postPdsDataProductRead` | `pds-read-data-product` | `pds.read_data_product` | `data-products` | `pds` | `pds:read` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/data-products` | `getPdsDataProducts` | `pds-list-data-products` | `pds.list_data_products` | `data-products` | `pds` | `pds:read` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/data-products` | `postPdsDataProduct` | `pds-register-data-product` | `pds.register_data_product` | `data-products` | `pds-application` | `pds:write` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/governance-profiles` | `getPdsGovernanceProfiles` | `pds-list-governance-profiles` | `pds.list_governance_profiles` | `governance` | `pds` | `pds:read` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/governance-profiles` | `postPdsGovernanceProfile` | `pds-register-governance-profile` | `pds.register_governance_profile` | `governance` | `pds-application` | `pds:write` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/lifecycle-cases` | `getPdsLifecycleCases` | `pds-list-lifecycle-cases` | `pds.list_lifecycle_cases` | `lifecycle` | `pds` | `pds:read` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/lifecycle-cases` | `postPdsLifecycleCase` | `pds-register-lifecycle-case` | `pds.register_lifecycle_case` | `lifecycle` | `pds-application` | `pds:write` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/lifecycle-events` | `getPdsLifecycleEvents` | `pds-list-lifecycle-events` | `pds.list_lifecycle_events` | `lifecycle` | `pds` | `pds:read` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/lifecycle-events` | `postPdsLifecycleEvent` | `pds-register-lifecycle-event` | `pds.register_lifecycle_event` | `lifecycle` | `pds-application` | `pds:write` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/metadata-exports` | `getPdsMetadataExports` | `pds-export-metadata` | `pds.export_metadata` | `inventory-export` | `pds-application` | `pds:read` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/provider-bindings` | `getPdsProviderBindings` | `pds-list-provider-bindings` | `pds.list_provider_bindings` | `provider-bindings` | `pds` | `pds:read` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/provider-bindings` | `postPdsProviderBinding` | `pds-register-provider-binding` | `pds.register_provider_binding` | `provider-bindings` | `pds` | `pds:write` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/record-inventory` | `getPdsRecordInventory` | `pds-read-record-inventory` | `pds.read_record_inventory` | `inventory-export` | `pds-application` | `pds:read` |
| `spec/openapi/kiva-pds.yaml` | `GET` | `/pds/retention-policies` | `getPdsRetentionPolicies` | `pds-list-retention-policies` | `pds.list_retention_policies` | `lifecycle` | `pds` | `pds:read` |
| `spec/openapi/kiva-pds.yaml` | `POST` | `/pds/retention-policies` | `postPdsRetentionPolicy` | `pds-register-retention-policy` | `pds.register_retention_policy` | `lifecycle` | `pds-application` | `pds:write` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-aggregates` | `postConfidentialAdditiveAggregate` | `confidential-release-signed-additive-aggregate` | `confidential.release_signed_additive_aggregate` |  |  | `confidential:compute` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/additive-share-replay-records` | `getConfidentialAdditiveShareReplayRecords` | `confidential-list-additive-replay-records` | `confidential.list_replay_records` |  |  | `confidential:read` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-share-signing-key-revocations` | `postConfidentialShareSigningKeyRevocation` | `confidential-revoke-share-signing-key` | `confidential.revoke_share_signing_key` |  |  | `confidential:write` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-share-signing-key-rotations` | `postConfidentialShareSigningKeyRotation` | `confidential-rotate-share-signing-key` | `confidential.rotate_share_signing_key` |  |  | `confidential:write` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/additive-share-signing-keys` | `getConfidentialShareSigningKeys` | `confidential-list-share-signing-keys` | `confidential.list_share_signing_keys` |  |  | `confidential:read` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-share-signing-keys` | `postConfidentialShareSigningKey` | `confidential-register-share-signing-key` | `confidential.register_share_signing_key` |  |  | `confidential:write` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/additive-share-submissions` | `getConfidentialAdditiveShareSubmissions` | `confidential-list-submitted-shares` | `confidential.list_submitted_shares` |  |  | `confidential:read` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/additive-share-submissions` | `postConfidentialAdditiveShareSubmission` | `confidential-submit-additive-share` | `confidential.submit_additive_share` |  |  | `confidential:write` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/differential-privacy-budget-spends` | `postConfidentialDifferentialPrivacyBudgetSpend` | `confidential-spend-differential-privacy-budget` | `confidential.spend_differential_privacy_budget` |  |  | `confidential:write` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/differential-privacy-budgets` | `getConfidentialDifferentialPrivacyBudgets` | `confidential-list-differential-privacy-budgets` | `confidential.list_differential_privacy_budgets` |  |  | `confidential:read` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/differential-privacy-budgets` | `postConfidentialDifferentialPrivacyBudget` | `confidential-register-differential-privacy-budget` | `confidential.register_differential_privacy_budget` |  |  | `confidential:write` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/differential-privacy-composition-summaries` | `getConfidentialDifferentialPrivacyCompositionSummaries` | `confidential-list-differential-privacy-composition-summaries` | `confidential.list_differential_privacy_composition_summaries` |  |  | `confidential:read` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/differential-privacy-releases` | `getConfidentialDifferentialPrivacyReleases` | `confidential-list-differential-privacy-releases` | `confidential.list_differential_privacy_releases` |  |  | `confidential:read` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/differential-privacy-releases` | `postConfidentialDifferentialPrivacyRelease` | `confidential-release-differential-privacy-aggregate` | `confidential.release_differential_privacy_aggregate` |  |  | `confidential:release` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/differential-privacy-sampled-releases` | `postConfidentialDifferentialPrivacySampledRelease` | `confidential-release-sampled-differential-privacy-aggregate` | `confidential.release_sampled_differential_privacy_aggregate` |  |  | `confidential:release` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/job-audit-events` | `getConfidentialJobAuditEvents` | `confidential-list-job-audit-events` | `confidential.list_job_audit_events` |  |  | `confidential:read` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/job-transitions` | `postConfidentialJobTransition` | `confidential-transition-job` | `confidential.transition_job` |  |  | `confidential:write` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/jobs` | `getConfidentialJobs` | `confidential-list-jobs` | `confidential.list_jobs` |  |  | `confidential:read` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/jobs` | `postConfidentialJob` | `confidential-register-job` | `confidential.register_job` |  |  | `confidential:write` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/participant-audit-events` | `getConfidentialParticipantAuditEvents` | `confidential-list-participant-audit-events` | `confidential.list_participant_audit_events` |  |  | `confidential:read` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/participant-transitions` | `postConfidentialParticipantTransition` | `confidential-transition-participant` | `confidential.transition_participant` |  |  | `confidential:write` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/participants` | `getConfidentialParticipants` | `confidential-list-participants` | `confidential.list_participants` |  |  | `confidential:read` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/participants` | `postConfidentialParticipant` | `confidential-register-participant` | `confidential.register_participant` |  |  | `confidential:write` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/shamir-threshold-reconstructions` | `getConfidentialShamirThresholdReconstructions` | `confidential-list-shamir-threshold-reconstructions` | `confidential.list_shamir_threshold_reconstructions` |  |  | `confidential:read` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/shamir-threshold-reconstructions` | `postConfidentialShamirThresholdReconstruction` | `confidential-reconstruct-shamir-threshold-secret` | `confidential.reconstruct_shamir_threshold_secret` |  |  | `confidential:compute` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/threshold-release-approvals` | `postConfidentialThresholdReleaseApproval` | `confidential-approve-threshold-release` | `confidential.approve_threshold_release` |  |  | `confidential:release` |
| `spec/openapi/kiva-confidential.yaml` | `GET` | `/confidential/threshold-release-requests` | `getConfidentialThresholdReleaseRequests` | `confidential-list-threshold-release-requests` | `confidential.list_threshold_release_requests` |  |  | `confidential:read` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/threshold-release-requests` | `postConfidentialThresholdReleaseRequest` | `confidential-register-threshold-release-request` | `confidential.register_threshold_release_request` |  |  | `confidential:write` |
| `spec/openapi/kiva-confidential.yaml` | `POST` | `/confidential/threshold-releases` | `postConfidentialThresholdRelease` | `confidential-release-threshold-aggregate` | `confidential.release_threshold_aggregate` |  |  | `confidential:release` |
