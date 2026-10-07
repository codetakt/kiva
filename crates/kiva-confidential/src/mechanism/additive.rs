//! Additive secure-aggregation boundary facade.

pub use crate::primitive::{
    AdditiveShareVerificationKey, SecureAggregateCommitment, SecureAggregateEnvelopeSequence,
    SecureAggregateFieldElement, SecureAggregateSignature, SecureAggregateSigningKeyId,
    SecureAggregateSigningPublicKey, SecureAggregateTranscriptId, SecureAggregateTransportProof,
};

pub use super::core::{
    ADDITIVE_SHARE_ENVELOPE_SIGNATURE_DOMAIN, AcceptedAdditiveShare, AcceptedAdditiveShareRecord,
    AdditiveSecureAggregatePlan, AdditiveShareCommitmentSlot, AdditiveShareReplayWindow,
    AuthenticatedAdditiveShareEnvelope, ReplayWindowedAdditiveAggregate, SecureAggregateError,
    SecureAggregateShare, SignedAuthenticatedAdditiveShareEnvelope,
    accept_authenticated_additive_share, accept_replay_windowed_authenticated_additive_share,
    accept_signed_replay_windowed_authenticated_additive_share,
    additive_share_envelope_signature_payload, combine_additive_shares,
    combine_authenticated_additive_share_envelopes,
    combine_signed_authenticated_additive_share_envelopes,
    validate_additive_share_commitment_slots, validate_additive_share_verification_keys,
    verify_signed_authenticated_additive_share_envelope,
};
