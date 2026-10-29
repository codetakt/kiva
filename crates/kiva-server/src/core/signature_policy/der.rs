//! Canonical DER checks complementary to the provider's typed X.509 decoder.
//! Never rewrite a certificate: signature verification receives its original bytes.

use x509_parser::{extensions::ParsedExtension, prelude::X509Certificate};

use super::{HttpError, invalid};

type Result<T> = std::result::Result<T, HttpError>;

pub(super) fn rsa_spki(input: &[u8]) -> Result<Vec<u8>> {
    use ::der::Decode;
    canonical(input)?;
    let spki = spki::SubjectPublicKeyInfoRef::from_der(input).map_err(super::provider_error)?;
    if spki.algorithm.oid.to_string() != "1.2.840.113549.1.1.1"
        || spki.algorithm.parameters != Some(::der::asn1::AnyRef::NULL)
    {
        return Err(invalid(
            "configured JWS key must be an unrestricted RSA SPKI",
        ));
    }
    let bytes = spki.subject_public_key.as_bytes().ok_or_else(malformed)?;
    canonical(bytes)?;
    let integers = children(single(bytes, 0x30)?)?;
    if integers.len() != 2
        || integers
            .iter()
            .any(|v| v.tag != 2 || v.content[0] & 0x80 != 0 || v.content.iter().all(|b| *b == 0))
    {
        return Err(malformed());
    }
    // RFC 7518 measures the actual modulus, not its octet storage width.
    // ring's lower bound rounds up to bytes and otherwise accepts 2041..2047
    // bit keys in its "2048" verifier. Enforce the JWS calling contract here.
    if num_bigint::BigUint::from_bytes_be(integers[0].content).bits() < 2048 {
        return Err(invalid("JWS RSA modulus must contain at least 2048 bits"));
    }
    Ok(bytes.to_vec())
}

#[derive(Clone, Copy)]
struct Element<'a> {
    tag: u8,
    encoded: &'a [u8],
    content: &'a [u8],
}

fn malformed() -> HttpError {
    invalid("certificate is not canonical typed DER")
}

fn element(input: &[u8]) -> Result<(Element<'_>, &[u8])> {
    let tag = *input.first().ok_or_else(malformed)?;
    let mut position = 1;
    if tag & 31 == 31 {
        loop {
            let byte = *input.get(position).ok_or_else(malformed)?;
            if position == 1 && (byte == 0x80 || byte < 31) {
                return Err(malformed());
            }
            position += 1;
            if byte & 0x80 == 0 {
                break;
            }
        }
    }
    let first_length = *input.get(position).ok_or_else(malformed)?;
    position += 1;
    let length = if first_length & 0x80 == 0 {
        usize::from(first_length)
    } else {
        let count = usize::from(first_length & 0x7f);
        if count == 0 || input.get(position) == Some(&0) {
            return Err(malformed());
        }
        let bytes = input
            .get(position..position + count)
            .ok_or_else(malformed)?;
        let mut length = 0_usize;
        for byte in bytes {
            length = length
                .checked_mul(256)
                .and_then(|n| n.checked_add(usize::from(*byte)))
                .ok_or_else(malformed)?;
        }
        if length < 128 {
            return Err(malformed());
        }
        position += count;
        length
    };
    let end = position.checked_add(length).ok_or_else(malformed)?;
    let encoded = input.get(..end).ok_or_else(malformed)?;
    Ok((
        Element {
            tag,
            encoded,
            content: &encoded[position..],
        },
        &input[end..],
    ))
}

fn single(input: &[u8], tag: u8) -> Result<Element<'_>> {
    let (value, rest) = element(input)?;
    if value.tag != tag || !rest.is_empty() {
        return Err(malformed());
    }
    Ok(value)
}

