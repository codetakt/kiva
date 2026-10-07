//! Confidential threshold server-local state.

#![allow(clippy::wildcard_imports)]

use super::super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfidentialThresholdReleaseApproval {
    pub(crate) authority_subject: String,
    pub(crate) approved_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfidentialThresholdReleaseState {
    pub(crate) release_id: SecureAggregateTranscriptId,
    pub(crate) transcript_id: SecureAggregateTranscriptId,
    pub(crate) aggregate: SecureAggregateFieldElement,
    pub(crate) requester_subject: String,
    pub(crate) left_authority_subject: String,
    pub(crate) right_authority_subject: String,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) released_at: Option<OffsetDateTime>,
    pub(crate) approvals: Vec<ConfidentialThresholdReleaseApproval>,
}

impl ConfidentialThresholdReleaseState {
    pub(crate) fn from_request(
        principal: &AuthenticatedPrincipal,
        request: ConfidentialThresholdReleaseRequest,
    ) -> Result<Self, ConfidentialThresholdReleaseError> {
        let release = Self {
            release_id: request.release_id,
            transcript_id: request.transcript_id,
            aggregate: request.aggregate,
            requester_subject: principal.subject.clone(),
            left_authority_subject: request.left_authority_subject,
            right_authority_subject: request.right_authority_subject,
            created_at: OffsetDateTime::now_utc(),
            released_at: None,
            approvals: Vec::new(),
        };
        release.validate_authority_set()?;
        Ok(release)
    }

    pub(crate) fn same_request(&self, other: &Self) -> bool {
        self.release_id == other.release_id
            && self.transcript_id == other.transcript_id
            && self.aggregate == other.aggregate
            && self.requester_subject == other.requester_subject
            && self.left_authority_subject == other.left_authority_subject
            && self.right_authority_subject == other.right_authority_subject
    }

    pub(crate) fn validate_authority_set(&self) -> Result<(), ConfidentialThresholdReleaseError> {
        validate_threshold_release_authority_set(
            !self.left_authority_subject.trim().is_empty(),
            !self.right_authority_subject.trim().is_empty(),
            self.left_authority_subject != self.right_authority_subject,
        )
        .map_err(ConfidentialThresholdReleaseError::from)
    }

    pub(crate) fn ensure_authority(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<(), ConfidentialThresholdReleaseError> {
        self.authority_side(principal).map(|_side| ())
    }

    pub(crate) fn ensure_threshold_approved(
        &self,
    ) -> Result<(), ConfidentialThresholdReleaseError> {
        validate_threshold_release_approval_threshold(self.approval_set())
            .map_err(ConfidentialThresholdReleaseError::from)
    }

    pub(crate) fn authority_side(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<ThresholdReleaseAuthoritySide, ConfidentialThresholdReleaseError> {
        classify_threshold_release_authority(
            principal.subject == self.left_authority_subject,
            principal.subject == self.right_authority_subject,
        )
        .map_err(ConfidentialThresholdReleaseError::from)
    }

    pub(crate) fn approval_set(&self) -> ThresholdReleaseApprovalSet {
        let mut approvals = ThresholdReleaseApprovalSet::empty();
        for approval in &self.approvals {
            if approval.authority_subject == self.left_authority_subject {
                approvals = record_threshold_release_approval(
                    approvals,
                    ThresholdReleaseAuthoritySide::Left,
                );
            } else if approval.authority_subject == self.right_authority_subject {
                approvals = record_threshold_release_approval(
                    approvals,
                    ThresholdReleaseAuthoritySide::Right,
                );
            }
        }
        approvals
    }

    pub(crate) fn sort_approvals(&mut self) {
        self.approvals
            .sort_by(|left, right| left.authority_subject.cmp(&right.authority_subject));
    }
}
