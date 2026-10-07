#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn confidential_participant_lifecycle_persists_metadata_and_audit_in_memory()
-> Result<(), String> {
    let state = super::AppState::default();
    let request = super::ConfidentialParticipantRequest {
        participant_subject: "agency-a".to_owned(),
        participant_index: nonzero(1),
        mechanism_roles: vec!["data-custodian".to_owned()],
        authorization_scope: "scope://kiva/confidential/coordinator/agency-a/1".to_owned(),
        key_ceremony_ref: "ceremony://kiva/confidential/agency-a/1".to_owned(),
        recovery_contact_ref: "recovery://kiva/confidential/agency-a/1".to_owned(),
        evidence_refs: vec!["evidence://cc10/agency-a/invited".to_owned()],
    };
    let created = super::post_confidential_participant(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_participant_body(&request)?,
    )
    .await;
    assert_eq!(created.status(), StatusCode::OK);
    let created_body = response_json(created).await?;
    assert_eq!(created_body.pointer("/status"), Some(&json!("invited")));
    assert_eq!(
        created_body.pointer("/participantSubject"),
        Some(&json!("agency-a"))
    );
    assert!(created_body.pointer("/share").is_none());
    assert!(created_body.pointer("/signature").is_none());
    assert!(created_body.pointer("/plaintext").is_none());

    let activated = super::post_confidential_participant_transition(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_participant_transition_body(
            &super::ConfidentialParticipantTransitionRequest {
                participant_subject: "agency-a".to_owned(),
                participant_index: nonzero(1),
                target_status: super::ConfidentialParticipantStatus::Active,
                reason: Some("activate participant".to_owned()),
                evidence_refs: vec!["evidence://cc10/agency-a/active".to_owned()],
            },
        )?,
    )
    .await;
    assert_eq!(activated.status(), StatusCode::OK);
    let activated_body = response_json(activated).await?;
    assert_eq!(activated_body.pointer("/status"), Some(&json!("active")));

    let invalid = super::post_confidential_participant_transition(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_participant_transition_body(
            &super::ConfidentialParticipantTransitionRequest {
                participant_subject: "agency-a".to_owned(),
                participant_index: nonzero(1),
                target_status: super::ConfidentialParticipantStatus::Invited,
                reason: Some("invalid rewind".to_owned()),
                evidence_refs: vec!["evidence://cc10/agency-a/rewind".to_owned()],
            },
        )?,
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::CONFLICT);

    let mut query = BTreeMap::new();
    query.insert("participantSubject".to_owned(), "agency-a".to_owned());
    let participants = super::get_confidential_participants(
        confidential_state(&state),
        confidential_headers(),
        Query(query),
    )
    .await;
    assert_eq!(participants.status(), StatusCode::OK);
    let participants_body = response_json(participants).await?;
    assert_eq!(
        participants_body.pointer("/participants/0/status"),
        Some(&json!("active"))
    );
    assert_eq!(
        participants_body
            .pointer("/participants/0/evidenceRefs")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(2)
    );

    let mut query = BTreeMap::new();
    query.insert("participantSubject".to_owned(), "agency-a".to_owned());
    let audit = super::get_confidential_participant_audit_events(
        confidential_state(&state),
        confidential_headers(),
        Query(query),
    )
    .await;
    assert_eq!(audit.status(), StatusCode::OK);
    let audit_body = response_json(audit).await?;
    assert_eq!(
        audit_body
            .pointer("/events")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(2)
    );
    assert!(audit_body.pointer("/events/0/share").is_none());
    assert!(audit_body.pointer("/events/0/signature").is_none());
    assert!(audit_body.pointer("/events/0/plaintext").is_none());
    Ok(())
}
