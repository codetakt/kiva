//! Signature payload roles follow the received attachment owner.
#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_signature_owner_roles() -> Result<(), String> {
    verify(memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_signature_owner_roles() -> Result<(), String> {
    verify(
        postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn verify(mut state: AppState) -> Result<(), String> {
    state.xapi.signature_policy = SignaturePolicy::from_json(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/controls/kiva_certificate_policy.json"
    )))
    .map_err(|e| e.message)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let base = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let router = evidence::router(&state);
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    for nested in [true, false] {
        for method in ["PUT", "POST", "batch"] {
            for variant in [
                "valid",
                "case",
                "excluded",
                "both",
                "different-ifi",
                "different-verb",
                "different-object",
                "different-duration",
                "wrong-type",
                "missing-type",
                "id",
                "stored",
                "authority",
                "version",
                "nested",
                "timestamp",
                "result",
                "context",
                "actor",
                "verb",
                "metadata",
                "signature",
            ] {
                check(&state, &client, &base, nested, method, variant).await?;
            }
        }
    }
    server.abort();
    Ok(())
}

fn received() -> Value {
    let mut root = statement(Uuid::new_v4());
    root["timestamp"] = json!("2026-10-29T00:00:00Z");
    root["actor"]["name"] = json!("Alice Example");
    root["result"] = json!({"duration":"PT1S"});
    root
}

fn mutate(payload: &mut Value, variant: &str) {
    match variant {
        "case" => payload["actor"]["name"] = json!("ALICE EXAMPLE"),
        "excluded" => {
            payload["timestamp"] = json!("2020-01-01T00:00:00Z");
            payload["context"] = json!({"language":"ja"});
        }
        "different-ifi" => payload["actor"]["mbox"] = json!("mailto:other@example.org"),
        "different-verb" => payload["verb"]["id"] = json!("https://example.org/other"),
        "different-object" => payload["object"]["id"] = json!("https://example.org/other"),
        "different-duration" => payload["result"]["duration"] = json!("PT2S"),
        "wrong-type" => payload["objectType"] = json!("Activity"),
        "missing-type" => {
            payload.as_object_mut().unwrap().remove("objectType");
        }
        "id" => payload["id"] = json!(Uuid::new_v4()),
        "stored" => payload["stored"] = json!("2020-01-01T00:00:00Z"),
        "authority" => payload["authority"] = json!({"mbox":"mailto:authority@example.org"}),
        "version" => payload["version"] = json!("2.0.0"),
        "nested" => {
            let mut inner = received();
            inner.as_object_mut().unwrap().remove("id");
            inner["objectType"] = json!("SubStatement");
            payload["object"] = inner;
        }
        "timestamp" => payload["timestamp"] = json!("2020-01-01"),
        "result" => payload["result"]["success"] = json!("true"),
        "context" => payload["context"] = json!({"registration":"invalid"}),
        "actor" => {
            payload["actor"].as_object_mut().unwrap().remove("mbox");
        }
        "verb" => payload["verb"]["id"] = json!("relative"),
        "metadata" => payload["attachments"] = json!([{"usageType":"relative"}]),
        _ => {}
    }
}

fn attach(owner: &mut Value, raw: &str) {
    owner["attachments"] = json!([{"usageType":SIGNATURE_ATTACHMENT_USAGE_TYPE,
        "display":{"en":"signature"},"contentType":"application/octet-stream",
        "length":raw.len(),"sha2":hex_lower(&Sha256::digest(raw.as_bytes()))}]);
}

#[allow(clippy::too_many_lines)]
async fn check(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
    nested: bool,
    method: &str,
    variant: &str,
) -> Result<(), String> {
    let mut root = received();
    let mut owner = received();
    if nested {
        owner.as_object_mut().unwrap().remove("id");
        owner["objectType"] = json!("SubStatement");
    } else {
        owner = root.clone();
    }
    let mut payload = owner.clone();
    mutate(&mut payload, variant);
    let equivalent = matches!(variant, "valid" | "case" | "excluded" | "both")
        || (!nested
            && matches!(
                variant,
                "missing-type" | "id" | "stored" | "authority" | "version"
            ));
    let comparator_runs =
        equivalent || variant.starts_with("different-") || (!nested && variant == "nested");
    let mut raw = sign_payload(&payload)?;
    if variant == "signature" {
        let (input, signature) = raw.rsplit_once('.').ok_or("signature")?;
        let mut bytes = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|e| e.to_string())?;
        bytes[0] ^= 1;
        raw = format!("{input}.{}", URL_SAFE_NO_PAD.encode(bytes));
    }
    attach(&mut owner, &raw);
    if nested {
        root["object"] = owner;
    } else {
        root = owner;
    }
    let mut signatures = vec![raw];
    if nested && variant == "both" {
        let outer = sign_payload(&root)?;
        attach(&mut root, &outer);
        signatures.push(outer);
    }
    let sibling = received();
    let submitted = if method == "batch" {
        json!([sibling, root])
    } else {
        root.clone()
    };
    let mut body = format!("--owners\r\nContent-Type: application/json\r\n\r\n{submitted}\r\n");
    for raw in &signatures {
        write!(&mut body,"--owners\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: {}\r\n\r\n{raw}\r\n",hex_lower(&Sha256::digest(raw.as_bytes()))).unwrap();
    }
    body.push_str("--owners--\r\n");
    let request = if method == "PUT" {
        client.put(format!(
            "{base}/statements?statementId={}",
            root["id"].as_str().unwrap()
        ))
    } else {
        client.post(format!("{base}/statements"))
    };
    take_logical_observations();
    let reply = request
        .header("X-Experience-API-Version", "2.0.0")
        .header("Content-Type", "multipart/mixed; boundary=owners")
        .body(body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let reply = evidence::received(reply).await?;
    let status = reply.status();
    let text = reply.text().await.map_err(|e| e.to_string())?;
    assert_eq!(
        status,
        if equivalent {
            if method == "PUT" {
                StatusCode::NO_CONTENT
            } else {
                StatusCode::OK
            }
        } else {
            StatusCode::BAD_REQUEST
        },
        "{nested}/{method}/{variant}: {text}"
    );
    let calls = take_logical_observations();
    assert_eq!(
        calls.len(),
        if comparator_runs { signatures.len() } else { 0 },
        "{nested}/{method}/{variant}"
    );
    if comparator_runs {
        assert_eq!(calls.last().unwrap()["right"], payload);
        assert_eq!(calls.last().unwrap()["equivalent"], json!(equivalent));
    }
    for value in if method == "batch" {
        vec![&root, &sibling]
    } else {
        vec![&root]
    } {
        let id = Uuid::parse_str(value["id"].as_str().ok_or("id")?).map_err(|e| e.to_string())?;
        let saved = state
            .xapi
            .statements
            .get(id)
            .await
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            saved.is_some(),
            equivalent,
            "batch admission must be atomic"
        );
        if let Some(saved) = saved {
            assert_eq!(saved.statement["actor"], value["actor"]);
            assert_eq!(saved.statement["object"], value["object"]);
            if value == &root {
                assert_eq!(saved.attachments.len(), signatures.len());
                for raw in &signatures {
                    let hash = hex_lower(&Sha256::digest(raw.as_bytes()));
                    assert_eq!(saved.attachments[&hash].body, raw.as_bytes());
                }
            }
        }
    }
    evidence::observation(
        "signature-owner-role",
        &json!({"nested":nested,"method":method,
        "variant":variant,"expected_admission":equivalent,"calls":calls}),
    );
    Ok(())
}
