use std::collections::BTreeSet;

use kiva_identity::{AuthenticatedPrincipal, StatementWritePermission};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{super::PdsWorkflowKind, admit_domain_event_for_principal};
use crate::pds::PdsApplicationStoreError;
use crate::runtime::{ApplicationWorkflowContext, ApplicationWorkflowRun};

#[test]
fn pds_domain_event_admission_rejects_principal_subject_mismatch() {
    let principal = AuthenticatedPrincipal::new(
        "subject-a",
        StatementWritePermission::Denied,
        BTreeSet::new(),
    );
    let command = kiva_pds::RevokeConsentGrantCommand::new(
        "subject-b",
        kiva_pds::ConsentRevocationCommandInput {
            grant_id: Uuid::nil(),
        },
        OffsetDateTime::UNIX_EPOCH,
    );
    let event = command.consent_grant_revocation_requested();
    let workflow_run = ApplicationWorkflowRun::new(
        PdsWorkflowKind::RevokeConsentGrant.workflow_spec(),
        Uuid::nil(),
    );
    let context = ApplicationWorkflowContext::new(workflow_run);

    let error = admit_domain_event_for_principal(context, &principal, event.into())
        .expect_err("PDS application admission must reject events owned by a different subject");

    assert_eq!(
        error,
        PdsApplicationStoreError::InvalidRequest(
            "PDS domain event subject must match the authenticated principal".to_owned(),
        ),
    );
}
