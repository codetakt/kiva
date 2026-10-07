#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn confidential_differential_privacy_sampled_release_samples_once_and_hides_raw_evidence()
-> Result<(), String> {
    let state = super::AppState::default();
    compute_confidential_aggregate_result(&state).await?;
    let budget = create_differential_privacy_budget(&state).await?;
    assert_eq!(budget.status(), StatusCode::NO_CONTENT);

    let first = release_sampled_differential_privacy_aggregate(
        &state,
        "confidential-coordinator",
        differential_privacy_sampled_release_request(),
    )
    .await?;
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = response_json(first).await?;
    let first_noised_value = first_body
        .pointer("/noisedValue")
        .and_then(Value::as_i64)
        .ok_or_else(|| "sampled release should expose a noised integer".to_owned())?;
    assert_eq!(
        first_body.pointer("/mechanism"),
        Some(&json!("discreteLaplace"))
    );
    assert_eq!(
        first_body.pointer("/adjacencyModel"),
        Some(&json!("addRemoveOneSubject"))
    );
    assert!(first_body.pointer("/aggregate").is_none());
    assert!(first_body.pointer("/noiseValue").is_none());

    let duplicate = release_sampled_differential_privacy_aggregate(
        &state,
        "confidential-coordinator",
        differential_privacy_sampled_release_request(),
    )
    .await?;
    assert_eq!(duplicate.status(), StatusCode::OK);
    let duplicate_body = response_json(duplicate).await?;
    assert_eq!(
        duplicate_body
            .pointer("/noisedValue")
            .and_then(Value::as_i64),
        Some(first_noised_value)
    );

    let budget_audit = differential_privacy_budget_metadata(&state, transcript(51)).await;
    assert_eq!(budget_audit.status(), StatusCode::OK);
    let budget_body = response_json(budget_audit).await?;
    assert_eq!(
        budget_body.pointer("/budgets/0/spentEpsilonMicrounits"),
        Some(&json!(250_000))
    );
    assert_eq!(
        budget_body.pointer("/budgets/0/remainingEpsilonMicrounits"),
        Some(&json!(750_000))
    );

    let release_audit = differential_privacy_release_metadata(&state, transcript(63)).await;
    assert_eq!(release_audit.status(), StatusCode::OK);
    let release_body = response_json(release_audit).await?;
    assert_eq!(
        release_body
            .pointer("/releases")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        release_body
            .pointer("/releases/0/noisedValue")
            .and_then(Value::as_i64),
        Some(first_noised_value)
    );
    assert_eq!(
        release_body.pointer("/releases/0/adjacencyModel"),
        Some(&json!("addRemoveOneSubject"))
    );
    assert!(release_body.pointer("/releases/0/aggregate").is_none());
    assert!(release_body.pointer("/releases/0/noiseValue").is_none());
    Ok(())
}
