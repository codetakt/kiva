//! RFC 5280 4.2.1.4 receiver handling, independent of policy-tree survival.

use crate::Error;
use der::{
    asn1::{AnyRef, BmpString, Ia5StringRef, IntRef, ObjectIdentifier, Utf8StringRef},
    Reader, Tag, Tagged,
};
use x509_cert::ext::pkix::certpolicy::CertificatePolicies;

const CPS: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.2.1");
const USER_NOTICE: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.2.2");

pub(super) fn validate(
    policies: &CertificatePolicies,
    critical: bool,
    index: usize,
) -> Result<(), Error> {
    let malformed = || Error::MalformedCertificate { index };
    if policies.0.is_empty() {
        return Err(malformed());
    }
    for policy in &policies.0 {
        let Some(qualifiers) = &policy.policy_qualifiers else {
            continue;
        };
        if qualifiers.is_empty() {
            return Err(malformed());
        }
        for qualifier in qualifiers {
            // The upstream type permits omission; ASN.1 PolicyQualifierInfo
            // requires a value even when its OID is unknown to this receiver.
            let value = qualifier.qualifier.as_ref().ok_or_else(malformed)?;
            let value = AnyRef::from(value);
            match qualifier.policy_qualifier_id {
                CPS => {
                    let uri = value
                        .decode_as::<Ia5StringRef<'_>>()
                        .map_err(|_| malformed())?;
                    iri_string::types::UriStr::new(uri.as_str()).map_err(|_| malformed())?;
                }
                USER_NOTICE => user_notice(value).map_err(|_| malformed())?,
                _ if critical => return Err(Error::UnhandledCriticalExtension { index }),
                _ => {}
            }
        }
    }
    Ok(())
}

fn user_notice(value: AnyRef<'_>) -> der::Result<()> {
    value.sequence(|reader| {
        if !reader.is_finished() && reader.peek_tag()? == Tag::Sequence {
            let reference: AnyRef<'_> = reader.decode()?;
            reference.sequence(|reader| {
                display_text(reader.decode()?, false)?;
                let numbers: AnyRef<'_> = reader.decode()?;
                numbers.sequence(|reader| {
                    while !reader.is_finished() {
                        // INTEGER is signed and unbounded; do not narrow it to
                        // a machine integer or require a nonempty sequence.
                        let _: IntRef<'_> = reader.decode()?;
                    }
                    Ok(())
                })
            })?;
        }
        if !reader.is_finished() {
            display_text(reader.decode()?, true)?;
        }
        Ok(())
    })
}

fn display_text(value: AnyRef<'_>, explicit_text: bool) -> der::Result<()> {
    let length = match value.tag() {
        Tag::Ia5String => value.decode_as::<Ia5StringRef<'_>>()?.as_str().len(),
        Tag::VisibleString => {
            if !value
                .value()
                .iter()
                .all(|byte| (0x20..=0x7e).contains(byte))
            {
                return Err(value.tag().value_error());
            }
            value.value().len()
        }
        Tag::Utf8String => value
            .decode_as::<Utf8StringRef<'_>>()?
            .as_str()
            .chars()
            .count(),
        Tag::BmpString => value.decode_as::<BmpString>()?.chars().count(),
        _ => return Err(value.tag().value_error()),
    };
    // RFC 5280 explicitly recommends graceful receipt of explicitText over
    // 200 characters. Issuer advice about UTF8, NFC and control characters
    // does not create additional mandatory receiver rejection conditions.
    if length == 0 || (!explicit_text && length > 200) {
        return Err(value.tag().value_error());
    }
    Ok(())
}
