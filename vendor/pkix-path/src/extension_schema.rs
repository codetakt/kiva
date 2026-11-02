//! Receiver schema checks shared with Kiva's original-certificate DER gate.

use crate::Vec;
use der::{
    asn1::{BitString, Int, ObjectIdentifier, OctetString, Uint},
    Decode, DecodeOwned, Encode, Sequence, Tag,
};
use x509_cert::ext::pkix::{
    name::{DistributionPointName, GeneralName, GeneralNames},
    AuthorityInfoAccessSyntax, ExtendedKeyUsage, NameConstraints, PolicyMappings, SubjectAltName,
};

const ISSUER_ALT_NAME: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.18");
const AUTHORITY_KEY_ID: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.35");
const AUTHORITY_INFO: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.1.1");
const SUBJECT_INFO: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.1.11");
const CRL_POINTS: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.31");
const FRESHEST_CRL: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.46");

// Keep ASN.1 INTEGER/BIT STRING domains here. The dependency's SerialNumber
// and ReasonFlags types add machine/profile limits beyond these field types.
#[derive(Clone, Debug, Sequence)]
struct AuthorityKeyIdentifier {
    #[asn1(context_specific = "0", optional = "true", tag_mode = "IMPLICIT")]
    key_identifier: Option<OctetString>,
    #[asn1(context_specific = "1", optional = "true", tag_mode = "IMPLICIT")]
    authority_cert_issuer: Option<GeneralNames>,
    #[asn1(context_specific = "2", optional = "true", tag_mode = "IMPLICIT")]
    authority_cert_serial_number: Option<Int>,
}

#[derive(Clone, Debug, Sequence)]
struct DistributionPoint {
    #[asn1(context_specific = "0", optional = "true", tag_mode = "EXPLICIT")]
    distribution_point: Option<DistributionPointName>,
    #[asn1(context_specific = "1", optional = "true", tag_mode = "IMPLICIT")]
    reasons: Option<BitString>,
    #[asn1(context_specific = "2", optional = "true", tag_mode = "IMPLICIT")]
    crl_issuer: Option<GeneralNames>,
}

// RFC5280 INTEGER (0..MAX) has no u8/u32 upper bound. Retain its exact unsigned
// magnitude through decoding; clamp only when comparing with a bounded counter.
#[derive(Clone, Debug, Sequence)]
pub(crate) struct BasicConstraints {
    #[asn1(default = "Default::default")]
    pub ca: bool,
    pub path_len_constraint: Option<Uint>,
}

#[derive(Clone, Debug, Sequence)]
pub(crate) struct PolicyConstraints {
    #[asn1(context_specific = "0", optional = "true", tag_mode = "IMPLICIT")]
    pub require_explicit_policy: Option<Uint>,
    #[asn1(context_specific = "1", optional = "true", tag_mode = "IMPLICIT")]
    pub inhibit_policy_mapping: Option<Uint>,
}

pub(crate) fn bounded_u32(value: &Uint) -> u32 {
    value.as_bytes().iter().fold(0_u32, |n, byte| {
        n.saturating_mul(256).saturating_add(u32::from(*byte))
    })
}

pub(crate) fn bounded_usize(value: &Uint) -> usize {
    value.as_bytes().iter().fold(0_usize, |n, byte| {
        n.saturating_mul(256).saturating_add(usize::from(*byte))
    })
}

fn typed<T: DecodeOwned + Encode>(bytes: &[u8]) -> der::Result<T> {
    let value = T::from_der(bytes)?;
    // Also reject explicitly encoded DEFAULTs, discarded optional fields and
    // other noncanonical schema encodings. This is only a comparison: original
    // certificate/TBS bytes remain untouched for cryptographic verification.
    if value.to_der()? != bytes {
        return Err(Tag::Sequence.non_canonical_error());
    }
    Ok(value)
}

fn nonempty<T>(values: &[T]) -> der::Result<()> {
    if values.is_empty() {
        Err(Tag::Sequence.value_error())
    } else {
        Ok(())
    }
}

