//! Accepted ASCII NUL in a MIME part header must survive durable storage.
#![allow(clippy::wildcard_imports)]
use super::super::attachment_boundaries::decoded;
use super::*;

async fn verify(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    let raw = b"abc\0\xff\r\n";
    let hash = format!("{:x}", Sha256::digest(raw));
    for mode in 0..3 {
        for nested in [false, true] {
            for kind in [
                Some("application/octet-stream (a\0b)"),
                Some("application/octet-stream; p=\"a\0b\""),
                Some("application/octet-stream; p=\"a\\\0b\""),
                Some("application/octet-stream (a\\\0b)"),
                Some("application/octet-stream; p=\"a\nb\""),
                Some("application/octet-stream; p=\"literal\\u0000\""),
                Some("application/octet-stream"),
                None,
            ] {
                let (mut value, owner) = root_and_owner(nested);
                let mut metadata = attachment(raw, &hash);
                metadata["contentType"] = json!(kind.unwrap_or("application/octet-stream"));
                value.pointer_mut(owner).ok_or("owner")?["attachments"] = json!([metadata]);
                let payload = if mode == 2 {
                    json!([value])
                } else {
                    value.clone()
                };
                let body = multipart(&payload, &[(&hash, kind, raw)]);
                http.submit(&payload, mode != 0, Some(body.clone()), true)
                    .await?;
                for replay in [false, true] {
                    if replay {
                        // Reload through the transaction path as well as GET.
                        http.submit(&payload, mode != 0, Some(body.clone()), true)
                            .await?;
                    }
                    for format in ["exact", "canonical", "ids"] {
                        let reply = HttpProbe::receive(http.client.get(format!(
                            "{}/statements?statementId={}&attachments=true&format={format}",
                            http.base,
                            value["id"].as_str().ok_or("id")?
                        )))
                        .await?;
                        assert_eq!(reply.status(), StatusCode::OK);
                        let parts = decoded(reply).await?;
                        assert_eq!(parts.len(), 2);
                        assert_eq!(parts[1].1, raw);
                        // Split only at the field delimiter: a permitted bare
                        // LF inside qtext is part of the preserved field value.
                        let actual = parts[1]
                            .0
                            .split("\r\n")
                            .find_map(|line| line.strip_prefix("Content-Type: "));
                        assert_eq!(
                            actual, kind,
                            "mode {mode}, nested {nested}, replay {replay}"
                        );
                        let saved: Value =
                            serde_json::from_slice(&parts[0].1).map_err(|e| e.to_string())?;
                        let pointer = format!("{owner}/attachments");
                        assert_eq!(saved.pointer(&pointer), value.pointer(&pointer));
                    }
                }
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn memory_xapi_mime_part_header_octet_persistence() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_mime_part_header_octet_persistence() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}
