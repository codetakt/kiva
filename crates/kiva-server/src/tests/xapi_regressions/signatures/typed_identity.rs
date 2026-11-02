//! UUID and digest spellings change representation, not typed identity.
#![allow(clippy::wildcard_imports)]
use super::*;

#[allow(clippy::too_many_lines)]
pub(super) async fn verify(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
) -> Result<(), String> {
    for nested in [false, true] {
        for role in ["reference", "actor", "object", "member", "group"] {
            let original = if role == "reference" {
                "abcdef01-abcd-4abc-8abc-abcdef012345"
            } else {
                "abcdef0123456789abcdef0123456789abcdef01"
            };
            let mixed = original
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if i % 2 == 0 {
                        c.to_ascii_uppercase()
                    } else {
                        c
                    }
                })
                .collect::<String>();
            let different = format!("{}4", &original[..original.len() - 1]);
            for received_hash in [original.to_owned(), original.to_ascii_uppercase()] {
                for signed_hash in [
                    original.to_owned(),
                    original.to_ascii_uppercase(),
                    mixed.clone(),
                    different.clone(),
                ] {
                    let equivalent = signed_hash != different;
                    for batch in [false, true] {
                        let id = Uuid::new_v4();
                        let mut received = statement(id);
                        let pointer = match role {
                            "reference" => {
                                received["object"] =
                                    json!({"objectType":"StatementRef","id":received_hash});
                                "/object/id"
                            }
                            "object" => {
                                received["object"] =
                                    json!({"objectType":"Agent","mbox_sha1sum":received_hash});
                                "/object/mbox_sha1sum"
                            }
                            "member" => {
                                received["actor"] = json!({"objectType":"Group","member":[{"mbox_sha1sum":received_hash}]});
                                "/actor/member/0/mbox_sha1sum"
                            }
                            "group" => {
                                received["actor"] =
                                    json!({"objectType":"Group","mbox_sha1sum":received_hash});
                                "/actor/mbox_sha1sum"
                            }
                            _ => {
                                received["actor"] = json!({"mbox_sha1sum":received_hash});
                                "/actor/mbox_sha1sum"
                            }
                        };
                        let pointer = if nested {
                            received.as_object_mut().ok_or("statement")?.remove("id");
                            received["objectType"] = json!("SubStatement");
                            let mut root = statement(id);
                            root["object"] = received;
                            received = root;
                            format!("/object{pointer}")
                        } else {
                            pointer.to_owned()
                        };
                        let mut payload = received.clone();
                        *payload.pointer_mut(&pointer).ok_or("identity")? = json!(signed_hash);
                        let jws = sign_payload(&payload)?;
                        let reply =
                            send_comparison(client, base, received, &payload, batch, equivalent)
                                .await?;
                        assert_eq!(
                            reply.status(),
                            if equivalent {
                                if batch {
                                    StatusCode::OK
                                } else {
                                    StatusCode::NO_CONTENT
                                }
                            } else {
                                StatusCode::BAD_REQUEST
                            },
                            "{role}/{nested}/{received_hash}/{signed_hash}"
                        );
                        let saved = state
                            .xapi
                            .statements
                            .get(id)
                            .await
                            .map_err(|e| format!("{e:?}"))?;
                        assert_eq!(saved.is_some(), equivalent);
                        if let Some(saved) = saved {
                            assert_eq!(
                                saved.statement.pointer(&pointer),
                                Some(&json!(received_hash))
                            );
                            assert_eq!(
                                saved.attachments.values().next().ok_or("signature")?.body,
                                jws.as_bytes()
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
