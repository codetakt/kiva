//! Generated application operation identifiers.
//!
//! Generated from `generated/architecture/surface-catalog.yaml` by
//! `scripts/ci/generate_application_operation_catalog.py`.

use super::super::ApplicationOperationSpec;
use super::registry::APPLICATION_OPERATION_SPECS;

/// Typed operation identifiers generated from the application operation catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApplicationOperationId {
    /// Catalog operation `analytics-list-products`.
    AnalyticsListProducts,
    /// Catalog operation `analytics-read-product`.
    AnalyticsReadProduct,
    /// Catalog operation `analytics-read-aggregate`.
    AnalyticsReadAggregate,
    /// Catalog operation `pds-list-access-audit-events`.
    PdsListAccessAuditEvents,
    /// Catalog operation `pds-evaluate-access-check`.
    PdsEvaluateAccessCheck,
    /// Catalog operation `pds-list-consent-grants`.
    PdsListConsentGrants,
    /// Catalog operation `pds-register-consent-grant`.
    PdsRegisterConsentGrant,
    /// Catalog operation `pds-revoke-consent-grant`.
    PdsRevokeConsentGrant,
    /// Catalog operation `pds-export-portability`.
    PdsExportPortability,
    /// Catalog operation `pds-read-data-product`.
    PdsReadDataProduct,
    /// Catalog operation `pds-list-data-products`.
    PdsListDataProducts,
    /// Catalog operation `pds-register-data-product`.
    PdsRegisterDataProduct,
    /// Catalog operation `pds-list-governance-profiles`.
    PdsListGovernanceProfiles,
    /// Catalog operation `pds-register-governance-profile`.
    PdsRegisterGovernanceProfile,
    /// Catalog operation `pds-list-lifecycle-cases`.
    PdsListLifecycleCases,
    /// Catalog operation `pds-register-lifecycle-case`.
    PdsRegisterLifecycleCase,
    /// Catalog operation `pds-list-lifecycle-events`.
    PdsListLifecycleEvents,
    /// Catalog operation `pds-register-lifecycle-event`.
    PdsRegisterLifecycleEvent,
    /// Catalog operation `pds-export-metadata`.
    PdsExportMetadata,
    /// Catalog operation `pds-list-provider-bindings`.
    PdsListProviderBindings,
    /// Catalog operation `pds-register-provider-binding`.
    PdsRegisterProviderBinding,
    /// Catalog operation `pds-read-record-inventory`.
    PdsReadRecordInventory,
    /// Catalog operation `pds-list-retention-policies`.
    PdsListRetentionPolicies,
    /// Catalog operation `pds-register-retention-policy`.
    PdsRegisterRetentionPolicy,
    /// Catalog operation `confidential-release-signed-additive-aggregate`.
    ConfidentialReleaseSignedAdditiveAggregate,
    /// Catalog operation `confidential-list-additive-replay-records`.
    ConfidentialListAdditiveReplayRecords,
    /// Catalog operation `confidential-revoke-share-signing-key`.
    ConfidentialRevokeShareSigningKey,
    /// Catalog operation `confidential-rotate-share-signing-key`.
    ConfidentialRotateShareSigningKey,
    /// Catalog operation `confidential-list-share-signing-keys`.
    ConfidentialListShareSigningKeys,
    /// Catalog operation `confidential-register-share-signing-key`.
    ConfidentialRegisterShareSigningKey,
    /// Catalog operation `confidential-list-submitted-shares`.
    ConfidentialListSubmittedShares,
    /// Catalog operation `confidential-submit-additive-share`.
    ConfidentialSubmitAdditiveShare,
    /// Catalog operation `confidential-spend-differential-privacy-budget`.
    ConfidentialSpendDifferentialPrivacyBudget,
    /// Catalog operation `confidential-list-differential-privacy-budgets`.
    ConfidentialListDifferentialPrivacyBudgets,
    /// Catalog operation `confidential-register-differential-privacy-budget`.
    ConfidentialRegisterDifferentialPrivacyBudget,
    /// Catalog operation `confidential-list-differential-privacy-composition-summaries`.
    ConfidentialListDifferentialPrivacyCompositionSummaries,
    /// Catalog operation `confidential-list-differential-privacy-releases`.
    ConfidentialListDifferentialPrivacyReleases,
    /// Catalog operation `confidential-release-differential-privacy-aggregate`.
    ConfidentialReleaseDifferentialPrivacyAggregate,
    /// Catalog operation `confidential-release-sampled-differential-privacy-aggregate`.
    ConfidentialReleaseSampledDifferentialPrivacyAggregate,
    /// Catalog operation `confidential-list-job-audit-events`.
    ConfidentialListJobAuditEvents,
    /// Catalog operation `confidential-transition-job`.
    ConfidentialTransitionJob,
    /// Catalog operation `confidential-list-jobs`.
    ConfidentialListJobs,
    /// Catalog operation `confidential-register-job`.
    ConfidentialRegisterJob,
    /// Catalog operation `confidential-list-participant-audit-events`.
    ConfidentialListParticipantAuditEvents,
    /// Catalog operation `confidential-transition-participant`.
    ConfidentialTransitionParticipant,
    /// Catalog operation `confidential-list-participants`.
    ConfidentialListParticipants,
    /// Catalog operation `confidential-register-participant`.
    ConfidentialRegisterParticipant,
    /// Catalog operation `confidential-list-shamir-threshold-reconstructions`.
    ConfidentialListShamirThresholdReconstructions,
    /// Catalog operation `confidential-reconstruct-shamir-threshold-secret`.
    ConfidentialReconstructShamirThresholdSecret,
    /// Catalog operation `confidential-approve-threshold-release`.
    ConfidentialApproveThresholdRelease,
    /// Catalog operation `confidential-list-threshold-release-requests`.
    ConfidentialListThresholdReleaseRequests,
    /// Catalog operation `confidential-register-threshold-release-request`.
    ConfidentialRegisterThresholdReleaseRequest,
    /// Catalog operation `confidential-release-threshold-aggregate`.
    ConfidentialReleaseThresholdAggregate,
}


