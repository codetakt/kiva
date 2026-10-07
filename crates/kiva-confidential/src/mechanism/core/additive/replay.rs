#![allow(clippy::wildcard_imports)]

use super::super::*;
use super::envelope::SecureAggregateShare;

/// Replay record for one accepted additive-share envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedAdditiveShareRecord {
    transcript_id: SecureAggregateTranscriptId,
    participant_index: NonZeroUsize,
    sequence: SecureAggregateEnvelopeSequence,
    signing_key_id: SecureAggregateSigningKeyId,
}

impl AcceptedAdditiveShareRecord {
    /// Creates an accepted-envelope replay record.
    #[must_use]
    pub const fn new(
        transcript_id: SecureAggregateTranscriptId,
        participant_index: NonZeroUsize,
        sequence: SecureAggregateEnvelopeSequence,
        signing_key_id: SecureAggregateSigningKeyId,
    ) -> Self {
        Self {
            transcript_id,
            participant_index,
            sequence,
            signing_key_id,
        }
    }

    /// Returns the transcript identifier.
    #[must_use]
    pub const fn transcript_id(self) -> SecureAggregateTranscriptId {
        self.transcript_id
    }

    /// Returns the one-based participant index.
    #[must_use]
    pub const fn participant_index(self) -> NonZeroUsize {
        self.participant_index
    }

    /// Returns the accepted envelope sequence.
    #[must_use]
    pub const fn sequence(self) -> SecureAggregateEnvelopeSequence {
        self.sequence
    }

    /// Returns the signing-key identifier used by the accepted envelope.
    #[must_use]
    pub const fn signing_key_id(self) -> SecureAggregateSigningKeyId {
        self.signing_key_id
    }

    /// Returns whether two records conflict for single-use participant acceptance.
    #[must_use]
    pub fn conflicts_with(self, other: Self) -> bool {
        self.transcript_id == other.transcript_id
            && self.participant_index == other.participant_index
    }
}

/// Accepted additive share and the replay record that must be persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedAdditiveShare {
    share: SecureAggregateShare,
    replay_record: AcceptedAdditiveShareRecord,
}

impl AcceptedAdditiveShare {
    /// Creates an accepted additive share result.
    #[must_use]
    pub const fn new(
        share: SecureAggregateShare,
        replay_record: AcceptedAdditiveShareRecord,
    ) -> Self {
        Self {
            share,
            replay_record,
        }
    }

    /// Returns the accepted share.
    #[must_use]
    pub const fn share(self) -> SecureAggregateShare {
        self.share
    }

    /// Returns the replay record that must be persisted.
    #[must_use]
    pub const fn replay_record(self) -> AcceptedAdditiveShareRecord {
        self.replay_record
    }
}

/// Aggregate result and replay records produced by a windowed transcript.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayWindowedAdditiveAggregate {
    aggregate: SecureAggregateFieldElement,
    accepted_records: Vec<AcceptedAdditiveShareRecord>,
}

impl ReplayWindowedAdditiveAggregate {
    /// Creates a replay-windowed aggregate result.
    #[must_use]
    pub const fn new(
        aggregate: SecureAggregateFieldElement,
        accepted_records: Vec<AcceptedAdditiveShareRecord>,
    ) -> Self {
        Self {
            aggregate,
            accepted_records,
        }
    }

    /// Returns the reconstructed aggregate.
    #[must_use]
    pub const fn aggregate(&self) -> SecureAggregateFieldElement {
        self.aggregate
    }

    /// Returns the replay records that must be persisted after release.
    #[must_use]
    pub fn accepted_records(&self) -> &[AcceptedAdditiveShareRecord] {
        &self.accepted_records
    }
}
