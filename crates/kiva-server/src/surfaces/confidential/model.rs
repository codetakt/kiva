//! Confidential-computation server-local model and validation helpers.

use std::collections::{BTreeMap, BTreeSet};
use std::num::{NonZeroU64, NonZeroUsize};

use axum::http::StatusCode;
use kiva_confidential::{
    mechanism::{
        additive::{
            AcceptedAdditiveShareRecord, AdditiveShareCommitmentSlot, AdditiveShareReplayWindow,
            ReplayWindowedAdditiveAggregate, SignedAuthenticatedAdditiveShareEnvelope,
        },
        differential_privacy::{
            DifferentialPrivacyAccountingScope, DifferentialPrivacyAdvancedCompositionBound,
            DifferentialPrivacyBudgetLedger, DifferentialPrivacyCompositionSummary,
            DifferentialPrivacyNoiseMechanism, DifferentialPrivacyNoiseParameters,
            DifferentialPrivacyNoisedAggregate, DifferentialPrivacyReleaseContext,
            DifferentialPrivacyReleasePolicy, apply_differential_privacy_integer_noise,
            bound_differential_privacy_advanced_composition,
            sample_differential_privacy_discrete_laplace_noise_from_uniforms,
            summarize_differential_privacy_composition, validate_differential_privacy_release,
        },
        job::{
            ConfidentialJobAdmissionError, ConfidentialJobAdmissionFacts, ConfidentialJobStatus,
            ConfidentialJobTransitionError, confidential_job_transition_is_allowed,
            validate_confidential_job_admission,
        },
        shamir::{ShamirThresholdPlan, VerifiableShamirShare, reconstruct_verified_shamir_secret},
        threshold::{
            ThresholdReleaseApprovalSet, ThresholdReleaseAuthoritySide,
            classify_threshold_release_authority, record_threshold_release_approval,
            validate_threshold_release_approval_threshold,
            validate_threshold_release_authority_set,
        },
        workflow::{ConfidentialParticipantStatus, ReleasePolicy},
    },
    primitive::{
        AdditiveShareVerificationKey, SecureAggregateFieldElement, SecureAggregateSigningKeyId,
        SecureAggregateSigningPublicKey, SecureAggregateTranscriptId,
    },
};
use kiva_identity::AuthenticatedPrincipal;
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::core::{
    config::{
        CONFIDENTIAL_DIFFERENTIAL_PRIVACY_ADVANCED_DELTA_MICROUNITS,
        CONFIDENTIAL_JOB_MAX_ACTIVE_PER_QUOTA_KEY, CONFIDENTIAL_JOB_MAX_RUNNING_PER_TENANT,
        CONFIDENTIAL_SHAMIR_VERIFICATION_PROFILE,
    },
    request_util::optional_hex_32_query,
    response::{HttpError, postgres_microsecond_timestamp},
};
use crate::surfaces::confidential::outcome::{
    ConfidentialAggregateStoreError, ConfidentialDifferentialPrivacyBudgetError,
    ConfidentialDifferentialPrivacyReleaseError, ConfidentialJobError,
    ConfidentialParticipantError, ConfidentialShamirReconstructionError,
    ConfidentialThresholdReleaseError,
};
use crate::surfaces::confidential::storage::{
    ensure_matching_differential_privacy_aggregate_result, field_element_i64,
};

mod differential_privacy_helpers;
mod filters;
mod keys;
mod requests;
mod state;
mod validation;

pub(crate) use differential_privacy_helpers::*;
pub(crate) use filters::*;
pub(crate) use keys::*;
pub(crate) use requests::*;
pub(crate) use state::*;
pub(crate) use validation::*;
