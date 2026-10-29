//! RFC6068 mailbox syntax after RFC3987 IRI mapping. No delivery or DNS tests.

fn component(value: &str, address: bool) -> Option<String> {
    let mut decoded = Vec::new();
    let mut bytes = value.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let high = char::from(bytes.next()?).to_digit(16)?;
            let low = char::from(bytes.next()?).to_digit(16)?;
            decoded.push(u8::try_from(high * 16 + low).ok()?);
        } else if byte >= 128
            || byte.is_ascii_alphanumeric()
            || b"-._~!$'()*+,:@".contains(&byte)
            || !address && byte == b';'
        {
            // A non-ASCII IRI scalar maps to these UTF8 octets as %HH, then
            // decodes once to itself. Existing %HH is never decoded twice.
            decoded.push(byte);
        } else {
            return None;
        }
    }
    String::from_utf8(decoded).ok()
}

fn dot_atom(value: &str) -> bool {
    value.split('.').all(|atom| {
        !atom.is_empty()
            && atom.chars().all(|c| {
                !c.is_ascii() || c.is_ascii_alphanumeric() || "!#$%&'*+-/=?^_`{|}~".contains(c)
            })
    })
}

fn local_part(value: &str) -> bool {
    if let Some(quoted) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        let mut characters = quoted.chars();
        while let Some(character) = characters.next() {
            if character == '\\' {
                if !characters
                    .next()
                    .is_some_and(|c| c == ' ' || c.is_ascii_graphic())
                {
                    return false;
                }
            } else if character == '"' || character.is_ascii_control() {
                return false;
            }
        }
        true
    } else {
        dot_atom(value)
    }
}

fn domain(value: &str) -> bool {
    value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .map_or_else(
            || dot_atom(value),
            |literal| {
                literal
                    .bytes()
                    .all(|b| (33..=90).contains(&b) || (94..=126).contains(&b))
            },
        )
}

#[allow(clippy::redundant_pub_crate)] // Internal oracle; not an exported library API.
pub(super) fn valid(value: &str) -> bool {
    if !crate::identifier_has_valid_syntax(value, false) {
        return false;
    }
    let Some((scheme, rest)) = value.split_once(':') else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("mailto") || rest.contains('#') {
        return false;
    }
    let (addresses, fields) = rest
        .split_once('?')
        .map_or((rest, None), |(a, h)| (a, Some(h)));
    // The URI production allows no recipient, but an xAPI mbox identifies an address.
    if addresses.is_empty()
        || !addresses.split(',').all(|address| {
            let Some((local, host)) = address.rsplit_once('@') else {
                return false;
            };
            match (component(local, true), component(host, true)) {
                (Some(local), Some(host)) => local_part(&local) && domain(&host),
                _ => false,
            }
        })
    {
        return false;
    }
    fields.is_none_or(|fields| {
        fields.split('&').all(|field| {
            field.split_once('=').is_some_and(|(name, value)| {
                component(name, false).is_some() && component(value, false).is_some()
            })
        })
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn complete_mailto_reference_partitions() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/conformance/fixtures/mailto-contract.json"
        ))
        .expect("fixed mailbox corpus");
        for row in vectors["vectors"].as_array().expect("vector array") {
            assert_eq!(
                super::valid(row["iri"].as_str().expect("IRI")),
                row["valid"].as_bool().expect("verdict"),
                "{row}"
            );
        }
    }
}
