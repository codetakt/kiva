#![allow(clippy::wildcard_imports)]

use super::*;

#[test]
fn threshold_release_requires_matching_computed_aggregate() {
    assert_eq!(
        validate_threshold_release_computed_aggregate(false),
        Err(ThresholdReleaseTransitionError::MissingComputedAggregate)
    );
    assert_eq!(validate_threshold_release_computed_aggregate(true), Ok(()));
}

#[test]
fn threshold_release_duplicate_approval_is_idempotent() {
    let once = record_threshold_release_approval(
        ThresholdReleaseApprovalSet::empty(),
        ThresholdReleaseAuthoritySide::Left,
    );
    let twice = record_threshold_release_approval(once, ThresholdReleaseAuthoritySide::Left);

    assert_eq!(once, twice);
    assert_eq!(
        validate_threshold_release_approval_threshold(twice),
        Err(ThresholdReleaseTransitionError::MissingApproval)
    );
}

#[test]
fn threshold_release_authority_classification_is_exact() {
    assert_eq!(
        classify_threshold_release_authority(true, false),
        Ok(ThresholdReleaseAuthoritySide::Left)
    );
    assert_eq!(
        classify_threshold_release_authority(false, true),
        Ok(ThresholdReleaseAuthoritySide::Right)
    );
    assert_eq!(
        classify_threshold_release_authority(false, false),
        Err(ThresholdReleaseTransitionError::Forbidden)
    );
}

#[test]
fn signing_key_rotation_revokes_old_key_and_activates_new_key() {
    let precondition = ShareSigningKeyRotationPrecondition::new(
        ShareSigningKeyLifecycleState::Active,
        ShareSigningKeyLifecycleState::Missing,
        true,
        true,
    );

    let outcome = rotate_share_signing_key(precondition);

    assert!(matches!(
        outcome.map(|state| (state.old_state(), state.new_state())),
        Ok((
            ShareSigningKeyLifecycleState::Revoked,
            ShareSigningKeyLifecycleState::Active
        ))
    ));
    assert!(!share_signing_key_accepts_new_submissions(
        revoke_share_signing_key(ShareSigningKeyLifecycleState::Active)
    ));
    assert_eq!(
        rotate_share_signing_key(ShareSigningKeyRotationPrecondition::new(
            ShareSigningKeyLifecycleState::Revoked,
            ShareSigningKeyLifecycleState::Missing,
            true,
            true,
        )),
        Err(ShareSigningKeyLifecycleError::OldKeyNotActive)
    );
}

#[test]
fn serde_rejects_non_all_party_additive_secure_aggregate_plan() {
    let payload = "release_policy:\n  participant_count: 2\n  release_threshold: 1\n";
    let parsed: Result<AdditiveSecureAggregatePlan, serde_yaml::Error> =
        serde_yaml::from_str(payload);

    assert!(parsed.is_err());
}

#[test]
fn combines_secure_aggregate_test_vector() {
    let participant_count = nonzero(3);
    let shares = [
        share(1, 1_000_000_000),
        share(2, 2_000_000_000),
        share(3, 1_294_967_333),
    ];

    assert_eq!(
        combine_additive_shares(participant_count, &shares),
        Ok(field(42))
    );
}
