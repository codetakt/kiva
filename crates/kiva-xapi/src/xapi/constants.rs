/// Standard LRS resources in the Kiva xAPI surface.
pub const REQUIRED_LRS_RESOURCES: &[&str] = &[
    "statements",
    "agents/profile",
    "activities",
    "activities/profile",
    "activities/state",
    "about",
];

/// Account home page used for deterministic Kiva xAPI authority Agents.
pub const AUTHORITY_ACCOUNT_HOME_PAGE: &str = "https://example.org/kiva/conformance/auth";
