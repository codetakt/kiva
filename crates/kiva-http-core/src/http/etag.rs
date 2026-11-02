/// Matches an `ETag` condition using strong comparison (for writes and `If-Match`).
#[must_use]
pub fn etag_list_matches(value: &str, etag: &str) -> bool {
    matches_condition(value, etag, false)
}

/// Matches an `ETag` condition using weak comparison (for read `If-None-Match`).
#[must_use]
pub fn etag_list_matches_weak(value: &str, etag: &str) -> bool {
    matches_condition(value, etag, true)
}

fn matches_condition(value: &str, etag: &str, allow_weak: bool) -> bool {
    let mut remaining = value.as_bytes();
    skip_whitespace(&mut remaining);
    while matches!(remaining.last(), Some(b' ' | b'\t')) {
        remaining = &remaining[..remaining.len() - 1];
    }
    // RFC 2616 14.24/14.26: '*' is an alternative to the entire tag list,
    // never an element inside a quoted opaque tag or a mixed wildcard/list.
    if matches!(remaining, [b'*']) {
        return true;
    }
    let mut expected = etag.as_bytes();
    let Some(expected_tag) = entity_tag(&mut expected) else {
        return false;
    };
    if !expected.is_empty() {
        return false;
    }

    let mut matched = false;
    loop {
        skip_whitespace(&mut remaining);
        match remaining.first() {
            None => return matched,
            // RFC 2616 #rule permits empty list elements. They never match.
            Some(b',') => {
                remaining = &remaining[1..];
                continue;
            }
            _ => {}
        }
        let Some(candidate) = entity_tag(&mut remaining) else {
            return false;
        };
        matched |= (allow_weak || !candidate.weak && !expected_tag.weak)
            && opaque_equal(candidate.opaque, expected_tag.opaque);
        skip_whitespace(&mut remaining);
        // Validate the complete field, even after finding a matching tag.
        if !matches!(remaining.first(), None | Some(b',')) {
            return false;
        }
    }
}

struct EntityTag<'a> {
    weak: bool,
    opaque: &'a [u8],
}

fn entity_tag<'a>(remaining: &mut &'a [u8]) -> Option<EntityTag<'a>> {
    let weak = matches!(remaining, [b'W', b'/', ..]);
    if weak {
        *remaining = &remaining[2..];
        skip_whitespace(remaining);
    }
    if remaining.first() != Some(&b'"') {
        return None;
    }
    let body = &remaining[1..];
    let mut cursor = 0;
    while let Some(&byte) = body.get(cursor) {
        match byte {
            b'"' => {
                *remaining = &body[cursor + 1..];
                return Some(EntityTag {
                    weak,
                    opaque: &body[..cursor],
                });
            }
            b'\\' => {
                // The adopted RFC 2616 quoted-pair quotes one ASCII octet;
                // escaped quotes cannot terminate this opaque tag.
                cursor += 1;
                if !body.get(cursor)?.is_ascii() {
                    return None;
                }
            }
            b'\t' => {}
            0..=31 | 127 => return None,
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn skip_whitespace(remaining: &mut &[u8]) {
    while matches!(remaining.first(), Some(b' ' | b'\t')) {
        *remaining = &remaining[1..];
    }
}

fn opaque_equal(mut left: &[u8], mut right: &[u8]) -> bool {
    loop {
        match (opaque_octet(&mut left), opaque_octet(&mut right)) {
            (None, None) => return true,
            (Some(a), Some(b)) if a == b => {}
            _ => return false,
        }
    }
}

fn opaque_octet(remaining: &mut &[u8]) -> Option<u8> {
    let (&byte, rest) = remaining.split_first()?;
    *remaining = rest;
    if byte == b'\\' {
        let (&escaped, rest) = remaining.split_first()?;
        *remaining = rest;
        Some(escaped)
    } else {
        Some(byte)
    }
}
