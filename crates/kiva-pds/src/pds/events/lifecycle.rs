#![allow(clippy::wildcard_imports)]

use super::super::*;

/// Domain event produced when a lifecycle event recording command is admitted.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LifecycleEventRecordingRequested {
    data_subject: String,
    event_id: Uuid,
    event_type: LifecycleEventType,
    resource_kind: LifecycleResourceKind,
    resource_id: String,
    reason: String,
    created_at: OffsetDateTime,
}

impl LifecycleEventRecordingRequested {
    /// Projects a lifecycle-event command into a validated event.
    pub fn from_command(command: &RecordLifecycleEventCommand) -> Result<Self, ValidationError> {
        validate_lifecycle_event_request(command.request())?;
        let request = command.request();
        Ok(Self {
            data_subject: command.data_subject().to_owned(),
            event_id: request.event_id,
            event_type: request.event_type,
            resource_kind: request.resource_kind,
            resource_id: request.resource_id.trim().to_owned(),
            reason: request.reason.trim().to_owned(),
            created_at: command.created_at(),
        })
    }

    /// Returns the data subject owning the lifecycle event.
    #[must_use]
    pub fn data_subject(&self) -> &str {
        &self.data_subject
    }

    /// Returns the stable lifecycle event identifier.
    #[must_use]
    pub const fn event_id(&self) -> Uuid {
        self.event_id
    }

    /// Returns the lifecycle event type.
    #[must_use]
    pub const fn event_type(&self) -> LifecycleEventType {
        self.event_type
    }

    /// Returns the resource kind affected by the event.
    #[must_use]
    pub const fn resource_kind(&self) -> LifecycleResourceKind {
        self.resource_kind
    }

    /// Returns the normalized resource identifier.
    #[must_use]
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    /// Returns the normalized reason.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Returns the event timestamp.
    #[must_use]
    pub const fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

/// Domain event produced when a retention-policy registration command is admitted.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetentionPolicyRegistrationRequested {
    owner_subject: String,
    policy_id: Uuid,
    resource_kind: LifecycleResourceKind,
    data_class: DataClass,
    retention_period_days: u32,
    review_interval_days: u32,
    tombstone_after_days: Option<u32>,
    tombstone_publication: TombstonePublicationRule,
    legal_basis: LegalBasis,
    description: String,
    created_at: OffsetDateTime,
}

impl RetentionPolicyRegistrationRequested {
    /// Projects a retention-policy command into a validated event.
    pub fn from_command(command: &RegisterRetentionPolicyCommand) -> Result<Self, ValidationError> {
        let normalized = normalize_retention_policy_request(command.request())?;
        let request = command.request();
        Ok(Self {
            owner_subject: command.owner_subject().to_owned(),
            policy_id: request.policy_id,
            resource_kind: request.resource_kind,
            data_class: request.data_class,
            retention_period_days: request.retention_period_days,
            review_interval_days: request.review_interval_days,
            tombstone_after_days: request.tombstone_after_days,
            tombstone_publication: request.tombstone_publication,
            legal_basis: request.legal_basis,
            description: normalized.description,
            created_at: command.created_at(),
        })
    }

    /// Returns the subject that owns the policy.
    #[must_use]
    pub fn owner_subject(&self) -> &str {
        &self.owner_subject
    }

    /// Returns the stable retention policy identifier.
    #[must_use]
    pub const fn policy_id(&self) -> Uuid {
        self.policy_id
    }

    /// Returns the resource kind covered by the policy.
    #[must_use]
    pub const fn resource_kind(&self) -> LifecycleResourceKind {
        self.resource_kind
    }

    /// Returns the metadata class covered by the policy.
    #[must_use]
    pub const fn data_class(&self) -> DataClass {
        self.data_class
    }

    /// Returns the retention period in days.
    #[must_use]
    pub const fn retention_period_days(&self) -> u32 {
        self.retention_period_days
    }

    /// Returns the review interval in days.
    #[must_use]
    pub const fn review_interval_days(&self) -> u32 {
        self.review_interval_days
    }

    /// Returns the optional tombstone threshold in days.
    #[must_use]
    pub const fn tombstone_after_days(&self) -> Option<u32> {
        self.tombstone_after_days
    }

    /// Returns the tombstone publication rule.
    #[must_use]
    pub const fn tombstone_publication(&self) -> TombstonePublicationRule {
        self.tombstone_publication
    }

    /// Returns the legal basis attached to the policy.
    #[must_use]
    pub const fn legal_basis(&self) -> LegalBasis {
        self.legal_basis
    }

    /// Returns the normalized policy description.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the registration timestamp.
    #[must_use]
    pub const fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

/// Domain event produced when a lifecycle-case command is admitted.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LifecycleCaseUpsertRequested {
    data_subject: String,
    case_id: Uuid,
    case_type: LifecycleCaseType,
    resource_kind: LifecycleResourceKind,
    resource_id: String,
    status: LifecycleCaseStatus,
    retention_policy_id: Option<Uuid>,
    summary: String,
    resolution: Option<String>,
    tombstone_publication: Option<TombstonePublicationRule>,
    updated_at: OffsetDateTime,
}

impl LifecycleCaseUpsertRequested {
    /// Projects a lifecycle-case command into a validated event.
    pub fn from_command(command: &UpsertLifecycleCaseCommand) -> Result<Self, ValidationError> {
        let normalized = normalize_lifecycle_case_request(command.request())?;
        let request = command.request();
        Ok(Self {
            data_subject: command.data_subject().to_owned(),
            case_id: request.case_id,
            case_type: request.case_type,
            resource_kind: request.resource_kind,
            resource_id: normalized.resource_id,
            status: request.status,
            retention_policy_id: request.retention_policy_id,
            summary: normalized.summary,
            resolution: normalized.resolution,
            tombstone_publication: request.tombstone_publication,
            updated_at: command.updated_at(),
        })
    }

    /// Returns the data subject that owns the case.
    #[must_use]
    pub fn data_subject(&self) -> &str {
        &self.data_subject
    }

    /// Returns the lifecycle case identifier.
    #[must_use]
    pub const fn case_id(&self) -> Uuid {
        self.case_id
    }

    /// Returns the lifecycle case type.
    #[must_use]
    pub const fn case_type(&self) -> LifecycleCaseType {
        self.case_type
    }

    /// Returns the resource kind affected by the case.
    #[must_use]
    pub const fn resource_kind(&self) -> LifecycleResourceKind {
        self.resource_kind
    }

    /// Returns the normalized resource identifier.
    #[must_use]
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    /// Returns the requested lifecycle status.
    #[must_use]
    pub const fn status(&self) -> LifecycleCaseStatus {
        self.status
    }

    /// Returns the linked retention policy identifier.
    #[must_use]
    pub const fn retention_policy_id(&self) -> Option<Uuid> {
        self.retention_policy_id
    }

    /// Returns the normalized summary.
    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// Returns the normalized terminal resolution.
    #[must_use]
    pub fn resolution(&self) -> Option<&str> {
        self.resolution.as_deref()
    }

    /// Returns the requested tombstone publication rule.
    #[must_use]
    pub const fn tombstone_publication(&self) -> Option<TombstonePublicationRule> {
        self.tombstone_publication
    }

    /// Returns the case update timestamp.
    #[must_use]
    pub const fn updated_at(&self) -> OffsetDateTime {
        self.updated_at
    }
}
