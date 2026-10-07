//! Confidential additive server-local state.

#![allow(clippy::wildcard_imports)]

use super::super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfidentialAdditiveAggregateResult {
    pub(crate) transcript_id: SecureAggregateTranscriptId,
    pub(crate) aggregate: SecureAggregateFieldElement,
    pub(crate) participant_count: std::num::NonZeroUsize,
    pub(crate) computed_at: OffsetDateTime,
}

impl ConfidentialAdditiveAggregateResult {
    pub(crate) fn from_aggregate(
        participant_count: std::num::NonZeroUsize,
        transcript_id: SecureAggregateTranscriptId,
        aggregate: &ReplayWindowedAdditiveAggregate,
    ) -> Self {
        Self {
            transcript_id,
            aggregate: aggregate.aggregate(),
            participant_count,
            computed_at: OffsetDateTime::now_utc(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfidentialSubmittedShare {
    pub(crate) oauth_subject: String,
    pub(crate) signed_envelope: SignedAuthenticatedAdditiveShareEnvelope,
}

impl ConfidentialSubmittedShare {
    pub(crate) fn from_submission(
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSubmissionRequest,
    ) -> Self {
        Self {
            oauth_subject: principal.subject.clone(),
            signed_envelope: request.signed_envelope,
        }
    }
}
