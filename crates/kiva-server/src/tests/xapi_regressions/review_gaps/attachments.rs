#![allow(clippy::wildcard_imports)]
use super::*;

async fn integer_lengths(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    let data = b"123456789012345678901";
    let hash = format!("{:x}", Sha256::digest(data));
    for nested in [false, true] {
        for post in [false, true] {
            for file_url in [false, true] {
                for (token, integer, matches) in [
                    ("21", true, true),
                    ("21.0", true, true),
                    ("2.1e1", true, true),
                    ("210e-1", true, true),
                    ("21.0000000000000000000", true, true),
                    ("21.0000000000000000001", false, false),
                    ("20.9999999999999999999", false, false),
                    ("-21", false, false),
                    ("true", false, false),
                    ("\"21\"", false, false),
                    ("null", false, false),
                    ("20", true, false),
                    ("-0.0", true, false),
                    ("1e-99999999999999999", false, false),
                    ("1e99999999999999999", true, false),
                    ("18446744073709551616", true, false),
                ] {
                    let (mut value, path) = root_and_owner(nested);
                    let mut metadata = attachment(data, &hash);
                    metadata["length"] = serde_json::from_str(token).map_err(|e| e.to_string())?;
                    if file_url {
                        metadata["fileUrl"] = json!("https://example.invalid/file");
                    }
                    value.pointer_mut(path).ok_or("owner")?["attachments"] = json!([metadata]);
                    let value = if post { json!([value]) } else { value };
                    let body = (!file_url).then(|| {
                        multipart(&value, &[(&hash, Some("application/octet-stream"), data)])
                    });
                    http.submit(&value, post, body, integer && (file_url || matches))
                        .await?;
                }
            }
        }
    }
    Ok(())
}

async fn metadata_media_types(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    let data = b"media-type-body";
    let hash = format!("{:x}", Sha256::digest(data));
    for nested in [false, true] {
        for post in [false, true] {
            for route in ["fileUrl", "omitted-part-type", "matching-part-type"] {
                for (media, accepted) in [
                    ("text/plain", true),
                    ("text (a;b) / plain", true),
                    ("(a;b) text/plain", true),
                    ("Text/PLAIN; charset=\"UTF-8\"", true),
                    ("text (nested (comment)) / plain; note=\"a\\\"b\"", true),
                    ("text/plain; unknown=one", true),
                    ("text plain/plain", false),
                    ("text/plain/extra", false),
                    ("text/plain; charset=\"unfinished", false),
                    ("text/plain; =bad", false),
                    ("text/plain\r\nX-Bad: yes", false),
                ] {
                    let (mut value, path) = root_and_owner(nested);
                    let mut metadata = attachment(data, &hash);
                    metadata["contentType"] = json!(media);
                    if route == "fileUrl" {
                        metadata["fileUrl"] = json!("https://example.invalid/file");
                    }
                    value.pointer_mut(path).ok_or("owner")?["attachments"] = json!([metadata]);
                    // A valid sibling must not survive an invalid batch member.
                    let value = if post {
                        json!([statement(Uuid::new_v4()), value])
                    } else {
                        value
                    };
                    let body = (route != "fileUrl").then(|| {
                        multipart(
                            &value,
                            &[(
                                &hash,
                                (route == "matching-part-type").then_some(if accepted {
                                    media
                                } else {
                                    "text/plain"
                                }),
                                data,
                            )],
                        )
                    });
                    http.submit(&value, post, body, accepted).await?;
                }
            }
        }
    }
    Ok(())
}

fn mixed_case(hash: &str) -> String {
    hash.chars()
        .enumerate()
        .map(|(index, ch)| {
            if index % 2 == 0 {
                ch.to_ascii_uppercase()
            } else {
                ch
            }
        })
        .collect()
}

