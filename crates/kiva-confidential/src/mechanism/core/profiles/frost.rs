//! FROST threshold-signing profile domains.

/// Profile identifier for the CC13 review-gated FROST threshold boundary.
pub const THRESHOLD_FROST_PROFILE: &str = "kiva-threshold-frost-ed25519-review-gated-v1";

/// Domain separator for accepted FROST key-ceremony metadata.
pub const THRESHOLD_FROST_KEY_CEREMONY_DOMAIN: &[u8] =
    b"kiva-confidential:threshold-frost-key-ceremony:v1";

/// Domain separator for accepted FROST signing transcript metadata.
pub const THRESHOLD_FROST_SIGNATURE_TRANSCRIPT_DOMAIN: &[u8] =
    b"kiva-confidential:threshold-frost-signature-transcript:v1";
