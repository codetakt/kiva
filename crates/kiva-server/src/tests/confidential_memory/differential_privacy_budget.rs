#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn confidential_differential_privacy_budget_tracks_spends() -> Result<(), String> {
    let state = super::AppState::default();
    let response = create_differential_privacy_budget(&state).await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let duplicate = create_differential_privacy_budget(&state).await?;
    assert_eq!(duplicate.status(), StatusCode::NO_CONTENT);

    let spend =
        spend_differential_privacy_budget(&state, "confidential-coordinator", 300_000).await?;
    assert_eq!(spend.status(), StatusCode::OK);
    let body = response_json(spend).await?;
    assert_eq!(
        body.pointer("/spentEpsilonMicrounits"),
        Some(&json!(300_000))
    );
    assert_eq!(
        body.pointer("/remainingEpsilonMicrounits"),
        Some(&json!(700_000))
    );

    let audit = differential_privacy_budget_metadata(&state, transcript(51)).await;
    assert_eq!(audit.status(), StatusCode::OK);
    let audit_body = response_json(audit).await?;
    assert_eq!(
        audit_body.pointer("/budgets/0/spentEpsilonMicrounits"),
        Some(&json!(300_000))
    );
    assert_eq!(
        audit_body.pointer("/budgets/0/ownerSubject"),
        Some(&json!("confidential-coordinator"))
    );
    Ok(())
}

#[tokio::test]
async fn confidential_differential_privacy_budget_rejects_over_spend_and_foreign_subject()
-> Result<(), String> {
    let state = super::AppState::default();
    let response = create_differential_privacy_budget(&state).await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let foreign_spend =
        spend_differential_privacy_budget(&state, "policy-authority-left", 1).await?;
    assert_eq!(foreign_spend.status(), StatusCode::FORBIDDEN);

    let first =
        spend_differential_privacy_budget(&state, "confidential-coordinator", 800_000).await?;
    assert_eq!(first.status(), StatusCode::OK);

    let overflow =
        spend_differential_privacy_budget(&state, "confidential-coordinator", 200_001).await?;
    assert_eq!(overflow.status(), StatusCode::CONFLICT);

    let audit = differential_privacy_budget_metadata(&state, transcript(51)).await;
    assert_eq!(audit.status(), StatusCode::OK);
    let audit_body = response_json(audit).await?;
    assert_eq!(
        audit_body.pointer("/budgets/0/spentEpsilonMicrounits"),
        Some(&json!(800_000))
    );
    Ok(())
}
