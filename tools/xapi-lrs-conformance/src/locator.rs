//! Bounded DC-IRL provider using independently specified HTTP access mechanisms.

#![expect(
    clippy::redundant_pub_crate,
    reason = "Provider helpers are internal to the runner"
)]

use iri_string::types::{IriStr, UriStr};

pub(super) fn check(text: &str, pointer: &str) -> Result<(), String> {
    let iri = IriStr::new(text).map_err(|_| format!("{pointer} is not valid IRI syntax"))?;
    // RFC3987 section 3.1 maps non-ASCII characters to UTF-8 percent encodings.
    // Retain the mapped URI; neither URL normalization nor a network response
    // supplies the original identifier or its locator classification.
    let mapped = iri.encode_to_uri().to_string();
    let uri = UriStr::new(&mapped).map_err(|_| format!("{pointer} has no valid URI mapping"))?;
    if uri.scheme_str().eq_ignore_ascii_case("http")
        || uri.scheme_str().eq_ignore_ascii_case("https")
    {
        // RFC9110 sections 4.2.1–4.2.2 define both resource identity and the
        // primary HTTP(S) access mechanism, including a nonempty authority host.
        // Reachability, DNS and a successful retrieval are not requirements.
        return if uri
            .authority_components()
            .is_some_and(|a| !a.host().is_empty())
        {
            Ok(())
        } else {
            Err(format!(
                "{pointer} HTTP(S) locator requires an authority with a nonempty host"
            ))
        };
    }
    Err(format!(
        "ORACLE-UNAVAILABLE: {pointer} requires DC-IRL locator/access-mechanism evidence for this scheme/resource"
    ))
}

#[cfg(test)]
mod tests {
    use super::check;

    #[test]
    fn known_access_mechanisms_and_unknown_names_remain_distinct() {
        for value in [
            "https://example.invalid/absent",
            "HTTP://example.test/",
            "https://例え.example/経路?語=値#節",
        ] {
            assert!(check(value, "").is_ok(), "{value}");
        }
        for value in [
            "https:example.test/path",
            "http:///path",
            "https://:80/path",
            "https://example.test/%ZZ",
        ] {
            assert!(
                check(value, "").is_err_and(|e| !e.starts_with("ORACLE-UNAVAILABLE:")),
                "{value}"
            );
        }
        for value in ["urn:example:name", "custom:resource"] {
            assert!(
                check(value, "").is_err_and(|e| e.starts_with("ORACLE-UNAVAILABLE:")),
                "{value}"
            );
        }
    }
}
