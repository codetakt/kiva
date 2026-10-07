#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn confidential_shamir_threshold_reconstruction_persists_metadata() -> Result<(), String> {
    let state = super::AppState::default();

    let response = reconstruct_shamir_secret(&state, "confidential-coordinator").await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await?;
    assert_eq!(body.pointer("/reconstructedValue"), Some(&json!(1_234)));
    assert_eq!(
        body.pointer("/reconstruction/mechanism"),
        Some(&json!("shamirThreshold"))
    );
    assert_eq!(
        body.pointer("/reconstruction/verificationProfile"),
        Some(&json!(super::CONFIDENTIAL_SHAMIR_VERIFICATION_PROFILE))
    );
    assert_eq!(
        body.pointer("/reconstruction/verifiedShareCount"),
        Some(&json!(3))
    );
    assert_eq!(
        body.pointer("/reconstruction/complaintCount"),
        Some(&json!(0))
    );

    let metadata =
        shamir_reconstruction_metadata(&state, "confidential-coordinator", transcript(71)).await;
    assert_eq!(metadata.status(), StatusCode::OK);
    let metadata_body = response_json(metadata).await?;
    assert_eq!(
        metadata_body
            .pointer("/reconstructions")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        metadata_body.pointer("/reconstructions/0/threshold"),
        Some(&json!(3))
    );
    assert!(
        metadata_body
            .pointer("/reconstructions/0/reconstructedValue")
            .is_none()
    );
    assert!(metadata_body.pointer("/reconstructions/0/shares").is_none());
    assert!(
        metadata_body
            .pointer("/reconstructions/0/shareOpenings")
            .is_none()
    );
    assert!(
        metadata_body
            .pointer("/reconstructions/0/commitment")
            .is_none()
    );
    assert!(
        metadata_body
            .pointer("/reconstructions/0/openingNonce")
            .is_none()
    );

    let foreign_metadata =
        shamir_reconstruction_metadata(&state, "another-coordinator", transcript(71)).await;
    assert_eq!(foreign_metadata.status(), StatusCode::OK);
    let foreign_body = response_json(foreign_metadata).await?;
    assert_eq!(
        foreign_body
            .pointer("/reconstructions")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );
    Ok(())
}

#[tokio::test]
async fn confidential_shamir_threshold_reconstruction_rejects_conflicting_id() -> Result<(), String>
{
    let state = super::AppState::default();

    let first = reconstruct_shamir_secret(&state, "confidential-coordinator").await?;
    assert_eq!(first.status(), StatusCode::OK);

    let mut request = shamir_reconstruction_request();
    let transcript_id = request.transcript_id;
    request.share_openings = vec![
        verifiable_shamir_share(transcript_id, 1, 1_567, 91),
        verifiable_shamir_share(transcript_id, 2, 2_344, 92),
        verifiable_shamir_share(transcript_id, 4, 5_231, 94),
    ];
    let conflict =
        reconstruct_shamir_secret_with_request(&state, "confidential-coordinator", request).await?;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    Ok(())
}

#[tokio::test]
async fn confidential_shamir_threshold_reconstruction_rejects_tampered_opening()
-> Result<(), String> {
    let state = super::AppState::default();
    let mut request = shamir_reconstruction_request();
    let honest = request.share_openings[0];
    request.share_openings[0] = VerifiableShamirShare::new(
        shamir_share(1, 1_568),
        honest.commitment(),
        honest.opening_nonce(),
    );

    let response =
        reconstruct_shamir_secret_with_request(&state, "confidential-coordinator", request).await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}
