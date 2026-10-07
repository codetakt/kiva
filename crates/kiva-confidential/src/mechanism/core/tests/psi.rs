#![allow(clippy::wildcard_imports)]

use super::*;

#[test]
fn psi_cohort_identifier_canonicalization_is_strict() {
    assert_eq!(
        canonicalize_psi_cohort_identifier(" Learner-A@example.EDU "),
        Ok("learner-a@example.edu".to_owned())
    );
    assert_eq!(
        canonicalize_psi_cohort_identifier(" \t "),
        Err(SecureAggregateError::PsiIdentifierEmpty)
    );
    assert_eq!(
        canonicalize_psi_cohort_identifier("learner\nid"),
        Err(SecureAggregateError::PsiIdentifierContainsControl)
    );
}

#[test]
fn psi_cohort_intersection_exposes_only_matches() {
    let job_id = transcript(81);
    let namespace = "district-2026";
    let shared = psi_digest(job_id, namespace, "shared@example.edu");
    let first_only = psi_digest(job_id, namespace, "first-only@example.edu");
    let second_only = psi_digest(job_id, namespace, "second-only@example.edu");
    let datasets = [
        psi_dataset(1, vec![first_only, shared]),
        psi_dataset(2, vec![second_only, shared]),
    ];

    let result = compute_psi_cohort_intersection(nonzero(2), job_id, &[], &datasets);

    assert!(result.is_ok());
    let Ok(result) = result else {
        return;
    };
    assert_eq!(result.job_record(), PsiJobRecord::new(job_id));
    assert_eq!(result.participant_count(), nonzero(2));
    assert_eq!(result.intersection_digests(), &[shared]);
    assert!(!result.intersection_digests().contains(&first_only));
    assert!(!result.intersection_digests().contains(&second_only));
}

#[test]
fn psi_cohort_intersection_allows_empty_outputs() {
    let job_id = transcript(82);
    let namespace = "district-2026";
    let datasets = [
        psi_dataset(1, vec![psi_digest(job_id, namespace, "first@example.edu")]),
        psi_dataset(2, vec![psi_digest(job_id, namespace, "second@example.edu")]),
    ];

    let result = compute_psi_cohort_intersection(nonzero(2), job_id, &[], &datasets);

    assert!(result.is_ok());
    let Ok(result) = result else {
        return;
    };
    assert!(result.intersection_digests().is_empty());
}

#[test]
fn psi_cohort_intersection_rejects_duplicate_inputs_and_replayed_jobs() {
    let job_id = transcript(83);
    let namespace = "district-2026";
    let duplicate = psi_digest(job_id, namespace, "duplicate@example.edu");
    let duplicated_dataset = [
        psi_dataset(1, vec![duplicate, duplicate]),
        psi_dataset(2, vec![duplicate]),
    ];
    let valid_dataset = [
        psi_dataset(1, vec![duplicate]),
        psi_dataset(2, vec![duplicate]),
    ];

    assert_eq!(
        compute_psi_cohort_intersection(nonzero(2), job_id, &[], &duplicated_dataset),
        Err(SecureAggregateError::DuplicatePsiIdentifierDigest)
    );
    assert_eq!(
        compute_psi_cohort_intersection(
            nonzero(2),
            job_id,
            &[PsiJobRecord::new(job_id)],
            &valid_dataset,
        ),
        Err(SecureAggregateError::PsiReplayDetected)
    );
}

#[test]
fn psi_oprf_intersection_exposes_only_matches_and_records_transcript() {
    let job_id = transcript(84);
    let namespace = "district-2026";
    let shared = psi_oprf_element(job_id, namespace, "shared@example.edu");
    let first_only = psi_oprf_element(job_id, namespace, "first-only@example.edu");
    let second_only = psi_oprf_element(job_id, namespace, "second-only@example.edu");
    let plan = psi_oprf_plan(2, true);
    let transcripts = [
        psi_oprf_transcript(1, vec![first_only, shared]),
        psi_oprf_transcript(2, vec![second_only, shared]),
    ];

    let result = compute_psi_oprf_intersection(plan, job_id, &[], &transcripts);

    assert!(result.is_ok());
    let Ok(result) = result else {
        return;
    };
    assert_eq!(result.participant_count(), nonzero(2));
    assert_eq!(result.intersection_elements(), &[shared]);
    assert!(!result.intersection_elements().contains(&first_only));
    assert!(!result.intersection_elements().contains(&second_only));
    assert_eq!(result.record().job_id(), job_id);
}

#[test]
fn psi_oprf_rejects_abort_duplicate_replay_and_review_pending_release() {
    let job_id = transcript(85);
    let namespace = "district-2026";
    let shared = psi_oprf_element(job_id, namespace, "shared@example.edu");
    let plan = psi_oprf_plan(2, false);
    let duplicated = [
        psi_oprf_transcript(1, vec![shared, shared]),
        psi_oprf_transcript(2, vec![shared]),
    ];
    let aborted = [
        PsiOprfParticipantTranscript::new(
            nonzero(1),
            PsiOprfParticipantTranscriptStatus::Aborted,
            vec![shared],
        ),
        psi_oprf_transcript(2, vec![shared]),
    ];
    let valid = [
        psi_oprf_transcript(1, vec![shared]),
        psi_oprf_transcript(2, vec![shared]),
    ];

    assert_eq!(
        compute_psi_oprf_intersection(plan, job_id, &[], &duplicated),
        Err(SecureAggregateError::DuplicatePsiOprfElement)
    );
    assert_eq!(
        compute_psi_oprf_intersection(plan, job_id, &[], &aborted),
        Err(SecureAggregateError::PsiOprfTranscriptAborted)
    );
    assert_eq!(
        compute_psi_oprf_intersection(
            plan,
            job_id,
            &[PsiOprfTranscriptRecord::new(job_id, commitment(40))],
            &valid,
        ),
        Err(SecureAggregateError::PsiOprfTranscriptReplayDetected)
    );

    let result = compute_psi_oprf_intersection(plan, job_id, &[], &valid);
    assert!(result.is_ok());
    let Ok(result) = result else {
        return;
    };
    let review_pending = PsiOprfReleaseContext::new(PsiOprfReleaseFacts::from_bits(
        PsiOprfReleaseFacts::INTERSECTION_COMPUTED
            | PsiOprfReleaseFacts::PARTICIPANTS_ACTIVE
            | PsiOprfReleaseFacts::POLICY_APPROVED
            | PsiOprfReleaseFacts::GOVERNED_DATA_PRODUCT,
    ));
    assert_eq!(
        validate_psi_oprf_release(review_pending),
        Err(SecureAggregateError::PsiOprfExternalReviewRequired)
    );

    let reviewed = PsiOprfReleaseContext::new(PsiOprfReleaseFacts::from_bits(
        PsiOprfReleaseFacts::INTERSECTION_COMPUTED
            | PsiOprfReleaseFacts::PARTICIPANTS_ACTIVE
            | PsiOprfReleaseFacts::EXTERNAL_REVIEW_COMPLETED
            | PsiOprfReleaseFacts::POLICY_APPROVED
            | PsiOprfReleaseFacts::GOVERNED_DATA_PRODUCT,
    ));
    let released = release_psi_oprf_intersection(&result, reviewed);
    assert!(released.is_ok());
    let Ok(released) = released else {
        return;
    };
    assert_eq!(released.intersection_elements(), &[shared]);
    assert_eq!(
        released.transcript_digest(),
        result.record().transcript_digest()
    );
}
