#![allow(clippy::wildcard_imports)]

use super::*;

#[kani::proof]
pub(super) fn threshold_release_authority_set_rejects_missing_or_ambiguous_authorities() {
    let left_present: bool = kani::any();
    let right_present: bool = kani::any();
    let authorities_distinct: bool = kani::any();

    let result =
        validate_threshold_release_authority_set(left_present, right_present, authorities_distinct);

    if left_present && right_present && authorities_distinct {
        assert!(result.is_ok());
    } else {
        assert!(matches!(
            result,
            Err(ThresholdReleaseTransitionError::InvalidAuthoritySet)
        ));
    }
}

#[kani::proof]
pub(super) fn threshold_release_authority_classification_is_unambiguous() {
    let is_left_authority: bool = kani::any();
    let is_right_authority: bool = kani::any();

    let result = classify_threshold_release_authority(is_left_authority, is_right_authority);

    if is_left_authority && is_right_authority {
        assert!(matches!(
            result,
            Err(ThresholdReleaseTransitionError::InvalidAuthoritySet)
        ));
    } else if is_left_authority {
        assert!(matches!(result, Ok(ThresholdReleaseAuthoritySide::Left)));
    } else if is_right_authority {
        assert!(matches!(result, Ok(ThresholdReleaseAuthoritySide::Right)));
    } else {
        assert!(matches!(
            result,
            Err(ThresholdReleaseTransitionError::Forbidden)
        ));
    }
}

#[kani::proof]
pub(super) fn threshold_release_duplicate_approval_is_idempotent() {
    let approve_left: bool = kani::any();
    let side = if approve_left {
        ThresholdReleaseAuthoritySide::Left
    } else {
        ThresholdReleaseAuthoritySide::Right
    };

    let once = record_threshold_release_approval(ThresholdReleaseApprovalSet::empty(), side);
    let twice = record_threshold_release_approval(once, side);

    assert_eq!(once, twice);
    assert!(!twice.is_threshold_met());
}

#[kani::proof]
pub(super) fn threshold_release_requires_both_authority_approvals() {
    let left_approved: bool = kani::any();
    let right_approved: bool = kani::any();
    let approvals = ThresholdReleaseApprovalSet::new(left_approved, right_approved);

    let result = validate_threshold_release_approval_threshold(approvals);

    if left_approved && right_approved {
        assert!(result.is_ok());
    } else {
        assert!(matches!(
            result,
            Err(ThresholdReleaseTransitionError::MissingApproval)
        ));
    }
}

#[kani::proof]
pub(super) fn threshold_release_request_requires_matching_computed_aggregate() {
    let computed_aggregate_matches: bool = kani::any();

    let result = validate_threshold_release_computed_aggregate(computed_aggregate_matches);

    if computed_aggregate_matches {
        assert!(result.is_ok());
    } else {
        assert!(matches!(
            result,
            Err(ThresholdReleaseTransitionError::MissingComputedAggregate)
        ));
    }
}

#[kani::proof]
pub(super) fn threshold_release_completion_requires_actor_and_full_threshold() {
    let left_approved: bool = kani::any();
    let right_approved: bool = kani::any();
    let is_left_authority: bool = kani::any();
    let is_right_authority: bool = kani::any();
    let approvals = ThresholdReleaseApprovalSet::new(left_approved, right_approved);

    let result =
        validate_threshold_release_completion(approvals, is_left_authority, is_right_authority);

    if is_left_authority && is_right_authority {
        assert!(matches!(
            result,
            Err(ThresholdReleaseTransitionError::InvalidAuthoritySet)
        ));
    } else if !is_left_authority && !is_right_authority {
        assert!(matches!(
            result,
            Err(ThresholdReleaseTransitionError::Forbidden)
        ));
    } else if left_approved && right_approved {
        assert!(result.is_ok());
    } else {
        assert!(matches!(
            result,
            Err(ThresholdReleaseTransitionError::MissingApproval)
        ));
    }
}
