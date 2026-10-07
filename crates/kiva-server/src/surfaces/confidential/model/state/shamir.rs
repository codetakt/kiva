//! Confidential shamir server-local state.

#![allow(clippy::wildcard_imports)]

use super::super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfidentialShamirThresholdReconstructionState {
    pub(crate) reconstruction_id: SecureAggregateTranscriptId,
    pub(crate) transcript_id: SecureAggregateTranscriptId,
    pub(crate) owner_subject: String,
    pub(crate) participant_count: NonZeroUsize,
    pub(crate) threshold: NonZeroUsize,
    pub(crate) share_count: usize,
    pub(crate) verification_profile: String,
    pub(crate) verified_share_count: usize,
    pub(crate) complaint_count: usize,
    pub(crate) reconstructed_value: SecureAggregateFieldElement,
    pub(crate) created_at: OffsetDateTime,
}

impl ConfidentialShamirThresholdReconstructionState {
    pub(crate) fn from_request(
        principal: &AuthenticatedPrincipal,
        request: &ConfidentialShamirThresholdReconstructionRequest,
    ) -> Result<Self, ConfidentialShamirReconstructionError> {
        let release_policy = ReleasePolicy::new(request.participant_count, request.threshold)
            .map_err(ConfidentialShamirReconstructionError::Workflow)?;
        let plan = ShamirThresholdPlan::new(release_policy)
            .map_err(ConfidentialShamirReconstructionError::Secure)?;
        let reconstructed_value = reconstruct_verified_shamir_secret(
            plan,
            request.transcript_id,
            &request.share_openings,
        )
        .map_err(ConfidentialShamirReconstructionError::Secure)?;

        Ok(Self {
            reconstruction_id: request.reconstruction_id,
            transcript_id: request.transcript_id,
            owner_subject: principal.subject.clone(),
            participant_count: request.participant_count,
            threshold: request.threshold,
            share_count: request.share_openings.len(),
            verification_profile: CONFIDENTIAL_SHAMIR_VERIFICATION_PROFILE.to_owned(),
            verified_share_count: request.share_openings.len(),
            complaint_count: 0,
            reconstructed_value,
            created_at: OffsetDateTime::now_utc(),
        })
    }

    pub(crate) fn same_reconstruction(&self, other: &Self) -> bool {
        self.reconstruction_id == other.reconstruction_id
            && self.transcript_id == other.transcript_id
            && self.owner_subject == other.owner_subject
            && self.participant_count == other.participant_count
            && self.threshold == other.threshold
            && self.share_count == other.share_count
            && self.verification_profile == other.verification_profile
            && self.verified_share_count == other.verified_share_count
            && self.complaint_count == other.complaint_count
            && self.reconstructed_value == other.reconstructed_value
    }
}
