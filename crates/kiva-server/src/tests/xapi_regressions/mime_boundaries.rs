#![allow(clippy::wildcard_imports)]
use super::*;
use sha2::{Digest, Sha256};

async fn receive(request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let reply = request
        .header("X-Experience-API-Version", "2.0.0")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let reply = evidence::received(reply).await?;
    assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
    assert!(
        reply
            .headers()
            .contains_key("x-experience-api-consistent-through")
    );
    Ok(reply)
}

#[tokio::test]
async fn memory_xapi_mime_framing_and_receiver_roles() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_mime_framing_and_receiver_roles() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

// Hand-authored wire values, independent of the server's MIME parser and
// response encoder. Every row changes one semantic dimension of valid input.
struct Probe {
    name: &'static str,
    first: String,
    part: String,
    metadata: String,
    encoding: String,
    outer: String,
    boundary: String,
    raw: Vec<u8>,
    accepted: bool,
}

impl Probe {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            first: "Content-Type: application/json\r\n".into(),
            part: "Content-Type: application/octet-stream\r\n".into(),
            metadata: "application/octet-stream".into(),
            encoding: "Content-Transfer-Encoding: binary\r\n".into(),
            outer: "multipart/mixed; boundary=mime-outer".into(),
            boundary: "mime-outer".into(),
            raw: b"\x00\xff\r\n--mime-other\r\nembedded--mime-outer\r\n".to_vec(),
            accepted: true,
        }
    }
}