/// Validate the supported extension schemas whose dependency types omit size
/// constraints or narrow unbounded integers. Returns `false` for other OIDs;
/// that is not a claim that an unknown extension is valid or understood.
///
/// Callers must still enforce path semantics and unknown-critical handling.
/// Criticalness does not waive a known extension's required ASN.1 structure.
pub fn validate_extension_schema(oid: ObjectIdentifier, bytes: &[u8]) -> der::Result<bool> {
    match oid {
        crate::OID_BASIC_CONSTRAINTS => {
            typed::<BasicConstraints>(bytes)?;
        }
        crate::OID_POLICY_CONSTRAINTS => {
            typed::<PolicyConstraints>(bytes)?;
        }
        crate::OID_INHIBIT_ANY_POLICY => {
            Uint::from_der(bytes)?;
        }
        // SubjectAltName and IssuerAltName share GeneralNames (SIZE 1..MAX).
        crate::OID_SUBJECT_ALT_NAME | ISSUER_ALT_NAME => {
            general_names(&typed::<SubjectAltName>(bytes)?.0)?;
        }
        crate::OID_EXTENDED_KEY_USAGE => {
            nonempty(&typed::<ExtendedKeyUsage>(bytes)?.0)?;
        }
        crate::OID_POLICY_MAPPINGS => {
            nonempty(&typed::<PolicyMappings>(bytes)?.0)?;
        }
        crate::OID_NAME_CONSTRAINTS => {
            let value = typed::<NameConstraints>(bytes)?;
            for subtrees in [value.permitted_subtrees, value.excluded_subtrees]
                .into_iter()
                .flatten()
            {
                nonempty(&subtrees)?;
                for subtree in subtrees {
                    general_name(&subtree.base)?;
                }
            }
        }
        AUTHORITY_KEY_ID => {
            if let Some(names) = typed::<AuthorityKeyIdentifier>(bytes)?.authority_cert_issuer {
                general_names(&names)?;
            }
        }
        AUTHORITY_INFO | SUBJECT_INFO => {
            // Both use SEQUENCE SIZE (1..MAX) OF AccessDescription.
            let descriptions = typed::<AuthorityInfoAccessSyntax>(bytes)?.0;
            nonempty(&descriptions)?;
            for description in descriptions {
                general_name(&description.access_location)?;
            }
        }
        CRL_POINTS | FRESHEST_CRL => {
            let points = typed::<Vec<DistributionPoint>>(bytes)?;
            nonempty(&points)?;
            for point in points {
                if let Some(reasons) = point.reasons {
                    named_bits(&reasons)?;
                }
                match point.distribution_point {
                    Some(DistributionPointName::FullName(names)) => general_names(&names)?,
                    Some(DistributionPointName::NameRelativeToCRLIssuer(rdn)) => {
                        crate::name_schema::validate_rdn(&rdn)?
                    }
                    None => {}
                }
                if let Some(names) = point.crl_issuer {
                    general_names(&names)?;
                }
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}

fn named_bits(value: &BitString) -> der::Result<()> {
    // A generic BIT STRING preserves its supplied padding. ReasonFlags is a
    // named bit list: DER requires zero unused bits and omission of all trailing
    // zero bits. Equal trailing-zero and unused-bit counts enforce both without
    // imposing a machine-word width or rewriting the original signed bytes.
    let canonical = value
        .raw_bytes()
        .last()
        .map_or(value.unused_bits() == 0, |last| {
            *last != 0 && last.trailing_zeros() == u32::from(value.unused_bits())
        });
    if canonical {
        Ok(())
    } else {
        Err(Tag::BitString.non_canonical_error())
    }
}

fn general_names(names: &[GeneralName]) -> der::Result<()> {
    nonempty(names)?;
    for name in names {
        general_name(name)?;
    }
    Ok(())
}

fn general_name(name: &GeneralName) -> der::Result<()> {
    if let GeneralName::DirectoryName(name) = name {
        crate::validate_name_schema(name)?;
    }
    Ok(())
}
