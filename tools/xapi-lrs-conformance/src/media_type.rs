//! RFC 2045 field-value comparison used by the test oracle (DC-MIME).
//! Comments, quoted pairs, parameter order and type/name case are syntax,
//! whereas parameter values are case-sensitive unless their definition says otherwise.

#![expect(
    clippy::redundant_pub_crate,
    reason = "MIME helpers are shared by sibling oracles and are not a public crate API"
)]

use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

/// Parse an unfolded HTTP media type (RFC 2616 sections 2.2 and 3.7).
/// LWS is permitted around semicolons, but not around '/' or '='. HTTP
/// quoted-string permits empty values and quoted-pairs; mime's parser does not.
pub(super) fn parse_http(text: &str) -> Result<MediaType, String> {
    if text.contains(['\r', '\n']) {
        return Err("unremoved HTTP field folding".into());
    }
    let bytes = text.as_bytes();
    let mut pos = 0;
    whitespace(bytes, &mut pos);
    let kind = read_http_token(bytes, &mut pos)?.to_ascii_lowercase();
    if bytes.get(pos) != Some(&b'/') {
        return Err("missing HTTP media subtype separator".into());
    }
    pos += 1;
    let subtype = read_http_token(bytes, &mut pos)?.to_ascii_lowercase();
    let mut parameters = BTreeMap::new();
    let mut ambiguous = BTreeSet::new();
    loop {
        whitespace(bytes, &mut pos);
        if pos == bytes.len() {
            break;
        }
        if bytes[pos] != b';' {
            return Err("invalid HTTP media parameter separator".into());
        }
        pos += 1;
        whitespace(bytes, &mut pos);
        let name = read_http_token(bytes, &mut pos)?.to_ascii_lowercase();
        if bytes.get(pos) != Some(&b'=') {
            return Err("missing HTTP media parameter value".into());
        }
        pos += 1;
        let mut value = read_http_value(bytes, &mut pos)?;
        if name == "charset" {
            value.make_ascii_lowercase();
        }
        match parameters.entry(name) {
            Entry::Occupied(entry) => {
                ambiguous.insert(entry.key().clone());
            }
            Entry::Vacant(entry) => {
                entry.insert(value);
            }
        }
    }
    Ok(MediaType {
        essence: format!("{kind}/{subtype}"),
        parameters,
        ambiguous,
    })
}

fn read_http_token(bytes: &[u8], pos: &mut usize) -> Result<String, String> {
    let start = *pos;
    while bytes
        .get(*pos)
        .is_some_and(|b| token(*b) && !b"{}".contains(b))
    {
        *pos += 1;
    }
    if start == *pos {
        return Err("missing HTTP media token".into());
    }
    Ok(bytes[start..*pos].iter().map(|b| char::from(*b)).collect())
}

fn read_http_value(bytes: &[u8], pos: &mut usize) -> Result<String, String> {
    if bytes.get(*pos) != Some(&b'"') {
        return read_http_token(bytes, pos);
    }
    *pos += 1;
    let mut value = Vec::new();
    loop {
        let byte = *bytes.get(*pos).ok_or("unterminated HTTP quoted-string")?;
        *pos += 1;
        match byte {
            b'"' => return String::from_utf8(value).map_err(|e| e.to_string()),
            b'\\' => {
                let escaped = *bytes.get(*pos).ok_or("unterminated HTTP quoted-pair")?;
                *pos += 1;
                if !escaped.is_ascii() {
                    return Err("HTTP quoted-pair requires CHAR".into());
                }
                value.push(escaped);
            }
            b'\t' | b' '..=b'~' | 128..=255 => value.push(byte),
            _ => return Err("invalid HTTP quoted-string byte".into()),
        }
    }
}

#[derive(Debug)]
pub(super) struct MediaType {
    pub essence: String,
    pub parameters: BTreeMap<String, String>,
    // Presence remains determinate; a value with this name must not be used.
    pub ambiguous: BTreeSet<String>,
}

