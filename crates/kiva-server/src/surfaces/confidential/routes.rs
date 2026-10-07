//! Confidential-computation HTTP routes.

use axum::{
    Router,
    routing::{get, post},
};

use crate::core::{
    paths::{
        CONFIDENTIAL_ADDITIVE_AGGREGATES_PATH, CONFIDENTIAL_ADDITIVE_SHARE_REPLAY_RECORDS_PATH,
        CONFIDENTIAL_ADDITIVE_SHARE_SUBMISSIONS_PATH,
        CONFIDENTIAL_DIFFERENTIAL_PRIVACY_BUDGET_SPENDS_PATH,
        CONFIDENTIAL_DIFFERENTIAL_PRIVACY_BUDGETS_PATH,
        CONFIDENTIAL_DIFFERENTIAL_PRIVACY_COMPOSITION_SUMMARIES_PATH,
        CONFIDENTIAL_DIFFERENTIAL_PRIVACY_RELEASES_PATH,
        CONFIDENTIAL_DIFFERENTIAL_PRIVACY_SAMPLED_RELEASES_PATH,
        CONFIDENTIAL_JOB_AUDIT_EVENTS_PATH, CONFIDENTIAL_JOB_TRANSITIONS_PATH,
        CONFIDENTIAL_JOBS_PATH, CONFIDENTIAL_PARTICIPANT_AUDIT_EVENTS_PATH,
        CONFIDENTIAL_PARTICIPANT_TRANSITIONS_PATH, CONFIDENTIAL_PARTICIPANTS_PATH,
        CONFIDENTIAL_SHAMIR_THRESHOLD_RECONSTRUCTIONS_PATH,
        CONFIDENTIAL_SHARE_SIGNING_KEY_REVOCATIONS_PATH,
        CONFIDENTIAL_SHARE_SIGNING_KEY_ROTATIONS_PATH, CONFIDENTIAL_SHARE_SIGNING_KEYS_PATH,
        CONFIDENTIAL_THRESHOLD_RELEASE_APPROVALS_PATH,
        CONFIDENTIAL_THRESHOLD_RELEASE_REQUESTS_PATH, CONFIDENTIAL_THRESHOLD_RELEASES_PATH,
    },
    state::ConfidentialHttpContext,
};

use super::handlers::{
    get_confidential_additive_share_replay_records, get_confidential_additive_share_submissions,
    get_confidential_differential_privacy_budgets,
    get_confidential_differential_privacy_composition_summaries,
    get_confidential_differential_privacy_releases, get_confidential_job_audit_events,
    get_confidential_jobs, get_confidential_participant_audit_events,
    get_confidential_participants, get_confidential_shamir_threshold_reconstructions,
    get_confidential_share_signing_keys, get_confidential_threshold_release_requests,
    post_confidential_additive_aggregate, post_confidential_additive_share_submission,
    post_confidential_differential_privacy_budget,
    post_confidential_differential_privacy_budget_spend,
    post_confidential_differential_privacy_release,
    post_confidential_differential_privacy_sampled_release, post_confidential_job,
    post_confidential_job_transition, post_confidential_participant,
    post_confidential_participant_transition, post_confidential_shamir_threshold_reconstruction,
    post_confidential_share_signing_key, post_confidential_share_signing_key_revocation,
    post_confidential_share_signing_key_rotation, post_confidential_threshold_release,
    post_confidential_threshold_release_approval, post_confidential_threshold_release_request,
};

pub(crate) fn router(state: ConfidentialHttpContext) -> Router {
    debug_assert_eq!(
        ConfidentialHttpContext::surface_profile_spec().application_surface,
        Some("confidential")
    );

    Router::new()
        .route(
            CONFIDENTIAL_ADDITIVE_AGGREGATES_PATH,
            post(post_confidential_additive_aggregate),
        )
        .route(
            CONFIDENTIAL_ADDITIVE_SHARE_SUBMISSIONS_PATH,
            get(get_confidential_additive_share_submissions)
                .post(post_confidential_additive_share_submission),
        )
        .route(
            CONFIDENTIAL_ADDITIVE_SHARE_REPLAY_RECORDS_PATH,
            get(get_confidential_additive_share_replay_records),
        )
        .route(
            CONFIDENTIAL_SHARE_SIGNING_KEYS_PATH,
            get(get_confidential_share_signing_keys).post(post_confidential_share_signing_key),
        )
        .route(
            CONFIDENTIAL_SHARE_SIGNING_KEY_ROTATIONS_PATH,
            post(post_confidential_share_signing_key_rotation),
        )
        .route(
            CONFIDENTIAL_SHARE_SIGNING_KEY_REVOCATIONS_PATH,
            post(post_confidential_share_signing_key_revocation),
        )
        .route(
            CONFIDENTIAL_THRESHOLD_RELEASE_REQUESTS_PATH,
            get(get_confidential_threshold_release_requests)
                .post(post_confidential_threshold_release_request),
        )
        .route(
            CONFIDENTIAL_THRESHOLD_RELEASE_APPROVALS_PATH,
            post(post_confidential_threshold_release_approval),
        )
        .route(
            CONFIDENTIAL_THRESHOLD_RELEASES_PATH,
            post(post_confidential_threshold_release),
        )
        .route(
            CONFIDENTIAL_SHAMIR_THRESHOLD_RECONSTRUCTIONS_PATH,
            get(get_confidential_shamir_threshold_reconstructions)
                .post(post_confidential_shamir_threshold_reconstruction),
        )
        .route(
            CONFIDENTIAL_DIFFERENTIAL_PRIVACY_BUDGETS_PATH,
            get(get_confidential_differential_privacy_budgets)
                .post(post_confidential_differential_privacy_budget),
        )
        .route(
            CONFIDENTIAL_DIFFERENTIAL_PRIVACY_BUDGET_SPENDS_PATH,
            post(post_confidential_differential_privacy_budget_spend),
        )
        .route(
            CONFIDENTIAL_DIFFERENTIAL_PRIVACY_COMPOSITION_SUMMARIES_PATH,
            get(get_confidential_differential_privacy_composition_summaries),
        )
        .route(
            CONFIDENTIAL_DIFFERENTIAL_PRIVACY_RELEASES_PATH,
            get(get_confidential_differential_privacy_releases)
                .post(post_confidential_differential_privacy_release),
        )
        .route(
            CONFIDENTIAL_DIFFERENTIAL_PRIVACY_SAMPLED_RELEASES_PATH,
            post(post_confidential_differential_privacy_sampled_release),
        )
        .route(
            CONFIDENTIAL_JOBS_PATH,
            get(get_confidential_jobs).post(post_confidential_job),
        )
        .route(
            CONFIDENTIAL_JOB_TRANSITIONS_PATH,
            post(post_confidential_job_transition),
        )
        .route(
            CONFIDENTIAL_JOB_AUDIT_EVENTS_PATH,
            get(get_confidential_job_audit_events),
        )
        .route(
            CONFIDENTIAL_PARTICIPANTS_PATH,
            get(get_confidential_participants).post(post_confidential_participant),
        )
        .route(
            CONFIDENTIAL_PARTICIPANT_TRANSITIONS_PATH,
            post(post_confidential_participant_transition),
        )
        .route(
            CONFIDENTIAL_PARTICIPANT_AUDIT_EVENTS_PATH,
            get(get_confidential_participant_audit_events),
        )
        .with_state(state)
}
