#![allow(clippy::wildcard_imports)]

use super::*;

#[test]
fn reconstructs_shamir_secret_from_threshold_shares() {
    let plan = shamir_plan(5, 3);
    let shares = [
        shamir_share(2, 2_344),
        shamir_share(4, 5_230),
        shamir_share(5, 7_339),
    ];

    assert_eq!(reconstruct_shamir_secret(plan, &shares), Ok(field(1_234)));
}

#[test]
fn reconstructs_shamir_secret_with_extra_unique_shares() {
    let plan = shamir_plan(5, 3);
    let shares = [
        shamir_share(1, 1_567),
        shamir_share(2, 2_344),
        shamir_share(3, 3_565),
        shamir_share(4, 5_230),
    ];

    assert_eq!(reconstruct_shamir_secret(plan, &shares), Ok(field(1_234)));
}

#[test]
fn shamir_reconstruction_rejects_insufficient_or_duplicate_shares() {
    let plan = shamir_plan(5, 3);

    assert_eq!(
        reconstruct_shamir_secret(plan, &[shamir_share(1, 1_567), shamir_share(2, 2_344)]),
        Err(SecureAggregateError::ShamirShareCountBelowThreshold)
    );
    assert_eq!(
        reconstruct_shamir_secret(
            plan,
            &[
                shamir_share(1, 1_567),
                shamir_share(1, 1_567),
                shamir_share(3, 3_565),
            ],
        ),
        Err(SecureAggregateError::DuplicateParticipantIndex)
    );
}

#[test]
fn verifies_shamir_share_opening_commitments() {
    let transcript_id = transcript(72);
    let opening = verifiable_shamir_share(transcript_id, 1, 1_567, 91);

    assert_eq!(
        reconstruct_verified_shamir_secret(
            shamir_plan(5, 3),
            transcript_id,
            &[
                opening,
                verifiable_shamir_share(transcript_id, 2, 2_344, 92),
                verifiable_shamir_share(transcript_id, 3, 3_565, 93),
            ],
        ),
        Ok(field(1_234))
    );
    assert_eq!(
        adjudicate_shamir_share_complaint(transcript_id, opening),
        ShamirShareComplaintOutcome::Rejected
    );
}

#[test]
fn verified_shamir_reconstruction_rejects_tampered_share_opening() {
    let transcript_id = transcript(72);
    let honest = verifiable_shamir_share(transcript_id, 1, 1_567, 91);
    let tampered = VerifiableShamirShare::new(
        shamir_share(1, 1_568),
        honest.commitment(),
        honest.opening_nonce(),
    );

    assert_eq!(
        reconstruct_verified_shamir_secret(
            shamir_plan(5, 3),
            transcript_id,
            &[
                tampered,
                verifiable_shamir_share(transcript_id, 2, 2_344, 92),
                verifiable_shamir_share(transcript_id, 3, 3_565, 93),
            ],
        ),
        Err(SecureAggregateError::ShamirShareCommitmentMismatch)
    );
    assert_eq!(
        adjudicate_shamir_share_complaint(transcript_id, tampered),
        ShamirShareComplaintOutcome::Sustained
    );
}

#[test]
fn verified_shamir_reconstruction_keeps_duplicate_rejection() {
    let transcript_id = transcript(72);

    assert_eq!(
        reconstruct_verified_shamir_secret(
            shamir_plan(5, 3),
            transcript_id,
            &[
                verifiable_shamir_share(transcript_id, 1, 1_567, 91),
                verifiable_shamir_share(transcript_id, 1, 1_567, 92),
                verifiable_shamir_share(transcript_id, 3, 3_565, 93),
            ],
        ),
        Err(SecureAggregateError::DuplicateParticipantIndex)
    );
}
