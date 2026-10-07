#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn pds_governance_profiles_are_owner_scoped_idempotent_and_strict() -> Result<(), String> {
    let state = super::AppState::default();
    let profile_id = pds_uuid(28)?;
    let request = pds_governance_profile_request(profile_id);

    let first = create_pds_governance_profile(&state, "public-admin", request.clone()).await?;
    assert_eq!(first.status(), StatusCode::NO_CONTENT);
    let duplicate = create_pds_governance_profile(&state, "public-admin", request).await?;
    assert_eq!(duplicate.status(), StatusCode::NO_CONTENT);

    let owner_profiles = pds_governance_profiles(&state, "public-admin", Some(profile_id)).await;
    assert_eq!(owner_profiles.status(), StatusCode::OK);
    let owner_body = response_json(owner_profiles).await?;
    assert_eq!(
        owner_body
            .pointer("/profiles")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        owner_body.pointer("/profiles/0/profileKind"),
        Some(&json!("school-district"))
    );
    assert_eq!(
        owner_body.pointer("/profiles/0/platformOperatorSubject"),
        Some(&json!("regional-platform-operator"))
    );
    assert_no_pds_raw_payload_fields(&owner_body);

    let foreign_profiles =
        pds_governance_profiles(&state, "another-public-admin", Some(profile_id)).await;
    assert_eq!(foreign_profiles.status(), StatusCode::OK);
    let foreign_body = response_json(foreign_profiles).await?;
    assert_eq!(
        foreign_body
            .pointer("/profiles")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );

    let mut overlapping_subject = pds_governance_profile_request(pds_uuid(29)?);
    overlapping_subject.platform_operator_subject =
        overlapping_subject.policy_authority_subject.clone();
    let overlap =
        create_pds_governance_profile(&state, "public-admin", overlapping_subject).await?;
    assert_eq!(overlap.status(), StatusCode::BAD_REQUEST);

    let mut overlapping_identity = pds_governance_profile_request(pds_uuid(31)?);
    overlapping_identity.identity_authority_subject =
        Some(overlapping_identity.data_custodian_subject.clone());
    let identity_overlap =
        create_pds_governance_profile(&state, "public-admin", overlapping_identity).await?;
    assert_eq!(identity_overlap.status(), StatusCode::BAD_REQUEST);

    let mut duplicate_runbook = pds_governance_profile_request(pds_uuid(30)?);
    duplicate_runbook.runbook_refs = vec![
        "runbook:key-rotation".to_owned(),
        "runbook:key-rotation".to_owned(),
    ];
    let duplicate_ref =
        create_pds_governance_profile(&state, "public-admin", duplicate_runbook).await?;
    assert_eq!(duplicate_ref.status(), StatusCode::BAD_REQUEST);

    let export = pds_metadata_exports(&state, "public-admin").await;
    assert_eq!(export.status(), StatusCode::OK);
    let export_body = response_json(export).await?;
    assert_eq!(
        export_body.pointer("/governanceProfiles/0/profileId"),
        Some(&json!(profile_id.to_string()))
    );
    assert_no_pds_raw_payload_fields(&export_body);
    Ok(())
}
