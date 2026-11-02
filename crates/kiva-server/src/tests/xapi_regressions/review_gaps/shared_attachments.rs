//! MIME type consistency across every metadata reference to shared raw bytes.
#![allow(clippy::wildcard_imports)]
use super::super::attachment_boundaries::decoded;
use super::*;

const RAW: &[u8] = b"hello";
const HASH: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

async fn single(
    http: &HttpProbe,
    value: &Value,
    owner: &str,
    has_raw: bool,
    expected_type: Option<&str>,
) -> Result<(), String> {
    let reply = HttpProbe::receive(http.client.get(format!(
        "{}/statements?statementId={}&attachments=true",
        http.base,
        value["id"].as_str().ok_or("id")?
    )))
    .await?;
    assert_eq!(reply.status(), StatusCode::OK);
    let parts = decoded(reply).await?;
    assert_eq!(parts.len(), if has_raw { 2 } else { 1 });
    let saved: Value = serde_json::from_slice(&parts[0].1).map_err(|e| e.to_string())?;
    assert_eq!(
        saved.pointer(&format!("{owner}/attachments")),
        value.pointer(&format!("{owner}/attachments"))
    );
    if has_raw {
        check_raw(&parts[1]);
        let content_type = parts[1]
            .0
            .lines()
            .find_map(|line| line.strip_prefix("Content-Type: "));
        assert_eq!(content_type, expected_type);
    }
    Ok(())
}

fn check_raw(part: &(String, Vec<u8>)) {
    assert_eq!(part.1, RAW);
    assert!(
        part.0
            .lines()
            .any(|line| line == format!("X-Experience-API-Hash: {HASH}"))
    );
    assert!(
        part.0
            .lines()
            .any(|line| line == "Content-Transfer-Encoding: binary")
    );
}

#[allow(clippy::too_many_lines)]
async fn shared_attachment_types(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    for (types, coherent) in [
        (["text/plain", "application/octet-stream"], false),
        (["text/plain", "TEXT/PLAIN; charset=US-ASCII"], true),
        (
            [
                "text/plain; charset=UTF-8",
                "text/plain; charset=ISO-8859-1",
            ],
            false,
        ),
        (
            ["application/example; x=A", "application/example; x=B"],
            false,
        ),
    ] {
        for file_url_only in [false, true] {
            for nested in [false, true] {
                let verb = format!("https://example.org/shared-raw/{}", Uuid::new_v4());
                let mut stored = Vec::new();
                // A/B/A ensures a later matching value cannot erase a mismatch.
                for (index, kind) in [types[0], types[1], types[0]].into_iter().enumerate() {
                    let has_raw = !(file_url_only && index == 1);
                    let (mut value, owner) = root_and_owner(nested && index == 1);
                    value["verb"]["id"] = json!(verb);
                    let digest = if index == 1 {
                        HASH.to_ascii_uppercase()
                    } else {
                        HASH.to_owned()
                    };
                    let mut meta = attachment(RAW, &digest);
                    meta["contentType"] = json!(kind);
                    if !has_raw {
                        meta["fileUrl"] = json!("https://unresolvable.invalid/hello");
                    }
                    value.pointer_mut(owner).ok_or("owner")?["attachments"] = json!([meta]);
                    let body = has_raw.then(|| multipart(&value, &[(HASH, Some(kind), RAW)]));
                    http.submit(&value, false, body, true).await?;
                    single(&http, &value, owner, has_raw, Some(kind)).await?;
                    stored.push((value, owner, has_raw, kind));
                }
                for ascending in [false, true] {
                    for format in ["exact", "canonical", "ids"] {
                        let mut url = reqwest::Url::parse(&format!("{}/statements", http.base))
                            .map_err(|e| e.to_string())?;
                        url.query_pairs_mut()
                            .append_pair("verb", &verb)
                            .append_pair("attachments", "true")
                            .append_pair("ascending", if ascending { "true" } else { "false" })
                            .append_pair("format", format);
                        let reply = HttpProbe::receive(http.client.get(url)).await?;
                        assert_eq!(reply.status(), StatusCode::OK);
                        let parts = decoded(reply).await?;
                        assert_eq!(parts.len(), 2, "one raw part for the shared digest");
                        check_raw(&parts[1]);
                        let content_type = parts[1]
                            .0
                            .lines()
                            .find_map(|line| line.strip_prefix("Content-Type: "));
                        if coherent {
                            assert!(content_type.is_some_and(|kind| types.contains(&kind)));
                        } else {
                            assert_eq!(
                                content_type, None,
                                "incompatible metadata types require the optional header to be omitted"
                            );
                        }
                        let body: Value =
                            serde_json::from_slice(&parts[0].1).map_err(|e| e.to_string())?;
                        let values = body["statements"].as_array().ok_or("StatementResult")?;
                        assert_eq!(values.len(), stored.len());
                        for (expected, owner, _, _) in &stored {
                            let returned = values
                                .iter()
                                .find(|v| v["id"] == expected["id"])
                                .ok_or("missing Statement")?;
                            let path = format!("{owner}/attachments");
                            assert_eq!(returned.pointer(&path), expected.pointer(&path));
                        }
                    }
                }
                // Projection must not mutate any stored metadata or part header.
                for (value, owner, has_raw, kind) in &stored {
                    single(&http, value, owner, *has_raw, Some(kind)).await?;
                }
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn memory_xapi_shared_attachment_types() -> Result<(), String> {
    shared_attachment_types(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_shared_attachment_types() -> Result<(), String> {
    shared_attachment_types(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}
