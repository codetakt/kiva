//! Confidential keys server-local state.

#![allow(clippy::wildcard_imports)]

use super::super::*;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub(crate) struct ConfidentialShareSigningKeyRequest {
    pub(crate) signing_key_id: SecureAggregateSigningKeyId,
    pub(crate) participant_index: std::num::NonZeroUsize,
    pub(crate) public_key: SecureAggregateSigningPublicKey,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub(crate) struct ConfidentialShareSigningKeyRevocationRequest {
    pub(crate) signing_key_id: SecureAggregateSigningKeyId,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub(crate) struct ConfidentialShareSigningKeyRotationRequest {
    pub(crate) old_signing_key_id: SecureAggregateSigningKeyId,
    pub(crate) new_signing_key_id: SecureAggregateSigningKeyId,
    pub(crate) participant_index: std::num::NonZeroUsize,
    pub(crate) public_key: SecureAggregateSigningPublicKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfidentialShareSigningKey {
    pub(crate) oauth_subject: String,
    pub(crate) signing_key_id: SecureAggregateSigningKeyId,
    pub(crate) participant_index: std::num::NonZeroUsize,
    pub(crate) public_key: SecureAggregateSigningPublicKey,
    pub(crate) revoked_at: Option<OffsetDateTime>,
}

impl ConfidentialShareSigningKey {
    pub(crate) fn from_request(
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSigningKeyRequest,
    ) -> Self {
        Self {
            oauth_subject: principal.subject.clone(),
            signing_key_id: request.signing_key_id,
            participant_index: request.participant_index,
            public_key: request.public_key,
            revoked_at: None,
        }
    }

    pub(crate) fn from_rotation(
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSigningKeyRotationRequest,
    ) -> Self {
        Self {
            oauth_subject: principal.subject.clone(),
            signing_key_id: request.new_signing_key_id,
            participant_index: request.participant_index,
            public_key: request.public_key,
            revoked_at: None,
        }
    }

    pub(crate) const fn verification_key(&self) -> AdditiveShareVerificationKey {
        AdditiveShareVerificationKey::new(self.signing_key_id, self.public_key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfidentialShareSigningKeyRevocationOutcome {
    Revoked,
    AlreadyRevoked,
    NotFound,
    Forbidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfidentialShareSigningKeyRotationOutcome {
    Rotated,
    OldKeyRevoked,
    NewKeyConflict,
    NotFound,
    Forbidden,
}
