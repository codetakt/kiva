//! Application workflow execution context.

use super::{
    Admitted, ApplicationWorkflowRun, ApplicationWorkflowSpec, PrincipalBound, WorkflowBound,
};

/// Context handed to application code running inside one workflow boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationWorkflowContext {
    workflow_run: ApplicationWorkflowRun,
}

impl ApplicationWorkflowContext {
    /// Creates a workflow context from an active workflow run.
    #[must_use]
    pub(crate) const fn new(workflow_run: ApplicationWorkflowRun) -> Self {
        Self { workflow_run }
    }

    /// Returns the active workflow run marker.
    #[must_use]
    pub const fn workflow_run(self) -> ApplicationWorkflowRun {
        self.workflow_run
    }

    /// Returns the active workflow specification.
    #[must_use]
    pub const fn workflow_spec(self) -> ApplicationWorkflowSpec {
        self.workflow_run.workflow_spec()
    }

    /// Admits a value against the active workflow boundary.
    #[must_use]
    pub const fn admit_workflow<T>(self, value: T) -> Admitted<T, WorkflowBound> {
        Admitted::workflow_bound(value, WorkflowBound::new(self.workflow_run))
    }

    /// Admits a value against an authenticated principal subject.
    #[must_use]
    pub fn admit_principal<T>(
        self,
        principal_subject: impl Into<String>,
        value: T,
    ) -> Admitted<T, PrincipalBound> {
        Admitted::principal_bound(value, PrincipalBound::new(principal_subject))
    }
}
