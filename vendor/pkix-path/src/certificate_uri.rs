//! RFC5280 §7.4 comparison view for already encoded certificate URI names.
//! This is separate from xAPI IRI equality and never rewrites signed bytes.

#[cfg(not(feature = "std"))]
use alloc::{format, string::{String, ToString}, vec::Vec};
use iri_string::types::UriStr;

fn encoded_host(host: &str) -> Option<String> {
    if host.starts_with('[') { return Some(host.to_string()); }
    let mut input = host.bytes();
    let mut bytes = Vec::new();
    while let Some(byte) = input.next() {
        bytes.push(if byte == b'%' {
            ((input.next()? as char).to_digit(16)? * 16
                + (input.next()? as char).to_digit(16)?) as u8
        } else { byte });
    }
    // Reserved ASCII escapes remain escapes. Only an actual IDN invokes the
    // Unicode3.2 stored-label profile; no decoding of userinfo/path/query here.
    if bytes.is_ascii() { return Some(host.to_string()); }
    Some(super::certificate_dns_to_ascii(core::str::from_utf8(&bytes).ok()?)?.to_ascii_lowercase())
}

fn ldap_field(value: &str) -> Option<String> {
    let mut bytes = value.bytes();
    let mut output = String::new();
    while let Some(byte) = bytes.next() {
        let byte = if byte == b'%' {
            let high = (bytes.next()? as char).to_digit(16)?;
            let low = (bytes.next()? as char).to_digit(16)?;
            (high * 16 + low) as u8
        } else { byte };
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            output.push(byte as char);
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    Some(output)
}

/// Normalize an ASCII URI using syntax and the four called scheme contracts.
/// An invalid URI returns `None`; no retrieval or resource-identity inference
/// takes place. LDAP defaults have no overriding external context here.
pub fn normalize_certificate_uri(value: &str) -> Option<String> {
    let uri = UriStr::new(value).ok()?;
    uri.ensure_rfc3986_normalizable().ok()?;
    let syntax = uri.normalize().to_string();
    let uri = UriStr::new(&syntax).ok()?;
    let scheme = uri.scheme_str();
    let default_port = match scheme { "http" => Some(80), "https" => Some(443),
        "ftp" => Some(21), "ldap" => Some(389), _ => None };
    let mut output = format!("{scheme}:");
    if let Some(authority) = uri.authority_str() {
        let (userinfo, host_port) = authority.rsplit_once('@')
            .map_or((None, authority), |(user, host)| (Some(user), host));
        let port_separator = if host_port.starts_with('[') {
            let end = host_port.find(']')?;
            host_port[end+1..].starts_with(':').then_some(end+1)
        } else { host_port.rfind(':') };
        let mut normalized_host_port = host_port;
        if let (Some(port_start), Some(default)) = (port_separator, default_port) {
            let port = &host_port[port_start+1..];
            if port.is_empty() || port.parse::<u32>().ok() == Some(default) {
                normalized_host_port = &host_port[..port_start];
            }
        }
        output.push_str("//");
        if let Some(user) = userinfo { output.push_str(user); output.push('@'); }
        let host_end = if normalized_host_port.starts_with('[') {
            normalized_host_port.find(']')? + 1
        } else { normalized_host_port.find(':').unwrap_or(normalized_host_port.len()) };
        output.push_str(&encoded_host(&normalized_host_port[..host_end])?);
        output.push_str(&normalized_host_port[host_end..]);
    }
    let path = uri.path_str();
    output.push_str(if path.is_empty() && uri.authority_str().is_some() && default_port.is_some() {
        "/"
    } else { path });
    if scheme == "ldap" && uri.authority_str().is_some() {
        let fields: Vec<_> = uri.query_str().unwrap_or("").split('?').collect();
        if fields.len() > 4 { return None; }
        for (index, default) in ["*", "base", "(objectClass=*)", ""].iter().enumerate() {
            output.push('?');
            let field = fields.get(index).copied().filter(|s| !s.is_empty()).unwrap_or(default);
            output.push_str(&ldap_field(field)?);
        }
    } else if let Some(query) = uri.query_str() { output.push('?'); output.push_str(query); }
    if let Some(fragment) = uri.fragment_str() { output.push('#'); output.push_str(fragment); }
    Some(output)
}

/// Compare stored certificate URI names, retaining Undefined for invalid input.
pub fn certificate_uris_match(left: &str, right: &str) -> Option<bool> {
    Some(normalize_certificate_uri(left)? == normalize_certificate_uri(right)?)
}

pub(crate) fn dns_host(value: &str) -> Option<String> {
    let normalized = normalize_certificate_uri(value)?;
    let uri = UriStr::new(&normalized).ok()?;
    let authority = uri.authority_str()?;
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
    if host_port.starts_with('[') { return None; }
    let host = host_port.split_once(':').map_or(host_port, |(host, _)| host);
    if host.parse::<core::net::Ipv4Addr>().is_ok() || host.is_empty()
        || host.split('.').any(|label| label.is_empty() || label.len() > 63
            || !label.as_bytes()[0].is_ascii_alphanumeric()
            || !label.as_bytes()[label.len()-1].is_ascii_alphanumeric()
            || !label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')) {
        return None;
    }
    Some(host.to_string())
}
