//! Memory confidential repository implementation modules split by resource family.

#![allow(clippy::wildcard_imports)]

use super::*;

#[derive(Debug, Clone, Default)]
pub(super) struct MemoryAdditiveStore {
    pub(super) replay_records:
        Arc<RwLock<BTreeMap<ConfidentialReplayRecordKey, AcceptedAdditiveShareRecord>>>,
    pub(super) signing_keys:
        Arc<RwLock<BTreeMap<ConfidentialSigningKeyStorageKey, ConfidentialShareSigningKey>>>,
    pub(super) submitted_shares:
        Arc<RwLock<BTreeMap<ConfidentialSubmittedShareKey, ConfidentialSubmittedShare>>>,
    pub(super) aggregate_results:
        Arc<RwLock<BTreeMap<ConfidentialAggregateResultKey, ConfidentialAdditiveAggregateResult>>>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct MemoryDifferentialPrivacyStore {
    pub(super) budgets: Arc<
        RwLock<
            BTreeMap<
                ConfidentialDifferentialPrivacyBudgetKey,
                ConfidentialDifferentialPrivacyBudgetState,
            >,
        >,
    >,
    pub(super) releases: Arc<
        RwLock<
            BTreeMap<
                ConfidentialDifferentialPrivacyReleaseKey,
                ConfidentialDifferentialPrivacyReleaseState,
            >,
        >,
    >,
}

#[derive(Debug, Clone, Default)]
pub(super) struct MemoryJobStore {
    pub(super) states: Arc<RwLock<BTreeMap<ConfidentialJobKey, ConfidentialJobState>>>,
    pub(super) audit_events:
        Arc<RwLock<BTreeMap<ConfidentialJobAuditEventKey, ConfidentialJobAuditEventState>>>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct MemoryParticipantStore {
    pub(super) states:
        Arc<RwLock<BTreeMap<ConfidentialParticipantKey, ConfidentialParticipantState>>>,
    pub(super) audit_events: Arc<
        RwLock<
            BTreeMap<ConfidentialParticipantAuditEventKey, ConfidentialParticipantAuditEventState>,
        >,
    >,
}

#[derive(Debug, Clone, Default)]
pub(super) struct MemoryShamirStore {
    pub(super) reconstructions: Arc<
        RwLock<
            BTreeMap<
                ConfidentialShamirThresholdReconstructionKey,
                ConfidentialShamirThresholdReconstructionState,
            >,
        >,
    >,
}

#[derive(Debug, Clone, Default)]
pub(super) struct MemoryThresholdStore {
    pub(super) releases:
        Arc<RwLock<BTreeMap<ConfidentialThresholdReleaseKey, ConfidentialThresholdReleaseState>>>,
}

mod additive;
mod differential_privacy;
mod jobs;
mod participants;
mod release_artifacts;
mod shamir;
mod threshold;
