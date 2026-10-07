#![allow(clippy::wildcard_imports)]

use super::*;

#[kani::proof]
pub(super) fn confidential_job_lifecycle_matches_strict_transition_table() {
    let current = any_confidential_job_status();
    let requested = any_confidential_job_status();

    let result = transition_confidential_job_status(current, requested);
    if confidential_job_transition_is_allowed(current, requested) {
        assert_eq!(result, Ok(requested));
    } else {
        assert!(matches!(
            result,
            Err(ConfidentialJobTransitionError::InvalidTransition)
        ));
    }
}

#[kani::proof]
pub(super) fn confidential_job_admission_gate_fails_closed() {
    let participants_ok: bool = kani::any();
    let policy_ok: bool = kani::any();
    let deadline_ok: bool = kani::any();
    let quota_ok: bool = kani::any();
    let scheduler_ok: bool = kani::any();
    let backpressure_ok: bool = kani::any();
    let retry_ok: bool = kani::any();
    let facts = ConfidentialJobAdmissionFacts::new(
        participants_ok,
        policy_ok,
        deadline_ok,
        quota_ok,
        scheduler_ok,
        backpressure_ok,
        retry_ok,
    );

    let result = validate_confidential_job_admission(facts);
    if !participants_ok {
        assert!(matches!(
            result,
            Err(ConfidentialJobAdmissionError::TooFewParticipants)
        ));
    } else if !policy_ok {
        assert!(matches!(
            result,
            Err(ConfidentialJobAdmissionError::MissingPolicyBinding)
        ));
    } else if !deadline_ok {
        assert!(matches!(
            result,
            Err(ConfidentialJobAdmissionError::DeadlineNotFuture)
        ));
    } else if !quota_ok {
        assert!(matches!(
            result,
            Err(ConfidentialJobAdmissionError::AdmissionQuotaExceeded)
        ));
    } else if !scheduler_ok || !backpressure_ok {
        assert!(matches!(
            result,
            Err(ConfidentialJobAdmissionError::SchedulerBackpressure)
        ));
    } else if !retry_ok {
        assert!(matches!(
            result,
            Err(ConfidentialJobAdmissionError::RetryBudgetExceeded)
        ));
    } else {
        assert!(result.is_ok());
    }
}
