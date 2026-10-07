#![allow(clippy::wildcard_imports)]

use super::*;

/// Scope labels carried by PDS consent grants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ConsentScope {
    /// Direct access to raw learner records.
    #[serde(rename = "raw-records")]
    RawRecords,
    /// Personal-data metadata and derived views.
    #[serde(rename = "personal-data")]
    PersonalData,
    /// Aggregate insight products.
    #[serde(rename = "aggregate-insights")]
    AggregateInsights,
}

impl ConsentScope {
    /// Returns the wire label for this consent scope.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::RawRecords => "raw-records",
            Self::PersonalData => "personal-data",
            Self::AggregateInsights => "aggregate-insights",
        }
    }

    /// Parses a consent scope from its wire label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "raw-records" => Some(Self::RawRecords),
            "personal-data" => Some(Self::PersonalData),
            "aggregate-insights" => Some(Self::AggregateInsights),
            _other => None,
        }
    }
}

/// Persisted lifecycle state for a PDS consent grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ConsentState {
    /// Consent is active.
    #[serde(rename = "active")]
    Active,
    /// Consent has been revoked.
    #[serde(rename = "revoked")]
    Revoked,
}

impl ConsentState {
    /// Returns the wire label for this consent state.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
        }
    }

    /// Parses a consent state from its wire label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "revoked" => Some(Self::Revoked),
            _other => None,
        }
    }
}

/// Legal basis labels carried by PDS consent, retention, and data-product metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LegalBasis {
    /// Consent basis.
    #[serde(rename = "consent")]
    Consent,
    /// Public-task basis.
    #[serde(rename = "public-task")]
    PublicTask,
    /// Legitimate-interest basis.
    #[serde(rename = "legitimate-interest")]
    LegitimateInterest,
    /// Contractual basis.
    #[serde(rename = "contract")]
    Contract,
    /// Legal-obligation basis.
    #[serde(rename = "legal-obligation")]
    LegalObligation,
}

impl LegalBasis {
    /// Returns the wire label for this legal basis.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Consent => "consent",
            Self::PublicTask => "public-task",
            Self::LegitimateInterest => "legitimate-interest",
            Self::Contract => "contract",
            Self::LegalObligation => "legal-obligation",
        }
    }

    /// Parses a legal basis from its wire label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "consent" => Some(Self::Consent),
            "public-task" => Some(Self::PublicTask),
            "legitimate-interest" => Some(Self::LegitimateInterest),
            "contract" => Some(Self::Contract),
            "legal-obligation" => Some(Self::LegalObligation),
            _other => None,
        }
    }
}

/// Metadata class exposed by the PDS extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DataClass {
    /// Statement metadata.
    #[serde(rename = "statement-metadata")]
    Statement,
    /// Consent metadata.
    #[serde(rename = "consent-metadata")]
    Consent,
    /// Data-product metadata.
    #[serde(rename = "data-product-metadata")]
    DataProduct,
    /// Access-audit metadata.
    #[serde(rename = "access-audit-metadata")]
    AccessAudit,
    /// Lifecycle metadata.
    #[serde(rename = "lifecycle-metadata")]
    Lifecycle,
}

impl DataClass {
    /// Returns the wire label for this data class.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Statement => "statement-metadata",
            Self::Consent => "consent-metadata",
            Self::DataProduct => "data-product-metadata",
            Self::AccessAudit => "access-audit-metadata",
            Self::Lifecycle => "lifecycle-metadata",
        }
    }

    /// Parses a data class from its wire label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "statement-metadata" => Some(Self::Statement),
            "consent-metadata" => Some(Self::Consent),
            "data-product-metadata" => Some(Self::DataProduct),
            "access-audit-metadata" => Some(Self::AccessAudit),
            "lifecycle-metadata" => Some(Self::Lifecycle),
            _other => None,
        }
    }
}

/// Lifecycle event type labels exposed by the PDS lifecycle ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LifecycleEventType {
    /// A metadata export was produced.
    #[serde(rename = "exported")]
    Exported,
    /// Retention metadata was reviewed.
    #[serde(rename = "retention-reviewed")]
    RetentionReviewed,
    /// A correction workflow was requested.
    #[serde(rename = "correction-requested")]
    CorrectionRequested,
    /// A dispute workflow was opened.
    #[serde(rename = "dispute-opened")]
    DisputeOpened,
    /// A tombstone record was recorded.
    #[serde(rename = "tombstone-recorded")]
    TombstoneRecorded,
}

impl LifecycleEventType {
    /// Returns the wire label for this lifecycle event type.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Exported => "exported",
            Self::RetentionReviewed => "retention-reviewed",
            Self::CorrectionRequested => "correction-requested",
            Self::DisputeOpened => "dispute-opened",
            Self::TombstoneRecorded => "tombstone-recorded",
        }
    }

    /// Parses a lifecycle event type from its wire label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "exported" => Some(Self::Exported),
            "retention-reviewed" => Some(Self::RetentionReviewed),
            "correction-requested" => Some(Self::CorrectionRequested),
            "dispute-opened" => Some(Self::DisputeOpened),
            "tombstone-recorded" => Some(Self::TombstoneRecorded),
            _other => None,
        }
    }
}

