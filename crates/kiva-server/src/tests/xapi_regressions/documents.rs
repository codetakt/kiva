#![allow(clippy::wildcard_imports)]

use super::*;
use crate::surfaces::xapi::model::{
    DocumentAddress, DocumentContent, DocumentIdRequirement, DocumentKind, DocumentScope,
    DocumentSelection, Preconditions,
};
use crate::surfaces::xapi::outcome::{DocumentDeleteOutcome, DocumentWriteOutcome};

#[tokio::test]
async fn memory_xapi_document_raw_representation_and_merge() -> Result<(), String> {
    verify_raw(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_document_raw_representation_and_merge() -> Result<(), String> {
    verify_raw(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn raw_reply(
    request: reqwest::RequestBuilder,
    status: StatusCode,
) -> Result<reqwest::Response, String> {
    let reply = evidence::received(
        request
            .header("X-Experience-API-Version", "2.0.0")
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await?;
    assert_eq!(reply.status(), status);
    assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
    Ok(reply)
}

#[allow(clippy::too_many_lines)]
async fn verify_raw(state: &AppState) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let base = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let router = evidence::router(state);
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let object=br#" { "n":1, "n":2, "nested":{ "x":1,"x":2 }, "replace": {"old":true}, "$serde_json::private::Number":"literal", "fine":1.0000000000000001 } "#;
    let json_media = b"Application/JSON; charset=UTF-8".as_slice();
    let octet_media = [
        b"text/plain; title=\"".to_vec(),
        (0x80..=0xff).collect(),
        vec![b'"'],
    ]
    .concat();
    for family in ["activities/state", "agents/profile", "activities/profile"] {
        for (media, bytes) in [
            (json_media, object.as_slice()),
            (json_media, br#"{"a":1,"\u0061":2}"#),
            (json_media, b" [ 1.0000000000000001 ] \n"),
            (json_media, b"null"),
            (json_media, b"{"),
            (json_media, b"\0\xff{}"),
            (json_media, b""),
            (b"application/octet-stream", b"\0\xff{}"),
            (b"text/plain; title=\"\xe9\"", b"\0\xff{}"),
            (b"text/plain; title=\"\x80\x9f\xa0\xff\"", b"\0\xff{}"),
            (b"text/plain; title=\"\xc3\xa9\"", b"\0\xff{}"),
            (octet_media.as_slice(), b"\0\xff{}"),
            (b"application/json; title=\"\xe9\"", object.as_slice()),
        ] {
            let mut url =
                reqwest::Url::parse(&format!("{base}/{family}")).map_err(|e| e.to_string())?;
            {
                let mut pairs = url.query_pairs_mut();
                if family != "agents/profile" {
                    pairs.append_pair("activityId", "https://example.org/raw-document");
                }
                if family != "activities/profile" {
                    pairs.append_pair("agent", r#"{"mbox":"mailto:raw@example.org"}"#);
                }
                pairs.append_pair(
                    if family == "activities/state" {
                        "stateId"
                    } else {
                        "profileId"
                    },
                    &Uuid::new_v4().to_string(),
                );
            }
            raw_reply(
                client
                    .put(url.clone())
                    .header("Content-Type", media)
                    .body(bytes.to_vec()),
                StatusCode::NO_CONTENT,
            )
            .await?;
            let saved = raw_reply(client.get(url.clone()), StatusCode::OK).await?;
            assert_eq!(saved.headers()["content-type"].as_bytes(), media);
            let etag = saved.headers()["etag"].clone();
            let modified = saved.headers()["last-modified"].clone();
            assert_eq!(&saved.bytes().await.map_err(|e| e.to_string())?[..], bytes);
            let head = raw_reply(client.head(url.clone()), StatusCode::OK).await?;
            // RFC2616 9.4 entity metadata is a SHOULD observation. Omission or
            // difference must not turn this into an unconditional failure.
            evidence::observation(
                "document-head-content-type",
                &json!({
                    "strength": "SHOULD",
                    "content_type_hex": head.headers().get("content-type")
                        .map(|value| crate::core::response::hex_lower(value.as_bytes())),
                    "matches_get": head.headers().get("content-type")
                        .is_some_and(|value| value.as_bytes() == media),
                }),
            );
            assert!(head.bytes().await.map_err(|e| e.to_string())?.is_empty());
            let conflict = raw_reply(
                client
                    .put(url.clone())
                    .header("Content-Type", media)
                    .body(bytes.to_vec()),
                StatusCode::CONFLICT,
            )
            .await?;
            let explanation = conflict.text().await.map_err(|e| e.to_string())?;
            for required in ["current state", "If-Match", "ETag"] {
                assert!(explanation.contains(required), "{explanation}");
            }
            let identical = raw_reply(client.get(url.clone()), StatusCode::OK).await?;
            assert_eq!(identical.headers()["content-type"].as_bytes(), media);
            assert_eq!(identical.headers()["etag"], etag);
            assert_eq!(identical.headers()["last-modified"], modified);
            assert_eq!(
                &identical.bytes().await.map_err(|e| e.to_string())?[..],
                bytes
            );
            let mut changed = bytes.to_vec();
            changed.push(b' ');
            raw_reply(
                client
                    .put(url.clone())
                    .header("Content-Type", media)
                    .body(changed),
                StatusCode::CONFLICT,
            )
            .await?;
            for (patch_media, patch) in [
                ("application/json", b"{".as_slice()),
                ("application/json", b"[]"),
                ("application/json", b"null"),
                ("text/plain", b"{}"),
            ] {
                raw_reply(
                    client
                        .post(url.clone())
                        .header("Content-Type", patch_media)
                        .body(patch.to_vec()),
                    StatusCode::BAD_REQUEST,
                )
                .await?;
            }
            let unchanged = raw_reply(client.get(url.clone()), StatusCode::OK).await?;
            assert_eq!(unchanged.headers()["etag"], etag);
            assert_eq!(unchanged.headers()["last-modified"], modified);
            assert_eq!(
                &unchanged.bytes().await.map_err(|e| e.to_string())?[..],
                bytes
            );
            let patch = br#"{"replace":{ "new":1,"new":2 },"added":18446744073709551616}"#;
            let object_input = bytes == object || bytes == br#"{"a":1,"\u0061":2}"#;
            raw_reply(
                client
                    .post(url.clone())
                    .header("Content-Type", media)
                    .body(patch.to_vec()),
                if object_input {
                    StatusCode::NO_CONTENT
                } else {
                    StatusCode::BAD_REQUEST
                },
            )
            .await?;
            let after = raw_reply(client.get(url), StatusCode::OK).await?;
            if object_input {
                assert_ne!(after.headers()["etag"], etag);
                let raw = after.text().await.map_err(|e| e.to_string())?;
                // Independent fixed value witnesses: this is shallow replacement,
                // and the opaque nested member sequence/number tokens survive.
                assert!(raw.contains(r#""replace":{ "new":1,"new":2 }"#), "{raw}");
                assert!(raw.contains(r#""added":18446744073709551616"#));
                assert!(!raw.contains(r#""old":true"#));
                if bytes == object {
                    for member in [
                        r#""n":1"#,
                        r#""n":2"#,
                        r#""nested":{ "x":1,"x":2 }"#,
                        r#""fine":1.0000000000000001"#,
                        r#""$serde_json::private::Number":"literal"#,
                    ] {
                        assert!(raw.contains(member), "missing {member}: {raw}");
                    }
                } else {
                    assert!(raw.contains(r#""a":1"#) && raw.contains(r#""a":2"#));
                }
            } else {
                assert_eq!(after.headers()["etag"], etag);
                assert_eq!(after.headers()["last-modified"], modified);
                assert_eq!(&after.bytes().await.map_err(|e| e.to_string())?[..], bytes);
            }
        }
    }
    server.abort();
    Ok(())
}

#[tokio::test]
async fn memory_xapi_document_unicode_and_atomic_writes() -> Result<(), String> {
    verify_documents(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_document_unicode_and_atomic_writes() -> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or("PostgreSQL URL must be set")?;
    verify_documents(&state).await
}

async fn verify_documents(state: &AppState) -> Result<(), String> {
    let repository = &state.xapi.documents;
    for kind in [
        DocumentKind::State,
        DocumentKind::AgentProfile,
        DocumentKind::ActivityProfile,
    ] {
        let mut query = BTreeMap::from([
            (
                "agent".to_owned(),
                json!({"account":{"homePage":"https://example.org/accounts","name":"a\u{0}b"}})
                    .to_string(),
            ),
            (
                "activityId".to_owned(),
                format!("https://example.org/{}", Uuid::new_v4()),
            ),
            (kind.id_param().to_owned(), "doc\u{0}\\u0000".to_owned()),
        ]);
        let DocumentSelection::Single(address) =
            DocumentAddress::from_query(kind, &query, DocumentIdRequirement::Required)
                .map_err(|error| format!("{error:?}"))?
        else {
            return Err("address".into());
        };
        let body =
            json!({"nul\u{0}key":"nul\u{0}value", "literal":"\\u0000", "maximum":"\u{10ffff}"});
        let empty = Preconditions::from_headers(&HeaderMap::new());
        assert_eq!(
            repository
                .put(
                    address.clone(),
                    DocumentContent::Json(body.clone()),
                    empty.clone()
                )
                .await
                .map_err(|error| format!("{error:?}"))?,
            DocumentWriteOutcome::Stored
        );
        let record = repository
            .get(&address)
            .await
            .map_err(|error| format!("{error:?}"))?
            .ok_or("document")?;
        assert_eq!(record.content, DocumentContent::Json(body));
        query.remove(kind.id_param());
        let scope =
            DocumentScope::from_query(kind, &query).map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            repository
                .list_ids(&scope)
                .await
                .map_err(|error| format!("{error:?}"))?,
            vec![address.document_id.clone()]
        );
        let precondition = Preconditions {
            if_match: Some(record.etag),
            if_none_match: None,
            ..Preconditions::default()
        };
        let (left, right) = tokio::join!(
            repository.put(
                address.clone(),
                DocumentContent::Json(json!({"left":1})),
                precondition.clone()
            ),
            repository.put(
                address.clone(),
                DocumentContent::Json(json!({"right":2})),
                precondition
            )
        );
        let outcomes = [
            left.map_err(|error| format!("{error:?}"))?,
            right.map_err(|error| format!("{error:?}"))?,
        ];
        assert_eq!(
            outcomes
                .iter()
                .filter(|o| **o == DocumentWriteOutcome::Stored)
                .count(),
            1
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|o| **o == DocumentWriteOutcome::PreconditionFailed)
                .count(),
            1
        );
        // Concurrent JSON merges must retain both disjoint updates.
        let (left, right) = tokio::join!(
            repository.post_json_merge(
                address.clone(),
                DocumentContent::Json(json!({"merge-left":1})),
                empty.clone()
            ),
            repository.post_json_merge(
                address.clone(),
                DocumentContent::Json(json!({"merge-right":2})),
                empty.clone()
            )
        );
        assert_eq!(
            left.map_err(|error| format!("{error:?}"))?,
            DocumentWriteOutcome::Stored
        );
        assert_eq!(
            right.map_err(|error| format!("{error:?}"))?,
            DocumentWriteOutcome::Stored
        );
        let record = repository
            .get(&address)
            .await
            .map_err(|error| format!("{error:?}"))?
            .ok_or("merged document")?;
        let value = match record.content {
            DocumentContent::Json(value) => value,
            DocumentContent::Bytes { body, .. } => {
                serde_json::from_slice::<crate::surfaces::xapi::lossless::StoredJson>(&body)
                    .map_err(|e| e.to_string())?
                    .0
            }
        };
        assert_eq!(value["merge-left"], 1);
        assert_eq!(value["merge-right"], 2);
        let selection = if kind == DocumentKind::State {
            DocumentSelection::Scope(scope.clone())
        } else {
            DocumentSelection::Single(address)
        };
        assert_eq!(
            repository
                .delete(selection, empty)
                .await
                .map_err(|error| format!("{error:?}"))?,
            DocumentDeleteOutcome::Deleted
        );
        assert!(
            repository
                .list_ids(&scope)
                .await
                .map_err(|error| format!("{error:?}"))?
                .is_empty()
        );
    }
    Ok(())
}
