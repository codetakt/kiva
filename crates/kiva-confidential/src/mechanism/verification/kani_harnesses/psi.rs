#![allow(clippy::wildcard_imports)]

use super::*;

#[kani::proof]
pub(super) fn psi_cohort_intersection_exposes_only_common_digests() {
    let job_id = fixed_transcript_id(81);
    let common = fixed_psi_digest(11);
    let left_only = fixed_psi_digest(12);
    let right_only = fixed_psi_digest(13);
    let datasets = [
        PsiParticipantDataset::new(NonZeroUsize::MIN, vec![left_only, common]),
        PsiParticipantDataset::new(
            NonZeroUsize::new(2).unwrap_or(NonZeroUsize::MIN),
            vec![right_only, common],
        ),
    ];

    let result = compute_psi_cohort_intersection(
        NonZeroUsize::new(2).unwrap_or(NonZeroUsize::MIN),
        job_id,
        &[],
        &datasets,
    );

    let Ok(result) = result else {
        assert!(false);
        return;
    };
    assert_eq!(result.intersection_digests(), &[common]);
    assert!(!result.intersection_digests().contains(&left_only));
    assert!(!result.intersection_digests().contains(&right_only));
    assert_eq!(result.job_record(), PsiJobRecord::new(job_id));
}

#[kani::proof]
pub(super) fn psi_oprf_intersection_exposes_only_common_elements() {
    let job_id = fixed_transcript_id(84);
    let common = fixed_psi_oprf_element(21);
    let left_only = fixed_psi_oprf_element(22);
    let right_only = fixed_psi_oprf_element(23);
    let transcripts = [
        PsiOprfParticipantTranscript::new(
            NonZeroUsize::MIN,
            PsiOprfParticipantTranscriptStatus::Submitted,
            vec![left_only, common],
        ),
        PsiOprfParticipantTranscript::new(
            NonZeroUsize::new(2).unwrap_or(NonZeroUsize::MIN),
            PsiOprfParticipantTranscriptStatus::Submitted,
            vec![right_only, common],
        ),
    ];

    let result = compute_psi_oprf_intersection(fixed_psi_oprf_plan(), job_id, &[], &transcripts);

    let Ok(result) = result else {
        assert!(false);
        return;
    };
    assert_eq!(result.intersection_elements(), &[common]);
    assert!(!result.intersection_elements().contains(&left_only));
    assert!(!result.intersection_elements().contains(&right_only));
    assert_eq!(result.record().job_id(), job_id);
}

#[kani::proof]
pub(super) fn psi_oprf_release_gate_fails_closed() {
    let mut bits = 0_u8;
    if kani::any::<bool>() {
        bits |= PsiOprfReleaseFacts::INTERSECTION_COMPUTED;
    }
    if kani::any::<bool>() {
        bits |= PsiOprfReleaseFacts::PARTICIPANTS_ACTIVE;
    }
    if kani::any::<bool>() {
        bits |= PsiOprfReleaseFacts::EXTERNAL_REVIEW_COMPLETED;
    }
    if kani::any::<bool>() {
        bits |= PsiOprfReleaseFacts::POLICY_APPROVED;
    }
    if kani::any::<bool>() {
        bits |= PsiOprfReleaseFacts::GOVERNED_DATA_PRODUCT;
    }

    let context = PsiOprfReleaseContext::new(PsiOprfReleaseFacts::from_bits(bits));
    let result = validate_psi_oprf_release(context);
    let admissible = context.intersection_computed()
        && context.participants_active()
        && context.external_review_completed()
        && context.policy_approved()
        && context.governed_data_product();

    if admissible {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
}