impl MediaType {
    pub(super) fn unavailable(&self) -> Option<String> {
        (!self.ambiguous.is_empty()).then(|| {
            "ORACLE-UNAVAILABLE: duplicate MIME parameter requires a specific disposition".into()
        })
    }
}

fn token(byte: u8) -> bool {
    byte > 32 && byte < 127 && !b"()<>@,;:\\\"/[]?=".contains(&byte)
}

fn without_comments(text: &str) -> Result<Vec<u8>, String> {
    // Multipart readers unfold CRLF + LWSP before parsing field values.
    // RFC 822 section 3.3 permits every ASCII CHAR in quoted-pair. Qtext and
    // ctext exclude their delimiters, backslash and CR, but otherwise permit
    // controls too; their control characters are not restricted to HTAB.
    if !text.is_ascii() || text.contains("\r\n") {
        return Err("invalid MIME field octet or unremoved folding".into());
    }
    let mut output = Vec::new();
    let mut depth = 0_usize;
    let mut quoted = false;
    let mut escaped = false;
    for byte in text.bytes() {
        if escaped {
            if depth == 0 {
                output.push(byte);
            }
            escaped = false;
        } else if byte == b'\r' {
            return Err("unquoted CR in MIME field".into());
        } else if byte == b'\\' && (quoted || depth > 0) {
            if depth == 0 {
                output.push(byte);
            }
            escaped = true;
        } else if depth > 0 {
            match byte {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
        } else if byte == b'"' {
            quoted = !quoted;
            output.push(byte);
        } else if byte == b'(' && !quoted {
            depth = 1;
            output.push(b' ');
        } else {
            output.push(byte);
        }
    }
    if depth > 0 || quoted || escaped {
        return Err("unterminated MIME comment/quoted-string".into());
    }
    Ok(output)
}

fn whitespace(bytes: &[u8], pos: &mut usize) {
    while bytes.get(*pos).is_some_and(|b| matches!(b, b' ' | b'\t')) {
        *pos += 1;
    }
}

fn read_token(bytes: &[u8], pos: &mut usize) -> Result<String, String> {
    whitespace(bytes, pos);
    let start = *pos;
    while bytes.get(*pos).is_some_and(|b| token(*b)) {
        *pos += 1;
    }
    if start == *pos {
        return Err("missing MIME token".into());
    }
    Ok(bytes[start..*pos].iter().map(|b| char::from(*b)).collect())
}

pub(super) fn parse(text: &str) -> Result<MediaType, String> {
    let bytes = without_comments(text)?;
    let mut pos = 0;
    let kind = read_token(&bytes, &mut pos)?.to_ascii_lowercase();
    whitespace(&bytes, &mut pos);
    if bytes.get(pos) != Some(&b'/') {
        return Err("missing MIME subtype separator".into());
    }
    pos += 1;
    let subtype = read_token(&bytes, &mut pos)?.to_ascii_lowercase();
    let mut parameters = BTreeMap::new();
    let mut ambiguous = BTreeSet::new();
    loop {
        whitespace(&bytes, &mut pos);
        if pos == bytes.len() {
            break;
        }
        if bytes[pos] != b';' {
            return Err("invalid MIME parameter separator".into());
        }
        pos += 1;
        let name = read_token(&bytes, &mut pos)?.to_ascii_lowercase();
        whitespace(&bytes, &mut pos);
        if bytes.get(pos) != Some(&b'=') {
            return Err("missing MIME parameter value".into());
        }
        pos += 1;
        whitespace(&bytes, &mut pos);
        let mut value = if bytes.get(pos) == Some(&b'"') {
            pos += 1;
            let mut value = String::new();
            loop {
                let byte = *bytes.get(pos).ok_or("unterminated MIME value")?;
                pos += 1;
                if byte == b'"' {
                    break;
                }
                let byte = if byte == b'\\' {
                    let escaped = *bytes.get(pos).ok_or("unterminated quoted-pair")?;
                    pos += 1;
                    escaped
                } else {
                    byte
                };
                // without_comments validated ASCII and excluded unquoted CR;
                // do not apply token's CTL restriction to qtext/quoted-pair.
                value.push(char::from(byte));
            }
            value
        } else {
            read_token(&bytes, &mut pos)?
        };
        // Apply only the recognized subtype/parameter comparison rules.
        // Unsupported parameter semantics retain their spelling for the
        // comparator's unavailable outcome, not arbitrary case-sensitive FAIL.
        if name == "charset" && kind == "text"
            || name == "access-type" && kind == "message" && subtype == "external-body"
        {
            value.make_ascii_lowercase();
        }
        match parameters.entry(name) {
            Entry::Occupied(entry) => {
                ambiguous.insert(entry.key().clone());
            }
            Entry::Vacant(entry) => {
                entry.insert(value);
            }
        }
    }
    Ok(MediaType {
        essence: format!("{kind}/{subtype}"),
        parameters,
        ambiguous,
    })
}

pub(super) fn equivalent(left: &str, right: &str) -> Result<bool, String> {
    let mut left = parse(left)?;
    let mut right = parse(right)?;
    if left.essence != right.essence {
        return Ok(false);
    }
    // RFC 2046 specifies this default for text/plain. Do not invent defaults
    // for arbitrary media types or copy it into extension parameter semantics.
    if left.essence == "text/plain" {
        left.parameters
            .entry("charset".into())
            .or_insert_with(|| "us-ascii".into());
        right
            .parameters
            .entry("charset".into())
            .or_insert_with(|| "us-ascii".into());
    }
    let names: BTreeSet<_> = left
        .parameters
        .keys()
        .chain(right.parameters.keys())
        .collect();
    let mut unknown = left.unavailable().or_else(|| right.unavailable());
    for name in names {
        if left.ambiguous.contains(name) || right.ambiguous.contains(name) {
            continue;
        }
        match (left.parameters.get(name), right.parameters.get(name)) {
            (Some(a), Some(b)) if a != b => {
                if name == "charset" && left.essence.starts_with("text/")
                    || name == "boundary" && left.essence.starts_with("multipart/")
                    || name == "access-type" && left.essence == "message/external-body"
                {
                    return Ok(false);
                }
                unknown.get_or_insert_with(|| {
                    "ORACLE-UNAVAILABLE: subtype parameter recognition/comparison needs its contract".into()
                });
            }
            (Some(_), None) | (None, Some(_)) if name == "charset" || name == "boundary" => {
                // For non-text/plain charset, a default needs the subtype's
                // contract; absence is not generally equivalent or different.
                if name == "boundary" && left.essence.starts_with("multipart/") {
                    return Ok(false);
                }
                unknown.get_or_insert_with(|| {
                    "ORACLE-UNAVAILABLE: media subtype parameter default needs its contract".into()
                });
            }
            (Some(_), None) | (None, Some(_)) => {
                unknown.get_or_insert_with(|| {
                    "ORACLE-UNAVAILABLE: extension MIME parameter presence needs its contract"
                        .into()
                });
            }
            _ => {}
        }
    }
    unknown.map_or(Ok(true), Err)
}

/// RFC 2045 CTE is one structured mechanism token, with ignorable comments.
/// Header folding has already been unfolded by the multipart field reader.
pub(super) fn transfer_encoding(text: &str) -> Result<String, String> {
    let bytes = without_comments(text)?;
    let mut pos = 0;
    let mechanism = read_token(&bytes, &mut pos)?;
    whitespace(&bytes, &mut pos);
    if pos != bytes.len() {
        return Err("invalid MIME transfer-encoding field".into());
    }
    Ok(mechanism)
}