impl ApplicationOperationId {
    /// Returns the stable catalog identifier.
    #[must_use]
    pub const fn catalog_id(self) -> &'static str {
        match self {
            Self::AnalyticsListProducts => "analytics-list-products",
            Self::AnalyticsReadProduct => "analytics-read-product",
            Self::AnalyticsReadAggregate => "analytics-read-aggregate",
            Self::PdsListAccessAuditEvents => "pds-list-access-audit-events",
            Self::PdsEvaluateAccessCheck => "pds-evaluate-access-check",
            Self::PdsListConsentGrants => "pds-list-consent-grants",
            Self::PdsRegisterConsentGrant => "pds-register-consent-grant",
            Self::PdsRevokeConsentGrant => "pds-revoke-consent-grant",
            Self::PdsExportPortability => "pds-export-portability",
            Self::PdsReadDataProduct => "pds-read-data-product",
            Self::PdsListDataProducts => "pds-list-data-products",
            Self::PdsRegisterDataProduct => "pds-register-data-product",
            Self::PdsListGovernanceProfiles => "pds-list-governance-profiles",
            Self::PdsRegisterGovernanceProfile => "pds-register-governance-profile",
            Self::PdsListLifecycleCases => "pds-list-lifecycle-cases",
            Self::PdsRegisterLifecycleCase => "pds-register-lifecycle-case",
            Self::PdsListLifecycleEvents => "pds-list-lifecycle-events",
            Self::PdsRegisterLifecycleEvent => "pds-register-lifecycle-event",
            Self::PdsExportMetadata => "pds-export-metadata",
            Self::PdsListProviderBindings => "pds-list-provider-bindings",
            Self::PdsRegisterProviderBinding => "pds-register-provider-binding",
            Self::PdsReadRecordInventory => "pds-read-record-inventory",
            Self::PdsListRetentionPolicies => "pds-list-retention-policies",
            Self::PdsRegisterRetentionPolicy => "pds-register-retention-policy",
            Self::ConfidentialReleaseSignedAdditiveAggregate => "confidential-release-signed-additive-aggregate",
            Self::ConfidentialListAdditiveReplayRecords => "confidential-list-additive-replay-records",
            Self::ConfidentialRevokeShareSigningKey => "confidential-revoke-share-signing-key",
            Self::ConfidentialRotateShareSigningKey => "confidential-rotate-share-signing-key",
            Self::ConfidentialListShareSigningKeys => "confidential-list-share-signing-keys",
            Self::ConfidentialRegisterShareSigningKey => "confidential-register-share-signing-key",
            Self::ConfidentialListSubmittedShares => "confidential-list-submitted-shares",
            Self::ConfidentialSubmitAdditiveShare => "confidential-submit-additive-share",
            Self::ConfidentialSpendDifferentialPrivacyBudget => "confidential-spend-differential-privacy-budget",
            Self::ConfidentialListDifferentialPrivacyBudgets => "confidential-list-differential-privacy-budgets",
            Self::ConfidentialRegisterDifferentialPrivacyBudget => "confidential-register-differential-privacy-budget",
            Self::ConfidentialListDifferentialPrivacyCompositionSummaries => "confidential-list-differential-privacy-composition-summaries",
            Self::ConfidentialListDifferentialPrivacyReleases => "confidential-list-differential-privacy-releases",
            Self::ConfidentialReleaseDifferentialPrivacyAggregate => "confidential-release-differential-privacy-aggregate",
            Self::ConfidentialReleaseSampledDifferentialPrivacyAggregate => "confidential-release-sampled-differential-privacy-aggregate",
            Self::ConfidentialListJobAuditEvents => "confidential-list-job-audit-events",
            Self::ConfidentialTransitionJob => "confidential-transition-job",
            Self::ConfidentialListJobs => "confidential-list-jobs",
            Self::ConfidentialRegisterJob => "confidential-register-job",
            Self::ConfidentialListParticipantAuditEvents => "confidential-list-participant-audit-events",
            Self::ConfidentialTransitionParticipant => "confidential-transition-participant",
            Self::ConfidentialListParticipants => "confidential-list-participants",
            Self::ConfidentialRegisterParticipant => "confidential-register-participant",
            Self::ConfidentialListShamirThresholdReconstructions => "confidential-list-shamir-threshold-reconstructions",
            Self::ConfidentialReconstructShamirThresholdSecret => "confidential-reconstruct-shamir-threshold-secret",
            Self::ConfidentialApproveThresholdRelease => "confidential-approve-threshold-release",
            Self::ConfidentialListThresholdReleaseRequests => "confidential-list-threshold-release-requests",
            Self::ConfidentialRegisterThresholdReleaseRequest => "confidential-register-threshold-release-request",
            Self::ConfidentialReleaseThresholdAggregate => "confidential-release-threshold-aggregate",
        }
    }

    /// Returns the generated operation specification.
    #[must_use]
    pub fn spec(self) -> ApplicationOperationSpec {
        match self {
            Self::AnalyticsListProducts => APPLICATION_OPERATION_SPECS[0],
            Self::AnalyticsReadProduct => APPLICATION_OPERATION_SPECS[1],
            Self::AnalyticsReadAggregate => APPLICATION_OPERATION_SPECS[2],
            Self::PdsListAccessAuditEvents => APPLICATION_OPERATION_SPECS[3],
            Self::PdsEvaluateAccessCheck => APPLICATION_OPERATION_SPECS[4],
            Self::PdsListConsentGrants => APPLICATION_OPERATION_SPECS[5],
            Self::PdsRegisterConsentGrant => APPLICATION_OPERATION_SPECS[6],
            Self::PdsRevokeConsentGrant => APPLICATION_OPERATION_SPECS[7],
            Self::PdsExportPortability => APPLICATION_OPERATION_SPECS[8],
            Self::PdsReadDataProduct => APPLICATION_OPERATION_SPECS[9],
            Self::PdsListDataProducts => APPLICATION_OPERATION_SPECS[10],
            Self::PdsRegisterDataProduct => APPLICATION_OPERATION_SPECS[11],
            Self::PdsListGovernanceProfiles => APPLICATION_OPERATION_SPECS[12],
            Self::PdsRegisterGovernanceProfile => APPLICATION_OPERATION_SPECS[13],
            Self::PdsListLifecycleCases => APPLICATION_OPERATION_SPECS[14],
            Self::PdsRegisterLifecycleCase => APPLICATION_OPERATION_SPECS[15],
            Self::PdsListLifecycleEvents => APPLICATION_OPERATION_SPECS[16],
            Self::PdsRegisterLifecycleEvent => APPLICATION_OPERATION_SPECS[17],
            Self::PdsExportMetadata => APPLICATION_OPERATION_SPECS[18],
            Self::PdsListProviderBindings => APPLICATION_OPERATION_SPECS[19],
            Self::PdsRegisterProviderBinding => APPLICATION_OPERATION_SPECS[20],
            Self::PdsReadRecordInventory => APPLICATION_OPERATION_SPECS[21],
            Self::PdsListRetentionPolicies => APPLICATION_OPERATION_SPECS[22],
            Self::PdsRegisterRetentionPolicy => APPLICATION_OPERATION_SPECS[23],
            Self::ConfidentialReleaseSignedAdditiveAggregate => APPLICATION_OPERATION_SPECS[24],
            Self::ConfidentialListAdditiveReplayRecords => APPLICATION_OPERATION_SPECS[25],
            Self::ConfidentialRevokeShareSigningKey => APPLICATION_OPERATION_SPECS[26],
            Self::ConfidentialRotateShareSigningKey => APPLICATION_OPERATION_SPECS[27],
            Self::ConfidentialListShareSigningKeys => APPLICATION_OPERATION_SPECS[28],
            Self::ConfidentialRegisterShareSigningKey => APPLICATION_OPERATION_SPECS[29],
            Self::ConfidentialListSubmittedShares => APPLICATION_OPERATION_SPECS[30],
            Self::ConfidentialSubmitAdditiveShare => APPLICATION_OPERATION_SPECS[31],
            Self::ConfidentialSpendDifferentialPrivacyBudget => APPLICATION_OPERATION_SPECS[32],
            Self::ConfidentialListDifferentialPrivacyBudgets => APPLICATION_OPERATION_SPECS[33],
            Self::ConfidentialRegisterDifferentialPrivacyBudget => APPLICATION_OPERATION_SPECS[34],
            Self::ConfidentialListDifferentialPrivacyCompositionSummaries => APPLICATION_OPERATION_SPECS[35],
            Self::ConfidentialListDifferentialPrivacyReleases => APPLICATION_OPERATION_SPECS[36],
            Self::ConfidentialReleaseDifferentialPrivacyAggregate => APPLICATION_OPERATION_SPECS[37],
            Self::ConfidentialReleaseSampledDifferentialPrivacyAggregate => APPLICATION_OPERATION_SPECS[38],
            Self::ConfidentialListJobAuditEvents => APPLICATION_OPERATION_SPECS[39],
            Self::ConfidentialTransitionJob => APPLICATION_OPERATION_SPECS[40],
            Self::ConfidentialListJobs => APPLICATION_OPERATION_SPECS[41],
            Self::ConfidentialRegisterJob => APPLICATION_OPERATION_SPECS[42],
            Self::ConfidentialListParticipantAuditEvents => APPLICATION_OPERATION_SPECS[43],
            Self::ConfidentialTransitionParticipant => APPLICATION_OPERATION_SPECS[44],
            Self::ConfidentialListParticipants => APPLICATION_OPERATION_SPECS[45],
            Self::ConfidentialRegisterParticipant => APPLICATION_OPERATION_SPECS[46],
            Self::ConfidentialListShamirThresholdReconstructions => APPLICATION_OPERATION_SPECS[47],
            Self::ConfidentialReconstructShamirThresholdSecret => APPLICATION_OPERATION_SPECS[48],
            Self::ConfidentialApproveThresholdRelease => APPLICATION_OPERATION_SPECS[49],
            Self::ConfidentialListThresholdReleaseRequests => APPLICATION_OPERATION_SPECS[50],
            Self::ConfidentialRegisterThresholdReleaseRequest => APPLICATION_OPERATION_SPECS[51],
            Self::ConfidentialReleaseThresholdAggregate => APPLICATION_OPERATION_SPECS[52],
        }
    }

    /// Resolves a stable catalog identifier to a typed operation id.
    #[must_use]
    pub fn from_catalog_id(catalog_id: &str) -> Option<Self> {
        match catalog_id {
            "analytics-list-products" => Some(Self::AnalyticsListProducts),
            "analytics-read-product" => Some(Self::AnalyticsReadProduct),
            "analytics-read-aggregate" => Some(Self::AnalyticsReadAggregate),
            "pds-list-access-audit-events" => Some(Self::PdsListAccessAuditEvents),
            "pds-evaluate-access-check" => Some(Self::PdsEvaluateAccessCheck),
            "pds-list-consent-grants" => Some(Self::PdsListConsentGrants),
            "pds-register-consent-grant" => Some(Self::PdsRegisterConsentGrant),
            "pds-revoke-consent-grant" => Some(Self::PdsRevokeConsentGrant),
            "pds-export-portability" => Some(Self::PdsExportPortability),
            "pds-read-data-product" => Some(Self::PdsReadDataProduct),
            "pds-list-data-products" => Some(Self::PdsListDataProducts),
            "pds-register-data-product" => Some(Self::PdsRegisterDataProduct),
            "pds-list-governance-profiles" => Some(Self::PdsListGovernanceProfiles),
            "pds-register-governance-profile" => Some(Self::PdsRegisterGovernanceProfile),
            "pds-list-lifecycle-cases" => Some(Self::PdsListLifecycleCases),
            "pds-register-lifecycle-case" => Some(Self::PdsRegisterLifecycleCase),
            "pds-list-lifecycle-events" => Some(Self::PdsListLifecycleEvents),
            "pds-register-lifecycle-event" => Some(Self::PdsRegisterLifecycleEvent),
            "pds-export-metadata" => Some(Self::PdsExportMetadata),
            "pds-list-provider-bindings" => Some(Self::PdsListProviderBindings),
            "pds-register-provider-binding" => Some(Self::PdsRegisterProviderBinding),
            "pds-read-record-inventory" => Some(Self::PdsReadRecordInventory),
            "pds-list-retention-policies" => Some(Self::PdsListRetentionPolicies),
            "pds-register-retention-policy" => Some(Self::PdsRegisterRetentionPolicy),
            "confidential-release-signed-additive-aggregate" => Some(Self::ConfidentialReleaseSignedAdditiveAggregate),
            "confidential-list-additive-replay-records" => Some(Self::ConfidentialListAdditiveReplayRecords),
            "confidential-revoke-share-signing-key" => Some(Self::ConfidentialRevokeShareSigningKey),
            "confidential-rotate-share-signing-key" => Some(Self::ConfidentialRotateShareSigningKey),
            "confidential-list-share-signing-keys" => Some(Self::ConfidentialListShareSigningKeys),
            "confidential-register-share-signing-key" => Some(Self::ConfidentialRegisterShareSigningKey),
            "confidential-list-submitted-shares" => Some(Self::ConfidentialListSubmittedShares),
            "confidential-submit-additive-share" => Some(Self::ConfidentialSubmitAdditiveShare),
            "confidential-spend-differential-privacy-budget" => Some(Self::ConfidentialSpendDifferentialPrivacyBudget),
            "confidential-list-differential-privacy-budgets" => Some(Self::ConfidentialListDifferentialPrivacyBudgets),
            "confidential-register-differential-privacy-budget" => Some(Self::ConfidentialRegisterDifferentialPrivacyBudget),
            "confidential-list-differential-privacy-composition-summaries" => Some(Self::ConfidentialListDifferentialPrivacyCompositionSummaries),
            "confidential-list-differential-privacy-releases" => Some(Self::ConfidentialListDifferentialPrivacyReleases),
            "confidential-release-differential-privacy-aggregate" => Some(Self::ConfidentialReleaseDifferentialPrivacyAggregate),
            "confidential-release-sampled-differential-privacy-aggregate" => Some(Self::ConfidentialReleaseSampledDifferentialPrivacyAggregate),
            "confidential-list-job-audit-events" => Some(Self::ConfidentialListJobAuditEvents),
            "confidential-transition-job" => Some(Self::ConfidentialTransitionJob),
            "confidential-list-jobs" => Some(Self::ConfidentialListJobs),
            "confidential-register-job" => Some(Self::ConfidentialRegisterJob),
            "confidential-list-participant-audit-events" => Some(Self::ConfidentialListParticipantAuditEvents),
            "confidential-transition-participant" => Some(Self::ConfidentialTransitionParticipant),
            "confidential-list-participants" => Some(Self::ConfidentialListParticipants),
            "confidential-register-participant" => Some(Self::ConfidentialRegisterParticipant),
            "confidential-list-shamir-threshold-reconstructions" => Some(Self::ConfidentialListShamirThresholdReconstructions),
            "confidential-reconstruct-shamir-threshold-secret" => Some(Self::ConfidentialReconstructShamirThresholdSecret),
            "confidential-approve-threshold-release" => Some(Self::ConfidentialApproveThresholdRelease),
            "confidential-list-threshold-release-requests" => Some(Self::ConfidentialListThresholdReleaseRequests),
            "confidential-register-threshold-release-request" => Some(Self::ConfidentialRegisterThresholdReleaseRequest),
            "confidential-release-threshold-aggregate" => Some(Self::ConfidentialReleaseThresholdAggregate),
            _ => None,
        }
    }

    /// Resolves an `OpenAPI` contract path and operationId to a typed operation id.
    #[must_use]
    pub fn from_openapi_operation(openapi_contract: &str, operation_id: &str) -> Option<Self> {
        match (openapi_contract, operation_id) {
            ("spec/openapi/kiva-analytics.yaml", "getAnalyticsProducts") => Some(Self::AnalyticsListProducts),
            ("spec/openapi/kiva-analytics.yaml", "getAnalyticsProduct") => Some(Self::AnalyticsReadProduct),
            ("spec/openapi/kiva-analytics.yaml", "getAnalyticsProductAggregate") => Some(Self::AnalyticsReadAggregate),
            ("spec/openapi/kiva-pds.yaml", "getPdsAccessAuditEvents") => Some(Self::PdsListAccessAuditEvents),
            ("spec/openapi/kiva-pds.yaml", "postPdsAccessCheck") => Some(Self::PdsEvaluateAccessCheck),
            ("spec/openapi/kiva-pds.yaml", "getPdsConsentGrants") => Some(Self::PdsListConsentGrants),
            ("spec/openapi/kiva-pds.yaml", "postPdsConsentGrant") => Some(Self::PdsRegisterConsentGrant),
            ("spec/openapi/kiva-pds.yaml", "postPdsConsentRevocation") => Some(Self::PdsRevokeConsentGrant),
            ("spec/openapi/kiva-pds.yaml", "getPdsDataPortabilityExports") => Some(Self::PdsExportPortability),
            ("spec/openapi/kiva-pds.yaml", "postPdsDataProductRead") => Some(Self::PdsReadDataProduct),
            ("spec/openapi/kiva-pds.yaml", "getPdsDataProducts") => Some(Self::PdsListDataProducts),
            ("spec/openapi/kiva-pds.yaml", "postPdsDataProduct") => Some(Self::PdsRegisterDataProduct),
            ("spec/openapi/kiva-pds.yaml", "getPdsGovernanceProfiles") => Some(Self::PdsListGovernanceProfiles),
            ("spec/openapi/kiva-pds.yaml", "postPdsGovernanceProfile") => Some(Self::PdsRegisterGovernanceProfile),
            ("spec/openapi/kiva-pds.yaml", "getPdsLifecycleCases") => Some(Self::PdsListLifecycleCases),
            ("spec/openapi/kiva-pds.yaml", "postPdsLifecycleCase") => Some(Self::PdsRegisterLifecycleCase),
            ("spec/openapi/kiva-pds.yaml", "getPdsLifecycleEvents") => Some(Self::PdsListLifecycleEvents),
            ("spec/openapi/kiva-pds.yaml", "postPdsLifecycleEvent") => Some(Self::PdsRegisterLifecycleEvent),
            ("spec/openapi/kiva-pds.yaml", "getPdsMetadataExports") => Some(Self::PdsExportMetadata),
            ("spec/openapi/kiva-pds.yaml", "getPdsProviderBindings") => Some(Self::PdsListProviderBindings),
            ("spec/openapi/kiva-pds.yaml", "postPdsProviderBinding") => Some(Self::PdsRegisterProviderBinding),
            ("spec/openapi/kiva-pds.yaml", "getPdsRecordInventory") => Some(Self::PdsReadRecordInventory),
            ("spec/openapi/kiva-pds.yaml", "getPdsRetentionPolicies") => Some(Self::PdsListRetentionPolicies),
            ("spec/openapi/kiva-pds.yaml", "postPdsRetentionPolicy") => Some(Self::PdsRegisterRetentionPolicy),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialAdditiveAggregate") => Some(Self::ConfidentialReleaseSignedAdditiveAggregate),
            ("spec/openapi/kiva-confidential.yaml", "getConfidentialAdditiveShareReplayRecords") => Some(Self::ConfidentialListAdditiveReplayRecords),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialShareSigningKeyRevocation") => Some(Self::ConfidentialRevokeShareSigningKey),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialShareSigningKeyRotation") => Some(Self::ConfidentialRotateShareSigningKey),
            ("spec/openapi/kiva-confidential.yaml", "getConfidentialShareSigningKeys") => Some(Self::ConfidentialListShareSigningKeys),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialShareSigningKey") => Some(Self::ConfidentialRegisterShareSigningKey),
            ("spec/openapi/kiva-confidential.yaml", "getConfidentialAdditiveShareSubmissions") => Some(Self::ConfidentialListSubmittedShares),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialAdditiveShareSubmission") => Some(Self::ConfidentialSubmitAdditiveShare),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialDifferentialPrivacyBudgetSpend") => Some(Self::ConfidentialSpendDifferentialPrivacyBudget),
            ("spec/openapi/kiva-confidential.yaml", "getConfidentialDifferentialPrivacyBudgets") => Some(Self::ConfidentialListDifferentialPrivacyBudgets),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialDifferentialPrivacyBudget") => Some(Self::ConfidentialRegisterDifferentialPrivacyBudget),
            ("spec/openapi/kiva-confidential.yaml", "getConfidentialDifferentialPrivacyCompositionSummaries") => Some(Self::ConfidentialListDifferentialPrivacyCompositionSummaries),
            ("spec/openapi/kiva-confidential.yaml", "getConfidentialDifferentialPrivacyReleases") => Some(Self::ConfidentialListDifferentialPrivacyReleases),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialDifferentialPrivacyRelease") => Some(Self::ConfidentialReleaseDifferentialPrivacyAggregate),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialDifferentialPrivacySampledRelease") => Some(Self::ConfidentialReleaseSampledDifferentialPrivacyAggregate),
            ("spec/openapi/kiva-confidential.yaml", "getConfidentialJobAuditEvents") => Some(Self::ConfidentialListJobAuditEvents),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialJobTransition") => Some(Self::ConfidentialTransitionJob),
            ("spec/openapi/kiva-confidential.yaml", "getConfidentialJobs") => Some(Self::ConfidentialListJobs),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialJob") => Some(Self::ConfidentialRegisterJob),
            ("spec/openapi/kiva-confidential.yaml", "getConfidentialParticipantAuditEvents") => Some(Self::ConfidentialListParticipantAuditEvents),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialParticipantTransition") => Some(Self::ConfidentialTransitionParticipant),
            ("spec/openapi/kiva-confidential.yaml", "getConfidentialParticipants") => Some(Self::ConfidentialListParticipants),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialParticipant") => Some(Self::ConfidentialRegisterParticipant),
            ("spec/openapi/kiva-confidential.yaml", "getConfidentialShamirThresholdReconstructions") => Some(Self::ConfidentialListShamirThresholdReconstructions),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialShamirThresholdReconstruction") => Some(Self::ConfidentialReconstructShamirThresholdSecret),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialThresholdReleaseApproval") => Some(Self::ConfidentialApproveThresholdRelease),
            ("spec/openapi/kiva-confidential.yaml", "getConfidentialThresholdReleaseRequests") => Some(Self::ConfidentialListThresholdReleaseRequests),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialThresholdReleaseRequest") => Some(Self::ConfidentialRegisterThresholdReleaseRequest),
            ("spec/openapi/kiva-confidential.yaml", "postConfidentialThresholdRelease") => Some(Self::ConfidentialReleaseThresholdAggregate),
            _ => None,
        }
    }
}

