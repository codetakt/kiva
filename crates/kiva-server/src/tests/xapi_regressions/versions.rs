#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_version_roles_and_about_exemption() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_version_roles_and_about_exemption() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

const VERSIONS: &[(Option<&str>, bool)] = &[
    (None, true),
    (Some("2.0"), true),
    (Some("2.0.0"), true),
    (Some("2.0.1"), true),
    (Some("2.0.999999"), true),
    (
        Some("2.0.184467440737095516160000000000000000000000000000001"),
        true,
    ),
    (Some("1.0.3"), false),
    (Some("2.1.0"), false),
    (Some("3.0.0"), false),
    (Some("malformed"), false),
    (Some("2.0.01"), false),
    (Some("2.0.1-beta"), false),
];

async fn receive(
    request: reqwest::RequestBuilder,
    version: Option<&str>,
) -> Result<reqwest::Response, String> {
    let request = if let Some(version) = version {
        request.header("X-Experience-API-Version", version)
    } else {
        request
    };
    let reply = evidence::received(request.send().await.map_err(|e| e.to_string())?).await?;
    // Truthful implemented version, even when the request used an unknown
    // normal patch, old edition, malformed string or no header at all.
    assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
    if reply.url().path().starts_with("/statements") {
        OffsetDateTime::parse(
            reply.headers()["x-experience-api-consistent-through"]
                .to_str()
                .map_err(|e| e.to_string())?,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(reply)
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
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
    evidence::observation(
        "version-policy",
        &json!({"normal_patch":"D-08=A: every normal 2.0.x patch is compatible without a fixed integer range","implemented_response_version":"2.0.0","legacy_routing":false,"missing_header":"default 2.0 API routing; project disposition, not mandatory IEEE status","unknown_about_parameter":"auditUnknown","independently_refused_credential":"invalid-credential"}),
    );
    let agent = json!({"mbox":"mailto:version@example.org"}).to_string();
    let urls = [
        ("/statements", vec![]),
        (
            "/activities",
            vec![("activityId", "https://example.org/version")],
        ),
        ("/agents", vec![("agent", agent.as_str())]),
        (
            "/activities/state",
            vec![
                ("agent", agent.as_str()),
                ("activityId", "https://example.org/version"),
            ],
        ),
        ("/agents/profile", vec![("agent", agent.as_str())]),
        (
            "/activities/profile",
            vec![("activityId", "https://example.org/version")],
        ),
    ];
    for &(version, compatible) in VERSIONS {
        for (path, pairs) in &urls {
            let url = reqwest::Url::parse_with_params(&format!("{base}{path}"), pairs)
                .map_err(|e| e.to_string())?;
            for head in [false, true] {
                let request = if head {
                    client.head(url.clone())
                } else {
                    client.get(url.clone())
                };
                let reply = receive(request, version).await?;
                assert_eq!(
                    reply.status(),
                    if compatible {
                        StatusCode::OK
                    } else {
                        StatusCode::BAD_REQUEST
                    },
                    "{path}/{version:?}"
                );
                if head {
                    assert!(reply.bytes().await.map_err(|e| e.to_string())?.is_empty());
                } else if !compatible {
                    let explanation = reply.text().await.map_err(|e| e.to_string())?;
                    assert!(
                        explanation.contains("Version") && explanation.contains("unsupported"),
                        "{explanation}"
                    );
                } else {
                    reply.json::<Value>().await.map_err(|e| e.to_string())?;
                }
            }
        }
        for post in [false, true] {
            let id = Uuid::new_v4();
            let value = statement(id);
            let request = if post {
                client.post(format!("{base}/statements"))
            } else {
                client.put(format!("{base}/statements?statementId={id}"))
            };
            let reply = receive(
                request.json(&if post { json!([value]) } else { value.clone() }),
                version,
            )
            .await?;
            assert_eq!(
                reply.status(),
                if compatible {
                    if post {
                        StatusCode::OK
                    } else {
                        StatusCode::NO_CONTENT
                    }
                } else {
                    StatusCode::BAD_REQUEST
                }
            );
            if compatible && post {
                assert_eq!(
                    reply.json::<Value>().await.map_err(|e| e.to_string())?,
                    json!([id])
                );
            } else if !compatible {
                let explanation = reply.text().await.map_err(|e| e.to_string())?;
                assert!(explanation.contains("Version") && explanation.contains("unsupported"));
            }
            let reply = receive(
                client.get(format!("{base}/statements?statementId={id}&format=exact")),
                Some("2.0.0"),
            )
            .await?;
            assert_eq!(
                reply.status(),
                if compatible {
                    StatusCode::OK
                } else {
                    StatusCode::NOT_FOUND
                }
            );
            if compatible {
                let stored = reply.json::<Value>().await.map_err(|e| e.to_string())?;
                assert_eq!(stored["id"], value["id"]);
                assert_eq!(stored["verb"], value["verb"]);
                assert_eq!(stored["version"], "2.0.0");
            } else {
                assert!(
                    state
                        .xapi
                        .statements
                        .get(id)
                        .await
                        .map_err(|e| format!("{e:?}"))?
                        .is_none()
                );
            }
        }
        for head in [false, true] {
            for (cause, status) in [
                ("none", StatusCode::OK),
                ("unknown", StatusCode::BAD_REQUEST),
                ("refused", StatusCode::UNAUTHORIZED),
            ] {
                let url = if cause == "unknown" {
                    format!("{base}/about?auditUnknown=1")
                } else {
                    format!("{base}/about")
                };
                let request = if head {
                    client.head(url)
                } else {
                    client.get(url)
                };
                let request = if cause == "refused" {
                    request.bearer_auth("invalid-credential")
                } else {
                    request
                };
                let reply = receive(request, version).await?;
                assert_eq!(reply.status(), status, "About/{version:?}/{cause}");
                if head {
                    assert!(reply.bytes().await.map_err(|e| e.to_string())?.is_empty());
                } else if cause == "none" {
                    assert_eq!(
                        reply.json::<Value>().await.map_err(|e| e.to_string())?["version"],
                        json!(["2.0.0"])
                    );
                }
            }
        }
    }
    verify_body_versions(&client, &base).await?;
    server.abort();
    Ok(())
}

async fn verify_body_versions(client: &reqwest::Client, base: &str) -> Result<(), String> {
    for version in [
        None,
        Some("2.0"),
        Some("2.0.0"),
        Some("1.0.3"),
        Some("2.1.0"),
        Some("not-semver"),
    ] {
        for post in [false, true] {
            let id = Uuid::new_v4();
            let mut value = statement(id);
            if let Some(version) = version {
                value["version"] = json!(version);
            }
            let request = if post {
                client.post(format!("{base}/statements"))
            } else {
                client.put(format!("{base}/statements?statementId={id}"))
            };
            let reply = receive(request.json(&value), Some("2.0.0")).await?;
            let status = reply.status();
            evidence::observation(
                "body-version-admission-advisory",
                &json!({"body_version":version,"post":post,"status":status.as_u16(),"strength":"SHOULD NOT reject solely for supplied body version; accepted preservation and absence default are SHALL"}),
            );
            if version.is_some() && status == StatusCode::BAD_REQUEST {
                assert_eq!(
                    receive(
                        client.get(format!("{base}/statements?statementId={id}")),
                        Some("2.0.0")
                    )
                    .await?
                    .status(),
                    StatusCode::NOT_FOUND
                );
                continue;
            }
            assert_eq!(
                status,
                if post {
                    StatusCode::OK
                } else {
                    StatusCode::NO_CONTENT
                }
            );
            let read = receive(
                client.get(format!("{base}/statements?statementId={id}&format=exact")),
                Some("2.0.0"),
            )
            .await?;
            assert_eq!(read.status(), StatusCode::OK);
            assert_eq!(
                read.json::<Value>().await.map_err(|e| e.to_string())?["version"],
                json!(version.unwrap_or("2.0.0"))
            );
        }
    }
    Ok(())
}
