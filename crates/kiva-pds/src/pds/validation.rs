#![allow(clippy::wildcard_imports)]

use super::*;

/// Validation error for PDS boundary request rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    message: String,
}

impl ValidationError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the stable human-readable validation message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Consumes the validation error and returns its stable message.
    #[must_use]
    pub fn into_message(self) -> String {
        self.message
    }
}

/// Normalized retention-policy fields after PDS boundary validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedRetentionPolicyInput {
    /// Trimmed policy description.
    pub description: String,
}

/// Normalized lifecycle-case fields after PDS boundary validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedLifecycleCaseInput {
    /// Trimmed resource identifier.
    pub resource_id: String,
    /// Trimmed case summary.
    pub summary: String,
    /// Trimmed optional terminal resolution.
    pub resolution: Option<String>,
}

/// Normalized governance-profile fields after PDS boundary validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedGovernanceProfileInput {
    /// Trimmed tenant partition.
    pub tenant_partition: String,
    /// Trimmed subject partition.
    pub subject_partition: String,
    /// Trimmed data-custodian subject.
    pub data_custodian_subject: String,
    /// Trimmed platform-operator subject.
    pub platform_operator_subject: String,
    /// Trimmed policy-authority subject.
    pub policy_authority_subject: String,
    /// Trimmed analyst subject.
    pub analyst_subject: String,
    /// Trimmed auditor subject.
    pub auditor_subject: String,
    /// Trimmed optional identity-authority subject.
    pub identity_authority_subject: Option<String>,
    /// Trimmed emergency-access policy declaration.
    pub emergency_access_policy: String,
    /// Trimmed unique runbook references.
    pub runbook_refs: Vec<String>,
    /// Whether required operational roles are assigned to distinct subjects.
    pub distinct_role_subjects: bool,
    /// Whether identity authority is absent or distinct from required roles.
    pub identity_authority_absent_or_distinct: bool,
}

/// Validates PDS consent-grant request fields that are independent of storage.
pub fn validate_consent_grant_request(
    request: &ConsentGrantCommandInput,
    created_at: OffsetDateTime,
) -> Result<(), ValidationError> {
    if request.grantee_subject.trim().is_empty() {
        return Err(ValidationError::new(
            "PDS consent grant grantee_subject must not be empty",
        ));
    }
    if request.purpose.trim().is_empty() {
        return Err(ValidationError::new(
            "PDS consent grant purpose must not be empty",
        ));
    }
    if request
        .expires_at
        .is_some_and(|expires_at| expires_at <= created_at)
    {
        return Err(ValidationError::new(
            "PDS consent grant expires_at must be later than created_at",
        ));
    }
    Ok(())
}

/// Validates PDS data-product request shape, excluding external artifact lookup.
pub fn validate_data_product_request_shape(
    request: &DataProductRegistrationCommandInput,
) -> Result<(), ValidationError> {
    if request.title.trim().is_empty() {
        return Err(ValidationError::new(
            "PDS data product title must not be empty",
        ));
    }
    if request.purpose.trim().is_empty() {
        return Err(ValidationError::new(
            "PDS data product purpose must not be empty",
        ));
    }
    if request.data_classes.is_empty() {
        return Err(ValidationError::new(
            "PDS data product data_classes must not be empty",
        ));
    }
    let mut unique_data_classes = BTreeSet::new();
    if !request
        .data_classes
        .iter()
        .all(|data_class| unique_data_classes.insert(*data_class))
    {
        return Err(ValidationError::new(
            "PDS data product data_classes must not contain duplicates",
        ));
    }
    Ok(())
}

/// Validates PDS lifecycle-event request fields that are independent of storage.
pub fn validate_lifecycle_event_request(
    request: &LifecycleEventCommandInput,
) -> Result<(), ValidationError> {
    if request.resource_id.trim().is_empty() {
        return Err(ValidationError::new(
            "PDS lifecycle event resource_id must not be empty",
        ));
    }
    if request.reason.trim().is_empty() {
        return Err(ValidationError::new(
            "PDS lifecycle event reason must not be empty",
        ));
    }
    Ok(())
}

/// Validates and normalizes PDS retention-policy input.
pub fn normalize_retention_policy_request(
    request: &RetentionPolicyCommandInput,
) -> Result<NormalizedRetentionPolicyInput, ValidationError> {
    if request.retention_period_days == 0 {
        return Err(ValidationError::new(
            "PDS retention policy retention_period_days must be positive",
        ));
    }
    if request.review_interval_days == 0 {
        return Err(ValidationError::new(
            "PDS retention policy review_interval_days must be positive",
        ));
    }
    if request
        .tombstone_after_days
        .is_some_and(|days| days < request.retention_period_days)
    {
        return Err(ValidationError::new(
            "PDS retention policy tombstone_after_days must be greater than or equal to retention_period_days",
        ));
    }
    Ok(NormalizedRetentionPolicyInput {
        description: non_empty_trimmed_with_prefix(
            &request.description,
            "PDS retention policy",
            "description",
        )?,
    })
}