/// Resolves a stable catalog identifier to an operation specification.
#[must_use]
pub fn application_operation_spec_by_catalog_id(
    catalog_id: &str,
) -> Option<ApplicationOperationSpec> {
    ApplicationOperationId::from_catalog_id(catalog_id).map(ApplicationOperationId::spec)
}

/// Resolves an `OpenAPI` contract path and operationId to an operation specification.
#[must_use]
pub fn application_operation_spec_by_openapi_operation(
    openapi_contract: &str,
    operation_id: &str,
) -> Option<ApplicationOperationSpec> {
    ApplicationOperationId::from_openapi_operation(openapi_contract, operation_id)
        .map(ApplicationOperationId::spec)
}


#[cfg(test)]
mod tests {
    use super::{
        ApplicationOperationId, application_operation_spec_by_catalog_id,
        application_operation_spec_by_openapi_operation,
    };
    use super::super::registry::APPLICATION_OPERATION_SPECS;

    #[test]
    fn application_operation_ids_round_trip_to_specs() {
        for spec in APPLICATION_OPERATION_SPECS {
            let operation_id = ApplicationOperationId::from_catalog_id(spec.id)
                .expect("catalog id should resolve to a typed operation id");
            assert_eq!(operation_id.catalog_id(), spec.id);
            assert_eq!(operation_id.spec(), *spec);
            assert_eq!(
                ApplicationOperationId::from_openapi_operation(
                    spec.openapi_contract,
                    spec.operation_id,
                ),
                Some(operation_id),
            );
            assert_eq!(application_operation_spec_by_catalog_id(spec.id), Some(*spec));
            assert_eq!(
                application_operation_spec_by_openapi_operation(
                    spec.openapi_contract,
                    spec.operation_id,
                ),
                Some(*spec),
            );
        }
    }
}
