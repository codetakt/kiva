#![allow(clippy::wildcard_imports)]

use super::*;

/// Returns the canonical IFI string for an xAPI Actor-like JSON value.
#[must_use]
pub fn canonical_actor_ifi(actor: &Value) -> Option<String> {
    let actor = actor.as_object()?;
    if let Some(mbox) = actor.get("mbox").and_then(Value::as_str) {
        return Some(format!("mbox:{mbox}"));
    }
    if let Some(mbox_sha1sum) = actor.get("mbox_sha1sum").and_then(Value::as_str) {
        return Some(format!("mbox_sha1sum:{mbox_sha1sum}"));
    }
    if let Some(openid) = actor.get("openid").and_then(Value::as_str) {
        return Some(format!("openid:{openid}"));
    }
    actor.get("account").and_then(canonical_account_ifi)
}

fn canonical_account_ifi(account: &Value) -> Option<String> {
    let account = account.as_object()?;
    let home_page = account.get("homePage").and_then(Value::as_str)?;
    let name = account.get("name").and_then(Value::as_str)?;
    // Escape the final component so a ':' in the name cannot be confused
    // with a ':' in homePage. Preserve ordinary existing account keys.
    let name = name.replace('%', "%25").replace(':', "%3A");
    Some(format!("account:{home_page}:{name}"))
}

#[cfg(test)]
mod tests {
    use super::canonical_actor_ifi;
    use serde_json::json;

    #[test]
    fn account_components_cannot_alias_at_the_separator() {
        let left = json!({"account":{"homePage":"https://example.org/a:b","name":"c"}});
        let right = json!({"account":{"homePage":"https://example.org/a","name":"b:c"}});
        let escaped = json!({"account":{"homePage":"https://example.org/a","name":"b%3Ac"}});
        assert_ne!(canonical_actor_ifi(&left), canonical_actor_ifi(&right));
        assert_ne!(canonical_actor_ifi(&right), canonical_actor_ifi(&escaped));
    }
}
