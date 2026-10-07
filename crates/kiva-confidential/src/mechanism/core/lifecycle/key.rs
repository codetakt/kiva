#![allow(clippy::wildcard_imports)]

use super::super::*;

/// Lifecycle state for one registered additive-share signing key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShareSigningKeyLifecycleState {
    /// The key identifier has not been registered.
    Missing,
    /// The key identifier is registered and acceptable for new submissions.
    Active,
    /// The key identifier was revoked and cannot be accepted for new submissions.
    Revoked,
}

/// Signing-key lifecycle validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ShareSigningKeyLifecycleError {
    /// The key owner does not match the subject bound to the key.
    #[error("share signing-key owner does not match")]
    OwnerMismatch,

    /// The key is not bound to the expected computation participant.
    #[error("share signing-key participant does not match")]
    ParticipantMismatch,

    /// The old key is not active and cannot be rotated.
    #[error("old share signing-key is not active")]
    OldKeyNotActive,

    /// The new key identifier is already registered.
    #[error("new share signing-key is already registered")]
    NewKeyAlreadyRegistered,
}

/// Preconditions for rotating one participant's additive-share signing key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareSigningKeyRotationPrecondition {
    old_state: ShareSigningKeyLifecycleState,
    new_state: ShareSigningKeyLifecycleState,
    owner_matches: bool,
    participant_matches: bool,
}

impl ShareSigningKeyRotationPrecondition {
    /// Creates a signing-key rotation precondition set.
    #[must_use]
    pub const fn new(
        old_state: ShareSigningKeyLifecycleState,
        new_state: ShareSigningKeyLifecycleState,
        owner_matches: bool,
        participant_matches: bool,
    ) -> Self {
        Self {
            old_state,
            new_state,
            owner_matches,
            participant_matches,
        }
    }

    /// Returns the old key state.
    #[must_use]
    pub const fn old_state(self) -> ShareSigningKeyLifecycleState {
        self.old_state
    }

    /// Returns the new key state.
    #[must_use]
    pub const fn new_state(self) -> ShareSigningKeyLifecycleState {
        self.new_state
    }

    /// Returns whether the actor owns the key binding.
    #[must_use]
    pub const fn owner_matches(self) -> bool {
        self.owner_matches
    }

    /// Returns whether the key belongs to the expected participant.
    #[must_use]
    pub const fn participant_matches(self) -> bool {
        self.participant_matches
    }
}

/// Resulting states after a successful signing-key rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareSigningKeyRotationOutcome {
    old_state: ShareSigningKeyLifecycleState,
    new_state: ShareSigningKeyLifecycleState,
}

impl ShareSigningKeyRotationOutcome {
    /// Creates a signing-key rotation outcome.
    #[must_use]
    pub const fn new(
        old_state: ShareSigningKeyLifecycleState,
        new_state: ShareSigningKeyLifecycleState,
    ) -> Self {
        Self {
            old_state,
            new_state,
        }
    }

    /// Returns the old key state after rotation.
    #[must_use]
    pub const fn old_state(self) -> ShareSigningKeyLifecycleState {
        self.old_state
    }

    /// Returns the new key state after rotation.
    #[must_use]
    pub const fn new_state(self) -> ShareSigningKeyLifecycleState {
        self.new_state
    }
}

/// Returns whether a signing-key state is acceptable for new share submissions.
#[must_use]
pub const fn share_signing_key_accepts_new_submissions(
    state: ShareSigningKeyLifecycleState,
) -> bool {
    matches!(state, ShareSigningKeyLifecycleState::Active)
}

/// Revokes a signing key, preserving fail-closed idempotence.
#[must_use]
pub const fn revoke_share_signing_key(
    state: ShareSigningKeyLifecycleState,
) -> ShareSigningKeyLifecycleState {
    match state {
        ShareSigningKeyLifecycleState::Missing => ShareSigningKeyLifecycleState::Missing,
        ShareSigningKeyLifecycleState::Active | ShareSigningKeyLifecycleState::Revoked => {
            ShareSigningKeyLifecycleState::Revoked
        }
    }
}

/// Rotates a participant signing key from active old key to absent new key.
pub const fn rotate_share_signing_key(
    precondition: ShareSigningKeyRotationPrecondition,
) -> Result<ShareSigningKeyRotationOutcome, ShareSigningKeyLifecycleError> {
    if !precondition.owner_matches() {
        return Err(ShareSigningKeyLifecycleError::OwnerMismatch);
    }
    if !precondition.participant_matches() {
        return Err(ShareSigningKeyLifecycleError::ParticipantMismatch);
    }
    if !matches!(
        precondition.old_state(),
        ShareSigningKeyLifecycleState::Active
    ) {
        return Err(ShareSigningKeyLifecycleError::OldKeyNotActive);
    }
    if !matches!(
        precondition.new_state(),
        ShareSigningKeyLifecycleState::Missing
    ) {
        return Err(ShareSigningKeyLifecycleError::NewKeyAlreadyRegistered);
    }
    Ok(ShareSigningKeyRotationOutcome::new(
        ShareSigningKeyLifecycleState::Revoked,
        ShareSigningKeyLifecycleState::Active,
    ))
}
