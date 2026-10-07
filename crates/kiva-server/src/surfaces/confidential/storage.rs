//! Confidential-computation memory and `PostgreSQL` storage implementations.
#![allow(clippy::wildcard_imports)]

use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    num::{NonZeroU64, NonZeroUsize},
    sync::Arc,
};

use kiva_confidential::{
    mechanism::{
        additive::{
            AcceptedAdditiveShareRecord, AdditiveShareCommitmentSlot,
            ReplayWindowedAdditiveAggregate, SecureAggregateError,
            SignedAuthenticatedAdditiveShareEnvelope,
            combine_signed_authenticated_additive_share_envelopes,
            verify_signed_authenticated_additive_share_envelope,
        },
        differential_privacy::{
            DifferentialPrivacyAccountingScope, DifferentialPrivacyBudgetError,
            DifferentialPrivacyBudgetLedger, DifferentialPrivacyNoiseMechanism,
            DifferentialPrivacyNoiseParameters, DifferentialPrivacyReleaseContext,
            DifferentialPrivacyReleasePolicy, apply_differential_privacy_integer_noise,
            spend_differential_privacy_budget as spend_differential_privacy_budget_ledger,
            validate_differential_privacy_release,
        },
        job::{ConfidentialJobStatus, transition_confidential_job_status},
        shamir::ShamirThresholdPlan,
        threshold::validate_threshold_release_computed_aggregate,
        workflow::{
            ConfidentialParticipantStatus, ReleasePolicy, confidential_participant_accepts_jobs,
            confidential_participant_accepts_new_keys, transition_confidential_participant_status,
        },
    },
    primitive::{
        AdditiveShareVerificationKey, SecureAggregateEnvelopeSequence, SecureAggregateFieldElement,
        SecureAggregateSigningKeyId, SecureAggregateSigningPublicKey, SecureAggregateTranscriptId,
    },
};
use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::{
    ReleaseArtifactKind as PdsReleaseArtifactKind, ReleaseArtifactRef as PdsReleaseArtifactRef,
};
use sqlx::{PgPool, types::Json as SqlJson};
use time::OffsetDateTime;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::core::{
    config::{CONFIDENTIAL_SHAMIR_VERIFICATION_PROFILE, KIVA_TENANT_ID_ENV},
    outcome::*,
};
use crate::surfaces::{
    confidential::{model::*, outcome::*},
    pds::release_artifacts::release_artifact_available_from_metadata,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct MemoryConfidentialRepository {
    additive: memory::MemoryAdditiveStore,
    differential_privacy: memory::MemoryDifferentialPrivacyStore,
    jobs: memory::MemoryJobStore,
    participants: memory::MemoryParticipantStore,
    shamir: memory::MemoryShamirStore,
    threshold: memory::MemoryThresholdStore,
}

#[derive(Debug, Clone)]
pub(crate) struct PostgresConfidentialRepository {
    pool: PgPool,
    tenant_id: String,
}

fn pds_release_artifact_transcript_id(
    artifact: PdsReleaseArtifactRef,
) -> Option<SecureAggregateTranscriptId> {
    SecureAggregateTranscriptId::new(artifact.artifact_id()).ok()
}

mod common;
mod memory;
mod postgres;
mod rows;

use common::*;
pub(crate) use common::{ensure_matching_differential_privacy_aggregate_result, field_element_i64};
use rows::*;
