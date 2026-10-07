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
    Some(format!("account:{home_page}:{name}"))
}
