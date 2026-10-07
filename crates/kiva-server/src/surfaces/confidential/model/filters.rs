#![allow(clippy::wildcard_imports)]

use super::*;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ConfidentialThresholdReleaseFilter {
    pub(crate) release_id: Option<SecureAggregateTranscriptId>,
    pub(crate) transcript_id: Option<SecureAggregateTranscriptId>,
}

impl ConfidentialThresholdReleaseFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let release_id = optional_hex_32_query(query, "releaseId")?;
        let transcript_id = optional_hex_32_query(query, "transcriptId")?;
        Ok(Self {
            release_id,
            transcript_id,
        })
    }

    pub(crate) fn matches(self, release: &ConfidentialThresholdReleaseState) -> bool {
        self.release_id
            .is_none_or(|release_id| release.release_id == release_id)
            && self
                .transcript_id
                .is_none_or(|transcript_id| release.transcript_id == transcript_id)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ConfidentialShamirThresholdReconstructionFilter {
    pub(crate) reconstruction_id: Option<SecureAggregateTranscriptId>,
    pub(crate) transcript_id: Option<SecureAggregateTranscriptId>,
}

impl ConfidentialShamirThresholdReconstructionFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let reconstruction_id = optional_hex_32_query(query, "reconstructionId")?;
        let transcript_id = optional_hex_32_query(query, "transcriptId")?;
        Ok(Self {
            reconstruction_id,
            transcript_id,
        })
    }

    pub(crate) fn matches(
        self,
        reconstruction: &ConfidentialShamirThresholdReconstructionState,
    ) -> bool {
        self.reconstruction_id
            .is_none_or(|reconstruction_id| reconstruction.reconstruction_id == reconstruction_id)
            && self
                .transcript_id
                .is_none_or(|transcript_id| reconstruction.transcript_id == transcript_id)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ConfidentialDifferentialPrivacyBudgetFilter {
    pub(crate) budget_id: Option<SecureAggregateTranscriptId>,
}

impl ConfidentialDifferentialPrivacyBudgetFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let budget_id = optional_hex_32_query(query, "budgetId")?;
        Ok(Self { budget_id })
    }

    pub(crate) fn matches(self, budget: &ConfidentialDifferentialPrivacyBudgetState) -> bool {
        self.budget_id
            .is_none_or(|budget_id| budget.budget_id == budget_id)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ConfidentialDifferentialPrivacyReleaseFilter {
    pub(crate) release: Option<SecureAggregateTranscriptId>,
    pub(crate) budget: Option<SecureAggregateTranscriptId>,
    pub(crate) transcript: Option<SecureAggregateTranscriptId>,
}

impl ConfidentialDifferentialPrivacyReleaseFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let release = optional_hex_32_query(query, "releaseId")?;
        let budget = optional_hex_32_query(query, "budgetId")?;
        let transcript = optional_hex_32_query(query, "transcriptId")?;
        Ok(Self {
            release,
            budget,
            transcript,
        })
    }

    pub(crate) fn matches(self, release: &ConfidentialDifferentialPrivacyReleaseState) -> bool {
        self.release
            .is_none_or(|release_id| release.release_id == release_id)
            && self
                .budget
                .is_none_or(|budget_id| release.budget_id == budget_id)
            && self
                .transcript
                .is_none_or(|transcript_id| release.transcript_id == transcript_id)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ConfidentialJobFilter {
    pub(crate) job_id: Option<SecureAggregateTranscriptId>,
    pub(crate) status: Option<ConfidentialJobStatus>,
    pub(crate) mechanism: Option<String>,
}

impl ConfidentialJobFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let job_id = optional_hex_32_query(query, "jobId")?;
        let status = query
            .get("status")
            .map(|value| {
                ConfidentialJobStatus::from_label(value).ok_or_else(|| {
                    HttpError::new(
                        StatusCode::BAD_REQUEST,
                        "confidential job status filter is not supported",
                    )
                })
            })
            .transpose()?;
        let mechanism = query.get("mechanism").cloned();
        Ok(Self {
            job_id,
            status,
            mechanism,
        })
    }

    pub(crate) fn matches(&self, job: &ConfidentialJobState) -> bool {
        self.job_id.is_none_or(|job_id| job.job_id == job_id)
            && self.status.is_none_or(|status| job.status == status)
            && self
                .mechanism
                .as_ref()
                .is_none_or(|mechanism| &job.mechanism == mechanism)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ConfidentialJobAuditEventFilter {
    pub(crate) job_id: Option<SecureAggregateTranscriptId>,
}

impl ConfidentialJobAuditEventFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        Ok(Self {
            job_id: optional_hex_32_query(query, "jobId")?,
        })
    }

    pub(crate) fn matches(self, event: &ConfidentialJobAuditEventState) -> bool {
        self.job_id.is_none_or(|job_id| event.job_id == job_id)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ConfidentialParticipantFilter {
    pub(crate) participant_subject: Option<String>,
    pub(crate) participant_index: Option<std::num::NonZeroUsize>,
    pub(crate) status: Option<ConfidentialParticipantStatus>,
}

impl ConfidentialParticipantFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let participant_subject = query.get("participantSubject").cloned();
        let participant_index = query
            .get("participantIndex")
            .map(|value| {
                value
                    .parse::<usize>()
                    .ok()
                    .and_then(std::num::NonZeroUsize::new)
                    .ok_or_else(|| {
                        HttpError::new(
                            StatusCode::BAD_REQUEST,
                            "participantIndex must be a positive integer",
                        )
                    })
            })
            .transpose()?;
        let status = query
            .get("status")
            .map(|value| {
                ConfidentialParticipantStatus::from_label(value).ok_or_else(|| {
                    HttpError::new(
                        StatusCode::BAD_REQUEST,
                        "confidential participant status filter is not supported",
                    )
                })
            })
            .transpose()?;
        Ok(Self {
            participant_subject,
            participant_index,
            status,
        })
    }

    pub(crate) fn matches(&self, participant: &ConfidentialParticipantState) -> bool {
        self.participant_subject
            .as_ref()
            .is_none_or(|subject| &participant.participant_subject == subject)
            && self
                .participant_index
                .is_none_or(|index| participant.participant_index == index)
            && self
                .status
                .is_none_or(|status| participant.status == status)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ConfidentialParticipantAuditEventFilter {
    pub(crate) participant_subject: Option<String>,
    pub(crate) participant_index: Option<std::num::NonZeroUsize>,
}

impl ConfidentialParticipantAuditEventFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let participant_index = query
            .get("participantIndex")
            .map(|value| {
                value
                    .parse::<usize>()
                    .ok()
                    .and_then(std::num::NonZeroUsize::new)
                    .ok_or_else(|| {
                        HttpError::new(
                            StatusCode::BAD_REQUEST,
                            "participantIndex must be a positive integer",
                        )
                    })
            })
            .transpose()?;
        Ok(Self {
            participant_subject: query.get("participantSubject").cloned(),
            participant_index,
        })
    }

    pub(crate) fn matches(&self, event: &ConfidentialParticipantAuditEventState) -> bool {
        self.participant_subject
            .as_ref()
            .is_none_or(|subject| &event.participant_subject == subject)
            && self
                .participant_index
                .is_none_or(|index| event.participant_index == index)
    }
}
