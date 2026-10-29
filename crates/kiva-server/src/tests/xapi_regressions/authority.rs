//! Actual authentication delegation and authority overwrite, separately from
//! optional trusted-authority retention or per-Statement permission filtering.
#![allow(clippy::wildcard_imports)]
use super::*;
use kiva_idp_adapter::{BearerTokenVerifier, IdpAdapterError, VerificationFuture};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

pub(super) struct Verifier {
    pub(super) calls: Arc<Mutex<Vec<Value>>>,
}

impl BearerTokenVerifier for Verifier {
    fn verify<'a>(&'a self, token: &'a str) -> VerificationFuture<'a> {
        Box::pin(async move {
            let identity = match token {
                "authority-a" | "authority-a-alias" => Some(("subject-a", true)),
                "authority-b" => Some(("subject-b", true)),
                "authority-admin" => Some(("subject-admin", true)),
                "authority-reader" => Some(("subject-reader", false)),
                _ => None,
            };
            self.calls.lock().expect("verifier log").push(json!({
                "credential_sha256":format!("{:x}",Sha256::digest(token.as_bytes())),
                "authenticated_subject":identity.map(|(subject,_)|subject),
                "write_permitted":identity.map(|(_,write)|write),
                "decision":"independent configured verifier, before handler authority projection"
            }));
            identity.map_or(Err(IdpAdapterError::InactiveToken), |(subject, write)| {
                Ok(kiva_identity::AuthenticatedPrincipal::new(
                    subject,
                    if write {
                        kiva_identity::StatementWritePermission::Allowed
                    } else {
                        kiva_identity::StatementWritePermission::Denied
                    },
                    if write {
                        [
                            kiva_identity::OAuthScope::XapiRead,
                            kiva_identity::OAuthScope::XapiWrite,
                        ]
                        .into()
                    } else {
                        [kiva_identity::OAuthScope::XapiRead].into()
                    },
                ))
            })
        })
    }
}

