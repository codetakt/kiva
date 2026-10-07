//! Shamir threshold reconstruction boundary facade.

pub use super::core::{
    SHAMIR_SHARE_OPENING_COMMITMENT_DOMAIN, ShamirShare, ShamirShareComplaintOutcome,
    ShamirShareOpeningNonce, ShamirThresholdPlan, VerifiableShamirShare, VerifiedShamirShare,
    derive_shamir_share_opening_commitment, reconstruct_shamir_secret,
    reconstruct_verified_shamir_secret, resolve_shamir_share_complaint,
    verify_shamir_share_opening, verify_shamir_share_openings,
};