fn children(value: Element<'_>) -> Result<Vec<Element<'_>>> {
    let mut rest = value.content;
    let mut values = Vec::new();
    while !rest.is_empty() {
        let (value, tail) = element(rest)?;
        values.push(value);
        rest = tail;
    }
    Ok(values)
}

fn wrap(tag: u8, content: &[u8]) -> Result<Vec<u8>> {
    let mut encoded = vec![tag];
    if content.len() < 128 {
        encoded.push(u8::try_from(content.len()).map_err(super::provider_error)?);
    } else {
        let length = content.len().to_be_bytes();
        let start = length
            .iter()
            .position(|byte| *byte != 0)
            .ok_or_else(malformed)?;
        encoded.push(0x80 | u8::try_from(length.len() - start).map_err(super::provider_error)?);
        encoded.extend_from_slice(&length[start..]);
    }
    encoded.extend_from_slice(content);
    Ok(encoded)
}

/// The path library's time type excludes pre-1970 UTC and fractional `GeneralizedTime`.
/// Actual validity is checked independently on the original decimal fields. Only
/// the provider's time fields are replaced; `OriginalVerifier` binds this exact view
/// to the original TBS and verifies the signature on the ORIGINAL octets.
pub(super) fn provider_view(input: &[u8]) -> Result<Vec<u8>> {
    canonical(input)?;
    let outer = children(single(input, 0x30)?)?;
    if outer.len() != 3 || outer[0].tag != 0x30 {
        return Err(malformed());
    }
    let fields = children(outer[0])?;
    let offset = usize::from(fields.first().is_some_and(|field| field.tag == 0xa0));
    if fields.len() < offset + 6 {
        return Err(malformed());
    }
    let mut tbs = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        if index == offset + 3 {
            let mut validity = wrap(23, b"700101000000Z")?;
            validity.extend(wrap(23, b"491231235959Z")?);
            tbs.extend(wrap(0x30, &validity)?);
        } else {
            tbs.extend_from_slice(field.encoded);
        }
    }
    let mut result = wrap(0x30, &tbs)?;
    result.extend_from_slice(outer[1].encoded);
    result.extend_from_slice(outer[2].encoded);
    wrap(0x30, &result)
}

pub(super) fn original_tbs(input: &[u8]) -> Result<&[u8]> {
    let outer = children(single(input, 0x30)?)?;
    Ok(outer
        .first()
        .filter(|value| value.tag == 0x30)
        .ok_or_else(malformed)?
        .encoded)
}

fn compare_time(value: Element<'_>, now: time::OffsetDateTime) -> Result<std::cmp::Ordering> {
    let bytes = value.content;
    if !matches!(value.tag, 23 | 24) || !time_syntax(value.tag, bytes) {
        return Err(malformed());
    }
    let year_end = if value.tag == 23 { 2 } else { 4 };
    let number = |start, end| {
        bytes[start..end]
            .iter()
            .fold(0_u16, |n, b| n * 10 + u16::from(*b - b'0'))
    };
    let mut year = number(0, year_end);
    if value.tag == 23 {
        year += if year >= 50 { 1900 } else { 2000 };
    }
    let part = |start| u8::try_from(number(start, start + 2)).map_err(super::provider_error);
    let date = time::Date::from_calendar_date(
        i32::from(year),
        time::Month::try_from(part(year_end)?).map_err(super::provider_error)?,
        part(year_end + 2)?,
    )
    .map_err(super::provider_error)?;
    let whole = date
        .with_hms(
            part(year_end + 4)?,
            part(year_end + 6)?,
            part(year_end + 8)?,
        )
        .map_err(super::provider_error)?
        .assume_utc();
    let seconds = whole.unix_timestamp().cmp(&now.unix_timestamp());
    if !seconds.is_eq() {
        return Ok(seconds);
    }
    let end = year_end + 10;
    let fraction = if bytes.get(end) == Some(&b'.') {
        &bytes[end + 1..bytes.len() - 1]
    } else {
        &[]
    };
    let clock_fraction = format!("{:09}", now.nanosecond());
    let width = fraction.len().max(clock_fraction.len());
    Ok(fraction
        .iter()
        .copied()
        .chain(std::iter::repeat(b'0'))
        .take(width)
        .cmp(
            clock_fraction
                .bytes()
                .chain(std::iter::repeat(b'0'))
                .take(width),
        ))
}

