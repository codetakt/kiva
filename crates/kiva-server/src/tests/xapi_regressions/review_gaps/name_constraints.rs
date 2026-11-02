//! Product integration: provider results govern PUT/batch admission and storage.
#![allow(clippy::wildcard_imports)]
use super::*;
use crate::core::signature_policy::{SignaturePolicy, name_constraint_cases};
use jsonwebtoken::{Algorithm, EncodingKey};

async fn verify(mut state: AppState) -> Result<(), String> {
    let key = EncodingKey::from_rsa_pem(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-key.pem"
    )))
    .map_err(|e| e.to_string())?;
    for case in name_constraint_cases() {
        state.xapi.signature_policy =
            SignaturePolicy::from_json(case.policy.to_string().as_bytes())
                .map_err(|e| e.message)?;
        let http = HttpProbe::new(&state).await?;
        for post in [false, true] {
            let mut value = statement(Uuid::new_v4());
            let header = json!({"alg":"RS256", "x5c":case.chain});
            let message = format!(
                "{}.{}",
                URL_SAFE_NO_PAD.encode(header.to_string()),
                URL_SAFE_NO_PAD.encode(value.to_string())
            );
            let signature = jsonwebtoken::crypto::sign(message.as_bytes(), &key, Algorithm::RS256)
                .map_err(|e| e.to_string())?;
            let raw = format!("{message}.{signature}").into_bytes();
            let hash = format!("{:x}", Sha256::digest(&raw));
            let mut metadata = attachment(&raw, &hash);
            metadata["usageType"] = json!("http://adlnet.gov/expapi/attachments/signature");
            value["attachments"] = json!([metadata]);
            let id = value["id"].clone();
            let submitted = if post {
                // A separately valid sibling must also stay absent on rejection.
                json!([statement(Uuid::new_v4()), value])
            } else {
                value
            };
            let body = multipart(
                &submitted,
                &[(&hash, Some("application/octet-stream"), &raw)],
            );
            http.submit(&submitted, post, Some(body), case.accepted)
                .await
                .map_err(|e| format!("{}: {e}", case.label))?;
            if case.accepted {
                http.raw_attachment(&id, &raw, 1).await?;
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn memory_xapi_name_constraints_govern_admission_and_atomicity() -> Result<(), String> {
    verify(memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_name_constraints_govern_admission_and_atomicity() -> Result<(), String> {
    verify(
        postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}
