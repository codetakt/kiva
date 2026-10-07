use super::{
    ConfidentialDomainEvent, ConfidentialOutcome, ConfidentialWorkflowAdmission,
    ConfidentialWorkflowFamily, ConfidentialWorkflowKind,
};
use crate::runtime::{ApplicationWorkflowContext, ApplicationWorkflowRun};

fn admitted_outcome(workflow_kind: ConfidentialWorkflowKind) -> ConfidentialOutcome {
    let workflow_run =
        ApplicationWorkflowRun::new(workflow_kind.workflow_spec(), uuid::Uuid::nil());
    let context = ApplicationWorkflowContext::new(workflow_run);
    ConfidentialOutcome::Admitted(context.admit_workflow(ConfidentialWorkflowAdmission::new(
        "subject-a",
        workflow_kind,
    )))
}

#[test]
fn admitted_event_records_workflow_family() {
    let outcome = admitted_outcome(ConfidentialWorkflowKind::SubmitAdditiveShare);

    assert!(matches!(
        outcome.admitted_event(),
        ConfidentialDomainEvent::WorkflowAdmitted {
            principal_subject,
            workflow_kind: ConfidentialWorkflowKind::SubmitAdditiveShare,
            family: ConfidentialWorkflowFamily::Additive,
        } if principal_subject == "subject-a"
    ));
}

#[test]
fn workflow_kind_classifies_differential_privacy_family() {
    let outcome = admitted_outcome(ConfidentialWorkflowKind::ReleaseDifferentialPrivacyAggregate);

    assert!(matches!(
        outcome.admitted_event(),
        ConfidentialDomainEvent::WorkflowAdmitted {
            principal_subject,
            workflow_kind: ConfidentialWorkflowKind::ReleaseDifferentialPrivacyAggregate,
            family: ConfidentialWorkflowFamily::DifferentialPrivacy,
        } if principal_subject == "subject-a"
    ));
}

#[test]
fn workflow_kind_classifies_lifecycle_families() {
    let cases = [
        (
            ConfidentialWorkflowKind::RegisterJob,
            ConfidentialWorkflowFamily::Job,
        ),
        (
            ConfidentialWorkflowKind::RegisterParticipant,
            ConfidentialWorkflowFamily::Participant,
        ),
        (
            ConfidentialWorkflowKind::ReconstructShamirThresholdSecret,
            ConfidentialWorkflowFamily::Shamir,
        ),
        (
            ConfidentialWorkflowKind::ReleaseThresholdAggregate,
            ConfidentialWorkflowFamily::Threshold,
        ),
    ];

    for (workflow_kind, expected_family) in cases {
        let outcome = admitted_outcome(workflow_kind);

        assert_eq!(
            outcome.admitted_event().admitted_family(),
            Some(expected_family),
        );
    }
}
