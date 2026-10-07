#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn learner_principal_token_exercises_subject_bound_pds_flows() -> Result<(), String> {
    let state = memory_test_state_with_auth(learner_principal_auth_config());
    assert_learner_principal_subject_bound_flows(&state).await
}
