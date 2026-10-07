#![allow(clippy::wildcard_imports)]

use super::*;

/// Participant-side state for one FROST key-ceremony transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThresholdFrostParticipantTranscriptStatus {
    /// Participant submitted complete public key-ceremony metadata.
    Submitted,
    /// Participant was removed or inactive for this ceremony.
    Removed,
    /// Participant aborted before complete ceremony metadata existed.
    Aborted,
}

/// One participant's metadata-only FROST key-ceremony transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThresholdFrostParticipantTranscript {
    participant_index: NonZeroUsize,
    status: ThresholdFrostParticipantTranscriptStatus,
    public_commitment_digest: SecureAggregateCommitment,
    share_custody_digest: Option<SecureAggregateCommitment>,
    complaint_resolved: bool,
    equivocation_detected: bool,
}

impl ThresholdFrostParticipantTranscript {
    /// Creates one participant's FROST ceremony metadata.
    #[must_use]
    pub const fn new(
        participant_index: NonZeroUsize,
        status: ThresholdFrostParticipantTranscriptStatus,
        public_commitment_digest: SecureAggregateCommitment,
        share_custody_digest: Option<SecureAggregateCommitment>,
        complaint_resolved: bool,
        equivocation_detected: bool,
    ) -> Self {
        Self {
            participant_index,
            status,
            public_commitment_digest,
            share_custody_digest,
            complaint_resolved,
            equivocation_detected,
        }
    }

    /// Returns the one-based participant index.
    #[must_use]
    pub const fn participant_index(self) -> NonZeroUsize {
        self.participant_index
    }

    /// Returns the participant ceremony status.
    #[must_use]
    pub const fn status(self) -> ThresholdFrostParticipantTranscriptStatus {
        self.status
    }

    /// Returns the digest of public FROST commitments.
    #[must_use]
    pub const fn public_commitment_digest(self) -> SecureAggregateCommitment {
        self.public_commitment_digest
    }

    /// Returns metadata proving custody of the participant's private share.
    #[must_use]
    pub const fn share_custody_digest(self) -> Option<SecureAggregateCommitment> {
        self.share_custody_digest
    }

    /// Returns whether all complaints for this participant were resolved.
    #[must_use]
    pub const fn complaint_resolved(self) -> bool {
        self.complaint_resolved
    }

    /// Returns whether equivocation evidence was detected.
    #[must_use]
    pub const fn equivocation_detected(self) -> bool {
        self.equivocation_detected
    }
}
