//! Pinned IDNA2003 calling profile. Only the RFC3492 Punycode primitive is
//! delegated to `idna`; its IDNA2008/UTS46 mapping APIs are never called.

#[cfg(not(feature = "std"))]
use alloc::{format, string::{String, ToString}, vec::Vec};

/// RFC3490 ToASCII label with AllowUnassigned=false and explicit STD3 flag.
/// The flag remains explicit for RFC5280 §7.3's distinct stored/query scope.
pub fn certificate_label_to_ascii(label: &str, std3: bool) -> Option<String> {
    let prepared = if label.is_ascii() { label.to_string() }
        else { super::rfc4518::nameprep(label)? };
    if std3 && (prepared.starts_with('-') || prepared.ends_with('-')
        || prepared.bytes().any(|b| b.is_ascii() && !(b.is_ascii_alphanumeric() || b == b'-'))) {
        return None;
    }
    let encoded = if prepared.is_ascii() { prepared } else {
        if prepared.get(..4).is_some_and(|prefix| prefix.eq_ignore_ascii_case("xn--")) { return None; }
        format!("xn--{}", idna::punycode::encode_str(&prepared)?)
    };
    (1..=63).contains(&encoded.len()).then_some(encoded)
}

/// RFC5280 §7.2 stored DNS name, including the three IDNA2003 dot variants.
pub fn certificate_dns_to_ascii(name: &str) -> Option<String> {
    let dotted = name.replace(['\u{3002}', '\u{ff0e}', '\u{ff61}'], ".");
    let (body, root) = dotted.strip_suffix('.').map_or((dotted.as_str(), ""), |s| (s, "."));
    let labels = body.split('.').map(|s| certificate_label_to_ascii(s, true)).collect::<Option<Vec<_>>>()?;
    Some(format!("{}{root}", labels.join(".")))
}

/// Certificate producer mapping: NFC, then RFC3987 UTF-8 percent encoding.
/// IDNA conversion is deliberately deferred to URI comparison, as §7.4 says.
pub fn certificate_iri_to_uri(value: &str) -> Option<String> {
    iri_string::types::IriStr::new(value).ok()?;
    let mut output = String::new();
    for byte in super::rfc4518::nfc(value).bytes() {
        if byte.is_ascii() { output.push(byte as char); }
        else { output.push_str(&format!("%{byte:02X}")); }
    }
    Some(output)
}
