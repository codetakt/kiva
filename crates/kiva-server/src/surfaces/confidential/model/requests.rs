#![allow(clippy::wildcard_imports)]

use super::*;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ConfidentialAdditiveAggregateRequest {
    pub(crate) participant_count: std::num::NonZeroUsize,
    pub(crate) transcript_id: SecureAggregateTranscriptId,
    pub(crate) slots: Vec<AdditiveShareCommitmentSlot>,
    pub(crate) replay_window: AdditiveShareReplayWindow,
    #[serde(default)]
    pub(crate) signed_envelopes: Vec<SignedAuthenticatedAdditiveShareEnvelope>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub(crate) struct ConfidentialShareSubmissionRequest {
    pub(crate) signed_envelope: SignedAuthenticatedAdditiveShareEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ConfidentialThresholdReleaseRequest {
    pub(crate) release_id: SecureAggregateTranscriptId,
    pub(crate) transcript_id: SecureAggregateTranscriptId,
    pub(crate) aggregate: SecureAggregateFieldElement,
    pub(crate) left_authority_subject: String,
    pub(crate) right_authority_subject: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub(crate) struct ConfidentialThresholdReleaseApprovalRequest {
    pub(crate) release_id: SecureAggregateTranscriptId,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ConfidentialShamirThresholdReconstructionRequest {
    pub(crate) reconstruction_id: SecureAggregateTranscriptId,
    pub(crate) transcript_id: SecureAggregateTranscriptId,
    pub(crate) participant_count: NonZeroUsize,
    pub(crate) threshold: NonZeroUsize,
    pub(crate) share_openings: Vec<VerifiableShamirShare>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub(crate) struct ConfidentialDifferentialPrivacyBudgetRequest {
    pub(crate) budget_id: SecureAggregateTranscriptId,
    pub(crate) total_epsilon_microunits: NonZeroU64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub(crate) struct ConfidentialDifferentialPrivacyBudgetSpendRequest {
    pub(crate) budget_id: SecureAggregateTranscriptId,
    pub(crate) epsilon_microunits: NonZeroU64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub(crate) struct ConfidentialDifferentialPrivacyReleaseRequest {
    pub(crate) release_id: SecureAggregateTranscriptId,
    pub(crate) budget_id: SecureAggregateTranscriptId,
    pub(crate) transcript_id: SecureAggregateTranscriptId,
    pub(crate) aggregate: SecureAggregateFieldElement,
    pub(crate) data_subject_count: NonZeroUsize,
    pub(crate) minimum_population: NonZeroUsize,
    pub(crate) epsilon_microunits: NonZeroU64,
    pub(crate) l1_sensitivity: NonZeroU64,
    pub(crate) noise_value: i64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub(crate) struct ConfidentialDifferentialPrivacySampledReleaseRequest {
    pub(crate) release_id: SecureAggregateTranscriptId,
    pub(crate) budget_id: SecureAggregateTranscriptId,
    pub(crate) transcript_id: SecureAggregateTranscriptId,
    pub(crate) data_subject_count: NonZeroUsize,
    pub(crate) minimum_population: NonZeroUsize,
    pub(crate) epsilon_microunits: NonZeroU64,
    pub(crate) l1_sensitivity: NonZeroU64,
}

impl ConfidentialDifferentialPrivacySampledReleaseRequest {
    pub(crate) const fn into_release_request(
        self,
        aggregate: SecureAggregateFieldElement,
        noise_value: i64,
    ) -> ConfidentialDifferentialPrivacyReleaseRequest {
        ConfidentialDifferentialPrivacyReleaseRequest {
            release_id: self.release_id,
            budget_id: self.budget_id,
            transcript_id: self.transcript_id,
            aggregate,
            data_subject_count: self.data_subject_count,
            minimum_population: self.minimum_population,
            epsilon_microunits: self.epsilon_microunits,
            l1_sensitivity: self.l1_sensitivity,
            noise_value,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ConfidentialJobRequest {
    pub(crate) job_id: SecureAggregateTranscriptId,
    pub(crate) mechanism: String,
    pub(crate) mechanism_profile: String,
    pub(crate) workload_id: String,
    pub(crate) participant_subjects: Vec<String>,
    pub(crate) policy_binding_id: String,
    pub(crate) deadline_at: String,
    pub(crate) admission_quota_key: String,
    #[serde(default = "default_confidential_job_max_retry_count")]
    pub(crate) max_retry_count: i64,
    #[serde(default)]
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ConfidentialJobTransitionRequest {
    pub(crate) job_id: SecureAggregateTranscriptId,
    pub(crate) target_status: ConfidentialJobStatus,
    #[serde(default)]
    pub(crate) reason: Option<String>,
    #[serde(default)]
    pub(crate) evidence_refs: Vec<String>,
}

pub(crate) const fn default_confidential_job_max_retry_count() -> i64 {
    3
}
