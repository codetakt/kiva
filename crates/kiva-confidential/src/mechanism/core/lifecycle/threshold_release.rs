#![allow(clippy::wildcard_imports)]

use super::super::*;

/// Side of a two-authority threshold release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdReleaseAuthoritySide {
    /// The left policy authority.
    Left,
    /// The right policy authority.
    Right,
}

/// Pure approval state for a two-authority threshold release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThresholdReleaseApprovalSet {
    left: bool,
    right: bool,
}

impl ThresholdReleaseApprovalSet {
    /// Creates an empty approval set.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            left: false,
            right: false,
        }
    }

    /// Creates an approval set from explicit left/right approval flags.
    #[must_use]
    pub const fn new(left: bool, right: bool) -> Self {
        Self { left, right }
    }

    /// Returns whether the left authority has approved the release.
    #[must_use]
    pub const fn has_left(self) -> bool {
        self.left
    }

    /// Returns whether the right authority has approved the release.
    #[must_use]
    pub const fn has_right(self) -> bool {
        self.right
    }

    /// Returns whether the two-authority threshold is met.
    #[must_use]
    pub const fn is_threshold_met(self) -> bool {
        self.left && self.right
    }

    /// Records an approval from one authority side.
    #[must_use]
    pub const fn approve(self, side: ThresholdReleaseAuthoritySide) -> Self {
        match side {
            ThresholdReleaseAuthoritySide::Left => Self {
                left: true,
                right: self.right,
            },
            ThresholdReleaseAuthoritySide::Right => Self {
                left: self.left,
                right: true,
            },
        }
    }
}

/// Pure threshold-release transition validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ThresholdReleaseTransitionError {
    /// The threshold release request does not name two distinct authorities.
    #[error("threshold release requires two distinct non-empty authorities")]
    InvalidAuthoritySet,

    /// The acting principal is not one of the configured authorities.
    #[error("threshold release actor is not an authority")]
    Forbidden,

    /// The release is not bound to a persisted computed aggregate.
    #[error("threshold release requires a matching computed aggregate")]
    MissingComputedAggregate,

    /// The two-authority approval threshold is not met.
    #[error("threshold release requires both authority approvals")]
    MissingApproval,
}

/// Validates the structural authority set for a two-authority release request.
pub const fn validate_threshold_release_authority_set(
    left_present: bool,
    right_present: bool,
    authorities_distinct: bool,
) -> Result<(), ThresholdReleaseTransitionError> {
    if !left_present || !right_present || !authorities_distinct {
        Err(ThresholdReleaseTransitionError::InvalidAuthoritySet)
    } else {
        Ok(())
    }
}

/// Classifies an acting principal into exactly one threshold-release authority side.
pub const fn classify_threshold_release_authority(
    is_left_authority: bool,
    is_right_authority: bool,
) -> Result<ThresholdReleaseAuthoritySide, ThresholdReleaseTransitionError> {
    if is_left_authority && is_right_authority {
        Err(ThresholdReleaseTransitionError::InvalidAuthoritySet)
    } else if is_left_authority {
        Ok(ThresholdReleaseAuthoritySide::Left)
    } else if is_right_authority {
        Ok(ThresholdReleaseAuthoritySide::Right)
    } else {
        Err(ThresholdReleaseTransitionError::Forbidden)
    }
}

/// Validates that a release request is bound to a persisted computed aggregate.
pub const fn validate_threshold_release_computed_aggregate(
    computed_aggregate_matches: bool,
) -> Result<(), ThresholdReleaseTransitionError> {
    if computed_aggregate_matches {
        Ok(())
    } else {
        Err(ThresholdReleaseTransitionError::MissingComputedAggregate)
    }
}

/// Records one threshold-release approval in pure state.
#[must_use]
pub const fn record_threshold_release_approval(
    approvals: ThresholdReleaseApprovalSet,
    side: ThresholdReleaseAuthoritySide,
) -> ThresholdReleaseApprovalSet {
    approvals.approve(side)
}

/// Validates that the two-authority approval threshold is met.
pub const fn validate_threshold_release_approval_threshold(
    approvals: ThresholdReleaseApprovalSet,
) -> Result<(), ThresholdReleaseTransitionError> {
    if approvals.is_threshold_met() {
        Ok(())
    } else {
        Err(ThresholdReleaseTransitionError::MissingApproval)
    }
}

/// Validates the final release transition for an acting authority.
pub const fn validate_threshold_release_completion(
    approvals: ThresholdReleaseApprovalSet,
    is_left_authority: bool,
    is_right_authority: bool,
) -> Result<(), ThresholdReleaseTransitionError> {
    match classify_threshold_release_authority(is_left_authority, is_right_authority) {
        Ok(_side) => validate_threshold_release_approval_threshold(approvals),
        Err(error) => Err(error),
    }
}
