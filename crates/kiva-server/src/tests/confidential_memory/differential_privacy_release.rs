#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn confidential_differential_privacy_release_spends_once_and_hides_raw_evidence()
-> Result<(), String> {
    let state = super::AppState::default();
    compute_confidential_aggregate_result(&state).await?;
    let budget = create_differential_privacy_budget(&state).await?;
    assert_eq!(budget.status(), StatusCode::NO_CONTENT);

    let first = release_differential_privacy_aggregate(
        &state,
        "confidential-coordinator",
        differential_privacy_release_request(),
    )
    .await?;
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = response_json(first).await?;
    assert_eq!(first_body.pointer("/noisedValue"), Some(&json!(37)));
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

    let duplicate = release_differential_privacy_aggregate(
        &state,
        "confidential-coordinator",
        differential_privacy_release_request(),
    )
    .await?;
    assert_eq!(duplicate.status(), StatusCode::OK);
    let duplicate_body = response_json(duplicate).await?;
    assert_eq!(duplicate_body.pointer("/noisedValue"), Some(&json!(37)));

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

    let release_audit = differential_privacy_release_metadata(&state, transcript(61)).await;
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
        release_body.pointer("/releases/0/noisedValue"),
        Some(&json!(37))
    );
    assert_eq!(
        release_body.pointer("/releases/0/adjacencyModel"),
        Some(&json!("addRemoveOneSubject"))
    );
    assert!(release_body.pointer("/releases/0/aggregate").is_none());
    assert!(release_body.pointer("/releases/0/noiseValue").is_none());
    Ok(())
}

#[tokio::test]
async fn confidential_differential_privacy_composition_summary_tracks_release_and_manual_spend()
-> Result<(), String> {
    let state = super::AppState::default();
    compute_confidential_aggregate_result(&state).await?;
    let budget = create_differential_privacy_budget(&state).await?;
    assert_eq!(budget.status(), StatusCode::NO_CONTENT);
    let manual_spend =
        spend_differential_privacy_budget(&state, "confidential-coordinator", 300_000).await?;
    assert_eq!(manual_spend.status(), StatusCode::OK);
    let release = release_differential_privacy_aggregate(
        &state,
        "confidential-coordinator",
        differential_privacy_release_request(),
    )
    .await?;
    assert_eq!(release.status(), StatusCode::OK);

    let summaries = differential_privacy_composition_summaries(&state, transcript(51)).await;
    assert_eq!(summaries.status(), StatusCode::OK);
    let body = response_json(summaries).await?;
    assert_eq!(
        body.pointer("/summaries")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        body.pointer("/summaries/0/compositionRule"),
        Some(&json!("basicSequential"))
    );
    assert_eq!(
        body.pointer("/summaries/0/advancedCompositionRule"),
        Some(&json!("drvPureDp"))
    );
    assert_eq!(
        body.pointer("/summaries/0/advancedCompositionDeltaMicrounits"),
        Some(&json!(1))
    );
    assert_eq!(
        body.pointer("/summaries/0/budgetSpentEpsilonMicrounits"),
        Some(&json!(550_000))
    );
    assert_eq!(
        body.pointer("/summaries/0/releaseSpentEpsilonMicrounits"),
        Some(&json!(250_000))
    );
    assert_eq!(
        body.pointer("/summaries/0/releaseEpsilonSquareSumMicrounits2"),
        Some(&json!("62500000000"))
    );
    let advanced_epsilon = body
        .pointer("/summaries/0/advancedCompositionEpsilonMicrounits")
        .and_then(Value::as_u64)
        .ok_or_else(|| "advanced composition epsilon should be a u64".to_owned())?;
    assert!(advanced_epsilon >= 250_000);
    assert_eq!(
        body.pointer("/summaries/0/unattributedSpentEpsilonMicrounits"),
        Some(&json!(300_000))
    );
    assert_eq!(
        body.pointer("/summaries/0/remainingEpsilonMicrounits"),
        Some(&json!(450_000))
    );
    assert_eq!(body.pointer("/summaries/0/releaseCount"), Some(&json!(1)));
    assert_eq!(
        body.pointer("/summaries/0/maxReleaseEpsilonMicrounits"),
        Some(&json!(250_000))
    );
    Ok(())
}

#[tokio::test]
async fn confidential_differential_privacy_release_rejects_conflicts_without_spending()
-> Result<(), String> {
    let state = super::AppState::default();
    compute_confidential_aggregate_result(&state).await?;
    let budget = create_differential_privacy_budget(&state).await?;
    assert_eq!(budget.status(), StatusCode::NO_CONTENT);

    let first = release_differential_privacy_aggregate(
        &state,
        "confidential-coordinator",
        differential_privacy_release_request(),
    )
    .await?;
    assert_eq!(first.status(), StatusCode::OK);

    let conflicting = release_differential_privacy_aggregate(
        &state,
        "confidential-coordinator",
        differential_privacy_release_request_with_noise(-4),
    )
    .await?;
    assert_eq!(conflicting.status(), StatusCode::CONFLICT);

    let too_small = release_differential_privacy_aggregate(
        &state,
        "confidential-coordinator",
        differential_privacy_release_request_with_minimum_population(4),
    )
    .await?;
    assert_eq!(too_small.status(), StatusCode::CONFLICT);

    let budget_audit = differential_privacy_budget_metadata(&state, transcript(51)).await;
    assert_eq!(budget_audit.status(), StatusCode::OK);
    let budget_body = response_json(budget_audit).await?;
    assert_eq!(
        budget_body.pointer("/budgets/0/spentEpsilonMicrounits"),
        Some(&json!(250_000))
    );
    Ok(())
}

#[tokio::test]
async fn confidential_differential_privacy_release_requires_computed_aggregate_and_owner()
-> Result<(), String> {
    let missing_state = super::AppState::default();
    let budget = create_differential_privacy_budget(&missing_state).await?;
    assert_eq!(budget.status(), StatusCode::NO_CONTENT);
    let missing = release_differential_privacy_aggregate(
        &missing_state,
        "confidential-coordinator",
        differential_privacy_release_request(),
    )
    .await?;
    assert_eq!(missing.status(), StatusCode::CONFLICT);

    let state = super::AppState::default();
    compute_confidential_aggregate_result(&state).await?;
    let budget = create_differential_privacy_budget(&state).await?;
    assert_eq!(budget.status(), StatusCode::NO_CONTENT);
    let foreign = release_differential_privacy_aggregate(
        &state,
        "policy-authority-left",
        differential_privacy_release_request(),
    )
    .await?;
    assert_eq!(foreign.status(), StatusCode::FORBIDDEN);
    Ok(())
}