/// Compare original ASN.1 decimal fractions without rounding to the parser's
/// millisecond precision (or even to the clock's nanosecond resolution).
pub(super) fn valid_at(input: &[u8], now: time::OffsetDateTime) -> Result<bool> {
    let tbs = children(single(original_tbs(input)?, 0x30)?)?;
    let offset = usize::from(tbs.first().is_some_and(|v| v.tag == 0xa0));
    let validity = children(*tbs.get(offset + 3).ok_or_else(malformed)?)?;
    if validity.len() != 2 {
        return Err(malformed());
    }
    Ok(!compare_time(validity[0], now)?.is_gt() && !compare_time(validity[1], now)?.is_lt())
}

fn bit_string(bytes: &[u8]) -> bool {
    let Some((&unused, body)) = bytes.split_first() else {
        return false;
    };
    unused < 8
        && body
            .last()
            .map_or(unused == 0, |last| last & ((1_u8 << unused) - 1) == 0)
}

fn integer(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && (bytes.len() == 1
            || !((bytes[0] == 0 && bytes[1] & 0x80 == 0)
                || (bytes[0] == 0xff && bytes[1] & 0x80 != 0)))
}

fn oid(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let mut start = true;
    for byte in bytes {
        if start && *byte == 0x80 {
            return false;
        }
        start = byte & 0x80 == 0;
    }
    start
}

fn time_syntax(tag: u8, bytes: &[u8]) -> bool {
    let digits = if tag == 23 { 12 } else { 14 };
    if bytes.len() < digits + 1
        || !bytes[..digits].iter().all(u8::is_ascii_digit)
        || bytes.last() != Some(&b'Z')
    {
        return false;
    }
    if bytes.len() != digits + 1
        && (tag != 24
            || bytes[digits] != b'.'
            || bytes.len() < digits + 3
            || !bytes[digits + 1..bytes.len() - 1]
                .iter()
                .all(u8::is_ascii_digit)
            || bytes[bytes.len() - 2] == b'0')
    {
        return false;
    }
    let number = |start: usize, end: usize| {
        bytes[start..end]
            .iter()
            .fold(0_u16, |n, byte| n * 10 + u16::from(byte - b'0'))
    };
    let year_end = if tag == 23 { 2 } else { 4 };
    let mut year = number(0, year_end);
    if tag == 23 {
        year += if year >= 50 { 1900 } else { 2000 };
    }
    let Ok(month) =
        time::Month::try_from(u8::try_from(number(year_end, year_end + 2)).unwrap_or(0))
    else {
        return false;
    };
    time::Date::from_calendar_date(
        i32::from(year),
        month,
        u8::try_from(number(year_end + 2, year_end + 4)).unwrap_or(0),
    )
    .is_ok()
        && number(year_end + 4, year_end + 6) < 24
        && number(year_end + 6, year_end + 8) < 60
        && number(year_end + 8, year_end + 10) <= 60
}

fn primitive(value: Element<'_>) -> bool {
    let bytes = value.content;
    if value.tag & 0xc0 != 0 {
        return true;
    }
    match value.tag {
        1 => matches!(bytes, [0 | 0xff]),
        2 | 10 => integer(bytes),
        3 => bit_string(bytes),
        4 | 20 | 21 | 25 | 27 => true,
        5 => bytes.is_empty(),
        6 | 13 => oid(bytes),
        12 => std::str::from_utf8(bytes).is_ok(),
        18 => bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || *byte == b' '),
        19 => bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b" '()+,-./:=?".contains(byte)),
        22 => bytes.is_ascii(),
        23 | 24 => time_syntax(value.tag, bytes),
        26 => bytes.iter().all(|byte| (0x20..=0x7e).contains(byte)),
        28 => {
            bytes.len().is_multiple_of(4)
                && bytes.chunks_exact(4).all(|chunk| {
                    char::from_u32(u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                        .is_some()
                })
        }
        30 => {
            bytes.len().is_multiple_of(2)
                && bytes.chunks_exact(2).all(|chunk| {
                    !(0xd800..=0xdfff).contains(&u16::from_be_bytes([chunk[0], chunk[1]]))
                })
        }
        _ => false,
    }
}

