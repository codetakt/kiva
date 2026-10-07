#![allow(clippy::wildcard_imports)]

use super::*;

pub(in crate::tests) async fn compute_confidential_aggregate_result(
    state: &super::AppState,
) -> Result<(), String> {
    register_confidential_signing_keys(state).await?;
    let response = super::post_confidential_additive_aggregate(
        confidential_state(state),
        confidential_headers(),
        empty_query(),
        body_from_request(),
    )
    .await;
    let response = expect_status(
        response,
        StatusCode::OK,
        "confidential aggregate computation",
    )?;
    let body = response_json(response).await?;
    if body.pointer("/aggregate") != Some(&json!(42)) {
        return Err("confidential aggregate computation returned unexpected value".to_owned());
    }
    Ok(())
}

pub(in crate::tests) async fn create_threshold_release_request(
    state: &super::AppState,
) -> Result<super::ConfidentialThresholdReleaseRequest, String> {
    create_threshold_release_request_for(state, "confidential-coordinator").await
}

pub(in crate::tests) async fn create_threshold_release_request_for(
    state: &super::AppState,
    token: &'static str,
) -> Result<super::ConfidentialThresholdReleaseRequest, String> {
    let request = threshold_release_request();
    let response = super::post_confidential_threshold_release_request(
        confidential_state(state),
        confidential_headers_for(token),
        empty_query(),
        confidential_threshold_release_body(&request)?,
    )
    .await;
    let _response = expect_status(
        response,
        StatusCode::NO_CONTENT,
        "threshold release request",
    )?;
    Ok(request)
}

pub(in crate::tests) async fn approve_threshold_release(
    state: &super::AppState,
    token: &'static str,
) -> Result<axum::response::Response, String> {
    Ok(super::post_confidential_threshold_release_approval(
        confidential_state(state),
        confidential_headers_for(token),
        empty_query(),
        confidential_threshold_release_approval_body(&threshold_release_approval_request())?,
    )
    .await)
}

pub(in crate::tests) async fn release_threshold_aggregate(
    state: &super::AppState,
    token: &'static str,
) -> Result<axum::response::Response, String> {
    Ok(super::post_confidential_threshold_release(
        confidential_state(state),
        confidential_headers_for(token),
        empty_query(),
        confidential_threshold_release_approval_body(&threshold_release_approval_request())?,
    )
    .await)
}

pub(in crate::tests) async fn threshold_release_metadata(
    state: &super::AppState,
    release_id: SecureAggregateTranscriptId,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    query.insert(
        "releaseId".to_owned(),
        super::hex_lower(&release_id.bytes()),
    );
    super::get_confidential_threshold_release_requests(
        confidential_state(state),
        confidential_headers(),
        Query(query),
    )
    .await
}

pub(in crate::tests) async fn reconstruct_shamir_secret(
    state: &super::AppState,
    token: &'static str,
) -> Result<axum::response::Response, String> {
    reconstruct_shamir_secret_with_request(state, token, shamir_reconstruction_request()).await
}

pub(in crate::tests) async fn reconstruct_shamir_secret_with_request(
    state: &super::AppState,
    token: &'static str,
    request: super::ConfidentialShamirThresholdReconstructionRequest,
) -> Result<axum::response::Response, String> {
    Ok(super::post_confidential_shamir_threshold_reconstruction(
        confidential_state(state),
        confidential_headers_for(token),
        empty_query(),
        confidential_shamir_reconstruction_body(&request)?,
    )
    .await)
}

pub(in crate::tests) async fn shamir_reconstruction_metadata(
    state: &super::AppState,
    token: &'static str,
    reconstruction_id: SecureAggregateTranscriptId,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    query.insert(
        "reconstructionId".to_owned(),
        super::hex_lower(&reconstruction_id.bytes()),
    );
    super::get_confidential_shamir_threshold_reconstructions(
        confidential_state(state),
        confidential_headers_for(token),
        Query(query),
    )
    .await
}

