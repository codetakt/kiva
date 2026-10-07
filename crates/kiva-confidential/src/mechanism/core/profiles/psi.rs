//! PSI profile domains.

/// Domain separator for the bounded cross-school cohort PSI identifier profile.
pub const PSI_COHORT_IDENTIFIER_DOMAIN: &[u8] = b"kiva-confidential:psi-cohort-identifier:v1";

/// Profile identifier for the CC11 review-gated DH-OPRF PSI runtime boundary.
pub const PSI_OPRF_DH_VOPRF_PROFILE: &str = "kiva-psi-oprf-dh-voprf-review-gated-v1";

/// Domain separator for deterministic DH-OPRF PSI fixture elements.
pub const PSI_OPRF_DH_VOPRF_ELEMENT_DOMAIN: &[u8] =
    b"kiva-confidential:psi-oprf-dh-voprf-element:v1";

/// Domain separator for accepted DH-OPRF PSI transcript metadata.
pub const PSI_OPRF_DH_VOPRF_TRANSCRIPT_DOMAIN: &[u8] =
    b"kiva-confidential:psi-oprf-dh-voprf-transcript:v1";
