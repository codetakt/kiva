#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn confidential_additive_aggregate_persists_replay_records_in_memory() -> Result<(), String> {
    let state = super::AppState::default();
    register_confidential_signing_keys(&state).await?;
    let body = confidential_additive_aggregate_body(&confidential_aggregate_request())?;

    let response = super::post_confidential_additive_aggregate(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        body.clone(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await?;
    assert_eq!(body.pointer("/aggregate"), Some(&json!(42)));
    assert_eq!(
        body.pointer("/acceptedRecords")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(3)
    );

    let replay_response = super::post_confidential_additive_aggregate(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        body_from_request(),
    )
    .await;

    assert_eq!(replay_response.status(), StatusCode::CONFLICT);
    Ok(())
}

#[tokio::test]
async fn confidential_additive_aggregate_uses_submitted_shares() -> Result<(), String> {
    let state = super::AppState::default();
    register_confidential_signing_keys(&state).await?;
    submit_confidential_shares(&state).await?;

    let response = super::post_confidential_additive_aggregate(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_additive_aggregate_body(&confidential_aggregate_request_without_envelopes())?,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await?;
    assert_eq!(body.pointer("/aggregate"), Some(&json!(42)));
    Ok(())
}

#[tokio::test]
async fn confidential_additive_aggregate_rejects_unregistered_signing_keys() -> Result<(), String> {
    let response = super::post_confidential_additive_aggregate(
        confidential_state(&super::AppState::default()),
        confidential_headers(),
        Query(BTreeMap::new()),
        body_from_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn confidential_additive_aggregate_rejects_revoked_signing_key() -> Result<(), String> {
    let state = super::AppState::default();
    register_confidential_signing_keys(&state).await?;
    revoke_confidential_signing_key(&state, 22).await?;

    let response = super::post_confidential_additive_aggregate(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        body_from_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn confidential_additive_aggregate_accepts_rotated_signing_key() -> Result<(), String> {
    let state = super::AppState::default();
    register_confidential_signing_keys(&state).await?;
    rotate_confidential_signing_key(&state, 22, 24).await?;

    let response = super::post_confidential_additive_aggregate(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_additive_aggregate_body(&confidential_aggregate_request_with_rotated_key())?,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await?;
    assert_eq!(body.pointer("/aggregate"), Some(&json!(42)));
    Ok(())
}

#[tokio::test]
async fn confidential_share_signing_key_audit_lists_subject_keys() -> Result<(), String> {
    let state = super::AppState::default();
    register_confidential_signing_keys(&state).await?;
    revoke_confidential_signing_key(&state, 22).await?;

    let response = super::get_confidential_share_signing_keys(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await?;
    assert_eq!(
        body.pointer("/signingKeys")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(3)
    );
    assert_eq!(
        body.pointer("/signingKeys/1/status"),
        Some(&json!("revoked"))
    );
    assert_eq!(
        body.pointer("/signingKeys/0/oauthSubject"),
        Some(&json!("confidential-coordinator"))
    );
    Ok(())
}

#[tokio::test]
async fn confidential_replay_record_audit_lists_transcript_records() -> Result<(), String> {
    let state = super::AppState::default();
    register_confidential_signing_keys(&state).await?;
    let aggregate_response = super::post_confidential_additive_aggregate(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        body_from_request(),
    )
    .await;
    assert_eq!(aggregate_response.status(), StatusCode::OK);

    let mut query = BTreeMap::new();
    query.insert(
        "transcriptId".to_owned(),
        super::hex_lower(&transcript(1).bytes()),
    );
    let response = super::get_confidential_additive_share_replay_records(
        confidential_state(&state),
        confidential_headers(),
        Query(query),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await?;
    assert_eq!(
        body.pointer("/records")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(3)
    );
    assert!(body.pointer("/records/0/share").is_none());
    assert!(body.pointer("/records/0/signature").is_none());
    Ok(())
}

#[tokio::test]
async fn confidential_submitted_share_audit_lists_subject_metadata() -> Result<(), String> {
    let state = super::AppState::default();
    register_confidential_signing_keys(&state).await?;
    submit_confidential_shares(&state).await?;

    let mut query = BTreeMap::new();
    query.insert(
        "transcriptId".to_owned(),
        super::hex_lower(&transcript(1).bytes()),
    );
    let response = super::get_confidential_additive_share_submissions(
        confidential_state(&state),
        confidential_headers(),
        Query(query),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await?;
    assert_eq!(
        body.pointer("/submissions")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(3)
    );
    assert_eq!(
        body.pointer("/submissions/0/oauthSubject"),
        Some(&json!("confidential-coordinator"))
    );
    assert!(body.pointer("/submissions/0/share").is_none());
    assert!(body.pointer("/submissions/0/signature").is_none());
    assert!(body.pointer("/submissions/0/signedEnvelope").is_none());
    Ok(())
}
