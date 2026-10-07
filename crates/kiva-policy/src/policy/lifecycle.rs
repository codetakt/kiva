#![allow(clippy::wildcard_imports)]

use super::*;

/// Operational lifecycle case states for correction, dispute, and tombstone workflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleCaseStatus {
    /// The subject or operator opened a governed case.
    Open,
    /// The case is under policy or custodian review.
    UnderReview,
    /// The requested correction or dispute was resolved.
    Resolved,
    /// The requested correction or dispute was rejected with a reason.
    Rejected,
    /// A metadata-only tombstone publication was recorded.
    Tombstoned,
}

/// Returns whether an M12 lifecycle-case transition is valid.
///
/// New cases must start open. Terminal states cannot be reopened or changed
/// except through idempotent resubmission of the same terminal status.
#[must_use]
pub const fn pds_lifecycle_case_transition_is_allowed(
    current: Option<LifecycleCaseStatus>,
    next: LifecycleCaseStatus,
) -> bool {
    match current {
        None => matches!(next, LifecycleCaseStatus::Open),
        Some(LifecycleCaseStatus::Open) => {
            matches!(
                next,
                LifecycleCaseStatus::Open | LifecycleCaseStatus::UnderReview
            )
        }
        Some(LifecycleCaseStatus::UnderReview) => matches!(
            next,
            LifecycleCaseStatus::UnderReview
                | LifecycleCaseStatus::Resolved
                | LifecycleCaseStatus::Rejected
                | LifecycleCaseStatus::Tombstoned
        ),
        Some(LifecycleCaseStatus::Resolved) => matches!(next, LifecycleCaseStatus::Resolved),
        Some(LifecycleCaseStatus::Rejected) => matches!(next, LifecycleCaseStatus::Rejected),
        Some(LifecycleCaseStatus::Tombstoned) => matches!(next, LifecycleCaseStatus::Tombstoned),
    }
}
