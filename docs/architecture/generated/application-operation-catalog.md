# Generated Application Operation Catalog

This file is generated from `generated/architecture/surface-catalog.yaml` by
`scripts/ci/generate_application_operation_catalog.py`.

It is review evidence for the join between application workflow labels, Rust
workflow variants, OpenAPI `operationId` values, and OAuth/OIDC resource-server
scopes.

| Catalog ID | Surface | Workflow Label | Rust Workflow | OpenAPI Operation | Required Scopes |
| --- | --- | --- | --- | --- | --- |
| `analytics-list-products` | analytics | `analytics.list_products` | `AnalyticsWorkflowKind::ListProducts` | `spec/openapi/kiva-analytics.yaml#getAnalyticsProducts` | `analytics:read` |
| `analytics-read-product` | analytics | `analytics.read_product` | `AnalyticsWorkflowKind::ReadProduct` | `spec/openapi/kiva-analytics.yaml#getAnalyticsProduct` | `analytics:read` |
| `analytics-read-aggregate` | analytics | `analytics.read_aggregate` | `AnalyticsWorkflowKind::ReadAggregate` | `spec/openapi/kiva-analytics.yaml#getAnalyticsProductAggregate` | `analytics:read` |
| `pds-list-access-audit-events` | pds | `pds.list_access_audit_events` | `PdsWorkflowKind::ListAccessAuditEvents` | `spec/openapi/kiva-pds.yaml#getPdsAccessAuditEvents` | `pds:audit` |
| `pds-evaluate-access-check` | pds | `pds.evaluate_access_check` | `PdsWorkflowKind::EvaluateAccessCheck` | `spec/openapi/kiva-pds.yaml#postPdsAccessCheck` | `pds:read` |
| `pds-list-consent-grants` | pds | `pds.list_consent_grants` | `PdsWorkflowKind::ListConsentGrants` | `spec/openapi/kiva-pds.yaml#getPdsConsentGrants` | `pds:read` |
| `pds-register-consent-grant` | pds | `pds.register_consent_grant` | `PdsWorkflowKind::RegisterConsentGrant` | `spec/openapi/kiva-pds.yaml#postPdsConsentGrant` | `pds:delegate` |
| `pds-revoke-consent-grant` | pds | `pds.revoke_consent_grant` | `PdsWorkflowKind::RevokeConsentGrant` | `spec/openapi/kiva-pds.yaml#postPdsConsentRevocation` | `pds:revoke` |
| `pds-export-portability` | pds | `pds.export_portability` | `PdsWorkflowKind::ExportPortability` | `spec/openapi/kiva-pds.yaml#getPdsDataPortabilityExports` | `pds:read` |
| `pds-read-data-product` | pds | `pds.read_data_product` | `PdsWorkflowKind::ReadDataProduct` | `spec/openapi/kiva-pds.yaml#postPdsDataProductRead` | `pds:read` |
| `pds-list-data-products` | pds | `pds.list_data_products` | `PdsWorkflowKind::ListDataProducts` | `spec/openapi/kiva-pds.yaml#getPdsDataProducts` | `pds:read` |
| `pds-register-data-product` | pds | `pds.register_data_product` | `PdsWorkflowKind::RegisterDataProduct` | `spec/openapi/kiva-pds.yaml#postPdsDataProduct` | `pds:write` |
| `pds-list-governance-profiles` | pds | `pds.list_governance_profiles` | `PdsWorkflowKind::ListGovernanceProfiles` | `spec/openapi/kiva-pds.yaml#getPdsGovernanceProfiles` | `pds:read` |
| `pds-register-governance-profile` | pds | `pds.register_governance_profile` | `PdsWorkflowKind::RegisterGovernanceProfile` | `spec/openapi/kiva-pds.yaml#postPdsGovernanceProfile` | `pds:write` |
| `pds-list-lifecycle-cases` | pds | `pds.list_lifecycle_cases` | `PdsWorkflowKind::ListLifecycleCases` | `spec/openapi/kiva-pds.yaml#getPdsLifecycleCases` | `pds:read` |
| `pds-register-lifecycle-case` | pds | `pds.register_lifecycle_case` | `PdsWorkflowKind::RegisterLifecycleCase` | `spec/openapi/kiva-pds.yaml#postPdsLifecycleCase` | `pds:write` |
| `pds-list-lifecycle-events` | pds | `pds.list_lifecycle_events` | `PdsWorkflowKind::ListLifecycleEvents` | `spec/openapi/kiva-pds.yaml#getPdsLifecycleEvents` | `pds:read` |
| `pds-register-lifecycle-event` | pds | `pds.register_lifecycle_event` | `PdsWorkflowKind::RegisterLifecycleEvent` | `spec/openapi/kiva-pds.yaml#postPdsLifecycleEvent` | `pds:write` |
| `pds-export-metadata` | pds | `pds.export_metadata` | `PdsWorkflowKind::ExportMetadata` | `spec/openapi/kiva-pds.yaml#getPdsMetadataExports` | `pds:read` |
| `pds-list-provider-bindings` | pds | `pds.list_provider_bindings` | `PdsWorkflowKind::ListProviderBindings` | `spec/openapi/kiva-pds.yaml#getPdsProviderBindings` | `pds:read` |
| `pds-register-provider-binding` | pds | `pds.register_provider_binding` | `PdsWorkflowKind::RegisterProviderBinding` | `spec/openapi/kiva-pds.yaml#postPdsProviderBinding` | `pds:write` |
| `pds-read-record-inventory` | pds | `pds.read_record_inventory` | `PdsWorkflowKind::ReadRecordInventory` | `spec/openapi/kiva-pds.yaml#getPdsRecordInventory` | `pds:read` |
| `pds-list-retention-policies` | pds | `pds.list_retention_policies` | `PdsWorkflowKind::ListRetentionPolicies` | `spec/openapi/kiva-pds.yaml#getPdsRetentionPolicies` | `pds:read` |
| `pds-register-retention-policy` | pds | `pds.register_retention_policy` | `PdsWorkflowKind::RegisterRetentionPolicy` | `spec/openapi/kiva-pds.yaml#postPdsRetentionPolicy` | `pds:write` |
| `confidential-release-signed-additive-aggregate` | confidential | `confidential.release_signed_additive_aggregate` | `ConfidentialWorkflowKind::ReleaseSignedAdditiveAggregate` | `spec/openapi/kiva-confidential.yaml#postConfidentialAdditiveAggregate` | `confidential:compute` |
| `confidential-list-additive-replay-records` | confidential | `confidential.list_replay_records` | `ConfidentialWorkflowKind::ListReplayRecords` | `spec/openapi/kiva-confidential.yaml#getConfidentialAdditiveShareReplayRecords` | `confidential:read` |
| `confidential-revoke-share-signing-key` | confidential | `confidential.revoke_share_signing_key` | `ConfidentialWorkflowKind::RevokeShareSigningKey` | `spec/openapi/kiva-confidential.yaml#postConfidentialShareSigningKeyRevocation` | `confidential:write` |
| `confidential-rotate-share-signing-key` | confidential | `confidential.rotate_share_signing_key` | `ConfidentialWorkflowKind::RotateShareSigningKey` | `spec/openapi/kiva-confidential.yaml#postConfidentialShareSigningKeyRotation` | `confidential:write` |
| `confidential-list-share-signing-keys` | confidential | `confidential.list_share_signing_keys` | `ConfidentialWorkflowKind::ListShareSigningKeys` | `spec/openapi/kiva-confidential.yaml#getConfidentialShareSigningKeys` | `confidential:read` |
| `confidential-register-share-signing-key` | confidential | `confidential.register_share_signing_key` | `ConfidentialWorkflowKind::RegisterShareSigningKey` | `spec/openapi/kiva-confidential.yaml#postConfidentialShareSigningKey` | `confidential:write` |
| `confidential-list-submitted-shares` | confidential | `confidential.list_submitted_shares` | `ConfidentialWorkflowKind::ListSubmittedShares` | `spec/openapi/kiva-confidential.yaml#getConfidentialAdditiveShareSubmissions` | `confidential:read` |
| `confidential-submit-additive-share` | confidential | `confidential.submit_additive_share` | `ConfidentialWorkflowKind::SubmitAdditiveShare` | `spec/openapi/kiva-confidential.yaml#postConfidentialAdditiveShareSubmission` | `confidential:write` |
| `confidential-spend-differential-privacy-budget` | confidential | `confidential.spend_differential_privacy_budget` | `ConfidentialWorkflowKind::SpendDifferentialPrivacyBudget` | `spec/openapi/kiva-confidential.yaml#postConfidentialDifferentialPrivacyBudgetSpend` | `confidential:write` |
| `confidential-list-differential-privacy-budgets` | confidential | `confidential.list_differential_privacy_budgets` | `ConfidentialWorkflowKind::ListDifferentialPrivacyBudgets` | `spec/openapi/kiva-confidential.yaml#getConfidentialDifferentialPrivacyBudgets` | `confidential:read` |
| `confidential-register-differential-privacy-budget` | confidential | `confidential.register_differential_privacy_budget` | `ConfidentialWorkflowKind::RegisterDifferentialPrivacyBudget` | `spec/openapi/kiva-confidential.yaml#postConfidentialDifferentialPrivacyBudget` | `confidential:write` |
| `confidential-list-differential-privacy-composition-summaries` | confidential | `confidential.list_differential_privacy_composition_summaries` | `ConfidentialWorkflowKind::ListDifferentialPrivacyCompositionSummaries` | `spec/openapi/kiva-confidential.yaml#getConfidentialDifferentialPrivacyCompositionSummaries` | `confidential:read` |
| `confidential-list-differential-privacy-releases` | confidential | `confidential.list_differential_privacy_releases` | `ConfidentialWorkflowKind::ListDifferentialPrivacyReleases` | `spec/openapi/kiva-confidential.yaml#getConfidentialDifferentialPrivacyReleases` | `confidential:read` |
| `confidential-release-differential-privacy-aggregate` | confidential | `confidential.release_differential_privacy_aggregate` | `ConfidentialWorkflowKind::ReleaseDifferentialPrivacyAggregate` | `spec/openapi/kiva-confidential.yaml#postConfidentialDifferentialPrivacyRelease` | `confidential:release` |
| `confidential-release-sampled-differential-privacy-aggregate` | confidential | `confidential.release_sampled_differential_privacy_aggregate` | `ConfidentialWorkflowKind::ReleaseSampledDifferentialPrivacyAggregate` | `spec/openapi/kiva-confidential.yaml#postConfidentialDifferentialPrivacySampledRelease` | `confidential:release` |
| `confidential-list-job-audit-events` | confidential | `confidential.list_job_audit_events` | `ConfidentialWorkflowKind::ListJobAuditEvents` | `spec/openapi/kiva-confidential.yaml#getConfidentialJobAuditEvents` | `confidential:read` |
| `confidential-transition-job` | confidential | `confidential.transition_job` | `ConfidentialWorkflowKind::TransitionJob` | `spec/openapi/kiva-confidential.yaml#postConfidentialJobTransition` | `confidential:write` |
| `confidential-list-jobs` | confidential | `confidential.list_jobs` | `ConfidentialWorkflowKind::ListJobs` | `spec/openapi/kiva-confidential.yaml#getConfidentialJobs` | `confidential:read` |
| `confidential-register-job` | confidential | `confidential.register_job` | `ConfidentialWorkflowKind::RegisterJob` | `spec/openapi/kiva-confidential.yaml#postConfidentialJob` | `confidential:write` |
| `confidential-list-participant-audit-events` | confidential | `confidential.list_participant_audit_events` | `ConfidentialWorkflowKind::ListParticipantAuditEvents` | `spec/openapi/kiva-confidential.yaml#getConfidentialParticipantAuditEvents` | `confidential:read` |
| `confidential-transition-participant` | confidential | `confidential.transition_participant` | `ConfidentialWorkflowKind::TransitionParticipant` | `spec/openapi/kiva-confidential.yaml#postConfidentialParticipantTransition` | `confidential:write` |
| `confidential-list-participants` | confidential | `confidential.list_participants` | `ConfidentialWorkflowKind::ListParticipants` | `spec/openapi/kiva-confidential.yaml#getConfidentialParticipants` | `confidential:read` |
| `confidential-register-participant` | confidential | `confidential.register_participant` | `ConfidentialWorkflowKind::RegisterParticipant` | `spec/openapi/kiva-confidential.yaml#postConfidentialParticipant` | `confidential:write` |
| `confidential-list-shamir-threshold-reconstructions` | confidential | `confidential.list_shamir_threshold_reconstructions` | `ConfidentialWorkflowKind::ListShamirThresholdReconstructions` | `spec/openapi/kiva-confidential.yaml#getConfidentialShamirThresholdReconstructions` | `confidential:read` |
| `confidential-reconstruct-shamir-threshold-secret` | confidential | `confidential.reconstruct_shamir_threshold_secret` | `ConfidentialWorkflowKind::ReconstructShamirThresholdSecret` | `spec/openapi/kiva-confidential.yaml#postConfidentialShamirThresholdReconstruction` | `confidential:compute` |
| `confidential-approve-threshold-release` | confidential | `confidential.approve_threshold_release` | `ConfidentialWorkflowKind::ApproveThresholdRelease` | `spec/openapi/kiva-confidential.yaml#postConfidentialThresholdReleaseApproval` | `confidential:release` |
| `confidential-list-threshold-release-requests` | confidential | `confidential.list_threshold_release_requests` | `ConfidentialWorkflowKind::ListThresholdReleaseRequests` | `spec/openapi/kiva-confidential.yaml#getConfidentialThresholdReleaseRequests` | `confidential:read` |
| `confidential-register-threshold-release-request` | confidential | `confidential.register_threshold_release_request` | `ConfidentialWorkflowKind::RegisterThresholdReleaseRequest` | `spec/openapi/kiva-confidential.yaml#postConfidentialThresholdReleaseRequest` | `confidential:write` |
| `confidential-release-threshold-aggregate` | confidential | `confidential.release_threshold_aggregate` | `ConfidentialWorkflowKind::ReleaseThresholdAggregate` | `spec/openapi/kiva-confidential.yaml#postConfidentialThresholdRelease` | `confidential:release` |