async fn digest_identity(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    let data = b"case-independent-binary-content";
    for hash in [
        format!("{:x}", Sha224::digest(data)),
        format!("{:x}", Sha256::digest(data)),
        format!("{:x}", Sha384::digest(data)),
        format!("{:x}", Sha512::digest(data)),
    ] {
        // Seed the old internal uppercase key representation, then retry over
        // HTTP. This also exercises persisted pre-fix PostgreSQL rows.
        let mut legacy = statement(Uuid::new_v4());
        legacy["attachments"] = json!([attachment(data, &hash.to_ascii_uppercase())]);
        legacy["timestamp"] = json!("2026-01-01T00:00:00Z");
        let id = Uuid::parse_str(legacy["id"].as_str().ok_or("id")?).map_err(|e| e.to_string())?;
        let now = OffsetDateTime::now_utc();
        let record = normalized_statement(legacy.clone(), id, now, &authority_agent("kiva-lrs"))
            .map_err(|error| error.message)?;
        state
            .xapi
            .statements
            .put(
                id,
                StoredStatement::new(
                    record,
                    now,
                    BTreeMap::from([(
                        hash.to_ascii_uppercase(),
                        crate::surfaces::xapi::model::AttachmentRecord {
                            content_type: "application/octet-stream".to_owned(),
                            body: data.to_vec(),
                        },
                    )]),
                ),
            )
            .await
            .map_err(|error| format!("{error:?}"))?;
        let retry = multipart(&legacy, &[(&hash, Some("application/octet-stream"), data)]);
        http.submit(&legacy, false, Some(retry), true).await?;
        http.raw_attachment(&legacy["id"], data, 1).await?;
        let spellings = [hash.clone(), hash.to_ascii_uppercase(), mixed_case(&hash)];
        for metadata_hash in &spellings {
            for part_hash in &spellings {
                for nested in [false, true] {
                    for post in [false, true] {
                        let (mut value, path) = root_and_owner(nested);
                        value.pointer_mut(path).ok_or("owner")?["attachments"] =
                            json!([attachment(data, metadata_hash), attachment(data, &hash)]);
                        let id = value["id"].clone();
                        let value = if post {
                            let mut sibling = statement(Uuid::new_v4());
                            sibling["attachments"] =
                                json!([attachment(data, &hash.to_ascii_uppercase())]);
                            json!([sibling, value])
                        } else {
                            value
                        };
                        // Equivalent duplicate parts and references store one raw attachment.
                        let body = multipart(
                            &value,
                            &[
                                (part_hash, Some("application/octet-stream"), data),
                                (&hash, None, data),
                            ],
                        );
                        http.submit(&value, post, Some(body), true).await?;
                        let retry =
                            multipart(&value, &[(&hash, Some("application/octet-stream"), data)]);
                        http.submit(&value, post, Some(retry), true).await?;
                        http.raw_attachment(&id, data, 1).await?;
                    }
                }
            }
        }
        for (metadata_wrong, raw_wrong) in [(true, false), (false, false), (false, true)] {
            let mut value = statement(Uuid::new_v4());
            let mut wrong = hash.clone();
            wrong.replace_range(..1, if wrong.starts_with('0') { "1" } else { "0" });
            value["attachments"] = json!([attachment(
                data,
                if metadata_wrong { &wrong } else { &hash }
            )]);
            let value = json!([statement(Uuid::new_v4()), value]);
            let parts = if raw_wrong {
                vec![(&*hash, None, &data[..]), (&*hash, None, &b"wrong"[..])]
            } else {
                vec![(
                    if metadata_wrong { &*hash } else { &*wrong },
                    None,
                    &data[..],
                )]
            };
            http.submit(&value, true, Some(multipart(&value, &parts)), false)
                .await?;
        }
    }
    Ok(())
}

#[tokio::test]
async fn memory_xapi_integer_lengths() -> Result<(), String> {
    integer_lengths(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_integer_lengths() -> Result<(), String> {
    integer_lengths(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

#[tokio::test]
async fn memory_xapi_metadata_media_types() -> Result<(), String> {
    metadata_media_types(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_metadata_media_types() -> Result<(), String> {
    metadata_media_types(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

#[tokio::test]
async fn memory_xapi_digest_identity() -> Result<(), String> {
    digest_identity(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_digest_identity() -> Result<(), String> {
    digest_identity(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}
