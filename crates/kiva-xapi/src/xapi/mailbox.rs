//! Receiver syntax for an xAPI mailbox. No normalization, lookup or delivery.

pub(super) fn valid_mailbox(input: &str) -> bool {
    let Ok(iri) = iri_string::types::IriStr::new(input) else {
        return false;
    };
    if !iri.scheme_str().eq_ignore_ascii_case("mailto") || iri.fragment_str().is_some() {
        return false;
    }
    let Some((_, rest)) = input.split_once(':') else {
        return false;
    };
    let (recipients, query) = rest
        .split_once('?')
        .map_or((rest, None), |(r, q)| (r, Some(q)));
    if recipients.is_empty() {
        return false;
    }
    for recipient in recipients.split(',') {
        let Some((local, domain)) = recipient.rsplit_once('@') else {
            return false;
        };
        let (Some(local), Some(domain)) = (
            decode_component(local, false),
            decode_component(domain, false),
        ) else {
            return false;
        };
        if !local_valid(&local) || !domain_valid(&domain) {
            return false;
        }
    }
    query.is_none_or(|query| {
        query.split('&').all(|field| {
            field.split_once('=').is_some_and(|(name, value)| {
                decode_component(name, true).is_some() && decode_component(value, true).is_some()
            })
        })
    })
}

// Split URI components first. An encoded delimiter is data, and the decoded
// octets are never fed through another percent decoder or URL normalizer.
fn decode_component(value: &str, hfield: bool) -> Option<String> {
    let input = value.as_bytes();
    let mut output = Vec::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        let byte = input[index];
        if byte == b'%' {
            let a = char::from(*input.get(index + 1)?).to_digit(16)?;
            let b = char::from(*input.get(index + 2)?).to_digit(16)?;
            output.push(u8::try_from(a * 16 + b).ok()?);
            index += 3;
        } else {
            let permitted = !byte.is_ascii()
                || byte.is_ascii_alphanumeric()
                || b"-._~!$'()*+,:@".contains(&byte)
                || hfield && byte == b';';
            if !permitted {
                return None;
            }
            output.push(byte);
            index += 1;
        }
    }
    String::from_utf8(output).ok()
}

fn atom_valid(value: &str) -> bool {
    value.split('.').all(|part| {
        !part.is_empty()
            && part.chars().all(|c| {
                !c.is_ascii() || c.is_ascii_alphanumeric() || "!#$%&'*+-/=?^_`{|}~".contains(c)
            })
    })
}

fn local_valid(value: &str) -> bool {
    if !(value.len() >= 2 && value.starts_with('"') && value.ends_with('"')) {
        return atom_valid(value);
    }
    let mut escaped = false;
    for c in value[1..value.len() - 1].chars() {
        if escaped {
            if c != ' ' && !c.is_ascii_graphic() {
                return false;
            }
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' || c.is_ascii_control() {
            return false;
        }
    }
    !escaped
}

fn domain_valid(value: &str) -> bool {
    if value.starts_with('[') && value.ends_with(']') {
        value[1..value.len() - 1]
            .bytes()
            .all(|b| (33..=90).contains(&b) || (94..=126).contains(&b))
    } else {
        atom_valid(value)
    }
}
