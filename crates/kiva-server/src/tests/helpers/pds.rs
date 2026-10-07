#![allow(clippy::wildcard_imports)]

use super::*;

pub(in crate::tests) struct PdsDelegatedAccessScenario {
    pub(in crate::tests) owner_token: &'static str,
    pub(in crate::tests) grantee_token: &'static str,
    pub(in crate::tests) grant_id: Uuid,
    pub(in crate::tests) scope: super::PdsConsentScope,
}

impl PdsDelegatedAccessScenario {
    pub(in crate::tests) const fn aggregate_insights(grant_id: Uuid) -> Self {
        Self {
            owner_token: "learner-a",
            grantee_token: "analyst-a",
            grant_id,
            scope: super::PdsConsentScope::AggregateInsights,
        }
    }

    pub(in crate::tests) const fn raw_records(grant_id: Uuid) -> Self {
        Self {
            owner_token: "learner-a",
            grantee_token: "analyst-a",
            grant_id,
            scope: super::PdsConsentScope::RawRecords,
        }
    }

    pub(in crate::tests) async fn create_grant(
        &self,
        state: &super::AppState,
    ) -> Result<axum::response::Response, String> {
        create_pds_consent_grant(
            state,
            self.owner_token,
            self.grant_id,
            self.grantee_token,
            self.scope,
        )
        .await
    }

    pub(in crate::tests) async fn access_check(
        &self,
        state: &super::AppState,
        resource: &str,
        action: &str,
        data_product_id: Option<Uuid>,
    ) -> Result<axum::response::Response, String> {
        pds_access_check(
            state,
            self.grantee_token,
            super::PdsAccessCheckRequest {
                role: "analyst".to_owned(),
                resource: resource.to_owned(),
                action: action.to_owned(),
                grant_id: Some(self.grant_id),
                data_product_id,
            },
        )
        .await
    }
}

pub(in crate::tests) async fn create_pds_consent_grant(
    state: &super::AppState,
    owner_token: &'static str,
    grant_id: Uuid,
    grantee_subject: &str,
    scope: super::PdsConsentScope,
) -> Result<axum::response::Response, String> {
    Ok(super::post_pds_consent_grant(
        pds_application_state(state),
        confidential_headers_for(owner_token),
        Query(BTreeMap::new()),
        json_body(
            &super::PdsConsentGrantRequest {
                grant_id,
                grantee_subject: grantee_subject.to_owned(),
                scope,
                purpose: "subject delegated access".to_owned(),
                legal_basis: super::PdsLegalBasis::Consent,
                expires_at: None,
                renewed_from_grant_id: None,
            },
            "PDS consent grant",
        )?,
    )
    .await)
}

pub(in crate::tests) async fn revoke_pds_consent_grant(
    state: &super::AppState,
    owner_token: &'static str,
    grant_id: Uuid,
) -> Result<axum::response::Response, String> {
    Ok(super::post_pds_consent_revocation(
        pds_application_state(state),
        confidential_headers_for(owner_token),
        Query(BTreeMap::new()),
        json_body(
            &super::PdsConsentRevocationRequest { grant_id },
            "PDS consent revocation",
        )?,
    )
    .await)
}

pub(in crate::tests) async fn pds_consent_grants(
    state: &super::AppState,
    token: &'static str,
    grant_id: Option<Uuid>,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    if let Some(grant_id) = grant_id {
        query.insert("grantId".to_owned(), grant_id.to_string());
    }
    super::get_pds_consent_grants(
        pds_state(state),
        confidential_headers_for(token),
        Query(query),
    )
    .await
}

pub(in crate::tests) async fn create_pds_data_product(
    state: &super::AppState,
    owner_token: &'static str,
    data_product_id: Uuid,
    source_kind: super::PdsDataProductSourceKind,
    release_id: SecureAggregateTranscriptId,
    title: &str,
    purpose: &str,
) -> Result<axum::response::Response, String> {
    Ok(super::post_pds_data_product(
        pds_application_state(state),
        confidential_headers_for(owner_token),
        Query(BTreeMap::new()),
        json_body(
            &super::PdsDataProductRequest {
                data_product_id,
                source_kind,
                release_id: super::hex_lower(&release_id.bytes()),
                title: title.to_owned(),
                purpose: purpose.to_owned(),
                legal_basis: super::PdsLegalBasis::Consent,
                data_classes: vec![
                    super::PdsDataClass::DataProduct,
                    super::PdsDataClass::Lifecycle,
                ],
            },
            "PDS data product",
        )?,
    )
    .await)
}

