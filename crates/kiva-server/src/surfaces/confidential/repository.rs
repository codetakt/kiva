//! Confidential-computation repository facade.
#![allow(clippy::wildcard_imports)]

use std::io;

use kiva_confidential::{
    mechanism::additive::{AcceptedAdditiveShareRecord, ReplayWindowedAdditiveAggregate},
    primitive::SecureAggregateTranscriptId,
};
use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::ReleaseArtifactRef as PdsReleaseArtifactRef;
use sqlx::PgPool;

use crate::core::outcome::StoreOutcome;
use crate::surfaces::confidential::{
    model::*,
    outcome::{
        ConfidentialAggregateError, ConfidentialAggregateStoreError,
        ConfidentialDifferentialPrivacyBudgetError, ConfidentialDifferentialPrivacyReleaseError,
        ConfidentialJobError, ConfidentialJobRegistrationOutcome, ConfidentialJobTransitionOutcome,
        ConfidentialParticipantError, ConfidentialParticipantRegistrationOutcome,
        ConfidentialParticipantTransitionOutcome, ConfidentialShamirReconstructionError,
        ConfidentialShareSubmissionError, ConfidentialThresholdReleaseError,
    },
    storage::{MemoryConfidentialRepository, PostgresConfidentialRepository},
};

#[derive(Debug, Clone)]
pub(crate) enum ConfidentialRepository {
    Memory(MemoryConfidentialRepository),
    Postgres(PostgresConfidentialRepository),
}

impl Default for ConfidentialRepository {
    fn default() -> Self {
        Self::Memory(MemoryConfidentialRepository::default())
    }
}

mod additive;
mod core;
mod differential_privacy;
mod jobs;
mod participants;
mod shamir;
mod threshold;
