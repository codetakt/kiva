#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn confidential_participant_gates_keys_and_jobs_fail_closed_in_memory() -> Result<(), String>
{
    let state = super::AppState::default();
    let missing_key = super::post_confidential_share_signing_key(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_signing_key_body(&confidential_signing_key_request(21))?,
    )
    .await;
    assert_eq!(missing_key.status(), StatusCode::CONFLICT);

    let participant = super::ConfidentialParticipantRequest {
        participant_subject: "confidential-coordinator".to_owned(),
        participant_index: nonzero(1),
        mechanism_roles: vec!["additive-share-contributor".to_owned()],
        authorization_scope: "scope://kiva/confidential/coordinator/self/1".to_owned(),
        key_ceremony_ref: "ceremony://kiva/confidential/coordinator/1".to_owned(),
        recovery_contact_ref: "recovery://kiva/confidential/coordinator/1".to_owned(),
        evidence_refs: vec!["evidence://cc10/coordinator/invited".to_owned()],
    };
    let invited = super::post_confidential_participant(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_participant_body(&participant)?,
    )
    .await;
    assert_eq!(invited.status(), StatusCode::OK);

    let invited_key = super::post_confidential_share_signing_key(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_signing_key_body(&confidential_signing_key_request(21))?,
    )
    .await;
    assert_eq!(invited_key.status(), StatusCode::CONFLICT);

    let duplicate = super::post_confidential_participant(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_participant_body(&super::ConfidentialParticipantRequest {
            mechanism_roles: vec!["auditor".to_owned()],
            ..participant
        })?,
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);

    let activated = super::post_confidential_participant_transition(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_participant_transition_body(
            &super::ConfidentialParticipantTransitionRequest {
                participant_subject: "confidential-coordinator".to_owned(),
                participant_index: nonzero(1),
                target_status: super::ConfidentialParticipantStatus::Active,
                reason: Some("activate participant".to_owned()),
                evidence_refs: vec!["evidence://cc10/coordinator/active".to_owned()],
            },
        )?,
    )
    .await;
    assert_eq!(activated.status(), StatusCode::OK);
    let active_key = super::post_confidential_share_signing_key(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_signing_key_body(&confidential_signing_key_request(21))?,
    )
    .await;
    assert_eq!(active_key.status(), StatusCode::NO_CONTENT);

    register_active_confidential_participant(
        &state,
        "another-coordinator",
        "agency-z",
        1,
        vec!["data-custodian".to_owned()],
    )
    .await?;
    let mut wrong_owner_job = confidential_job_request(transcript(95));
    wrong_owner_job.participant_subjects = vec!["agency-z".to_owned(), "agency-b".to_owned()];
    let wrong_owner = super::post_confidential_job(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_job_body(&wrong_owner_job)?,
    )
    .await;
    assert_eq!(wrong_owner.status(), StatusCode::BAD_REQUEST);

    register_confidential_job_participants(
        &state,
        &confidential_job_request(transcript(96)).participant_subjects,
    )
    .await?;
    let suspended = super::post_confidential_participant_transition(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_participant_transition_body(
            &super::ConfidentialParticipantTransitionRequest {
                participant_subject: "agency-b".to_owned(),
                participant_index: nonzero(2),
                target_status: super::ConfidentialParticipantStatus::Suspended,
                reason: Some("stale key ceremony".to_owned()),
                evidence_refs: vec!["evidence://cc10/agency-b/suspended".to_owned()],
            },
        )?,
    )
    .await;
    assert_eq!(suspended.status(), StatusCode::OK);
    let stale_participant_job = super::post_confidential_job(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_job_body(&confidential_job_request(transcript(96)))?,
    )
    .await;
    assert_eq!(stale_participant_job.status(), StatusCode::BAD_REQUEST);
    Ok(())
}