pub(in crate::tests) async fn create_differential_privacy_budget(
    state: &super::AppState,
) -> Result<axum::response::Response, String> {
    create_differential_privacy_budget_for(state, "confidential-coordinator").await
}

pub(in crate::tests) async fn create_differential_privacy_budget_for(
    state: &super::AppState,
    token: &'static str,
) -> Result<axum::response::Response, String> {
    Ok(super::post_confidential_differential_privacy_budget(
        confidential_state(state),
        confidential_headers_for(token),
        Query(BTreeMap::new()),
        confidential_differential_privacy_budget_body(&differential_privacy_budget_request())?,
    )
    .await)
}

pub(in crate::tests) async fn spend_differential_privacy_budget(
    state: &super::AppState,
    token: &'static str,
    epsilon_microunits: u64,
) -> Result<axum::response::Response, String> {
    Ok(super::post_confidential_differential_privacy_budget_spend(
        confidential_state(state),
        confidential_headers_for(token),
        Query(BTreeMap::new()),
        confidential_differential_privacy_budget_spend_body(
            &differential_privacy_budget_spend_request(epsilon_microunits),
        )?,
    )
    .await)
}

pub(in crate::tests) async fn release_differential_privacy_aggregate(
    state: &super::AppState,
    token: &'static str,
    request: super::ConfidentialDifferentialPrivacyReleaseRequest,
) -> Result<axum::response::Response, String> {
    Ok(super::post_confidential_differential_privacy_release(
        confidential_state(state),
        confidential_headers_for(token),
        Query(BTreeMap::new()),
        confidential_differential_privacy_release_body(&request)?,
    )
    .await)
}

pub(in crate::tests) async fn release_sampled_differential_privacy_aggregate(
    state: &super::AppState,
    token: &'static str,
    request: super::ConfidentialDifferentialPrivacySampledReleaseRequest,
) -> Result<axum::response::Response, String> {
    Ok(
        super::post_confidential_differential_privacy_sampled_release(
            confidential_state(state),
            confidential_headers_for(token),
            Query(BTreeMap::new()),
            confidential_differential_privacy_sampled_release_body(&request)?,
        )
        .await,
    )
}

pub(in crate::tests) async fn differential_privacy_budget_metadata(
    state: &super::AppState,
    budget_id: SecureAggregateTranscriptId,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    query.insert("budgetId".to_owned(), super::hex_lower(&budget_id.bytes()));
    super::get_confidential_differential_privacy_budgets(
        confidential_state(state),
        confidential_headers(),
        Query(query),
    )
    .await
}

pub(in crate::tests) async fn differential_privacy_composition_summaries(
    state: &super::AppState,
    budget_id: SecureAggregateTranscriptId,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    query.insert("budgetId".to_owned(), super::hex_lower(&budget_id.bytes()));
    super::get_confidential_differential_privacy_composition_summaries(
        confidential_state(state),
        confidential_headers(),
        Query(query),
    )
    .await
}

pub(in crate::tests) async fn differential_privacy_release_metadata(
    state: &super::AppState,
    release_id: SecureAggregateTranscriptId,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    query.insert(
        "releaseId".to_owned(),
        super::hex_lower(&release_id.bytes()),
    );
    super::get_confidential_differential_privacy_releases(
        confidential_state(state),
        confidential_headers(),
        Query(query),
    )
    .await
}

