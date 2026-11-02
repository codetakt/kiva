//! RFC 5280 Appendix A naming types, separate from name-comparison semantics.

#[cfg(not(feature = "std"))]
use alloc::string::ToString;
use der::{
    asn1::{Any, BmpString, Ia5StringRef, PrintableStringRef, TeletexStringRef, Utf8StringRef},
    Tag, Tagged,
};
use x509_cert::name::{Name, RelativeDistinguishedName};

/// Check each present RDN and known attribute schema. Empty Name is permitted
/// by ASN.1; certificate-role rules (including critical SAN) are separate.
pub fn validate_name_schema(name: &Name) -> der::Result<()> {
    for rdn in &name.0 {
        validate_rdn(rdn)?;
    }
    Ok(())
}

pub(crate) fn validate_rdn(rdn: &RelativeDistinguishedName) -> der::Result<()> {
    if rdn.0.is_empty() {
        return Err(Tag::Set.value_error());
    }
    for attr in rdn.0.iter() {
        // RFC 5280 A.1 supplies these OID-defined types. Unknown attributes
        // remain opaque ANY values; do not impose a string type on them.
        let oid = attr.oid.to_string();
        let value = &attr.value;
        match oid.as_str() {
            "2.5.4.41" | "2.5.4.4" | "2.5.4.42" | "2.5.4.43" | "2.5.4.44" => {
                directory(value, 32768)?
            }
            "2.5.4.3" | "2.5.4.10" | "2.5.4.11" | "2.5.4.12" => directory(value, 64)?,
            "2.5.4.7" | "2.5.4.8" | "2.5.4.65" => directory(value, 128)?,
            "2.5.4.6" => size(
                value.decode_as::<PrintableStringRef<'_>>()?.as_str().len(),
                2,
                2,
            )?,
            "2.5.4.5" => size(
                value.decode_as::<PrintableStringRef<'_>>()?.as_str().len(),
                1,
                64,
            )?,
            "2.5.4.46" => {
                value.decode_as::<PrintableStringRef<'_>>()?;
            }
            "0.9.2342.19200300.100.1.25" => {
                value.decode_as::<Ia5StringRef<'_>>()?;
            }
            "1.2.840.113549.1.9.1" => size(
                value.decode_as::<Ia5StringRef<'_>>()?.as_str().len(),
                1,
                255,
            )?,
            _ => {}
        }
    }
    Ok(())
}

fn directory(value: &Any, max: usize) -> der::Result<()> {
    let count = match value.tag() {
        Tag::Utf8String => value
            .decode_as::<Utf8StringRef<'_>>()?
            .as_str()
            .chars()
            .count(),
        Tag::PrintableString => value.decode_as::<PrintableStringRef<'_>>()?.as_str().len(),
        Tag::BmpString => value.decode_as::<BmpString>()?.chars().count(),
        // Keep the dependency's ASCII-only optional Teletex decoding scope;
        // this does not enable Teletex issuer-name equality. UniversalString
        // remains outside the dependency's optional decoder capabilities.
        Tag::TeletexString => value.decode_as::<TeletexStringRef<'_>>()?.as_str().len(),
        _ => return Err(value.tag().value_error()),
    };
    size(count, 1, max)
}

fn size(count: usize, min: usize, max: usize) -> der::Result<()> {
    if (min..=max).contains(&count) {
        Ok(())
    } else {
        Err(Tag::Sequence.value_error())
    }
}
