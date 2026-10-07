#![allow(clippy::wildcard_imports)]

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ConfidentialSubmittedShareKey {
    pub(crate) transcript_id: [u8; 32],
    pub(crate) participant_index: usize,
}

impl ConfidentialSubmittedShareKey {
    pub(crate) const fn from_envelope(envelope: SignedAuthenticatedAdditiveShareEnvelope) -> Self {
        let envelope = envelope.envelope();
        Self {
            transcript_id: envelope.transcript_id().bytes(),
            participant_index: envelope.share().participant_index().get(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ConfidentialReplayRecordKey {
    pub(crate) transcript_id: [u8; 32],
    pub(crate) participant_index: usize,
}

impl ConfidentialReplayRecordKey {
    pub(crate) const fn from_record(record: AcceptedAdditiveShareRecord) -> Self {
        Self {
            transcript_id: record.transcript_id().bytes(),
            participant_index: record.participant_index().get(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ConfidentialAggregateResultKey {
    pub(crate) transcript_id: [u8; 32],
}

impl ConfidentialAggregateResultKey {
    pub(crate) const fn from_transcript_id(transcript_id: SecureAggregateTranscriptId) -> Self {
        Self {
            transcript_id: transcript_id.bytes(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ConfidentialSigningKeyStorageKey {
    pub(crate) signing_key_id: [u8; 32],
}

impl ConfidentialSigningKeyStorageKey {
    pub(crate) const fn from_signing_key_id(signing_key_id: SecureAggregateSigningKeyId) -> Self {
        Self {
            signing_key_id: signing_key_id.bytes(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ConfidentialParticipantKey {
    pub(crate) owner_subject: String,
    pub(crate) participant_subject: String,
    pub(crate) participant_index: usize,
}

impl ConfidentialParticipantKey {
    pub(crate) fn from_participant(participant: &ConfidentialParticipantState) -> Self {
        Self {
            owner_subject: participant.owner_subject.clone(),
            participant_subject: participant.participant_subject.clone(),
            participant_index: participant.participant_index.get(),
        }
    }

    pub(crate) fn new(
        owner_subject: &str,
        participant_subject: &str,
        participant_index: NonZeroUsize,
    ) -> Self {
        Self {
            owner_subject: owner_subject.to_owned(),
            participant_subject: participant_subject.to_owned(),
            participant_index: participant_index.get(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ConfidentialParticipantAuditEventKey {
    pub(crate) event_id: Uuid,
}

impl ConfidentialParticipantAuditEventKey {
    pub(crate) const fn from_event_id(event_id: Uuid) -> Self {
        Self { event_id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ConfidentialThresholdReleaseKey {
    pub(crate) release_id: [u8; 32],
}

impl ConfidentialThresholdReleaseKey {
    pub(crate) const fn from_release_id(release_id: SecureAggregateTranscriptId) -> Self {
        Self {
            release_id: release_id.bytes(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ConfidentialShamirThresholdReconstructionKey {
    pub(crate) reconstruction_id: [u8; 32],
}

impl ConfidentialShamirThresholdReconstructionKey {
    pub(crate) const fn from_reconstruction_id(
        reconstruction_id: SecureAggregateTranscriptId,
    ) -> Self {
        Self {
            reconstruction_id: reconstruction_id.bytes(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ConfidentialDifferentialPrivacyBudgetKey {
    pub(crate) budget_id: [u8; 32],
}

impl ConfidentialDifferentialPrivacyBudgetKey {
    pub(crate) const fn from_budget_id(budget_id: SecureAggregateTranscriptId) -> Self {
        Self {
            budget_id: budget_id.bytes(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ConfidentialDifferentialPrivacyReleaseKey {
    pub(crate) release_id: [u8; 32],
}

impl ConfidentialDifferentialPrivacyReleaseKey {
    pub(crate) const fn from_release_id(release_id: SecureAggregateTranscriptId) -> Self {
        Self {
            release_id: release_id.bytes(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ConfidentialJobKey {
    pub(crate) job_id: [u8; 32],
}

impl ConfidentialJobKey {
    pub(crate) const fn from_job_id(job_id: SecureAggregateTranscriptId) -> Self {
        Self {
            job_id: job_id.bytes(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ConfidentialJobAuditEventKey {
    pub(crate) event_id: Uuid,
}

impl ConfidentialJobAuditEventKey {
    pub(crate) const fn from_event_id(event_id: Uuid) -> Self {
        Self { event_id }
    }
}