pub(in crate::tests) async fn register_active_confidential_participant(
    state: &super::AppState,
    owner_token: &'static str,
    participant_subject: &str,
    participant_index: usize,
    mechanism_roles: Vec<String>,
) -> Result<(), String> {
    let request = super::ConfidentialParticipantRequest {
        participant_subject: participant_subject.to_owned(),
        participant_index: nonzero(participant_index),
        mechanism_roles,
        authorization_scope: format!(
            "scope://kiva/confidential/{owner_token}/{participant_subject}/{participant_index}"
        ),
        key_ceremony_ref: format!(
            "ceremony://kiva/confidential/{participant_subject}/{participant_index}"
        ),
        recovery_contact_ref: format!(
            "recovery://kiva/confidential/{participant_subject}/{participant_index}"
        ),
        evidence_refs: vec![format!(
            "evidence://cc10/participant/{participant_subject}/{participant_index}"
        )],
    };
    let response = super::post_confidential_participant(
        confidential_state(state),
        confidential_headers_for(owner_token),
        Query(BTreeMap::new()),
        confidential_participant_body(&request)?,
    )
    .await;
    if response.status() != StatusCode::OK {
        return Err(format!(
            "confidential participant registration should succeed, got {}",
            response.status()
        ));
    }

    let transition = super::ConfidentialParticipantTransitionRequest {
        participant_subject: participant_subject.to_owned(),
        participant_index: nonzero(participant_index),
        target_status: super::ConfidentialParticipantStatus::Active,
        reason: Some("activate test participant".to_owned()),
        evidence_refs: vec![format!(
            "evidence://cc10/participant/{participant_subject}/{participant_index}/active"
        )],
    };
    let response = super::post_confidential_participant_transition(
        confidential_state(state),
        confidential_headers_for(owner_token),
        Query(BTreeMap::new()),
        confidential_participant_transition_body(&transition)?,
    )
    .await;
    if response.status() != StatusCode::OK {
        return Err(format!(
            "confidential participant activation should succeed, got {}",
            response.status()
        ));
    }
    Ok(())
}

pub(in crate::tests) async fn register_confidential_signing_participants(
    state: &super::AppState,
) -> Result<(), String> {
    for participant_index in 1..=3 {
        register_active_confidential_participant(
            state,
            "confidential-coordinator",
            "confidential-coordinator",
            participant_index,
            vec!["additive-share-contributor".to_owned()],
        )
        .await?;
    }
    Ok(())
}

