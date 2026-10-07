#![allow(clippy::wildcard_imports)]

use super::*;

pub(crate) fn validate_confidential_job_transition_request(
    job: &ConfidentialJobState,
    request: &ConfidentialJobTransitionRequest,
    now: OffsetDateTime,
    active_for_quota: usize,
    running_jobs: usize,
) -> Result<(), ConfidentialJobError> {
    validate_confidential_job_evidence_ref_values(&request.evidence_refs)?;
    if let Some(reason) = request.reason.as_deref() {
        validate_non_empty_confidential_job_text("reason", reason)?;
    }
    if !confidential_job_transition_is_allowed(job.status, request.target_status) {
        return Err(ConfidentialJobError::Transition(
            ConfidentialJobTransitionError::InvalidTransition,
        ));
    }
    if request.target_status == ConfidentialJobStatus::Expired && now < job.deadline_at {
        return Err(ConfidentialJobError::InvalidRequest(
            "confidential job can expire only after its deadline".to_owned(),
        ));
    }
    if !job.status.is_terminal()
        && request.target_status != ConfidentialJobStatus::Expired
        && now >= job.deadline_at
    {
        return Err(ConfidentialJobError::Admission(
            ConfidentialJobAdmissionError::DeadlineNotFuture,
        ));
    }

    match request.target_status {
        ConfidentialJobStatus::Admitted => {
            validate_confidential_job_admission(ConfidentialJobAdmissionFacts::new(
                job.participant_subjects.len() >= 2,
                !job.policy_binding_id.trim().is_empty(),
                now < job.deadline_at,
                active_for_quota < CONFIDENTIAL_JOB_MAX_ACTIVE_PER_QUOTA_KEY,
                true,
                true,
                job.retry_count < job.max_retry_count,
            ))
            .map_err(ConfidentialJobError::Admission)
        }
        ConfidentialJobStatus::Running => {
            validate_confidential_job_admission(ConfidentialJobAdmissionFacts::new(
                job.participant_subjects.len() >= 2,
                !job.policy_binding_id.trim().is_empty(),
                now < job.deadline_at,
                true,
                running_jobs < CONFIDENTIAL_JOB_MAX_RUNNING_PER_TENANT,
                running_jobs < CONFIDENTIAL_JOB_MAX_RUNNING_PER_TENANT,
                job.retry_count < job.max_retry_count,
            ))
            .map_err(ConfidentialJobError::Admission)
        }
        _other => Ok(()),
    }
}

pub(crate) fn validate_confidential_participant_transition_request(
    request: &ConfidentialParticipantTransitionRequest,
) -> Result<(), ConfidentialParticipantError> {
    validate_non_empty_confidential_participant_text(
        "participant_subject",
        &request.participant_subject,
    )?;
    if let Some(reason) = request.reason.as_deref() {
        validate_non_empty_confidential_participant_text("reason", reason)?;
    }
    validate_confidential_participant_evidence_refs(&request.evidence_refs)
}

pub(crate) fn validate_confidential_job_mechanism(value: &str) -> Result<(), ConfidentialJobError> {
    match value {
        "additive-secure-aggregation"
        | "differential-privacy-release"
        | "shamir-vss-admission"
        | "threshold-frost-signing"
        | "psi-cohort-hash-token"
        | "psi-oprf-dh-voprf"
        | "tee-portable-attestation"
        | "tee-aws-nitro-attestation"
        | "fhe-sum-feasibility"
        | "fhe-sum-runtime" => Ok(()),
        _ => Err(ConfidentialJobError::InvalidRequest(
            "confidential job mechanism is not supported".to_owned(),
        )),
    }
}

