//! Regression witnesses for PR5 and subsequent local review gaps, through real HTTP.
#![allow(clippy::wildcard_imports)]
use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha224, Sha256, Sha384, Sha512};

#[path = "review_gaps/attachments.rs"]
mod attachments;
#[path = "review_gaps/document_lists.rs"]
mod document_lists;

#[path = "review_gaps/document_scope.rs"]
mod document_scope;

#[path = "review_gaps/document_since.rs"]
mod document_since;
#[path = "review_gaps/language.rs"]
mod language;
#[path = "review_gaps/leap_seconds.rs"]
mod leap_seconds;
#[path = "review_gaps/mime_persistence.rs"]
mod mime_persistence;
#[path = "review_gaps/name_constraints.rs"]
mod name_constraints;
#[path = "review_gaps/query.rs"]
mod query;
#[path = "review_gaps/query_limit.rs"]
mod query_limit;
#[path = "review_gaps/related_activities.rs"]
mod related_activities;
#[path = "review_gaps/shape.rs"]
mod shape;
#[path = "review_gaps/shared_attachments.rs"]
mod shared_attachments;
#[path = "review_gaps/timestamp_range.rs"]
mod timestamp_range;

struct HttpProbe {
    base: String,
    client: reqwest::Client,
    server: tokio::task::JoinHandle<std::io::Result<()>>,
}
impl Drop for HttpProbe {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl HttpProbe {
    async fn new(state: &AppState) -> Result<Self, String> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| e.to_string())?;
        let base = format!(
            "http://{}",
            listener.local_addr().map_err(|e| e.to_string())?
        );
        let router = evidence::router(state);
        let server = tokio::spawn(async move { axum::serve(listener, router).await });
        Ok(Self {
            base,
            client: reqwest::Client::new(),
            server,
        })
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
        Ok(reply)
    }

    async fn submit(
        &self,
        value: &Value,
        post: bool,
        mime: Option<Vec<u8>>,
        accepted: bool,
    ) -> Result<(), String> {
        let request = if post {
            self.client.post(format!("{}/statements", self.base))
        } else {
            self.client.put(format!(
                "{}/statements?statementId={}",
                self.base,
                value["id"].as_str().ok_or("id")?
            ))
        };
        let request = if let Some(body) = mime {
            request
                .header("Content-Type", "multipart/mixed; boundary=review-gap")
                .body(body)
        } else {
            request.json(value)
        };
        let reply = Self::receive(request).await?;
        assert_eq!(
            reply.status(),
            if accepted {
                if post {
                    StatusCode::OK
                } else {
                    StatusCode::NO_CONTENT
                }
            } else {
                StatusCode::BAD_REQUEST
            },
            "input {value}: {}",
            reply.text().await.map_err(|e| e.to_string())?
        );
        let values = value
            .as_array()
            .map_or_else(|| vec![value], |values| values.iter().collect());
        for expected in values {
            let id = expected["id"].as_str().ok_or("id")?;
            let reply = Self::receive(
                self.client
                    .get(format!("{}/statements?statementId={id}", self.base)),
            )
            .await?;
            assert_eq!(
                reply.status(),
                if accepted {
                    StatusCode::OK
                } else {
                    StatusCode::NOT_FOUND
                },
                "read after admission: {value}"
            );
            if accepted {
                let saved: Value = reply.json().await.map_err(|e| e.to_string())?;
                for pointer in ["/attachments", "/object/attachments"] {
                    assert_eq!(
                        saved.pointer(pointer),
                        expected.pointer(pointer),
                        "preserve metadata: {pointer}"
                    );
                }
            }
        }
        Ok(())
    }

    async fn raw_attachment(&self, id: &Value, data: &[u8], copies: usize) -> Result<(), String> {
        let reply = Self::receive(self.client.get(format!(
            "{}/statements?statementId={}&attachments=true",
            self.base,
            id.as_str().ok_or("id")?
        )))
        .await?;
        assert_eq!(reply.status(), StatusCode::OK);
        assert!(
            reply.headers()["content-type"]
                .to_str()
                .map_err(|e| e.to_string())?
                .starts_with("multipart/mixed;")
        );
        let body = reply.bytes().await.map_err(|e| e.to_string())?;
        assert_eq!(
            body.windows(data.len())
                .filter(|bytes| *bytes == data)
                .count(),
            copies
        );
        Ok(())
    }
}

fn root_and_owner(nested: bool) -> (Value, &'static str) {
    let mut value = statement(Uuid::new_v4());
    if nested {
        let mut child = statement(Uuid::new_v4());
        child.as_object_mut().expect("Statement").remove("id");
        child["objectType"] = json!("SubStatement");
        value["object"] = child;
    }
    (value, if nested { "/object" } else { "" })
}

fn attachment(data: &[u8], digest: &str) -> Value {
    json!({"usageType":"https://example.org/evidence", "display":{"en":"evidence"},
        "contentType":"application/octet-stream", "length":data.len(), "sha2":digest})
}

fn multipart(value: &Value, parts: &[(&str, Option<&str>, &[u8])]) -> Vec<u8> {
    let mut result =
        format!("--review-gap\r\nContent-Type: application/json\r\n\r\n{value}\r\n").into_bytes();
    for (hash, content_type, data) in parts {
        result.extend_from_slice(b"--review-gap\r\n");
        if let Some(content_type) = content_type {
            result.extend_from_slice(format!("Content-Type: {content_type}\r\n").as_bytes());
        }
        result.extend_from_slice(
            format!("Content-Transfer-Encoding: binary\r\nX-Experience-API-Hash: {hash}\r\n\r\n")
                .as_bytes(),
        );
        result.extend_from_slice(data);
        result.extend_from_slice(b"\r\n");
    }
    result.extend_from_slice(b"--review-gap--\r\n");
    result
}
