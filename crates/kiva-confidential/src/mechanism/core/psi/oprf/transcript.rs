#![allow(clippy::wildcard_imports)]

use super::*;

/// Participant-side transcript status for abort-safe DH-OPRF PSI admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PsiOprfParticipantTranscriptStatus {
    /// Participant submitted its evaluated OPRF elements.
    Submitted,
    /// Participant aborted before a complete runtime transcript existed.
    Aborted,
}

/// One participant's opaque DH-OPRF PSI transcript.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsiOprfParticipantTranscript {
    participant_index: NonZeroUsize,
    status: PsiOprfParticipantTranscriptStatus,
    evaluated_elements: Vec<PsiOprfElement>,
}

impl PsiOprfParticipantTranscript {
    /// Creates a participant transcript from opaque evaluated elements.
    #[must_use]
    pub const fn new(
        participant_index: NonZeroUsize,
        status: PsiOprfParticipantTranscriptStatus,
        evaluated_elements: Vec<PsiOprfElement>,
    ) -> Self {
        Self {
            participant_index,
            status,
            evaluated_elements,
        }
    }

    /// Returns the one-based participant index.
    #[must_use]
    pub const fn participant_index(&self) -> NonZeroUsize {
        self.participant_index
    }

    /// Returns the participant transcript status.
    #[must_use]
    pub const fn status(&self) -> PsiOprfParticipantTranscriptStatus {
        self.status
    }

    /// Returns the opaque evaluated OPRF elements.
    #[must_use]
    pub fn evaluated_elements(&self) -> &[PsiOprfElement] {
        &self.evaluated_elements
    }
}

/// Replay metadata for one accepted DH-OPRF PSI transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsiOprfTranscriptRecord {
    job_id: SecureAggregateTranscriptId,
    transcript_digest: SecureAggregateCommitment,
}

impl PsiOprfTranscriptRecord {
    /// Creates accepted DH-OPRF PSI transcript replay metadata.
    #[must_use]
    pub const fn new(
        job_id: SecureAggregateTranscriptId,
        transcript_digest: SecureAggregateCommitment,
    ) -> Self {
        Self {
            job_id,
            transcript_digest,
        }
    }

    /// Returns the job identifier.
    #[must_use]
    pub const fn job_id(self) -> SecureAggregateTranscriptId {
        self.job_id
    }

    /// Returns the accepted transcript digest.
    #[must_use]
    pub const fn transcript_digest(self) -> SecureAggregateCommitment {
        self.transcript_digest
    }

    /// Returns whether this record conflicts with a candidate transcript.
    #[must_use]
    pub fn conflicts_with(
        self,
        job_id: SecureAggregateTranscriptId,
        transcript_digest: SecureAggregateCommitment,
    ) -> bool {
        self.job_id == job_id || self.transcript_digest.bytes() == transcript_digest.bytes()
    }
}
