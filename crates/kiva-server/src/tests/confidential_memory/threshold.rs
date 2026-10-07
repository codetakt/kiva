#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn confidential_threshold_release_requires_both_authorities() -> Result<(), String> {
    let state = super::AppState::default();
    compute_confidential_aggregate_result(&state).await?;
    let request = create_threshold_release_request(&state).await?;

    let left_response = approve_threshold_release(&state, "policy-authority-left").await?;
    assert_eq!(left_response.status(), StatusCode::NO_CONTENT);

    let early_release = release_threshold_aggregate(&state, "policy-authority-left").await?;
    assert_eq!(early_release.status(), StatusCode::CONFLICT);

    let pending_metadata = threshold_release_metadata(&state, request.release_id).await;
    assert_eq!(pending_metadata.status(), StatusCode::OK);
    let pending_body = response_json(pending_metadata).await?;
    assert_eq!(
        pending_body.pointer("/releases/0/status"),
        Some(&json!("pending"))
    );
    assert!(pending_body.pointer("/releases/0/aggregate").is_none());

    let right_response = approve_threshold_release(&state, "policy-authority-right").await?;
    assert_eq!(right_response.status(), StatusCode::NO_CONTENT);

    let final_response = release_threshold_aggregate(&state, "policy-authority-right").await?;
    assert_eq!(final_response.status(), StatusCode::OK);
    let final_body = response_json(final_response).await?;
    assert_eq!(final_body.pointer("/aggregate"), Some(&json!(42)));
    assert_eq!(
        final_body.pointer("/release/status"),
        Some(&json!("released"))
    );

    let audit_response = threshold_release_metadata(&state, request.release_id).await;
    assert_eq!(audit_response.status(), StatusCode::OK);
    let audit_body = response_json(audit_response).await?;
    assert_eq!(
        audit_body.pointer("/releases/0/status"),
        Some(&json!("released"))
    );
    assert_eq!(
        audit_body
            .pointer("/releases/0/approvedAuthoritySubjects")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(2)
    );
    assert!(audit_body.pointer("/releases/0/aggregate").is_none());
    assert!(audit_body.pointer("/releases/0/share").is_none());
    assert!(audit_body.pointer("/releases/0/signature").is_none());
    Ok(())
}

#[tokio::test]
async fn confidential_threshold_release_rejects_non_authority() -> Result<(), String> {
    let state = super::AppState::default();
    compute_confidential_aggregate_result(&state).await?;
    create_threshold_release_request(&state).await?;

    let response = approve_threshold_release(&state, "policy-auditor").await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let final_response = release_threshold_aggregate(&state, "policy-auditor").await?;
    assert_eq!(final_response.status(), StatusCode::FORBIDDEN);
    Ok(())
}

#[tokio::test]
async fn confidential_threshold_release_requires_computed_aggregate() -> Result<(), String> {
    let state = super::AppState::default();
    let request = threshold_release_request();
    let response = super::post_confidential_threshold_release_request(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_threshold_release_body(&request)?,
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    Ok(())
}
