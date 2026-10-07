#![allow(clippy::wildcard_imports)]

use super::*;

/// Pure fact bitset evaluated before accepting FROST key-ceremony metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThresholdFrostKeyCeremonyFacts {
    bits: u8,
}

impl ThresholdFrostKeyCeremonyFacts {
    /// Fact bit for a configured runtime binding.
    pub const RUNTIME_BOUND: u8 = 1 << 0;
    /// Fact bit for completed external cryptographic review.
    pub const EXTERNAL_REVIEW_COMPLETED: u8 = 1 << 1;
    /// Fact bit for exactly one ceremony transcript per participant.
    pub const PARTICIPANT_COUNT_MATCHES: u8 = 1 << 2;
    /// Fact bit for active admitted ceremony participants.
    pub const PARTICIPANTS_ACTIVE: u8 = 1 << 3;
    /// Fact bit for complete share-custody metadata.
    pub const SHARE_CUSTODY_PRESENT: u8 = 1 << 4;
    /// Fact bit for resolved complaints.
    pub const COMPLAINTS_RESOLVED: u8 = 1 << 5;
    /// Fact bit for absence of detected equivocation.
    pub const NO_EQUIVOCATION: u8 = 1 << 6;
    /// Fact bit for absence of replay.
    pub const NO_REPLAY: u8 = 1 << 7;

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

/// Normalized facts for accepting FROST key-ceremony metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThresholdFrostKeyCeremonyGateContext {
    facts: ThresholdFrostKeyCeremonyFacts,
}

impl ThresholdFrostKeyCeremonyGateContext {
    /// Creates a key-ceremony gate context from normalized facts.
    #[must_use]
    pub const fn new(facts: ThresholdFrostKeyCeremonyFacts) -> Self {
        Self { facts }
    }

    /// Returns the ceremony fact set.
    #[must_use]
    pub const fn facts(self) -> ThresholdFrostKeyCeremonyFacts {
        self.facts
    }

    /// Returns whether the runtime is configured.
    #[must_use]
    pub const fn runtime_bound(self) -> bool {
        self.facts()
            .contains(ThresholdFrostKeyCeremonyFacts::RUNTIME_BOUND)
    }

    /// Returns whether external cryptographic review has completed.
    #[must_use]
    pub const fn external_review_completed(self) -> bool {
        self.facts()
            .contains(ThresholdFrostKeyCeremonyFacts::EXTERNAL_REVIEW_COMPLETED)
    }

    /// Returns whether the participant transcript count matches the plan.
    #[must_use]
    pub const fn participant_count_matches(self) -> bool {
        self.facts()
            .contains(ThresholdFrostKeyCeremonyFacts::PARTICIPANT_COUNT_MATCHES)
    }

    /// Returns whether all ceremony participants are active.
    #[must_use]
    pub const fn participants_active(self) -> bool {
        self.facts()
            .contains(ThresholdFrostKeyCeremonyFacts::PARTICIPANTS_ACTIVE)
    }

    /// Returns whether share-custody evidence is present.
    #[must_use]
    pub const fn share_custody_present(self) -> bool {
        self.facts()
            .contains(ThresholdFrostKeyCeremonyFacts::SHARE_CUSTODY_PRESENT)
    }

    /// Returns whether all complaints are resolved.
    #[must_use]
    pub const fn complaints_resolved(self) -> bool {
        self.facts()
            .contains(ThresholdFrostKeyCeremonyFacts::COMPLAINTS_RESOLVED)
    }

    /// Returns whether no equivocation evidence was detected.
    #[must_use]
    pub const fn no_equivocation(self) -> bool {
        self.facts()
            .contains(ThresholdFrostKeyCeremonyFacts::NO_EQUIVOCATION)
    }

    /// Returns whether no replay was detected.
    #[must_use]
    pub const fn no_replay(self) -> bool {
        self.facts()
            .contains(ThresholdFrostKeyCeremonyFacts::NO_REPLAY)
    }
}

/// Validates the pure fail-closed FROST key-ceremony admission gate.
pub const fn validate_threshold_frost_key_ceremony_gate_context(
    context: ThresholdFrostKeyCeremonyGateContext,
) -> Result<(), SecureAggregateError> {
    if !context.runtime_bound() {
        return Err(SecureAggregateError::ThresholdFrostRuntimeBindingMissing);
    }
    if !context.external_review_completed() {
        return Err(SecureAggregateError::ThresholdFrostExternalReviewRequired);
    }
    if !context.participant_count_matches() {
        return Err(SecureAggregateError::ThresholdFrostCeremonyParticipantCountMismatch);
    }
    if !context.participants_active() {
        return Err(SecureAggregateError::ThresholdFrostParticipantInactive);
    }
    if !context.share_custody_present() {
        return Err(SecureAggregateError::ThresholdFrostShareCustodyMissing);
    }
    if !context.complaints_resolved() {
        return Err(SecureAggregateError::ThresholdFrostComplaintUnresolved);
    }
    if !context.no_equivocation() {
        return Err(SecureAggregateError::ThresholdFrostEquivocationDetected);
    }
    if !context.no_replay() {
        return Err(SecureAggregateError::ThresholdFrostReplayDetected);
    }
    Ok(())
}
