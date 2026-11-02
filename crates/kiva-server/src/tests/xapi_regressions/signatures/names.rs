//! SP-01 Case exclusions apply to human-readable names at typed xAPI locations.
#![allow(clippy::wildcard_imports)]
use super::*;

pub(super) async fn verify(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
) -> Result<(), String> {
    for nested in [false, true] {
        for role in ["actor", "actor-group", "object", "object-group", "member"] {
            // Keep non-ASCII characters unchanged: SP-01 does not prescribe a
            // Unicode casefold algorithm. Received spelling must survive storage.
            for name in ["Alice Example", "Élodie Example"] {
                for variant in [
                    "identical",
                    "upper",
                    "lower",
                    "different-name",
                    "different-ifi",
                ] {
                    for batch in [false, true] {
                        let (received, pointer) = named_statement(role, name, nested)?;
                        let mut payload = received.clone();
                        let actor = payload.pointer_mut(&pointer).ok_or("named actor")?;
                        match variant {
                            "upper" => actor["name"] = json!(name.to_ascii_uppercase()),
                            "lower" => actor["name"] = json!(name.to_ascii_lowercase()),
                            "different-name" => actor["name"] = json!("Another Person"),
                            "different-ifi" => actor["mbox"] = json!("mailto:other@example.org"),
                            _ => {}
                        }
                        if role == "member" {
                            let pointer = if nested {
                                "/object/actor/member"
                            } else {
                                "/actor/member"
                            };
                            let members = payload
                                .pointer_mut(pointer)
                                .and_then(Value::as_array_mut)
                                .ok_or("members")?;
                            members[1]["name"] = json!("SECOND MEMBER");
                            members.reverse();
                        }
                        verify_pair(
                            state,
                            client,
                            base,
                            &received,
                            &payload,
                            batch,
                            !variant.starts_with("different-"),
                        )
                        .await?;
                    }
                }
            }
        }
        // The Case exclusion for names must not leak into IRI spelling or
        // Account names, whose identity is governed by the account provider.
        for (actor, pointer, changed) in [
            (
                json!({"mbox":"mailto:Alice@example.org"}),
                "/mbox",
                "mailto:alice@example.org",
            ),
            (
                json!({"openid":"https://example.org/Alice"}),
                "/openid",
                "https://example.org/alice",
            ),
            (
                json!({"account":{"homePage":"https://example.org/Accounts","name":"Alice"}}),
                "/account/homePage",
                "https://example.org/accounts",
            ),
            (
                json!({"account":{"homePage":"https://example.org/accounts","name":"Alice"}}),
                "/account/name",
                "alice",
            ),
        ] {
            for batch in [false, true] {
                let (mut received, actor_pointer) =
                    named_statement("actor", "Alice Example", nested)?;
                *received.pointer_mut(&actor_pointer).ok_or("actor")? = actor.clone();
                received.pointer_mut(&actor_pointer).ok_or("actor")?["name"] =
                    json!("Alice Example");
                let mut payload = received.clone();
                let target = payload.pointer_mut(&actor_pointer).ok_or("actor")?;
                target["name"] = json!("ALICE EXAMPLE");
                *target.pointer_mut(pointer).ok_or("identity")? = json!(changed);
                verify_pair(state, client, base, &received, &payload, batch, false).await?;
            }
        }
    }
    Ok(())
}

fn named_statement(role: &str, name: &str, nested: bool) -> Result<(Value, String), String> {
    let id = Uuid::new_v4();
    let mut received = statement(id);
    // Supply the nested timestamp so storage has no default to add when we
    // assert exact preservation of the complete received Object below.
    received["timestamp"] = json!("2026-10-29T00:00:00Z");
    let mut actor = json!({"name":name,"mbox":"mailto:named@example.org"});
    if role.ends_with("group") {
        actor["objectType"] = json!("Group");
    }
    let pointer = if role.starts_with("object") {
        if role == "object" {
            actor["objectType"] = json!("Agent");
        }
        received["object"] = actor;
        "/object"
    } else if role == "member" {
        received["actor"] = json!({"objectType":"Group","member":[actor,
            {"name":"Second Member","mbox":"mailto:second@example.org"}]});
        "/actor/member/0"
    } else {
        received["actor"] = actor;
        "/actor"
    };
    if nested {
        received.as_object_mut().ok_or("SubStatement")?.remove("id");
        received["objectType"] = json!("SubStatement");
        let mut root = statement(id);
        root["object"] = received;
        Ok((root, format!("/object{pointer}")))
    } else {
        Ok((received, pointer.to_owned()))
    }
}

async fn verify_pair(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
    received: &Value,
    payload: &Value,
    batch: bool,
    equivalent: bool,
) -> Result<(), String> {
    let jws = sign_payload(payload)?;
    let response =
        send_comparison(client, base, received.clone(), payload, batch, equivalent).await?;
    assert_eq!(
        response.status(),
        if equivalent {
            if batch {
                StatusCode::OK
            } else {
                StatusCode::NO_CONTENT
            }
        } else {
            StatusCode::BAD_REQUEST
        }
    );
    let id = Uuid::parse_str(received["id"].as_str().ok_or("id")?).map_err(|e| e.to_string())?;
    let saved = state
        .xapi
        .statements
        .get(id)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(saved.is_some(), equivalent);
    if let Some(saved) = saved {
        assert_eq!(saved.statement["actor"], received["actor"]);
        assert_eq!(saved.statement["object"], received["object"]);
        assert_eq!(
            saved.attachments.values().next().ok_or("signature")?.body,
            jws.as_bytes()
        );
    }
    Ok(())
}