pub(in crate::tests) async fn register_confidential_job_participants(
    state: &super::AppState,
    participant_subjects: &[String],
) -> Result<(), String> {
    let unique_subjects = participant_subjects
        .iter()
        .filter(|subject| !subject.trim().is_empty())
        .map(|subject| subject.trim().to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    for (offset, participant_subject) in unique_subjects.iter().enumerate() {
        register_active_confidential_participant(
            state,
            "confidential-coordinator",
            participant_subject,
            offset + 1,
            vec!["data-custodian".to_owned()],
        )
        .await?;
    }
    Ok(())
}

pub(in crate::tests) async fn register_confidential_signing_keys(
    state: &super::AppState,
) -> Result<(), String> {
    register_confidential_signing_participants(state).await?;
    for seed in [21, 22, 23] {
        let body = confidential_signing_key_body(&confidential_signing_key_request(seed))?;
        let response = super::post_confidential_share_signing_key(
            confidential_state(state),
            confidential_headers(),
            Query(BTreeMap::new()),
            body,
        )
        .await;
        if response.status() != StatusCode::NO_CONTENT {
            return Err(format!(
                "signing key registration should succeed, got {}",
                response.status()
            ));
        }
    }
    Ok(())
}

pub(in crate::tests) async fn submit_confidential_shares(
    state: &super::AppState,
) -> Result<(), String> {
    for signed_envelope in confidential_aggregate_request().signed_envelopes {
        let request = super::ConfidentialShareSubmissionRequest { signed_envelope };
        let body = confidential_share_submission_body(&request)?;
        let response = super::post_confidential_additive_share_submission(
            confidential_state(state),
            confidential_headers(),
            Query(BTreeMap::new()),
            body,
        )
        .await;
        if response.status() != StatusCode::NO_CONTENT {
            return Err(format!(
                "share submission should succeed, got {}",
                response.status()
            ));
        }
    }
    Ok(())
}

pub(in crate::tests) async fn rotate_confidential_signing_key(
    state: &super::AppState,
    old_seed: u8,
    new_seed: u8,
) -> Result<(), String> {
    let body = confidential_signing_key_rotation_body(&confidential_signing_key_rotation_request(
        old_seed, new_seed,
    ))?;
    let response = super::post_confidential_share_signing_key_rotation(
        confidential_state(state),
        confidential_headers(),
        Query(BTreeMap::new()),
        body,
    )
    .await;
    if response.status() != StatusCode::NO_CONTENT {
        return Err(format!(
            "signing key rotation should succeed, got {}",
            response.status()
        ));
    }
    Ok(())
}

pub(in crate::tests) async fn revoke_confidential_signing_key(
    state: &super::AppState,
    seed: u8,
) -> Result<(), String> {
    let body = confidential_signing_key_revocation_body(
        &confidential_signing_key_revocation_request(seed),
    )?;
    let response = super::post_confidential_share_signing_key_revocation(
        confidential_state(state),
        confidential_headers(),
        Query(BTreeMap::new()),
        body,
    )
    .await;
    if response.status() != StatusCode::NO_CONTENT {
        return Err(format!(
            "signing key revocation should succeed, got {}",
            response.status()
        ));
    }
    Ok(())
}

pub(in crate::tests) fn confidential_job_request(
    job_id: SecureAggregateTranscriptId,
) -> super::ConfidentialJobRequest {
    super::ConfidentialJobRequest {
        job_id,
        mechanism: "additive-secure-aggregation".to_owned(),
        mechanism_profile: "multi-agency-research-aggregate-v1".to_owned(),
        workload_id: "multi-agency-term-participation-count-v1".to_owned(),
        participant_subjects: vec![
            "agency-a".to_owned(),
            "agency-b".to_owned(),
            "agency-c".to_owned(),
        ],
        policy_binding_id: "policy://kiva/confidential/cc9/admission".to_owned(),
        deadline_at: super::rfc3339(OffsetDateTime::now_utc() + Duration::days(1)),
        admission_quota_key: "multi-agency-research-aggregate-v1".to_owned(),
        max_retry_count: 3,
        evidence_refs: vec!["evidence://cc9/job-created".to_owned()],
    }
}

pub(in crate::tests) async fn create_confidential_job(
    state: &super::AppState,
    request: super::ConfidentialJobRequest,
) -> Result<axum::response::Response, String> {
    register_confidential_job_participants(state, &request.participant_subjects).await?;
    Ok(super::post_confidential_job(
        confidential_state(state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_job_body(&request)?,
    )
    .await)
}

pub(in crate::tests) async fn transition_confidential_job(
    state: &super::AppState,
    job_id: SecureAggregateTranscriptId,
    target_status: ConfidentialJobStatus,
) -> Result<axum::response::Response, String> {
    let response = super::post_confidential_job_transition(
        confidential_state(state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_job_transition_body(&super::ConfidentialJobTransitionRequest {
            job_id,
            target_status,
            reason: Some(format!("transition to {}", target_status.label())),
            evidence_refs: vec![format!("evidence://cc9/{}", target_status.label())],
        })?,
    )
    .await;
    if response.status() != StatusCode::OK {
        return Err(format!(
            "confidential job transition to {} should succeed, got {}",
            target_status.label(),
            response.status(),
        ));
    }
    Ok(response)
}

pub(in crate::tests) async fn confidential_job_metadata(
    state: &super::AppState,
    job_id: SecureAggregateTranscriptId,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    query.insert("jobId".to_owned(), super::hex_lower(&job_id.bytes()));
    super::get_confidential_jobs(
        confidential_state(state),
        confidential_headers(),
        Query(query),
    )
    .await
}

pub(in crate::tests) async fn confidential_job_audit_events(
    state: &super::AppState,
    job_id: SecureAggregateTranscriptId,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    query.insert("jobId".to_owned(), super::hex_lower(&job_id.bytes()));
    super::get_confidential_job_audit_events(
        confidential_state(state),
        confidential_headers(),
        Query(query),
    )
    .await
}

pub(in crate::tests) async fn assert_confidential_job_status(
    state: &super::AppState,
    job_id: SecureAggregateTranscriptId,
    expected: &str,
) -> Result<(), String> {
    let response = confidential_job_metadata(state, job_id).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await?;
    assert_eq!(body.pointer("/jobs/0/status"), Some(&json!(expected)));
    Ok(())
}
