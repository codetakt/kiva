#![allow(clippy::wildcard_imports)]

use super::*;

#[kani::proof]
pub(super) fn shamir_reconstruction_rejects_insufficient_or_duplicate_shares() {
    let plan = fixed_shamir_plan();
    let first = bounded_shamir_share(NonZeroUsize::MIN, kani::any());
    let duplicate = bounded_shamir_share(NonZeroUsize::MIN, kani::any());

    let insufficient = [first];
    assert!(matches!(
        reconstruct_shamir_secret(plan, &insufficient),
        Err(SecureAggregateError::ShamirShareCountBelowThreshold)
    ));

    let duplicate_shares = [first, duplicate];
    assert!(matches!(
        reconstruct_shamir_secret(plan, &duplicate_shares),
        Err(SecureAggregateError::DuplicateParticipantIndex)
    ));
}

#[kani::proof]
pub(super) fn shamir_share_complaint_resolution_fails_closed() {
    let opening_verified: bool = kani::any();
    let outcome = resolve_shamir_share_complaint(opening_verified);

    if opening_verified {
        assert!(matches!(outcome, ShamirShareComplaintOutcome::Rejected));
    } else {
        assert!(matches!(outcome, ShamirShareComplaintOutcome::Sustained));
    }
}
