#![allow(clippy::wildcard_imports)]

use super::super::*;

/// Command requesting recording of a PDS lifecycle event.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecordLifecycleEventCommand {
    data_subject: String,
    request: LifecycleEventCommandInput,
    created_at: OffsetDateTime,
}

impl RecordLifecycleEventCommand {
    /// Creates a lifecycle-event recording command.
    #[must_use]
    pub fn new(
        data_subject: impl Into<String>,
        request: LifecycleEventCommandInput,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            data_subject: data_subject.into(),
            request,
            created_at,
        }
    }

    /// Returns the data subject owning the lifecycle event.
    #[must_use]
    pub fn data_subject(&self) -> &str {
        &self.data_subject
    }

    /// Returns the original lifecycle-event request.
    #[must_use]
    pub const fn request(&self) -> &LifecycleEventCommandInput {
        &self.request
    }

    /// Returns the event timestamp.
    #[must_use]
    pub const fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    /// Projects this command into the domain event it intends to append.
    pub fn lifecycle_event_recording_requested(
        &self,
    ) -> Result<LifecycleEventRecordingRequested, ValidationError> {
        LifecycleEventRecordingRequested::from_command(self)
    }
}

/// Command requesting registration of a PDS retention policy.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegisterRetentionPolicyCommand {
    owner_subject: String,
    request: RetentionPolicyCommandInput,
    created_at: OffsetDateTime,
}

impl RegisterRetentionPolicyCommand {
    /// Creates a retention-policy registration command.
    #[must_use]
    pub fn new(
        owner_subject: impl Into<String>,
        request: RetentionPolicyCommandInput,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            owner_subject: owner_subject.into(),
            request,
            created_at,
        }
    }

    /// Returns the subject that owns the retention policy.
    #[must_use]
    pub fn owner_subject(&self) -> &str {
        &self.owner_subject
    }

    /// Returns the original retention-policy request.
    #[must_use]
    pub const fn request(&self) -> &RetentionPolicyCommandInput {
        &self.request
    }

    /// Returns the registration timestamp.
    #[must_use]
    pub const fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    /// Projects this command into the domain event it intends to append.
    pub fn retention_policy_registration_requested(
        &self,
    ) -> Result<RetentionPolicyRegistrationRequested, ValidationError> {
        RetentionPolicyRegistrationRequested::from_command(self)
    }
}

/// Command requesting creation or transition of a PDS lifecycle case.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpsertLifecycleCaseCommand {
    data_subject: String,
    request: LifecycleCaseCommandInput,
    updated_at: OffsetDateTime,
}

impl UpsertLifecycleCaseCommand {
    /// Creates a lifecycle-case upsert command.
    #[must_use]
    pub fn new(
        data_subject: impl Into<String>,
        request: LifecycleCaseCommandInput,
        updated_at: OffsetDateTime,
    ) -> Self {
        Self {
            data_subject: data_subject.into(),
            request,
            updated_at,
        }
    }

    /// Returns the data subject that owns the case.
    #[must_use]
    pub fn data_subject(&self) -> &str {
        &self.data_subject
    }

    /// Returns the original lifecycle-case request.
    #[must_use]
    pub const fn request(&self) -> &LifecycleCaseCommandInput {
        &self.request
    }

    /// Returns the case update timestamp.
    #[must_use]
    pub const fn updated_at(&self) -> OffsetDateTime {
        self.updated_at
    }

    /// Projects this command into the domain event it intends to append.
    pub fn lifecycle_case_upsert_requested(
        &self,
    ) -> Result<LifecycleCaseUpsertRequested, ValidationError> {
        LifecycleCaseUpsertRequested::from_command(self)
    }
}
