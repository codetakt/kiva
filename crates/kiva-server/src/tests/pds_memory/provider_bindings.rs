#![allow(clippy::wildcard_imports)]

use super::*;

fn provider_hex(seed: u8) -> String {
    format!("{seed:02x}").repeat(32)
}

fn reference_provider_binding_request(binding_id: Uuid) -> super::PdsProviderBindingRequest {
    super::PdsProviderBindingRequest {
        binding_id,
        mechanism: "additive-n-of-n".to_owned(),
        provider_kind: "reference-primitive".to_owned(),
        capability: "reference-fixtures".to_owned(),
        profile: "kiva-reference-primitives-v1".to_owned(),
        provider_id_digest: provider_hex(11),
        artifact_digest: provider_hex(12),
        parameter_set_digest: None,
        review_evidence_digest: None,
        production_claim_requested: false,
    }
}

#[tokio::test]
async fn pds_provider_binding_registry_admits_reference_fixture_metadata() -> Result<(), String> {
    let state = super::AppState::default();
    let binding_id = pds_uuid(70)?;
    let request =
        JsonHandlerRequest::new("learner-a", reference_provider_binding_request(binding_id));

    let created = super::post_pds_provider_binding(
        pds_state(&state),
        request.headers(),
        request.query(),
        request.body("PDS provider binding request")?,
    )
    .await;
    assert_eq!(created.status(), StatusCode::NO_CONTENT);

    let query =
        JsonHandlerRequest::new("learner-a", ()).with_query("bindingId", binding_id.to_string());
    let listed =
        super::get_pds_provider_bindings(pds_state(&state), query.headers(), query.query()).await;
    assert_eq!(listed.status(), StatusCode::OK);
    let body = response_json(listed).await?;
    assert_eq!(
        body.pointer("/providerBindings/0/runtimeClass"),
        Some(&json!("reference"))
    );
    assert_eq!(
        body.pointer("/providerBindings/0/evidenceGate"),
        Some(&json!("reference-fixture"))
    );
    assert_eq!(
        body.pointer("/providerBindings/0/productionReady"),
        Some(&json!(false))
    );
    assert_no_pds_raw_payload_fields(&body);
    Ok(())
}

#[tokio::test]
async fn pds_provider_binding_rejects_unreviewed_production_claim() -> Result<(), String> {
    let state = super::AppState::default();
    let mut request = reference_provider_binding_request(pds_uuid(71)?);
    request.mechanism = "fhe-sum-runtime".to_owned();
    request.provider_kind = "fully-homomorphic-encryption".to_owned();
    request.capability = "fhe-evaluation".to_owned();
    request.profile = "kiva-secret-compute-provider-contract-v1".to_owned();
    request.parameter_set_digest = Some(provider_hex(13));
    request.production_claim_requested = true;
    let request = JsonHandlerRequest::new("learner-a", request);

    let response = super::post_pds_provider_binding(
        pds_state(&state),
        request.headers(),
        request.query(),
        request.body("PDS provider binding request")?,
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}