fn probes() -> Vec<Probe> {
    let mut rows = Vec::new();
    for byte in [0_u8, 1, b'\n', b'\r', b'"', b'\\', 127] {
        for escaped in [false, true] {
            if !escaped && matches!(byte, b'\r' | b'"' | b'\\') {
                continue;
            }
            let mut row = Probe::new("first-quoted-ascii");
            row.first = format!(
                "Content-Type: application/json; note=\"{}{}\"\r\n",
                if escaped { "\\" } else { "" },
                char::from(byte)
            );
            rows.push(row);
        }
    }
    for (name, first, good) in [
        (
            "first-comment",
            "Content-Type: (a (nested)) Application / JSON (end)\r\n",
            true,
        ),
        (
            "first-fold",
            "Content-Type:\r\n\tapplication/json (escaped \\) note)\r\n",
            true,
        ),
        ("first-wrong", "Content-Type: text/plain\r\n", false),
        ("first-missing", "", false),
        (
            "first-unclosed-comment",
            "Content-Type: application/json (open\r\n",
            false,
        ),
        (
            "first-nonascii",
            "Content-Type: application/json (日本語)\r\n",
            false,
        ),
        (
            "first-bare-lf",
            "Content-Type:\n application/json\r\n",
            false,
        ),
        (
            "first-orphan",
            " application/json\r\nContent-Type: application/json\r\n",
            false,
        ),
    ] {
        let mut row = Probe::new(name);
        row.first = first.into();
        row.accepted = good;
        rows.push(row);
    }
    for (name, encoding, good) in [
        ("cte-case", "BiNaRy", true),
        ("cte-nested", "binary (nested (note))", true),
        ("cte-escaped", "binary (escaped \\) note)", true),
        ("cte-folded", "\r\n\tbinary (raw octets)", true),
        ("cte-base64", "base64", false),
        ("cte-quoted", "\"binary\"", false),
        ("cte-extra", "binary token", false),
        ("cte-unclosed", "binary (open", false),
        ("cte-unmatched", "binary )", false),
        ("cte-nonascii", "binary (日本語)", false),
        ("cte-bare-cr", "binary (a\rb)", false),
        ("cte-bare-lf", "binary\n (note)", false),
        ("cte-token-merge", "bi(note)nary", false),
    ] {
        let mut row = Probe::new(name);
        row.encoding = format!("Content-Transfer-Encoding: {encoding}\r\n");
        row.accepted = good;
        rows.push(row);
    }
    for (name, part, metadata, good) in [
        (
            "type-case",
            "TEXT/PLAIN; CHARSET=\"utf-8\"",
            "text/plain; charset=UTF-8",
            true,
        ),
        (
            "type-quoted-pair",
            "text/plain; charset=\"utf\\-8\"",
            "text/plain; charset=UTF-8",
            true,
        ),
        (
            "type-comment",
            "text (a (b)) / plain; charset = UTF-8",
            "text/plain; charset=UTF-8",
            true,
        ),
        (
            "type-folded",
            "text/plain;\r\n\tcharset=UTF-8",
            "text/plain; charset=UTF-8",
            true,
        ),
        (
            "type-default",
            "text/plain",
            "text/plain; charset=US-ASCII",
            true,
        ),
        (
            "type-reordered",
            "text/plain; x=One; charset=UTF-8",
            "text/plain; charset=utf-8; x=One",
            true,
        ),
        (
            "type-access",
            "message/external-body; access-type=URL",
            "message/external-body; access-type=url",
            true,
        ),
        (
            "type-wrong-essence",
            "application/octet-stream",
            "text/plain; charset=UTF-8",
            false,
        ),
        (
            "type-wrong-charset",
            "text/plain; charset=US-ASCII",
            "text/plain; charset=UTF-8",
            false,
        ),
        (
            "type-wrong-boundary",
            "multipart/mixed; boundary=inner",
            "multipart/mixed; boundary=INNER",
            false,
        ),
        (
            "type-unclosed-comment",
            "text/plain; charset=UTF-8 (open",
            "text/plain; charset=UTF-8",
            false,
        ),
        (
            "type-nonascii",
            "text/plain (日本語); charset=UTF-8",
            "text/plain; charset=UTF-8",
            false,
        ),
        (
            "type-bare-cr",
            "text/plain (a\rb); charset=UTF-8",
            "text/plain; charset=UTF-8",
            false,
        ),
        (
            "type-extra",
            "text/plain extra; charset=UTF-8",
            "text/plain; charset=UTF-8",
            false,
        ),
        (
            "type-token-merge",
            "te(note)xt/plain; charset=UTF-8",
            "text/plain; charset=UTF-8",
            false,
        ),
    ] {
        let mut row = Probe::new(name);
        row.part = format!("Content-Type: {part}\r\n");
        row.metadata = metadata.into();
        row.raw = b"raw ASCII\r\n--mime-other\r\nembedded--mime-outer\r\n".to_vec();
        row.accepted = good;
        rows.push(row);
    }
    for (name, boundary, encoded, good) in [
        ("boundary-one", "x".into(), "x".into(), true),
        ("boundary-seventy", "a".repeat(70), "a".repeat(70), true),
        (
            "boundary-escaped",
            "good:boundary".into(),
            "\"good\\:boundary\"".into(),
            true,
        ),
        (
            "boundary-punctuation",
            "a'()+_,-./:=? z".into(),
            "\"a'()+_,-./:=? z\"".into(),
            true,
        ),
        (
            "boundary-at",
            "bad@boundary".into(),
            "\"bad@boundary\"".into(),
            false,
        ),
        (
            "boundary-trailing-space",
            "ends ".into(),
            "\"ends \"".into(),
            false,
        ),
        (
            "boundary-seventy-one",
            "a".repeat(71),
            "a".repeat(71),
            false,
        ),
        ("boundary-empty", String::new(), "\"\"".into(), false),
        (
            "boundary-case",
            "mime-OUTER".into(),
            "mime-outer".into(),
            false,
        ),
    ] {
        let mut row = Probe::new(name);
        row.boundary = boundary;
        row.outer = format!("Multipart/Mixed; BOUNDARY={encoded}");
        row.accepted = good;
        rows.push(row);
    }
    let mut row = Probe::new("input-cte-absent-advisory");
    row.encoding.clear();
    rows.push(row);
    let mut row = Probe::new("input-type-absent-advisory");
    row.part.clear();
    rows.push(row);
    let mut row = Probe::new("input-type-empty-invalid");
    row.part = "Content-Type: \r\n".into();
    row.accepted = false;
    rows.push(row);
    let mut row = Probe::new("nested-raw-outer-interrupt");
    row.raw = b"--inner\r\nContent-Type: application/octet-stream\r\n\r\n\xff\0unclosed inner body"
        .to_vec();
    row.part = "Content-Type: multipart/mixed; boundary=inner\r\n".into();
    row.metadata = "multipart/mixed; boundary=inner".into();
    rows.push(row);
    let mut row = Probe::new("chosen-prefix-collision");
    row.raw = b"\r\n--mime-outer-extra\r\nx".to_vec();
    row.accepted = false;
    rows.push(row);
    rows
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
    for row in probes() {
        for post in [false, true] {
            let id = Uuid::new_v4();
            let mut value = statement(id);
            value["actor"] = json!({"mbox":format!("mailto:mime-{id}@example.org")});
            let digest = crate::core::response::hex_lower(&Sha256::digest(&row.raw));
            value["attachments"] = json!([{"usageType":"https://example.org/mime","display":{"en":"raw"},"contentType":row.metadata,"length":row.raw.len(),"sha2":digest}]);
            let siblings = [Uuid::new_v4(), Uuid::new_v4()];
            let input = if post {
                json!([statement(siblings[0]), value, statement(siblings[1])])
            } else {
                value.clone()
            };
            let mut wire=format!("preamble\r\n--{} \t\r\n{}\r\n{input}\r\n--{}\r\n{}{}X-Experience-API-Hash: {digest}\r\nContent-Length: {}\r\n\r\n",row.boundary,row.first,row.boundary,row.part,row.encoding,row.raw.len()).into_bytes();
            wire.extend_from_slice(&row.raw);
            wire.extend_from_slice(format!("\r\n--{}-- \t\r\nepilogue", row.boundary).as_bytes());
            let before = state
                .xapi
                .statements
                .settled_snapshot()
                .await
                .map_err(|e| format!("{e:?}"))?;
            let request = if post {
                client.post(format!("{base}/statements"))
            } else {
                client.put(format!("{base}/statements?statementId={id}"))
            };
            let reply = receive(request.header("Content-Type", &row.outer).body(wire)).await?;
            let status = reply.status();
            assert_eq!(
                status,
                if row.accepted {
                    if post {
                        StatusCode::OK
                    } else {
                        StatusCode::NO_CONTENT
                    }
                } else {
                    StatusCode::BAD_REQUEST
                },
                "{}/POST={post}",
                row.name
            );
            let _ = reply.bytes().await.map_err(|e| e.to_string())?;
            if !row.accepted {
                assert_eq!(
                    before,
                    state
                        .xapi
                        .statements
                        .settled_snapshot()
                        .await
                        .map_err(|e| format!("{e:?}"))?,
                    "settled {}",
                    row.name
                );
                for absent in std::iter::once(id).chain(siblings.into_iter().filter(|_| post)) {
                    assert_eq!(
                        receive(client.get(format!("{base}/statements?statementId={absent}")))
                            .await?
                            .status(),
                        StatusCode::NOT_FOUND
                    );
                }
                continue;
            }
            for result in [false, true] {
                let request = if result {
                    client.get(format!("{base}/statements")).query(&[
                        ("agent", value["actor"].to_string()),
                        ("attachments", "true".into()),
                    ])
                } else {
                    client.get(format!(
                        "{base}/statements?statementId={id}&attachments=true"
                    ))
                };
                let reply = receive(request).await?;
                assert_eq!(reply.status(), StatusCode::OK);
                let parts = attachment_boundaries::decoded(reply).await?;
                let part = parts
                    .iter()
                    .skip(1)
                    .find(|(h, _)| h.contains(&digest))
                    .ok_or("returned digest")?;
                assert_eq!(part.1, row.raw, "{}/result={result}", row.name);
                assert!(part.0.contains("Content-Transfer-Encoding: binary"));
            }
            for choice in ["", "&attachments=false"] {
                let reply =
                    receive(client.get(format!("{base}/statements?statementId={id}{choice}")))
                        .await?;
                assert_eq!(reply.status(), StatusCode::OK);
                assert!(
                    reply.headers()["content-type"]
                        .to_str()
                        .map_err(|e| e.to_string())?
                        .starts_with("application/json")
                );
                let exact: Value = reply.json().await.map_err(|e| e.to_string())?;
                assert_eq!(exact["attachments"], value["attachments"]);
                assert!(exact["attachments"][0].get("fileUrl").is_none());
            }
            evidence::observation(
                "mime-receiver-role",
                &json!({"probe":row.name,"post":post,"status":status.as_u16(),"input_missing_cte":row.encoding.is_empty(),"input_missing_type":row.part.is_empty(),"output_binary_header":true,"raw_hex":crate::core::response::hex_lower(&row.raw)}),
            );
        }
    }
    verify_contract(state, &client, &base).await?;
    server.abort();
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn verify_contract(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
) -> Result<(), String> {
    // Independently evaluated with Python hashlib; B is also the published
    // SHA-256 abc known answer. These exact A/B bytes are the issued fixture.
    let data: [&[u8]; 2] = [b"\x00\xff\r\n", b"abc"];
    let hashes = [
        "e9489f37fb3051e9efa1dc916004d7274e7b63975e3209708947267f2393a9be",
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    ];
    for (raw, hash) in data.iter().zip(hashes) {
        assert_eq!(crate::core::response::hex_lower(&Sha256::digest(raw)), hash);
    }
    for method in ["put", "post-single", "post-batch"] {
        for scenario in [
            "json-none",
            "json-file",
            "multipart-none",
            "multipart-file",
            "raw-forward",
            "raw-reverse",
            "raw-duplicate",
            "missing-middle",
            "missing-hash",
            "wrong-hash",
            "wrong-bytes",
            "wrong-length",
            "wrong-part-length",
            "wrong-type",
            "duplicate-wrong-type",
        ] {
            let rejected = scenario.starts_with("missing-")
                || scenario.starts_with("wrong-")
                || scenario == "duplicate-wrong-type";
            let raw_mode = !scenario.starts_with("json-") && !scenario.starts_with("multipart-");
            let ids = (0..if method == "post-batch" { 3 } else { 1 })
                .map(|_| Uuid::new_v4())
                .collect::<Vec<_>>();
            let marker = format!("https://example.org/mime/{}", ids[0]);
            let metadata = json!([{"usageType":"https://example.org/mime/A","display":{"en":"A"},"contentType":"application/octet-stream","length":4,"sha2":hashes[0]},
                {"usageType":"https://example.org/mime/B","display":{"en":"B"},"contentType":"application/octet-stream","length":3,"sha2":hashes[1]}]);
            let mut values = ids
                .iter()
                .map(|id| {
                    let mut value = statement(*id);
                    value["verb"]["id"] = json!(marker);
                    if !scenario.ends_with("-none") {
                        value["attachments"] = metadata.clone();
                        if scenario.ends_with("-file") {
                            for (n, item) in value["attachments"]
                                .as_array_mut()
                                .expect("fixed array")
                                .iter_mut()
                                .enumerate()
                            {
                                item["fileUrl"] =
                                    json!(format!("https://unresolvable.invalid/file/{n}"));
                            }
                        }
                    }
                    value
                })
                .collect::<Vec<_>>();
            let changed = usize::from(method == "post-batch");
            if scenario == "missing-middle" {
                values[changed]["attachments"][1]["sha2"] = json!("0".repeat(64));
            } else if scenario == "wrong-length" {
                values[changed]["attachments"][1]["length"] = json!(4);
            }
            let input = if method == "post-batch" {
                json!(values)
            } else {
                values[0].clone()
            };
            let request = if method == "put" {
                client.put(format!("{base}/statements?statementId={}", ids[0]))
            } else {
                client.post(format!("{base}/statements"))
            };
            let request = if scenario.starts_with("json-") {
                request.json(&input)
            } else {
                let mut wire =
                    format!("--contract\r\nContent-Type: application/json\r\n\r\n{input}\r\n")
                        .into_bytes();
                let order = if !raw_mode {
                    vec![]
                } else if scenario == "raw-reverse" {
                    vec![1, 0]
                } else if scenario == "raw-duplicate" || scenario == "duplicate-wrong-type" {
                    vec![0, 0, 1]
                } else {
                    vec![0, 1]
                };
                for (at, n) in order.into_iter().enumerate() {
                    let media = if scenario == "wrong-type" && n == 1
                        || scenario == "duplicate-wrong-type" && at == 0
                    {
                        "text/plain"
                    } else {
                        "application/octet-stream"
                    };
                    wire.extend_from_slice(format!("--contract\r\nContent-Type: {media}\r\nContent-Transfer-Encoding: binary\r\n").as_bytes());
                    if !(scenario == "missing-hash" && n == 1) {
                        let hash = if scenario == "wrong-hash" && n == 1 {
                            "0".repeat(64)
                        } else {
                            hashes[n].into()
                        };
                        wire.extend_from_slice(
                            format!("X-Experience-API-Hash: {hash}\r\n").as_bytes(),
                        );
                    }
                    let length = if scenario == "wrong-part-length" && n == 1 {
                        4
                    } else {
                        data[n].len()
                    };
                    wire.extend_from_slice(format!("Content-Length: {length}\r\n\r\n").as_bytes());
                    wire.extend_from_slice(if scenario == "wrong-bytes" && n == 1 {
                        b"abd"
                    } else {
                        data[n]
                    });
                    wire.extend_from_slice(b"\r\n");
                }
                wire.extend_from_slice(b"--contract--\r\n");
                request
                    .header("Content-Type", "multipart/mixed; boundary=contract")
                    .body(wire)
            };
            let before = state
                .xapi
                .statements
                .settled_snapshot()
                .await
                .map_err(|e| format!("{e:?}"))?;
            let reply = receive(request).await?;
            assert_eq!(
                reply.status(),
                if rejected {
                    StatusCode::BAD_REQUEST
                } else if method == "put" {
                    StatusCode::NO_CONTENT
                } else {
                    StatusCode::OK
                },
                "{method}/{scenario}"
            );
            let _ = reply.bytes().await.map_err(|e| e.to_string())?;
            if rejected {
                let after = state
                    .xapi
                    .statements
                    .settled_snapshot()
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                assert_eq!(
                    before, after,
                    "complete committed state after inline handler + writer barrier: {method}/{scenario}"
                );
                for id in &ids {
                    assert_eq!(
                        receive(client.get(format!("{base}/statements?statementId={id}")))
                            .await?
                            .status(),
                        StatusCode::NOT_FOUND
                    );
                }
                continue;
            }
            for id in ids.iter().map(Some).chain(std::iter::once(None)) {
                for include in [None, Some(false), Some(true)] {
                    let query = id.map_or_else(
                        || vec![("verb", marker.clone())],
                        |id| vec![("statementId", id.to_string())],
                    );
                    let mut request = client.get(format!("{base}/statements")).query(&query);
                    if let Some(include) = include {
                        request = request.query(&[("attachments", include)]);
                    }
                    let reply = receive(request).await?;
                    assert_eq!(reply.status(), StatusCode::OK);
                    let returned: Value = if include == Some(true) {
                        let parts = attachment_boundaries::decoded(reply).await?;
                        let mut seen = std::collections::BTreeSet::new();
                        for (headers, bytes) in &parts[1..] {
                            let hash = headers
                                .lines()
                                .find_map(|line| line.strip_prefix("X-Experience-API-Hash: "))
                                .ok_or("output hash")?;
                            let n = hashes
                                .iter()
                                .position(|h| *h == hash)
                                .ok_or("unexpected output hash")?;
                            assert_eq!(bytes.as_slice(), data[n]);
                            assert!(headers.contains("Content-Transfer-Encoding: binary"));
                            seen.insert(n);
                        }
                        assert_eq!(
                            seen,
                            if raw_mode {
                                std::collections::BTreeSet::from([0, 1])
                            } else {
                                std::collections::BTreeSet::new()
                            }
                        );
                        evidence::observation(
                            "mime-shared-copy-advisory",
                            &json!({"method":method,"scenario":scenario,"result":id.is_none(),"raw_parts":parts.len()-1,"distinct_hashes":seen.len(),"metadata_order":"D-09 preservation is checked separately"}),
                        );
                        serde_json::from_slice(&parts[0].1).map_err(|e| e.to_string())?
                    } else {
                        assert!(
                            reply.headers()["content-type"]
                                .to_str()
                                .map_err(|e| e.to_string())?
                                .starts_with("application/json")
                        );
                        reply.json().await.map_err(|e| e.to_string())?
                    };
                    let items = if id.is_some() {
                        vec![&returned]
                    } else {
                        returned["statements"]
                            .as_array()
                            .ok_or("StatementResult")?
                            .iter()
                            .collect()
                    };
                    assert_eq!(items.len(), if id.is_some() { 1 } else { ids.len() });
                    let mut returned_ids = std::collections::BTreeSet::new();
                    for item in items {
                        let stored_id = Uuid::parse_str(item["id"].as_str().ok_or("id")?)
                            .map_err(|e| e.to_string())?;
                        let n = ids
                            .iter()
                            .position(|v| *v == stored_id)
                            .ok_or("unexpected Statement")?;
                        assert!(returned_ids.insert(stored_id));
                        assert_eq!(item.get("attachments"), values[n].get("attachments"));
                    }
                }
            }
        }
    }
    Ok(())
}