pub(in crate::tests) async fn pds_data_products(
    state: &super::AppState,
    token: &'static str,
    data_product_id: Option<Uuid>,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    if let Some(data_product_id) = data_product_id {
        query.insert("dataProductId".to_owned(), data_product_id.to_string());
    }
    super::get_pds_data_products(
        pds_state(state),
        confidential_headers_for(token),
        Query(query),
    )
    .await
}

pub(in crate::tests) async fn assert_pds_data_products(
    state: &super::AppState,
    token: &'static str,
    data_product_id: Uuid,
    expected_len: usize,
    expected_source_kind: Option<&str>,
) -> Result<(), String> {
    let response = pds_data_products(state, token, Some(data_product_id)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await?;
    assert_eq!(
        body.pointer("/dataProducts")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(expected_len)
    );
    if let Some(expected_source_kind) = expected_source_kind {
        assert_eq!(
            body.pointer("/dataProducts/0/sourceKind"),
            Some(&json!(expected_source_kind))
        );
    }
    Ok(())
}

pub(in crate::tests) async fn assert_pds_shamir_reconstruction_data_product_admission(
    state: &super::AppState,
    missing_product_id: Uuid,
    shamir_product_id: Uuid,
    foreign_product_id: Uuid,
    read_grant_id: Uuid,
) -> Result<(), String> {
    let missing = create_pds_data_product(
        state,
        "learner-a",
        missing_product_id,
        super::PdsDataProductSourceKind::ShamirThresholdReconstruction,
        transcript(71),
        "Missing Shamir reconstruction",
        "Should be rejected until reconstruction metadata exists",
    )
    .await?;
    assert_eq!(missing.status(), StatusCode::CONFLICT);

    let reconstruction = reconstruct_shamir_secret(state, "learner-a").await?;
    assert_eq!(reconstruction.status(), StatusCode::OK);
    let available = create_pds_data_product(
        state,
        "learner-a",
        shamir_product_id,
        super::PdsDataProductSourceKind::ShamirThresholdReconstruction,
        transcript(71),
        "Released Shamir reconstruction metadata",
        "School-level threshold reconstruction report",
    )
    .await?;
    assert_eq!(available.status(), StatusCode::NO_CONTENT);

    let foreign = create_pds_data_product(
        state,
        "analyst-a",
        foreign_product_id,
        super::PdsDataProductSourceKind::ShamirThresholdReconstruction,
        transcript(71),
        "Foreign Shamir reconstruction metadata",
        "Should be rejected for another subject",
    )
    .await?;
    assert_eq!(foreign.status(), StatusCode::CONFLICT);

    let products = pds_data_products(state, "learner-a", Some(shamir_product_id)).await;
    assert_eq!(products.status(), StatusCode::OK);
    let body = response_json(products).await?;
    assert_eq!(
        body.pointer("/dataProducts/0/sourceKind"),
        Some(&json!("shamir-threshold-reconstruction"))
    );
    assert!(body.pointer("/dataProducts/0/reconstructedValue").is_none());
    assert!(body.pointer("/dataProducts/0/share").is_none());
    assert!(body.pointer("/dataProducts/0/signature").is_none());

    let grant = create_pds_consent_grant(
        state,
        "learner-a",
        read_grant_id,
        "analyst-a",
        super::PdsConsentScope::AggregateInsights,
    )
    .await?;
    assert_eq!(grant.status(), StatusCode::NO_CONTENT);
    let read = pds_data_product_read(state, "analyst-a", read_grant_id, shamir_product_id).await?;
    assert_eq!(read.status(), StatusCode::OK);
    let read_body = response_json(read).await?;
    assert_eq!(read_body.pointer("/decision"), Some(&json!("allow")));
    assert_eq!(
        read_body.pointer("/dataProduct/sourceKind"),
        Some(&json!("shamir-threshold-reconstruction"))
    );
    assert_no_pds_raw_payload_fields(&read_body);
    Ok(())
}

pub(in crate::tests) async fn assert_pds_data_product_access_decision(
    state: &super::AppState,
    token: &'static str,
    grant_id: Uuid,
    data_product_id: Uuid,
    expected_decision: &str,
    expected_deny_reason: Option<&str>,
) -> Result<Value, String> {
    let response = pds_access_check(
        state,
        token,
        super::PdsAccessCheckRequest {
            role: "analyst".to_owned(),
            resource: "aggregate-insight".to_owned(),
            action: "read".to_owned(),
            grant_id: Some(grant_id),
            data_product_id: Some(data_product_id),
        },
    )
    .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await?;
    assert_eq!(body.pointer("/decision"), Some(&json!(expected_decision)));
    if let Some(expected_deny_reason) = expected_deny_reason {
        assert_eq!(
            body.pointer("/denyReason"),
            Some(&json!(expected_deny_reason))
        );
    } else {
        assert!(body.pointer("/denyReason").is_none());
    }
    Ok(body)
}

pub(in crate::tests) async fn assert_pds_differential_privacy_data_product_access_owner_scope(
    state: &super::AppState,
) -> Result<(), String> {
    let budget = create_differential_privacy_budget_for(state, "learner-a").await?;
    assert_eq!(budget.status(), StatusCode::NO_CONTENT);
    let dp_release = release_differential_privacy_aggregate(
        state,
        "learner-a",
        differential_privacy_release_request(),
    )
    .await?;
    assert_eq!(dp_release.status(), StatusCode::OK);

    let dp_product_id = pds_uuid(40)?;
    let dp_product = create_pds_data_product(
        state,
        "learner-a",
        dp_product_id,
        super::PdsDataProductSourceKind::DifferentialPrivacyRelease,
        transcript(61),
        "Attendance trend",
        "School-level BI dashboard",
    )
    .await?;
    assert_eq!(dp_product.status(), StatusCode::NO_CONTENT);

    let grant_id = pds_uuid(41)?;
    let grant = create_pds_consent_grant(
        state,
        "learner-a",
        grant_id,
        "analyst-a",
        super::PdsConsentScope::AggregateInsights,
    )
    .await?;
    assert_eq!(grant.status(), StatusCode::NO_CONTENT);
    assert_pds_data_product_access_decision(
        state,
        "analyst-a",
        grant_id,
        dp_product_id,
        "allow",
        None,
    )
    .await?;
    let read = pds_data_product_read(state, "analyst-a", grant_id, dp_product_id).await?;
    assert_eq!(read.status(), StatusCode::OK);
    let read_body = response_json(read).await?;
    assert_eq!(read_body.pointer("/decision"), Some(&json!("allow")));
    assert_eq!(
        read_body.pointer("/dataProduct/dataProductId"),
        Some(&json!(dp_product_id.to_string()))
    );
    assert_eq!(
        read_body.pointer("/dataProduct/sourceKind"),
        Some(&json!("differential-privacy-release"))
    );
    assert_no_pds_raw_payload_fields(&read_body);

    let foreign_grant_id = pds_uuid(42)?;
    let foreign_grant = create_pds_consent_grant(
        state,
        "learner-b",
        foreign_grant_id,
        "analyst-a",
        super::PdsConsentScope::AggregateInsights,
    )
    .await?;
    assert_eq!(foreign_grant.status(), StatusCode::NO_CONTENT);
    assert_pds_data_product_access_decision(
        state,
        "analyst-a",
        foreign_grant_id,
        dp_product_id,
        "deny",
        Some("dataProductUnavailable"),
    )
    .await?;
    let denied_read =
        pds_data_product_read(state, "analyst-a", foreign_grant_id, dp_product_id).await?;
    assert_eq!(denied_read.status(), StatusCode::FORBIDDEN);
    let denied_body = response_json(denied_read).await?;
    assert_eq!(denied_body.pointer("/decision"), Some(&json!("deny")));
    assert_eq!(
        denied_body.pointer("/denyReason"),
        Some(&json!("dataProductUnavailable"))
    );
    assert!(denied_body.pointer("/dataProduct").is_none());
    Ok(())
}

pub(in crate::tests) async fn pds_data_product_read(
    state: &super::AppState,
    token: &'static str,
    grant_id: Uuid,
    data_product_id: Uuid,
) -> Result<axum::response::Response, String> {
    Ok(super::post_pds_data_product_read(
        pds_state(state),
        confidential_headers_for(token),
        Query(BTreeMap::new()),
        json_body(
            &super::PdsDataProductReadRequest {
                grant_id,
                data_product_id,
            },
            "PDS data product read",
        )?,
    )
    .await)
}

pub(in crate::tests) async fn pds_access_check(
    state: &super::AppState,
    token: &'static str,
    request: super::PdsAccessCheckRequest,
) -> Result<axum::response::Response, String> {
    Ok(super::post_pds_access_check(
        pds_state(state),
        confidential_headers_for(token),
        Query(BTreeMap::new()),
        json_body(&request, "PDS access check")?,
    )
    .await)
}

pub(in crate::tests) async fn pds_access_audit_events(
    state: &super::AppState,
    token: &'static str,
    event_id: Option<Uuid>,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    if let Some(event_id) = event_id {
        query.insert("eventId".to_owned(), event_id.to_string());
    }
    super::get_pds_access_audit_events(
        pds_state(state),
        confidential_headers_for(token),
        Query(query),
    )
    .await
}

pub(in crate::tests) async fn store_pds_subject_statement(
    state: &super::AppState,
    subject: &str,
    statement_id: Uuid,
) -> Result<(), String> {
    let stored_at = fixed_time();
    let statement = super::normalized_statement(
        json!({
            "actor": super::wire::pds::subject_agent(subject),
            "verb": {
                "id": "https://example.org/xapi/verbs/completed",
                "display": {
                    "en-US": "completed",
                },
            },
            "object": {
                "id": "https://example.org/xapi/activities/course-1",
                "definition": {
                    "name": {
                        "en-US": "Course 1",
                    },
                },
            },
            "result": {
                "score": {
                    "raw": 99,
                },
                "success": true,
            },
        }),
        statement_id,
        stored_at,
        &super::authority_agent("pds-test"),
    )
    .map_err(|error| error.message)?;
    state
        .xapi
        .statements
        .put(
            statement_id,
            super::StoredStatement::new(statement, stored_at, BTreeMap::new()),
        )
        .await
        .map_err(|error| format!("statement should store: {error:?}"))?;
    Ok(())
}

pub(in crate::tests) async fn pds_record_inventory(
    state: &super::AppState,
    token: &'static str,
) -> axum::response::Response {
    super::get_pds_record_inventory(
        pds_application_state(state),
        confidential_headers_for(token),
        Query(BTreeMap::new()),
    )
    .await
}

pub(in crate::tests) async fn pds_metadata_exports(
    state: &super::AppState,
    token: &'static str,
) -> axum::response::Response {
    super::get_pds_metadata_exports(
        pds_application_state(state),
        confidential_headers_for(token),
        Query(BTreeMap::new()),
    )
    .await
}

pub(in crate::tests) async fn create_pds_lifecycle_event(
    state: &super::AppState,
    token: &'static str,
    request: super::PdsLifecycleEventRequest,
) -> Result<axum::response::Response, String> {
    Ok(super::post_pds_lifecycle_event(
        pds_application_state(state),
        confidential_headers_for(token),
        Query(BTreeMap::new()),
        json_body(&request, "PDS lifecycle event")?,
    )
    .await)
}

pub(in crate::tests) async fn pds_lifecycle_events(
    state: &super::AppState,
    token: &'static str,
    event_id: Option<Uuid>,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    if let Some(event_id) = event_id {
        query.insert("eventId".to_owned(), event_id.to_string());
    }
    super::get_pds_lifecycle_events(
        pds_state(state),
        confidential_headers_for(token),
        Query(query),
    )
    .await
}

pub(in crate::tests) async fn create_pds_retention_policy(
    state: &super::AppState,
    token: &'static str,
    request: super::PdsRetentionPolicyRequest,
) -> Result<axum::response::Response, String> {
    Ok(super::post_pds_retention_policy(
        pds_application_state(state),
        confidential_headers_for(token),
        Query(BTreeMap::new()),
        json_body(&request, "PDS retention policy")?,
    )
    .await)
}

pub(in crate::tests) async fn pds_retention_policies(
    state: &super::AppState,
    token: &'static str,
    policy_id: Option<Uuid>,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    if let Some(policy_id) = policy_id {
        query.insert("policyId".to_owned(), policy_id.to_string());
    }
    super::get_pds_retention_policies(
        pds_state(state),
        confidential_headers_for(token),
        Query(query),
    )
    .await
}

pub(in crate::tests) async fn create_pds_lifecycle_case(
    state: &super::AppState,
    token: &'static str,
    request: super::PdsLifecycleCaseRequest,
) -> Result<axum::response::Response, String> {
    Ok(super::post_pds_lifecycle_case(
        pds_application_state(state),
        confidential_headers_for(token),
        Query(BTreeMap::new()),
        json_body(&request, "PDS lifecycle case")?,
    )
    .await)
}

pub(in crate::tests) async fn pds_lifecycle_cases(
    state: &super::AppState,
    token: &'static str,
    case_id: Option<Uuid>,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    if let Some(case_id) = case_id {
        query.insert("caseId".to_owned(), case_id.to_string());
    }
    super::get_pds_lifecycle_cases(
        pds_state(state),
        confidential_headers_for(token),
        Query(query),
    )
    .await
}

pub(in crate::tests) fn pds_retention_policy_request(
    policy_id: Uuid,
) -> super::PdsRetentionPolicyRequest {
    super::PdsRetentionPolicyRequest {
        policy_id,
        resource_kind: super::PdsLifecycleResourceKind::XapiStatement,
        data_class: super::PdsDataClass::Statement,
        retention_period_days: 365,
        review_interval_days: 90,
        tombstone_after_days: Some(730),
        tombstone_publication: super::PdsTombstonePublicationRule::MetadataOnly,
        legal_basis: super::PdsLegalBasis::PublicTask,
        description: "annual public-sector retention policy for learner statements".to_owned(),
    }
}

pub(in crate::tests) fn pds_lifecycle_case_request(
    case_id: Uuid,
    case_type: super::PdsLifecycleCaseType,
    status: super::PdsLifecycleCaseStatus,
    resource_id: &str,
    retention_policy_id: Option<Uuid>,
    resolution: Option<&str>,
    tombstone_publication: Option<super::PdsTombstonePublicationRule>,
) -> super::PdsLifecycleCaseRequest {
    super::PdsLifecycleCaseRequest {
        case_id,
        case_type,
        resource_kind: super::PdsLifecycleResourceKind::XapiStatement,
        resource_id: resource_id.to_owned(),
        status,
        retention_policy_id,
        summary: "learner requested operational lifecycle review".to_owned(),
        resolution: resolution.map(str::to_owned),
        tombstone_publication,
    }
}

pub(in crate::tests) async fn create_pds_governance_profile(
    state: &super::AppState,
    token: &'static str,
    request: super::PdsGovernanceProfileRequest,
) -> Result<axum::response::Response, String> {
    Ok(super::post_pds_governance_profile(
        pds_application_state(state),
        confidential_headers_for(token),
        Query(BTreeMap::new()),
        json_body(&request, "PDS governance profile")?,
    )
    .await)
}

pub(in crate::tests) async fn pds_governance_profiles(
    state: &super::AppState,
    token: &'static str,
    profile_id: Option<Uuid>,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    if let Some(profile_id) = profile_id {
        query.insert("profileId".to_owned(), profile_id.to_string());
    }
    super::get_pds_governance_profiles(
        pds_state(state),
        confidential_headers_for(token),
        Query(query),
    )
    .await
}

pub(in crate::tests) async fn insert_expired_consent_grant(
    state: &super::AppState,
    grant_id: Uuid,
    data_subject: &str,
    grantee_subject: &str,
    scope: super::PdsConsentScope,
) -> Result<(), String> {
    let super::PdsRepository::Memory(repository) = state.pds_repository() else {
        return Err("expired grant helper requires the memory PDS repository".to_owned());
    };
    let created_at = fixed_time();
    let grant = super::PdsConsentGrantState {
        grant_id,
        data_subject: data_subject.to_owned(),
        grantee_subject: grantee_subject.to_owned(),
        scope,
        state: super::PdsConsentState::Active,
        purpose: "time-limited delegated access".to_owned(),
        legal_basis: super::PdsLegalBasis::Consent,
        created_at,
        expires_at: Some(created_at + Duration::seconds(1)),
        revoked_at: None,
        renewed_from_grant_id: None,
    };
    repository.insert_consent_grant_for_test(grant).await;
    Ok(())
}

pub(in crate::tests) async fn assert_pds_ledger_event(
    state: &super::AppState,
    event_kind: &str,
    owner_subject: &str,
    resource_kind: &str,
    resource_id: &str,
) -> Result<(), String> {
    let matches = state
        .pds_repository()
        .ledger_event_matches_for_test(event_kind, owner_subject, resource_kind, resource_id)
        .await?;
    if !matches {
        return Err(format!(
            "expected PDS ledger event {event_kind} for resource {resource_id}"
        ));
    }
    Ok(())
}

pub(in crate::tests) async fn assert_pds_ledger_event_count(
    state: &super::AppState,
    expected_count: usize,
) -> Result<(), String> {
    let super::PdsRepository::Memory(repository) = state.pds_repository() else {
        return Err("PDS ledger assertion requires the memory PDS repository".to_owned());
    };
    let event_count = repository.ledger_event_count_for_test().await;
    assert_eq!(event_count, expected_count);
    Ok(())
}

pub(in crate::tests) async fn assert_pds_ledger_event_count_at_least(
    state: &super::AppState,
    minimum: usize,
) -> Result<(), String> {
    let super::PdsRepository::Memory(repository) = state.pds_repository() else {
        return Ok(());
    };
    let event_count = repository.ledger_event_count_for_test().await;
    assert!(event_count >= minimum);
    Ok(())
}
