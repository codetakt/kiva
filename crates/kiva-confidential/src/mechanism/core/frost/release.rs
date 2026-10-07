#![allow(clippy::wildcard_imports)]

use super::*;

/// Pure fact bitset evaluated before releasing a FROST threshold signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThresholdFrostReleaseFacts {
    bits: u8,
}

impl ThresholdFrostReleaseFacts {
    /// Fact bit for an accepted key ceremony.
    pub const KEY_CEREMONY_COMPLETED: u8 = 1 << 0;
    /// Fact bit for an accepted signing transcript.
    pub const SIGNATURE_COMPUTED: u8 = 1 << 1;
    /// Fact bit for policy-authority approval.
    pub const POLICY_APPROVED: u8 = 1 << 2;
    /// Fact bit for an active key epoch.
    pub const ACTIVE_KEY_EPOCH: u8 = 1 << 3;
    /// Fact bit for governed key-use treatment.
    pub const GOVERNED_KEY_USE: u8 = 1 << 4;
    /// Fact bit for completed external cryptographic review.
    pub const EXTERNAL_REVIEW_COMPLETED: u8 = 1 << 5;

    /// Creates a fact set from raw bits.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    /// Returns the raw fact bits.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.bits
    }

    /// Returns whether a fact bit is present.
    #[must_use]
    pub const fn contains(self, bit: u8) -> bool {
        self.bits & bit == bit
    }
}

/// Runtime facts evaluated before releasing a FROST threshold signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThresholdFrostReleaseContext {
    facts: ThresholdFrostReleaseFacts,
}

impl ThresholdFrostReleaseContext {
    /// Creates a release context from normalized facts.
    #[must_use]
    pub const fn new(facts: ThresholdFrostReleaseFacts) -> Self {
        Self { facts }
    }

    /// Returns the release fact set.
    #[must_use]
    pub const fn facts(self) -> ThresholdFrostReleaseFacts {
        self.facts
    }

    /// Returns whether an accepted key ceremony is present.
    #[must_use]
    pub const fn key_ceremony_completed(self) -> bool {
        self.facts()
            .contains(ThresholdFrostReleaseFacts::KEY_CEREMONY_COMPLETED)
    }

    /// Returns whether the threshold signature was computed.
    #[must_use]
    pub const fn signature_computed(self) -> bool {
        self.facts()
            .contains(ThresholdFrostReleaseFacts::SIGNATURE_COMPUTED)
    }

    /// Returns whether policy authority approved release.
    #[must_use]
    pub const fn policy_approved(self) -> bool {
        self.facts()
            .contains(ThresholdFrostReleaseFacts::POLICY_APPROVED)
    }

    /// Returns whether the key epoch is active.
    #[must_use]
    pub const fn active_key_epoch(self) -> bool {
        self.facts()
            .contains(ThresholdFrostReleaseFacts::ACTIVE_KEY_EPOCH)
    }

    /// Returns whether key use is governed by policy and audit metadata.
    #[must_use]
    pub const fn governed_key_use(self) -> bool {
        self.facts()
            .contains(ThresholdFrostReleaseFacts::GOVERNED_KEY_USE)
    }

    /// Returns whether the external cryptographic review has completed.
    #[must_use]
    pub const fn external_review_completed(self) -> bool {
        self.facts()
            .contains(ThresholdFrostReleaseFacts::EXTERNAL_REVIEW_COMPLETED)
    }
}

/// Validates the fail-closed release gate for a FROST threshold signature.
pub const fn validate_threshold_frost_release(
    context: ThresholdFrostReleaseContext,
) -> Result<(), SecureAggregateError> {
    if !context.key_ceremony_completed() {
        return Err(SecureAggregateError::ThresholdFrostReleaseRequiresKeyCeremony);
    }
    if !context.signature_computed() {
        return Err(SecureAggregateError::ThresholdFrostReleaseRequiresSignature);
    }
    if !context.external_review_completed() {
        return Err(SecureAggregateError::ThresholdFrostExternalReviewRequired);
    }
    if !context.active_key_epoch() {
        return Err(SecureAggregateError::ThresholdFrostReleaseRequiresActiveKeyEpoch);
    }
    if !context.policy_approved() {
        return Err(SecureAggregateError::ThresholdFrostReleaseRequiresPolicyApproval);
    }
    if !context.governed_key_use() {
        return Err(SecureAggregateError::ThresholdFrostReleaseRequiresGovernedKeyUse);
    }
    Ok(())
}