#[tokio::test]
async fn memory_xapi_authenticated_authority_overwrite() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_authenticated_authority_overwrite() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn receive(request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let reply = evidence::received(
        request
            .header("X-Experience-API-Version", "2.0.0")
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await?;
    assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
    OffsetDateTime::parse(
        reply.headers()["x-experience-api-consistent-through"]
            .to_str()
            .map_err(|e| e.to_string())?,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| e.to_string())?;
    Ok(reply)
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    for required in [false, true] {
        let mut configured = state.clone();
        let calls = Arc::new(Mutex::new(Vec::new()));
        configured.auth = AuthConfig::with_token_verifier(
            required,
            Arc::new(Verifier {
                calls: Arc::clone(&calls),
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| e.to_string())?;
        let base = format!(
            "http://{}",
            listener.local_addr().map_err(|e| e.to_string())?
        );
        let router = evidence::router(&configured);
        let server = tokio::spawn(async move { axum::serve(listener, router).await });
        let client = reqwest::Client::new();
        let mut by_subject = BTreeMap::<String, Value>::new();
        evidence::observation(
            "authority-policy",
            &json!({
                "require_authorization":required,"verification":"external BearerTokenVerifier controls identity and read/write scopes; no fixture-token fallback",
                "input_authority_trusted_retention":false,"supplied_authority":"always replaced for accepted input, including authenticated trusted provider",
                "mapping":{"homePage":"https://example.org/kiva/conformance/auth","account_name":"verified principal subject, or kiva-lrs for allowed anonymous"},
                "alias":"two distinct credentials independently map to subject-a; injectivity is not an IEEE obligation",
                "scope":"authority/operation authorization only; no claim about per-Statement visibility filtering"
            }),
        );
        for (token, subject) in [
            (None, "kiva-lrs"),
            (Some("authority-a"), "subject-a"),
            (Some("authority-a-alias"), "subject-a"),
            (Some("authority-b"), "subject-b"),
        ] {
            for supplied in [
                None,
                Some(json!({"mbox":"mailto:untrusted@example.org"})),
                Some(json!({"objectType":"Group","member":[]})),
                Some(
                    json!({"objectType":"Group","member":[{"mbox":"mailto:a@example.org"},{"mbox":"mailto:b@example.org"}]}),
                ),
                Some(
                    json!({"objectType":"Group","account":{"homePage":"https://example.org/provider","name":"provider-supplied"},"member":[{"mbox":"mailto:provider@example.org"}]}),
                ),
            ] {
                for post in [false, true] {
                    let id = Uuid::new_v4();
                    let mut body = statement(id);
                    if let Some(authority) = &supplied {
                        body["authority"] = authority.clone();
                    }
                    let request = if post {
                        client
                            .post(format!("{base}/statements"))
                            .json(&json!([body]))
                    } else {
                        client
                            .put(format!("{base}/statements?statementId={id}"))
                            .json(&body)
                    };
                    let request = if let Some(token) = token {
                        request.bearer_auth(token)
                    } else {
                        request
                    };
                    let reply = receive(request).await?;
                    if required && token.is_none() {
                        assert_eq!(reply.status(), StatusCode::UNAUTHORIZED);
                        assert!(
                            state
                                .xapi
                                .statements
                                .get(id)
                                .await
                                .map_err(|e| format!("{e:?}"))?
                                .is_none()
                        );
                        continue;
                    }
                    assert_eq!(
                        reply.status(),
                        if post {
                            StatusCode::OK
                        } else {
                            StatusCode::NO_CONTENT
                        }
                    );
                    if post {
                        assert_eq!(
                            reply.json::<Value>().await.map_err(|e| e.to_string())?,
                            json!([id])
                        );
                    }
                    // Expected mapping is an independent fixture value, not a call
                    // to authority_agent or a copy of the target's first result.
                    let expected = json!({"objectType":"Agent","account":{"homePage":"https://example.org/kiva/conformance/auth","name":subject}});
                    for format in [None, Some("exact"), Some("canonical")] {
                        let mut url = reqwest::Url::parse_with_params(
                            &format!("{base}/statements"),
                            [("statementId", id.to_string())],
                        )
                        .map_err(|e| e.to_string())?;
                        if let Some(format) = format {
                            url.query_pairs_mut().append_pair("format", format);
                        }
                        let reply =
                            receive(client.get(url).bearer_auth("authority-reader")).await?;
                        assert_eq!(reply.status(), StatusCode::OK);
                        let returned = reply.json::<Value>().await.map_err(|e| e.to_string())?;
                        assert_eq!(returned["authority"], expected);
                        if let Some(previous) =
                            by_subject.insert(subject.into(), returned["authority"].clone())
                        {
                            assert_eq!(previous, returned["authority"]);
                        }
                    }
                    let stored = state
                        .xapi
                        .statements
                        .get(id)
                        .await
                        .map_err(|e| format!("{e:?}"))?
                        .ok_or("accepted authority")?;
                    assert_eq!(stored.statement["authority"], expected);
                }
            }
        }
        for (token, status) in [
            ("authority-reader", StatusCode::FORBIDDEN),
            ("unrecognized-authority", StatusCode::UNAUTHORIZED),
        ] {
            let id = Uuid::new_v4();
            assert_eq!(
                receive(
                    client
                        .put(format!("{base}/statements?statementId={id}"))
                        .bearer_auth(token)
                        .json(&statement(id))
                )
                .await?
                .status(),
                status
            );
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
        let observed = calls.lock().expect("verifier log").clone();
        assert!(
            observed
                .iter()
                .any(|v| v["authenticated_subject"] == "subject-a")
        );
        assert!(
            observed
                .iter()
                .any(|v| v["authenticated_subject"] == "subject-b")
        );
        assert!(observed.iter().any(|v| v["write_permitted"] == false));
        assert!(
            observed
                .iter()
                .any(|v| v["authenticated_subject"].is_null())
        );
        evidence::observation(
            "actual-authentication-decisions",
            &json!({"calls":observed,"require_authorization":required}),
        );
        server.abort();
    }
    Ok(())
}
