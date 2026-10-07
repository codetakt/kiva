#![allow(clippy::wildcard_imports)]

use super::*;

#[kani::proof]
pub(super) fn fhe_feasibility_parameter_profile_is_strict() {
    let degree_matches: bool = kani::any();
    let plaintext_modulus_matches: bool = kani::any();
    let ciphertext_modulus_matches: bool = kani::any();
    let security_matches: bool = kani::any();
    let max_participants_matches: bool = kani::any();
    let max_plaintext_matches: bool = kani::any();
    let Some(admitted_max_participants) =
        NonZeroUsize::new(super::FHE_FEASIBILITY_MAX_PARTICIPANTS)
    else {
        assert!(false);
        return;
    };
    let Some(alternate_max_participants) = NonZeroUsize::new(8) else {
        assert!(false);
        return;
    };
    let parameter_set = FheFeasibilityParameterSet::unchecked_for_verification(
        FheFeasibilitySchemeFamily::BfvStyleIntegerSum,
        super::FheFeasibilityOperation::Sum,
        if degree_matches {
            super::FHE_FEASIBILITY_POLYNOMIAL_MODULUS_DEGREE
        } else {
            2_048
        },
        if plaintext_modulus_matches {
            super::FHE_FEASIBILITY_PLAINTEXT_MODULUS
        } else {
            65_537
        },
        if ciphertext_modulus_matches {
            super::FHE_FEASIBILITY_CIPHERTEXT_MODULUS_BITS
        } else {
            80
        },
        if security_matches {
            super::FHE_FEASIBILITY_SECURITY_LEVEL_BITS
        } else {
            96
        },
        if max_participants_matches {
            admitted_max_participants
        } else {
            alternate_max_participants
        },
        if max_plaintext_matches {
            super::FHE_FEASIBILITY_MAX_PLAINTEXT_VALUE
        } else {
            1_000_000
        },
    );

    let result = validate_fhe_feasibility_parameters(parameter_set);
    if degree_matches
        && plaintext_modulus_matches
        && ciphertext_modulus_matches
        && security_matches
        && max_participants_matches
        && max_plaintext_matches
    {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
}

#[kani::proof]
pub(super) fn fhe_feasibility_release_gate_fails_closed() {
    let evaluation_completed: bool = kani::any();
    let policy_approved: bool = kani::any();
    let governed_data_product: bool = kani::any();
    let plaintext_boundary_clear: bool = kani::any();
    let mut bits = 0_u8;
    if evaluation_completed {
        bits |= FheFeasibilityReleaseFacts::EVALUATION_COMPLETED;
    }
    if policy_approved {
        bits |= FheFeasibilityReleaseFacts::POLICY_APPROVED;
    }
    if governed_data_product {
        bits |= FheFeasibilityReleaseFacts::GOVERNED_DATA_PRODUCT;
    }
    if plaintext_boundary_clear {
        bits |= FheFeasibilityReleaseFacts::PLAINTEXT_BOUNDARY_CLEAR;
    }
    let context = FheFeasibilityReleaseContext::new(FheFeasibilityReleaseFacts::from_bits(bits));

    let result = validate_fhe_feasibility_release(context);
    if evaluation_completed && policy_approved && governed_data_product && plaintext_boundary_clear
    {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
}

#[kani::proof]
pub(super) fn fhe_runtime_ciphertext_transcript_gate_fails_closed() {
    let mut bits = 0_u8;
    if kani::any::<bool>() {
        bits |= FheRuntimeCiphertextTranscriptFacts::RUNTIME_BOUND;
    }
    if kani::any::<bool>() {
        bits |= FheRuntimeCiphertextTranscriptFacts::EXTERNAL_REVIEW_COMPLETED;
    }
    if kani::any::<bool>() {
        bits |= FheRuntimeCiphertextTranscriptFacts::PARTICIPANT_COUNT_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= FheRuntimeCiphertextTranscriptFacts::PARTICIPANTS_ACTIVE;
    }
    if kani::any::<bool>() {
        bits |= FheRuntimeCiphertextTranscriptFacts::KEY_CUSTODY_PRESENT;
    }
    if kani::any::<bool>() {
        bits |= FheRuntimeCiphertextTranscriptFacts::PARAMETER_SET_BOUND;
    }
    if kani::any::<bool>() {
        bits |= FheRuntimeCiphertextTranscriptFacts::DIGEST_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= FheRuntimeCiphertextTranscriptFacts::NO_REPLAY;
    }
    let facts = FheRuntimeCiphertextTranscriptFacts::from_bits(bits);
    let context = FheRuntimeCiphertextTranscriptGateContext::new(facts);
    let result = validate_fhe_runtime_ciphertext_transcript_gate_context(context);
    let admissible = context.runtime_bound()
        && context.external_review_completed()
        && context.participant_count_matches()
        && context.participants_active()
        && context.key_custody_present()
        && context.parameter_set_bound()
        && context.digest_matches()
        && context.no_replay();

    if admissible {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
}

#[kani::proof]
pub(super) fn fhe_runtime_release_gate_fails_closed() {
    let mut bits = 0_u8;
    if kani::any::<bool>() {
        bits |= FheRuntimeReleaseFacts::CIPHERTEXT_TRANSCRIPT_ACCEPTED;
    }
    if kani::any::<bool>() {
        bits |= FheRuntimeReleaseFacts::EVALUATION_COMPLETED;
    }
    if kani::any::<bool>() {
        bits |= FheRuntimeReleaseFacts::EXTERNAL_REVIEW_COMPLETED;
    }
    if kani::any::<bool>() {
        bits |= FheRuntimeReleaseFacts::ACTIVE_KEY_EPOCH;
    }
    if kani::any::<bool>() {
        bits |= FheRuntimeReleaseFacts::POLICY_APPROVED;
    }
    if kani::any::<bool>() {
        bits |= FheRuntimeReleaseFacts::GOVERNED_DATA_PRODUCT;
    }
    let facts = FheRuntimeReleaseFacts::from_bits(bits);
    let context = FheRuntimeReleaseContext::new(facts);
    let result = validate_fhe_runtime_release(context);
    let admissible = context.ciphertext_transcript_accepted()
        && context.evaluation_completed()
        && context.external_review_completed()
        && context.active_key_epoch()
        && context.policy_approved()
        && context.governed_data_product();

    if admissible {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
}
