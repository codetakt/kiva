#![allow(clippy::wildcard_imports)]

use super::*;

/// Replay metadata for one accepted FROST key ceremony.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThresholdFrostKeyCeremonyRecord {
    job_id: SecureAggregateTranscriptId,
    ceremony_digest: SecureAggregateCommitment,
    key_epoch: SecureAggregateEnvelopeSequence,
}

impl ThresholdFrostKeyCeremonyRecord {
    /// Creates accepted FROST key-ceremony replay metadata.
    #[must_use]
    pub const fn new(
        job_id: SecureAggregateTranscriptId,
        ceremony_digest: SecureAggregateCommitment,
        key_epoch: SecureAggregateEnvelopeSequence,
    ) -> Self {
        Self {
            job_id,
            ceremony_digest,
            key_epoch,
        }
    }

    /// Returns the ceremony job identifier.
    #[must_use]
    pub const fn job_id(self) -> SecureAggregateTranscriptId {
        self.job_id
    }

    /// Returns the accepted ceremony digest.
    #[must_use]
    pub const fn ceremony_digest(self) -> SecureAggregateCommitment {
        self.ceremony_digest
    }

    /// Returns the accepted key epoch.
    #[must_use]
    pub const fn key_epoch(self) -> SecureAggregateEnvelopeSequence {
        self.key_epoch
    }

    /// Returns whether this record conflicts with a candidate ceremony.
    #[must_use]
    pub fn conflicts_with(
        self,
        job_id: SecureAggregateTranscriptId,
        ceremony_digest: SecureAggregateCommitment,
    ) -> bool {
        self.job_id == job_id || self.ceremony_digest.bytes() == ceremony_digest.bytes()
    }
}

/// Accepted metadata-only FROST key ceremony.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedThresholdFrostKeyCeremony {
    record: ThresholdFrostKeyCeremonyRecord,
    participant_count: NonZeroUsize,
    release_threshold: NonZeroUsize,
    runtime_binding: ThresholdFrostRuntimeBinding,
}

impl AcceptedThresholdFrostKeyCeremony {
    /// Creates an accepted FROST key-ceremony result.
    #[must_use]
    pub const fn new(
        record: ThresholdFrostKeyCeremonyRecord,
        participant_count: NonZeroUsize,
        release_threshold: NonZeroUsize,
        runtime_binding: ThresholdFrostRuntimeBinding,
    ) -> Self {
        Self {
            record,
            participant_count,
            release_threshold,
            runtime_binding,
        }
    }

    /// Returns the replay record that must be persisted.
    #[must_use]
    pub const fn record(self) -> ThresholdFrostKeyCeremonyRecord {
        self.record
    }

    /// Returns the configured participant count.
    #[must_use]
    pub const fn participant_count(self) -> NonZeroUsize {
        self.participant_count
    }

    /// Returns the threshold required for signing.
    #[must_use]
    pub const fn release_threshold(self) -> NonZeroUsize {
        self.release_threshold
    }

    /// Returns the runtime binding that admitted this ceremony.
    #[must_use]
    pub const fn runtime_binding(self) -> ThresholdFrostRuntimeBinding {
        self.runtime_binding
    }
}
