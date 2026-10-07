//! Shamir threshold-reconstruction profile domains.

/// Domain separator for hash-committed Shamir share openings.
pub const SHAMIR_SHARE_OPENING_COMMITMENT_DOMAIN: &[u8] =
    b"kiva-confidential:shamir-share-opening:v1";
