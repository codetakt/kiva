//! Confidential application command/event ledger.

#[cfg(test)]
mod tests;

use kiva_identity::AuthenticatedPrincipal;

use crate::runtime::{Admitted, ApplicationWorkflowSpec, WorkflowAdmission, WorkflowBound};

use super::{ConfidentialWorkflowFamily, ConfidentialWorkflowKind};

/// Normalized admission record for one confidential application workflow.
pub type ConfidentialWorkflowAdmission = WorkflowAdmission<ConfidentialWorkflowKind>;

/// Application-level confidential command admitted before repository execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfidentialCommand {
    workflow_kind: ConfidentialWorkflowKind,
}

impl ConfidentialCommand {
    /// Creates a confidential application command from a typed workflow kind.
    #[must_use]
    pub const fn new(workflow_kind: ConfidentialWorkflowKind) -> Self {
        Self { workflow_kind }
    }

    /// Returns the workflow kind carried by the command.
    #[must_use]
    pub const fn workflow_kind(self) -> ConfidentialWorkflowKind {
        self.workflow_kind
    }

    /// Returns the application workflow specification carried by the command.
    #[must_use]
    pub const fn workflow_spec(self) -> ApplicationWorkflowSpec {
        self.workflow_kind.workflow_spec()
    }

    /// Returns the stable workflow name carried by the command.
    #[must_use]
    pub const fn workflow_name(self) -> &'static str {
        self.workflow_kind.label()
    }

    /// Projects the command into a pre-admission domain event.
    #[must_use]
    pub fn admission_requested(
        self,
        principal: &AuthenticatedPrincipal,
    ) -> ConfidentialDomainEvent {
        ConfidentialDomainEvent::WorkflowAdmissionRequested {
            principal_subject: principal.subject.clone(),
            workflow_kind: self.workflow_kind,
        }
    }
}

/// Confidential application event used by admission ledgers and evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfidentialDomainEvent {
    /// A principal requested admission to a confidential workflow.
    WorkflowAdmissionRequested {
        /// Authenticated principal subject.
        principal_subject: String,
        /// Requested workflow kind.
        workflow_kind: ConfidentialWorkflowKind,
    },
    /// A principal was admitted to a confidential workflow family.
    WorkflowAdmitted {
        /// Authenticated principal subject.
        principal_subject: String,
        /// Admitted workflow kind.
        workflow_kind: ConfidentialWorkflowKind,
        /// Admitted workflow family.
        family: ConfidentialWorkflowFamily,
    },
}

impl ConfidentialDomainEvent {
    /// Projects an admitted workflow value into the domain-event ledger shape.
    #[must_use]
    pub fn workflow_admitted(
        admission: &Admitted<ConfidentialWorkflowAdmission, WorkflowBound>,
    ) -> Self {
        let admission = admission.as_ref();
        Self::WorkflowAdmitted {
            principal_subject: admission.principal_subject().to_owned(),
            workflow_kind: admission.workflow_kind(),
            family: admission.workflow_kind().family(),
        }
    }

    /// Returns the event principal subject.
    #[must_use]
    pub fn principal_subject(&self) -> &str {
        match self {
            Self::WorkflowAdmissionRequested {
                principal_subject, ..
            }
            | Self::WorkflowAdmitted {
                principal_subject, ..
            } => principal_subject,
        }
    }

    /// Returns the event workflow kind.
    #[must_use]
    pub const fn workflow_kind(&self) -> ConfidentialWorkflowKind {
        match self {
            Self::WorkflowAdmissionRequested { workflow_kind, .. }
            | Self::WorkflowAdmitted { workflow_kind, .. } => *workflow_kind,
        }
    }

    /// Returns the application workflow specification carried by this event.
    #[must_use]
    pub const fn workflow_spec(&self) -> ApplicationWorkflowSpec {
        self.workflow_kind().workflow_spec()
    }

    /// Returns the workflow family when the event has been admitted.
    #[must_use]
    pub const fn admitted_family(&self) -> Option<ConfidentialWorkflowFamily> {
        match self {
            Self::WorkflowAdmissionRequested { .. } => None,
            Self::WorkflowAdmitted { family, .. } => Some(*family),
        }
    }

    /// Returns the event workflow name.
    #[must_use]
    pub const fn workflow_name(&self) -> &'static str {
        self.workflow_spec().label()
    }

    /// Returns the Rust workflow marker used by drift evidence.
    #[must_use]
    pub const fn workflow_rust_kind(&self) -> &'static str {
        self.workflow_kind().rust_kind()
    }
}

/// Backend-neutral confidential command outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfidentialOutcome {
    /// The workflow command was admitted.
    Admitted(Admitted<ConfidentialWorkflowAdmission, WorkflowBound>),
}

impl ConfidentialOutcome {
    /// Converts an admission outcome into its admission record.
    #[must_use]
    pub fn into_admission(self) -> ConfidentialWorkflowAdmission {
        match self {
            Self::Admitted(admission) => admission.into_inner(),
        }
    }

    /// Projects the outcome into an admitted domain event.
    #[must_use]
    pub fn admitted_event(&self) -> ConfidentialDomainEvent {
        let Self::Admitted(admission) = self;
        ConfidentialDomainEvent::workflow_admitted(admission)
    }
}
