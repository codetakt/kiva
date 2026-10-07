#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn confidential_job_lifecycle_persists_metadata_and_audit_in_memory() -> Result<(), String> {
    let state = super::AppState::default();
    let request = confidential_job_request(transcript(91));

    let created = create_confidential_job(&state, request.clone()).await?;
    assert_eq!(created.status(), StatusCode::OK);
    let created_body = response_json(created).await?;
    assert_eq!(created_body.pointer("/status"), Some(&json!("draft")));
    assert_eq!(
        created_body.pointer("/mechanism"),
        Some(&json!("additive-secure-aggregation"))
    );
    assert!(created_body.pointer("/share").is_none());
    assert!(created_body.pointer("/signature").is_none());
    assert!(created_body.pointer("/plaintext").is_none());
    assert!(created_body.pointer("/noise").is_none());

    let repeated = create_confidential_job(&state, request).await?;
    assert_eq!(repeated.status(), StatusCode::OK);

    assert_confidential_job_status(&state, transcript(91), "draft").await?;
    transition_confidential_job(&state, transcript(91), ConfidentialJobStatus::Admitted).await?;
    assert_confidential_job_status(&state, transcript(91), "admitted").await?;
    transition_confidential_job(&state, transcript(91), ConfidentialJobStatus::Running).await?;
    let running = confidential_job_metadata(&state, transcript(91)).await;
    assert_eq!(running.status(), StatusCode::OK);
    let running_body = response_json(running).await?;
    assert_eq!(
        running_body.pointer("/jobs/0/status"),
        Some(&json!("running"))
    );
    assert_eq!(running_body.pointer("/jobs/0/retryCount"), Some(&json!(1)));

    transition_confidential_job(
        &state,
        transcript(91),
        ConfidentialJobStatus::AwaitingRelease,
    )
    .await?;
    transition_confidential_job(&state, transcript(91), ConfidentialJobStatus::Released).await?;
    assert_confidential_job_status(&state, transcript(91), "released").await?;

    let invalid = super::post_confidential_job_transition(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_job_transition_body(&super::ConfidentialJobTransitionRequest {
            job_id: transcript(91),
            target_status: ConfidentialJobStatus::Running,
            reason: Some("invalid reopen".to_owned()),
            evidence_refs: vec!["evidence://cc9/reopen".to_owned()],
        })?,
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::CONFLICT);

    let audit = confidential_job_audit_events(&state, transcript(91)).await;
    assert_eq!(audit.status(), StatusCode::OK);
    let audit_body = response_json(audit).await?;
    assert_eq!(
        audit_body
            .pointer("/events")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(5)
    );
    assert!(audit_body.pointer("/events/0/share").is_none());
    assert!(audit_body.pointer("/events/0/signature").is_none());
    assert!(audit_body.pointer("/events/0/plaintext").is_none());
    assert!(audit_body.pointer("/events/0/noise").is_none());
    Ok(())
}

#[tokio::test]
async fn confidential_job_admission_rejects_invalid_metadata_in_memory() -> Result<(), String> {
    let state = super::AppState::default();
    let mut request = confidential_job_request(transcript(92));
    request.participant_subjects = vec!["agency-a".to_owned()];

    let response = create_confidential_job(&state, request).await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn confidential_job_transition_rejects_expired_deadline_in_memory() -> Result<(), String> {
    let state = super::AppState::default();
    let mut request = confidential_job_request(transcript(93));
    request.deadline_at = super::rfc3339(OffsetDateTime::now_utc() - Duration::seconds(1));
    let created = create_confidential_job(&state, request).await?;
    assert_eq!(created.status(), StatusCode::OK);

    let response = super::post_confidential_job_transition(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_job_transition_body(&super::ConfidentialJobTransitionRequest {
            job_id: transcript(93),
            target_status: ConfidentialJobStatus::Admitted,
            reason: Some("late admission".to_owned()),
            evidence_refs: vec!["evidence://cc9/late".to_owned()],
        })?,
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    Ok(())
}