/// Resource kind labels used by PDS lifecycle and retention metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LifecycleResourceKind {
    /// An xAPI statement resource.
    #[serde(rename = "xapi-statement")]
    XapiStatement,
    /// A consent grant resource.
    #[serde(rename = "consent-grant")]
    ConsentGrant,
    /// A data-product resource.
    #[serde(rename = "data-product")]
    DataProduct,
    /// A metadata export resource.
    #[serde(rename = "metadata-export")]
    MetadataExport,
}

impl LifecycleResourceKind {
    /// Returns the wire label for this lifecycle resource kind.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::XapiStatement => "xapi-statement",
            Self::ConsentGrant => "consent-grant",
            Self::DataProduct => "data-product",
            Self::MetadataExport => "metadata-export",
        }
    }

    /// Parses a lifecycle resource kind from its wire label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "xapi-statement" => Some(Self::XapiStatement),
            "consent-grant" => Some(Self::ConsentGrant),
            "data-product" => Some(Self::DataProduct),
            "metadata-export" => Some(Self::MetadataExport),
            _other => None,
        }
    }
}

/// Publication policy for PDS tombstone lifecycle metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TombstonePublicationRule {
    /// Tombstone is metadata-only.
    #[serde(rename = "metadata-only")]
    MetadataOnly,
    /// Tombstone is visible to the subject.
    #[serde(rename = "subject-visible")]
    SubjectVisible,
    /// Tombstone may be published in redacted form.
    #[serde(rename = "public-redacted")]
    PublicRedacted,
}

impl TombstonePublicationRule {
    /// Returns the wire label for this tombstone publication rule.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::MetadataOnly => "metadata-only",
            Self::SubjectVisible => "subject-visible",
            Self::PublicRedacted => "public-redacted",
        }
    }

    /// Parses a tombstone publication rule from its wire label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "metadata-only" => Some(Self::MetadataOnly),
            "subject-visible" => Some(Self::SubjectVisible),
            "public-redacted" => Some(Self::PublicRedacted),
            _other => None,
        }
    }
}

/// PDS lifecycle case type labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LifecycleCaseType {
    /// Correction workflow.
    #[serde(rename = "correction")]
    Correction,
    /// Dispute workflow.
    #[serde(rename = "dispute")]
    Dispute,
    /// Tombstone workflow.
    #[serde(rename = "tombstone")]
    Tombstone,
}

impl LifecycleCaseType {
    /// Returns the wire label for this lifecycle case type.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Correction => "correction",
            Self::Dispute => "dispute",
            Self::Tombstone => "tombstone",
        }
    }

    /// Parses a lifecycle case type from its wire label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "correction" => Some(Self::Correction),
            "dispute" => Some(Self::Dispute),
            "tombstone" => Some(Self::Tombstone),
            _other => None,
        }
    }
}

/// PDS lifecycle case status labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LifecycleCaseStatus {
    /// Case is open.
    #[serde(rename = "open")]
    Open,
    /// Case is under review.
    #[serde(rename = "under-review")]
    UnderReview,
    /// Case is resolved.
    #[serde(rename = "resolved")]
    Resolved,
    /// Case is rejected.
    #[serde(rename = "rejected")]
    Rejected,
    /// Case produced a tombstone.
    #[serde(rename = "tombstoned")]
    Tombstoned,
}

impl LifecycleCaseStatus {
    /// Returns the wire label for this lifecycle case status.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::UnderReview => "under-review",
            Self::Resolved => "resolved",
            Self::Rejected => "rejected",
            Self::Tombstoned => "tombstoned",
        }
    }

    /// Returns whether this status requires a non-empty resolution.
    #[must_use]
    pub const fn requires_resolution(self) -> bool {
        matches!(self, Self::Resolved | Self::Rejected | Self::Tombstoned)
    }

    /// Parses a lifecycle case status from its wire label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "open" => Some(Self::Open),
            "under-review" => Some(Self::UnderReview),
            "resolved" => Some(Self::Resolved),
            "rejected" => Some(Self::Rejected),
            "tombstoned" => Some(Self::Tombstoned),
            _other => None,
        }
    }
}

/// Public-sector governance deployment profile labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GovernanceProfileKind {
    /// Single-operator deployment profile.
    #[serde(rename = "single-operator")]
    SingleOperator,
    /// School-district deployment profile.
    #[serde(rename = "school-district")]
    SchoolDistrict,
    /// Multi-agency deployment profile.
    #[serde(rename = "multi-agency")]
    MultiAgency,
}

impl GovernanceProfileKind {
    /// Returns the wire label for this governance profile kind.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::SingleOperator => "single-operator",
            Self::SchoolDistrict => "school-district",
            Self::MultiAgency => "multi-agency",
        }
    }

    /// Parses a governance profile kind from its wire label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "single-operator" => Some(Self::SingleOperator),
            "school-district" => Some(Self::SchoolDistrict),
            "multi-agency" => Some(Self::MultiAgency),
            _other => None,
        }
    }
}
