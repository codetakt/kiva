//! Positive product-capability gates, separate from advisory IEEE dispositions.
#![allow(clippy::wildcard_imports)]
use super::xapi_regressions::evidence;
use super::*;

#[path = "xapi_capabilities/durability.rs"]
mod durability;
#[path = "xapi_capabilities/fetch.rs"]
mod fetch;
#[path = "xapi_capabilities/interactions.rs"]
mod interactions;
#[path = "xapi_capabilities/metadata.rs"]
mod metadata;
#[path = "xapi_capabilities/queries.rs"]
mod queries;
#[path = "xapi_capabilities/replay.rs"]
mod replay;

fn statement(id: Uuid) -> Value {
    json!({"id":id,"actor":{"mbox":"mailto:capability@example.org"},
        "verb":{"id":"https://example.org/completed"},"object":{"id":"https://example.org/activity"}})
}

struct Server {
    state: AppState,
    base: String,
    client: reqwest::Client,
    task: tokio::task::JoinHandle<Result<(), std::io::Error>>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Server {
    async fn start(state: AppState) -> Result<Self, String> {
        Self::options(state, json!({})).await
    }

    async fn options(mut state: AppState, options: Value) -> Result<Self, String> {
        state.auth.require_authorization = true;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| e.to_string())?;
        let base = format!(
            "http://{}",
            listener.local_addr().map_err(|e| e.to_string())?
        );
        state.auth.register_bearer_subject("writer-b", "writer-b");
        state
            .auth
            .register_bearer_scopes("writer-b", [OAuthScope::XapiRead, OAuthScope::XapiWrite]);
        let mut configuration = json!({
            "canonical_inventory":true,"person_enrichment":true,"activity_hosting":true,
            "canonical_writers":["statement-writer-a"],"directory_writers":["statement-writer-a"],
            "trusted_authority_subjects":["statement-writer-a"],"public_base_url":base
        });
        configuration
            .as_object_mut()
            .unwrap()
            .extend(options.as_object().unwrap().clone());
        let config = crate::surfaces::xapi::capabilities::CapabilityConfig::from_json(
            &configuration.to_string(),
        )
        .map_err(|e| e.to_string())?;
        state.xapi.capabilities.configure(config);
        let router = evidence::router(&state);
        let task = tokio::spawn(async move { axum::serve(listener, router).await });
        Ok(Self {
            state,
            base,
            client: reqwest::Client::new(),
            task,
        })
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.request_as(method, path, "statement-writer-a")
    }

    fn request_as(
        &self,
        method: reqwest::Method,
        path: &str,
        token: &str,
    ) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}{path}", self.base))
            .bearer_auth(token)
            .header("X-Experience-API-Version", "2.0.0")
    }

    async fn receive(
        &self,
        request: reqwest::RequestBuilder,
        expected: StatusCode,
    ) -> Result<reqwest::Response, String> {
        let reply = evidence::received(request.send().await.map_err(|e| e.to_string())?).await?;
        assert_eq!(reply.status(), expected, "{}", reply.url());
        assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
        Ok(reply)
    }

    async fn json(&self, request: reqwest::RequestBuilder) -> Result<Value, String> {
        self.receive(request, StatusCode::OK)
            .await?
            .json()
            .await
            .map_err(|e| e.to_string())
    }
}

#[tokio::test]
async fn memory_xapi_product_error_negotiation() -> Result<(), String> {
    errors(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_product_error_negotiation() -> Result<(), String> {
    errors(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL fixture required")?,
    )
    .await
}

async fn errors(state: &AppState) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let base = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let router = evidence::router(state);
    let task = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    for (accept, media) in [
        ("text/plain", "text/plain"),
        ("text/html", "text/html"),
        ("application/json", "application/json"),
        ("text/html;q=0.9, application/json;q=0.1", "text/html"),
        (
            "text/html;q=0.9; note=\"a,b;c\"; flag, application/json;q=0.1",
            "text/html",
        ),
        ("text/*, text/html;q=0, application/json;q=0", "text/plain"),
    ] {
        for method in [reqwest::Method::GET, reqwest::Method::HEAD] {
            let reply = client
                .request(method.clone(), format!("{base}/statements"))
                .header("X-Experience-API-Version", "2.1.0")
                .header("Accept", accept)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let reply = evidence::received(reply).await?;
            assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
            assert_eq!(
                reply.headers()["content-type"]
                    .to_str()
                    .unwrap()
                    .split(';')
                    .next(),
                Some(media)
            );
            assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
            assert!(
                reply
                    .headers()
                    .contains_key("x-experience-api-consistent-through")
            );
            assert!(
                reply
                    .headers()
                    .get_all("vary")
                    .iter()
                    .any(|v| v == "Accept")
            );
            let body = reply.text().await.map_err(|e| e.to_string())?;
            if method == reqwest::Method::HEAD {
                assert!(body.is_empty());
            } else {
                assert!(
                    body.contains("Version") && body.contains("unsupported"),
                    "{body}"
                );
            }
        }
    }
    let mut invalid = statement(Uuid::new_v4());
    invalid[format!("<script>{}</script>", "x".repeat(70_000))] = json!(true);
    for media in ["text/plain", "text/html", "application/json"] {
        let reply = evidence::received(
            client
                .post(format!("{base}/statements"))
                .header("X-Experience-API-Version", "2.0.0")
                .header("Accept", media)
                .json(&invalid)
                .send()
                .await
                .map_err(|e| e.to_string())?,
        )
        .await?;
        assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            reply.headers()["content-type"]
                .to_str()
                .unwrap()
                .split(';')
                .next(),
            Some(media)
        );
        let body = reply.text().await.map_err(|e| e.to_string())?;
        assert!(
            body.contains("unsupported property"),
            "a long cause must not disappear"
        );
        if media == "text/html" {
            assert!(!body.contains("<script>"));
            assert!(body.contains("&lt;script&gt;"));
        }
    }
    task.abort();
    evidence::observation(
        "product-capability",
        &json!({"capabilities":["negotiated-errors"],"enabled":true,"representations":["application/json","text/html","text/plain"],"head":true,"preference_weights_and_exclusions":true}),
    );
    Ok(())
}