/// Validates and normalizes PDS lifecycle-case input.
pub fn normalize_lifecycle_case_request(
    request: &LifecycleCaseCommandInput,
) -> Result<NormalizedLifecycleCaseInput, ValidationError> {
    let resource_id =
        non_empty_trimmed_with_prefix(&request.resource_id, "PDS lifecycle case", "resource_id")?;
    let summary = non_empty_trimmed_with_prefix(&request.summary, "PDS lifecycle case", "summary")?;
    let resolution = request
        .resolution
        .as_ref()
        .map(|resolution| {
            non_empty_trimmed_with_prefix(resolution, "PDS lifecycle case", "resolution")
        })
        .transpose()?;
    if request.status.requires_resolution() && resolution.is_none() {
        return Err(ValidationError::new(
            "PDS lifecycle case terminal status requires resolution",
        ));
    }
    if request.status == LifecycleCaseStatus::Tombstoned
        && request.case_type != LifecycleCaseType::Tombstone
    {
        return Err(ValidationError::new(
            "PDS lifecycle case tombstoned status requires case_type tombstone",
        ));
    }
    if request.case_type == LifecycleCaseType::Tombstone && request.tombstone_publication.is_none()
    {
        return Err(ValidationError::new(
            "PDS lifecycle tombstone case requires tombstone_publication",
        ));
    }
    Ok(NormalizedLifecycleCaseInput {
        resource_id,
        summary,
        resolution,
    })
}

/// Validates and normalizes PDS governance-profile input.
pub fn normalize_governance_profile_request(
    request: &GovernanceProfileCommandInput,
) -> Result<NormalizedGovernanceProfileInput, ValidationError> {
    let tenant_partition = non_empty_trimmed(&request.tenant_partition, "tenant_partition")?;
    let subject_partition = non_empty_trimmed(&request.subject_partition, "subject_partition")?;
    let data_custodian_subject =
        non_empty_trimmed(&request.data_custodian_subject, "data_custodian_subject")?;
    let platform_operator_subject = non_empty_trimmed(
        &request.platform_operator_subject,
        "platform_operator_subject",
    )?;
    let policy_authority_subject = non_empty_trimmed(
        &request.policy_authority_subject,
        "policy_authority_subject",
    )?;
    let analyst_subject = non_empty_trimmed(&request.analyst_subject, "analyst_subject")?;
    let auditor_subject = non_empty_trimmed(&request.auditor_subject, "auditor_subject")?;
    let identity_authority_subject = request
        .identity_authority_subject
        .as_ref()
        .map(|subject| non_empty_trimmed(subject, "identity_authority_subject"))
        .transpose()?;
    let emergency_access_policy =
        non_empty_trimmed(&request.emergency_access_policy, "emergency_access_policy")?;
    let runbook_refs = non_empty_unique_list(&request.runbook_refs, "runbook_refs")?;
    let role_subjects = [
        data_custodian_subject.as_str(),
        platform_operator_subject.as_str(),
        policy_authority_subject.as_str(),
        analyst_subject.as_str(),
        auditor_subject.as_str(),
    ];
    let distinct_role_subjects =
        role_subjects.iter().copied().collect::<BTreeSet<_>>().len() == role_subjects.len();
    let identity_authority_absent_or_distinct =
        identity_authority_subject.as_ref().is_none_or(|subject| {
            ![
                data_custodian_subject.as_str(),
                platform_operator_subject.as_str(),
                policy_authority_subject.as_str(),
                analyst_subject.as_str(),
                auditor_subject.as_str(),
            ]
            .contains(&subject.as_str())
        });
    Ok(NormalizedGovernanceProfileInput {
        tenant_partition,
        subject_partition,
        data_custodian_subject,
        platform_operator_subject,
        policy_authority_subject,
        analyst_subject,
        auditor_subject,
        identity_authority_subject,
        emergency_access_policy,
        runbook_refs,
        distinct_role_subjects,
        identity_authority_absent_or_distinct,
    })
}

fn non_empty_trimmed(value: &str, field: &str) -> Result<String, ValidationError> {
    non_empty_trimmed_with_prefix(value, "PDS governance profile", field)
}

fn non_empty_trimmed_with_prefix(
    value: &str,
    prefix: &str,
    field: &str,
) -> Result<String, ValidationError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(ValidationError::new(format!(
            "{prefix} {field} must not be empty"
        )));
    }
    Ok(value)
}

fn non_empty_unique_list(values: &[String], field: &str) -> Result<Vec<String>, ValidationError> {
    if values.is_empty() {
        return Err(ValidationError::new(format!(
            "PDS governance profile {field} must not be empty"
        )));
    }
    let mut unique_values = BTreeSet::new();
    let mut normalized_values = Vec::with_capacity(values.len());
    for value in values {
        let value = non_empty_trimmed(value, field)?;
        if !unique_values.insert(value.clone()) {
            return Err(ValidationError::new(format!(
                "PDS governance profile {field} must not contain duplicates"
            )));
        }
        normalized_values.push(value);
    }
    Ok(normalized_values)
}