fn canonical(input: &[u8]) -> Result<()> {
    let (first, rest) = element(input)?;
    if !rest.is_empty() {
        return Err(malformed());
    }
    // Explicit heap stack avoids a certificate-controlled native recursion depth.
    let mut pending = vec![first];
    while let Some(value) = pending.pop() {
        if value.tag & 0x20 != 0 {
            if value.tag & 0xc0 == 0 && !matches!(value.tag, 0x30 | 0x31) {
                return Err(malformed());
            }
            let nodes = children(value)?;
            if value.tag == 0x31
                && nodes
                    .windows(2)
                    .any(|pair| pair[0].encoded > pair[1].encoded)
            {
                return Err(malformed());
            }
            pending.extend(nodes);
        } else if !primitive(value) {
            return Err(malformed());
        }
    }
    Ok(())
}

pub(super) fn certificate(input: &[u8], parsed: &X509Certificate<'_>) -> Result<()> {
    canonical(input)?;
    let outer = children(single(input, 0x30)?)?;
    if outer.len() != 3
        || outer[0].tag != 0x30
        || outer[1].tag != 0x30
        || outer[2].tag != 3
        || outer[2].content.first() != Some(&0)
    {
        return Err(malformed());
    }
    let tbs = children(outer[0])?;
    let offset = usize::from(tbs.first().is_some_and(|element| element.tag == 0xa0));
    if offset == 1 && !matches!(single(tbs[0].content, 2)?.content, [1 | 2]) {
        return Err(malformed());
    }
    let core = tbs.get(offset..offset + 6).ok_or_else(malformed)?;
    if core
        .iter()
        .map(|value| value.tag)
        .ne([2, 0x30, 0x30, 0x30, 0x30, 0x30])
    {
        return Err(malformed());
    }
    let validity = children(core[3])?;
    if validity.len() != 2 || validity.iter().any(|value| !matches!(value.tag, 23 | 24)) {
        return Err(malformed());
    }
    let mut last = 0;
    for field in &tbs[offset + 6..] {
        if !matches!(field.tag, 0x81 | 0x82 | 0xa3) || field.tag <= last {
            return Err(malformed());
        }
        last = field.tag;
        if field.tag == 0xa3 {
            extensions(*field)?;
        } else if !bit_string(field.content) {
            return Err(malformed());
        }
    }
    rsa_algorithm(
        &parsed.public_key().algorithm.algorithm.to_id_string(),
        parsed
            .public_key()
            .algorithm
            .parameters
            .as_ref()
            .map(|parameter| (parameter.tag().0, parameter.data)),
        true,
    )?;
    for algorithm in [
        &parsed.signature_algorithm,
        &parsed.tbs_certificate.signature,
    ] {
        rsa_algorithm(
            &algorithm.algorithm.to_id_string(),
            algorithm
                .parameters
                .as_ref()
                .map(|parameter| (parameter.tag().0, parameter.data)),
            false,
        )?;
    }
    if parsed.public_key().algorithm.algorithm.to_id_string() == "1.2.840.113549.1.1.1" {
        let key = &parsed.public_key().subject_public_key;
        if key.unused_bits != 0 {
            return Err(malformed());
        }
        canonical(&key.data)?;
        let integers = children(single(&key.data, 0x30)?)?;
        if integers.len() != 2
            || integers.iter().any(|value| {
                value.tag != 2
                    || value.content[0] & 0x80 != 0
                    || value.content.iter().all(|byte| *byte == 0)
            })
        {
            return Err(malformed());
        }
    }
    known_extensions(parsed)
}