pub(crate) fn validate_non_empty_confidential_job_text(
    name: &str,
    value: &str,
) -> Result<(), ConfidentialJobError> {
    if value.trim().is_empty() {
        Err(ConfidentialJobError::InvalidRequest(format!(
            "confidential job {name} must not be empty",
        )))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_confidential_job_subjects(
    subjects: &[String],
) -> Result<(), ConfidentialJobError> {
    if subjects.len() < 2 {
        return Err(ConfidentialJobError::Admission(
            ConfidentialJobAdmissionError::TooFewParticipants,
        ));
    }
    let mut canonical = BTreeSet::new();
    for subject in subjects {
        validate_non_empty_confidential_job_text("participant_subjects", subject)?;
        if !canonical.insert(subject.trim().to_owned()) {
            return Err(ConfidentialJobError::InvalidRequest(
                "confidential job participant_subjects must not contain duplicates".to_owned(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_confidential_job_evidence_refs(
    evidence_refs: &[String],
) -> Result<(), ConfidentialJobError> {
    if evidence_refs.is_empty() {
        return Err(ConfidentialJobError::InvalidRequest(
            "confidential job evidence_refs must not be empty".to_owned(),
        ));
    }
    validate_confidential_job_evidence_ref_values(evidence_refs)
}

pub(crate) fn validate_confidential_job_evidence_ref_values(
    evidence_refs: &[String],
) -> Result<(), ConfidentialJobError> {
    for evidence_ref in evidence_refs {
        validate_non_empty_confidential_job_text("evidence_refs", evidence_ref)?;
    }
    Ok(())
}

pub(crate) fn validate_non_empty_confidential_participant_text(
    name: &str,
    value: &str,
) -> Result<(), ConfidentialParticipantError> {
    if value.trim().is_empty() {
        Err(ConfidentialParticipantError::InvalidRequest(format!(
            "confidential participant {name} must not be empty"
        )))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_confidential_participant_roles(
    roles: &[String],
) -> Result<(), ConfidentialParticipantError> {
    if roles.is_empty() {
        return Err(ConfidentialParticipantError::InvalidRequest(
            "confidential participant mechanism_roles must not be empty".to_owned(),
        ));
    }
    let mut seen = BTreeSet::new();
    for role in roles {
        validate_non_empty_confidential_participant_text("mechanism_roles", role)?;
        match role.trim() {
            "additive-share-contributor"
            | "data-custodian"
            | "policy-authority"
            | "psi-participant"
            | "tee-operator"
            | "fhe-key-authority"
            | "auditor" => {}
            _ => {
                return Err(ConfidentialParticipantError::InvalidRequest(
                    "confidential participant mechanism role is not supported".to_owned(),
                ));
            }
        }
        if !seen.insert(role.trim()) {
            return Err(ConfidentialParticipantError::InvalidRequest(
                "confidential participant mechanism_roles must not contain duplicates".to_owned(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_confidential_participant_evidence_refs(
    evidence_refs: &[String],
) -> Result<(), ConfidentialParticipantError> {
    if evidence_refs.is_empty() {
        return Err(ConfidentialParticipantError::InvalidRequest(
            "confidential participant evidence_refs must not be empty".to_owned(),
        ));
    }
    for evidence_ref in evidence_refs {
        validate_non_empty_confidential_participant_text("evidence_refs", evidence_ref)?;
    }
    Ok(())
}

pub(crate) fn canonical_confidential_job_subjects(subjects: Vec<String>) -> Vec<String> {
    let mut values = subjects
        .into_iter()
        .map(|subject| subject.trim().to_owned())
        .collect::<Vec<_>>();
    values.sort();
    values
}

pub(crate) fn canonical_unique_strings(values: Vec<String>) -> Vec<String> {
    let mut set = BTreeSet::new();
    for value in values {
        set.insert(value.trim().to_owned());
    }
    set.into_iter().collect()
}

pub(crate) fn append_unique_strings(target: &mut Vec<String>, values: &[String]) {
    let mut set = target.iter().cloned().collect::<BTreeSet<_>>();
    for value in values {
        set.insert(value.trim().to_owned());
    }
    *target = set.into_iter().collect();
}

pub(crate) const fn confidential_job_status_counts_against_admission_quota(
    status: ConfidentialJobStatus,
) -> bool {
    matches!(
        status,
        ConfidentialJobStatus::Admitted
            | ConfidentialJobStatus::Running
            | ConfidentialJobStatus::AwaitingRelease
    )
}
