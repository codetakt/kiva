#![allow(clippy::wildcard_imports)]

use super::*;

/// Builds a deterministic xAPI authority Agent.
#[must_use]
pub fn authority_agent(name: &str) -> Value {
    json!({
        "objectType": "Agent",
        "account": {
            "homePage": AUTHORITY_ACCOUNT_HOME_PAGE,
            "name": name,
        }
    })
}
