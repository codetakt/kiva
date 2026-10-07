#![allow(clippy::too_many_lines)]

use std::collections::BTreeMap;
use std::num::{NonZeroU64, NonZeroUsize};

use super::*;
use axum::{
    body::{Bytes, to_bytes},
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::AUTHORIZATION},
};
use ed25519_dalek::{Signer, SigningKey};
use kiva_confidential::{
    mechanism::{
        additive::{
            AdditiveShareCommitmentSlot, AdditiveShareReplayWindow,
            AuthenticatedAdditiveShareEnvelope, SecureAggregateShare,
            SignedAuthenticatedAdditiveShareEnvelope, additive_share_envelope_signature_payload,
        },
        job::ConfidentialJobStatus,
        shamir::{
            ShamirShare, ShamirShareOpeningNonce, VerifiableShamirShare,
            derive_shamir_share_opening_commitment,
        },
    },
    primitive::{
        SECURE_AGGREGATE_ARTIFACT_BYTES, SecureAggregateCommitment,
        SecureAggregateEnvelopeSequence, SecureAggregateFieldElement, SecureAggregateSignature,
        SecureAggregateSigningKeyId, SecureAggregateSigningPublicKey, SecureAggregateTranscriptId,
        SecureAggregateTransportProof,
    },
};
use kiva_identity::{OAuthScope, ResourceServerOperation};
use serde_json::{Value, json};
use time::{Date, Duration, Month, OffsetDateTime, Time};
use uuid::Uuid;

#[path = "tests/helpers/analytics.rs"]
mod analytics_helpers;
#[path = "tests/helpers/confidential.rs"]
mod confidential_helpers;
#[path = "tests/helpers/confidential_openapi.rs"]
mod confidential_openapi_helpers;
#[path = "tests/helpers/fixtures.rs"]
mod fixture_helpers;
#[path = "tests/helpers/learner_principal.rs"]
mod learner_principal_helpers;
#[path = "tests/helpers/pds.rs"]
mod pds_helpers;
#[path = "tests/helpers/pds_portability.rs"]
mod pds_portability_helpers;
#[path = "tests/helpers/request_dsl.rs"]
mod request_dsl_helpers;
#[path = "tests/helpers/state.rs"]
mod state_helpers;

use analytics_helpers::*;
use confidential_helpers::*;
use confidential_openapi_helpers::*;
use fixture_helpers::*;
use learner_principal_helpers::*;
use pds_helpers::*;
use pds_portability_helpers::*;
use request_dsl_helpers::*;
use state_helpers::*;

#[path = "tests/analytics_memory.rs"]
mod analytics_memory;
#[path = "tests/confidential_memory.rs"]
mod confidential_memory;
#[path = "tests/pds_memory.rs"]
mod pds_memory;
#[path = "tests/postgres.rs"]
mod postgres;
#[path = "tests/xapi_auth.rs"]
mod xapi_auth;

#[test]
fn router_builds_with_split_pds_contexts() {
    let _router = router();
}