fn known_extensions(parsed: &X509Certificate<'_>) -> Result<()> {
    for extension in parsed.extensions() {
        match extension.parsed_extension() {
            ParsedExtension::UnsupportedExtension { .. } | ParsedExtension::Unparsed => {}
            ParsedExtension::ParseError { .. } => return Err(malformed()),
            _ => {
                canonical(extension.value)?;
                match extension.oid.to_id_string().as_str() {
                    "2.5.29.19" => {
                        // BasicConstraints cA has DEFAULT FALSE.
                        if children(single(extension.value, 0x30)?)?
                            .first()
                            .is_some_and(|value| value.tag == 1 && value.content == [0])
                        {
                            return Err(malformed());
                        }
                    }
                    "2.5.29.15" => {
                        // Named bit list: trailing zero bits are omitted.
                        let bits = single(extension.value, 3)?.content;
                        if bits.len() > 1
                            && (bits.last() == Some(&0)
                                || bits.last().map(|last| last.trailing_zeros())
                                    != Some(u32::from(bits[0])))
                        {
                            return Err(malformed());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn extensions(field: Element<'_>) -> Result<()> {
    for extension in children(single(field.content, 0x30)?)? {
        if extension.tag != 0x30 {
            return Err(malformed());
        }
        let fields = children(extension)?;
        if !matches!(fields.len(), 2 | 3)
            || fields[0].tag != 6
            || fields.last().is_none_or(|value| value.tag != 4)
            || (fields.len() == 3 && (fields[1].tag != 1 || fields[1].content != [0xff]))
        {
            return Err(malformed());
        }
    }
    Ok(())
}

fn rsa_algorithm(oid: &str, parameters: Option<(u32, &[u8])>, spki: bool) -> Result<()> {
    let rsa_key = spki && oid == "1.2.840.113549.1.1.1";
    let rsa_signature = !spki
        && matches!(
            oid,
            "1.2.840.113549.1.1.11"
                | "1.2.840.113549.1.1.12"
                | "1.2.840.113549.1.1.13"
                | "1.2.840.113549.1.1.14"
        );
    if rsa_key && parameters != Some((5, &[][..]))
        || rsa_signature && parameters.is_some_and(|value| value != (5, &[][..]))
    {
        return Err(malformed());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::{canonical, children, single, time_syntax};
    use base64::Engine;
    use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
    use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};

    fn wrap(tag: u8, bytes: &[u8]) -> Vec<u8> {
        let mut result = vec![tag];
        if bytes.len() < 128 {
            result.push(u8::try_from(bytes.len()).unwrap());
        } else {
            let encoded = bytes.len().to_be_bytes();
            let encoded = &encoded[encoded.iter().position(|byte| *byte != 0).unwrap()..];
            result.push(0x80 | u8::try_from(encoded.len()).unwrap());
            result.extend_from_slice(encoded);
        }
        result.extend_from_slice(bytes);
        result
    }

    // Every negative encoding below is signed over its actual malformed TBS.
    // A passing rejection therefore cannot be explained by stale signature bytes.
    fn signed(tbs: &[u8], algorithm: &[u8]) -> Vec<u8> {
        signed_with_hash(tbs, algorithm, Algorithm::RS256)
    }

    fn signed_with_hash(tbs: &[u8], algorithm: &[u8], hash: Algorithm) -> Vec<u8> {
        let key = EncodingKey::from_rsa_pem(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/signing/synthetic-key.pem"
        )))
        .unwrap();
        let signature = jsonwebtoken::crypto::sign(tbs, &key, hash).unwrap();
        let certificate = STANDARD.encode(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/signing/synthetic-certificate.der"
        )));
        let pem =
            format!("-----BEGIN CERTIFICATE-----\n{certificate}\n-----END CERTIFICATE-----\n");
        let public_key = DecodingKey::from_rsa_pem(pem.as_bytes()).unwrap();
        assert!(jsonwebtoken::crypto::verify(&signature, tbs, &public_key, hash).unwrap());
        let signature = URL_SAFE_NO_PAD.decode(signature).unwrap();
        let mut bits = vec![0];
        bits.extend(signature);
        let mut content = tbs.to_vec();
        content.extend(algorithm);
        content.extend(wrap(3, &bits));
        wrap(0x30, &content)
    }

    #[test]
    fn noncanonical_certificates_fail_even_with_correct_original_byte_signatures() {
        let original = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/signing/synthetic-certificate.der"
        ));
        let outer = children(single(original, 0x30).unwrap()).unwrap();
        let source_fields = children(outer[0]).unwrap();
        assert_eq!(source_fields[0].tag, 0xa0);
        // Build a schema-valid v1 control by omitting the version and v3
        // extensions, then sign it independently before mutating DEFAULT.
        let fields = &source_fields[1..7];
        let base_content = fields
            .iter()
            .flat_map(|field| field.encoded.iter().copied())
            .collect::<Vec<_>>();
        let base_tbs = wrap(0x30, &base_content);
        let parse = |bytes: &[u8]| {
            super::super::parse_certificate(
                &base64::engine::general_purpose::STANDARD.encode(bytes),
            )
        };
        assert!(parse(original).is_ok());
        assert!(parse(&signed(&base_tbs, outer[1].encoded)).is_ok());
        let mut explicit_v1 = vec![0xa0, 3, 2, 1, 0];
        explicit_v1.extend(&base_content);
        assert!(parse(&signed(&wrap(0x30, &explicit_v1), outer[1].encoded)).is_err());
        let mut padded_serial = vec![0];
        padded_serial.extend(fields[0].content);
        let mut changed = wrap(2, &padded_serial);
        for field in fields.iter().skip(1) {
            changed.extend(field.encoded);
        }
        assert!(parse(&signed(&wrap(0x30, &changed), outer[1].encoded)).is_err());
        let mut changed = Vec::new();
        for (index, field) in fields.iter().enumerate() {
            if index == 2 {
                changed.extend(wrap(0x31, field.content));
            } else {
                changed.extend(field.encoded);
            }
        }
        assert!(parse(&signed(&wrap(0x30, &changed), outer[1].encoded)).is_err());
        // Canonical negative INTEGER is a valid encoding, independent of issuer
        // serial-number positivity. Do not misclassify it as malformed DER.
        let mut changed = wrap(2, &[0x80]);
        for field in fields.iter().skip(1) {
            changed.extend(field.encoded);
        }
        assert!(parse(&signed(&wrap(0x30, &changed), outer[1].encoded)).is_ok());
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn canonical_primitives_distinguish_encoding_from_value_domain() {
        for bytes in [
            &[2, 1, 0x7f][..],
            &[2, 2, 0, 0x80],
            &[2, 1, 0x80],
            &[2, 2, 0xff, 0x7f],
            &[1, 1, 0xff],
            &[3, 1, 0],
            &[3, 2, 7, 0x80],
            &[5, 0],
            &[6, 3, 0x88, 0x37, 3],
            &[12, 2, 0xc3, 0xa9],
            &[0x13, 7, b'A', b'l', b'i', b'c', b'e', b'+', b'1'],
        ] {
            assert!(canonical(bytes).is_ok(), "{bytes:?}");
        }
        for bytes in [
            &[2, 0][..],
            &[2, 2, 0, 0x7f],
            &[2, 2, 0xff, 0x80],
            &[1, 1, 1],
            &[3, 2, 8, 0],
            &[3, 2, 1, 1],
            &[5, 1, 0],
            &[0x25, 0],
            &[6, 2, 0x80, 0],
            &[6, 1, 0x80],
            &[12, 3, 0xed, 0xa0, 0x80],
            &[0x13, 1, b'@'],
            &[0x16, 1, 0x80],
            &[0x24, 0],
            &[2, 0x81, 1, 0],
            &[0x30, 0x80, 0, 0],
            &[0x30, 0x82, 0, 0x80],
        ] {
            assert!(canonical(bytes).is_err(), "{bytes:?}");
        }
        assert!(canonical(&[0x31, 6, 2, 1, 1, 2, 1, 2]).is_ok());
        assert!(canonical(&[0x31, 6, 2, 1, 2, 2, 1, 1]).is_err());
        for bytes in [
            &[0x9f, 31, 0][..],
            &[0x9f, 0x81, 0, 0],
            &[0xbf, 31, 3, 2, 1, 0],
            &[10, 1, 0],
            &[1, 1, 0],
            &[6, 1, 0],
            &[6, 1, 39],
            &[6, 1, 40],
            &[6, 1, 79],
            &[6, 1, 80],
            &[6, 2, 0x81, 0],
        ] {
            assert!(canonical(bytes).is_ok(), "generic called type {bytes:?}");
        }
        for bytes in [
            &[0x9f, 30, 0][..],
            &[0x9f, 0x80, 31, 0],
            &[0x9f, 0x81],
            &[0x30, 2, 2, 1, 0],
            &[4, 0x81, 127],
        ] {
            assert!(
                canonical(bytes).is_err(),
                "tag/minimal length/nested scope {bytes:?}"
            );
        }
        for unused in 0..=7 {
            assert!(canonical(&[3, 2, unused, 0xff_u8 << unused]).is_ok());
            if unused != 0 {
                assert!(canonical(&[3, 2, unused, (0xff_u8 << unused) | 1]).is_err());
            }
        }
        for text in [
            "\0",
            "\u{7f}",
            "\u{80}",
            "\u{7ff}",
            "\u{800}",
            "\u{d7ff}",
            "\u{e000}",
            "\u{ffff}",
            "\u{10000}",
            "\u{10ffff}",
        ] {
            assert!(
                canonical(&wrap(12, text.as_bytes())).is_ok(),
                "UTF8 scalar syntax, separate from name profile"
            );
        }
        for bytes in [
            &[0xc0, 0xaf][..],
            &[0xc2],
            &[0xe0, 0x80, 0x80],
            &[0xed, 0xa0, 0x80],
            &[0xf4, 0x90, 0x80, 0x80],
        ] {
            assert!(canonical(&wrap(12, bytes)).is_err());
        }
        for length in [0, 127, 128] {
            let mut bytes = if length < 128 {
                vec![4, u8::try_from(length).unwrap()]
            } else {
                vec![4, 0x81, 0x80]
            };
            bytes.extend(vec![0; length]);
            assert!(canonical(&bytes).is_ok());
            bytes.push(0);
            assert!(canonical(&bytes).is_err());
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn typed_certificate_wrappers_defaults_and_rsa_parameters_are_discriminated() {
        let original = include_bytes!(
            "../../../../../tests/conformance/fixtures/signing/synthetic-certificate.der"
        );
        let outer = children(single(original, 0x30).unwrap()).unwrap();
        let fields = children(outer[0]).unwrap();
        let base = fields[..7]
            .iter()
            .map(|f| f.encoded.to_vec())
            .collect::<Vec<_>>();
        let parse = |fields: &[Vec<u8>]| {
            super::super::parse_certificate(
                &STANDARD.encode(signed(&wrap(0x30, &fields.concat()), outer[1].encoded)),
            )
        };
        assert!(parse(&base).is_ok());
        // Every malformed variant is freshly signed over its own TBS.
        for index in 1..7 {
            let mut changed = base.clone();
            changed.remove(index);
            assert!(parse(&changed).is_err(), "missing mandatory {index}");
        }
        for version in [
            vec![0x80, 1, 2],
            vec![0xa0, 6, 2, 1, 2, 2, 1, 2],
            vec![0xa0, 3, 2, 1, 0],
        ] {
            let mut changed = base.clone();
            changed[0] = version;
            assert!(parse(&changed).is_err());
        }
        let mut v2 = base.clone();
        v2[0] = vec![0xa0, 3, 2, 1, 1];
        assert!(parse(&v2).is_ok());
        for tag in [0x31, 0x70, 0xa3] {
            let mut changed = base.clone();
            changed[3] = wrap(tag, fields[3].content);
            assert!(parse(&changed).is_err());
        }
        let mut extra = base.clone();
        extra.push(vec![5, 0]);
        assert!(parse(&extra).is_err());
        let extension = |oid: &[u8], critical: Option<u8>, inner: &[u8]| {
            let mut body = wrap(6, oid);
            if let Some(value) = critical {
                body.extend(wrap(1, &[value]));
            }
            body.extend(wrap(4, inner));
            wrap(0xa3, &wrap(0x30, &wrap(0x30, &body)))
        };
        for (oid, critical, inner, valid) in [
            (&[0x55, 0x1d, 0x13][..], None, &[0x30, 0][..], true),
            (
                &[0x55, 0x1d, 0x13],
                Some(0xff),
                &[0x30, 3, 1, 1, 0xff],
                true,
            ),
            (&[0x55, 0x1d, 0x13], Some(0), &[0x30, 0], false),
            (&[0x55, 0x1d, 0x13], Some(1), &[0x30, 0], false),
            (&[0x55, 0x1d, 0x13], None, &[0x30, 3, 1, 1, 0], false),
            (&[0x55, 0x1d, 0x13], None, &[2, 1, 1], false),
            (&[0x55, 0x1d, 0x0f], None, &[3, 2, 7, 0x80], true),
            (&[0x55, 0x1d, 0x0f], None, &[3, 2, 0, 0x80], false),
            (&[0x55, 0x1d, 0x0f], None, &[3, 1, 0], true),
            (&[42, 3, 4], None, &[0xff, 0, 0x80], true),
        ] {
            let mut changed = base.clone();
            changed.push(extension(oid, critical, inner));
            assert_eq!(
                parse(&changed).is_ok(),
                valid,
                "extension {oid:?}/{critical:?}/{inner:?}"
            );
        }
        for (last, hash) in [
            (11, Algorithm::RS256),
            (12, Algorithm::RS384),
            (13, Algorithm::RS512),
        ] {
            for null in [false, true] {
                let mut oid = vec![0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 1, last];
                let mut algorithm = wrap(6, &oid);
                if null {
                    algorithm.extend([5, 0]);
                }
                let algorithm = wrap(0x30, &algorithm);
                let mut changed = base.clone();
                changed[2] = algorithm.clone();
                let cert = signed_with_hash(&wrap(0x30, &changed.concat()), &algorithm, hash);
                assert!(
                    super::super::parse_certificate(&STANDARD.encode(cert)).is_ok(),
                    "signature AlgorithmIdentifier NULL/absent {hash:?}/{null}"
                );
                oid[8] = 1;
                let key = children(fields[6]).unwrap();
                for (parameters, valid) in [
                    (vec![5, 0], true),
                    (vec![], false),
                    (vec![5, 1, 0], false),
                    (vec![0x25, 0], false),
                    (vec![4, 0], false),
                ] {
                    let key_algorithm = wrap(0x30, &[wrap(6, &oid), parameters].concat());
                    let spki = wrap(0x30, &[key_algorithm, key[1].encoded.to_vec()].concat());
                    assert_eq!(super::rsa_spki(&spki).is_ok(), valid);
                }
                for restricted in [7, 10] {
                    oid[8] = restricted;
                    let key_algorithm = wrap(0x30, &[wrap(6, &oid), vec![0x30, 0]].concat());
                    let spki = wrap(0x30, &[key_algorithm, key[1].encoded.to_vec()].concat());
                    assert!(
                        super::rsa_spki(&spki).is_err(),
                        "OAEP/PSS key is not an unrestricted RS key"
                    );
                }
            }
        }
    }

    #[test]
    fn canonical_time_keeps_der_and_issuer_profile_separate() {
        for (tag, text) in [
            (23, "491231235959Z"),
            (23, "500101000000Z"),
            (24, "19920722132100.3Z"),
            (24, "20260101000000Z"),
        ] {
            assert!(time_syntax(tag, text.as_bytes()), "{text}");
        }
        for (tag, text) in [
            (23, "2601010000Z"),
            (23, "260101000000+0000"),
            (24, "19920722132100.30Z"),
            (24, "19920722132100.0Z"),
            (24, "20260230000000Z"),
            (24, "20260101000000,Z"),
        ] {
            assert!(!time_syntax(tag, text.as_bytes()), "{text}");
        }
    }
}
