#![allow(clippy::wildcard_imports)]

use super::*;

#[kani::proof]
pub(super) fn threshold_frost_key_ceremony_gate_fails_closed() {
    let mut bits = 0_u8;
    if kani::any::<bool>() {
        bits |= ThresholdFrostKeyCeremonyFacts::RUNTIME_BOUND;
    }
    if kani::any::<bool>() {
        bits |= ThresholdFrostKeyCeremonyFacts::EXTERNAL_REVIEW_COMPLETED;
    }
    if kani::any::<bool>() {
        bits |= ThresholdFrostKeyCeremonyFacts::PARTICIPANT_COUNT_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= ThresholdFrostKeyCeremonyFacts::PARTICIPANTS_ACTIVE;
    }
    if kani::any::<bool>() {
        bits |= ThresholdFrostKeyCeremonyFacts::SHARE_CUSTODY_PRESENT;
    }
    if kani::any::<bool>() {
        bits |= ThresholdFrostKeyCeremonyFacts::COMPLAINTS_RESOLVED;
    }
    if kani::any::<bool>() {
        bits |= ThresholdFrostKeyCeremonyFacts::NO_EQUIVOCATION;
    }
    if kani::any::<bool>() {
        bits |= ThresholdFrostKeyCeremonyFacts::NO_REPLAY;
    }
    let facts = ThresholdFrostKeyCeremonyFacts::from_bits(bits);
    let context = ThresholdFrostKeyCeremonyGateContext::new(facts);
    let result = validate_threshold_frost_key_ceremony_gate_context(context);
    let admissible = context.runtime_bound()
        && context.external_review_completed()
        && context.participant_count_matches()
        && context.participants_active()
        && context.share_custody_present()
        && context.complaints_resolved()
        && context.no_equivocation()
        && context.no_replay();

    if admissible {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
}

#[kani::proof]
pub(super) fn threshold_frost_release_gate_fails_closed() {
    let mut bits = 0_u8;
    if kani::any::<bool>() {
        bits |= ThresholdFrostReleaseFacts::KEY_CEREMONY_COMPLETED;
    }
    if kani::any::<bool>() {
        bits |= ThresholdFrostReleaseFacts::SIGNATURE_COMPUTED;
    }
    if kani::any::<bool>() {
        bits |= ThresholdFrostReleaseFacts::EXTERNAL_REVIEW_COMPLETED;
    }
    if kani::any::<bool>() {
        bits |= ThresholdFrostReleaseFacts::ACTIVE_KEY_EPOCH;
    }
    if kani::any::<bool>() {
        bits |= ThresholdFrostReleaseFacts::POLICY_APPROVED;
    }
    if kani::any::<bool>() {
        bits |= ThresholdFrostReleaseFacts::GOVERNED_KEY_USE;
    }
    let facts = ThresholdFrostReleaseFacts::from_bits(bits);
    let context = ThresholdFrostReleaseContext::new(facts);
    let result = validate_threshold_frost_release(context);
    let admissible = context.key_ceremony_completed()
        && context.signature_computed()
        && context.external_review_completed()
        && context.active_key_epoch()
        && context.policy_approved()
        && context.governed_key_use();

    if admissible {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
}
