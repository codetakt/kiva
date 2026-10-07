//! Application admission witnesses.

use super::ApplicationWorkflowRun;

/// Value admitted by an application boundary before crossing a port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted<T, W = UnscopedAdmission> {
    value: T,
    witness: W,
}

impl<T, W> Admitted<T, W> {
    /// Creates an admitted value inside the application crate.
    #[must_use]
    pub(crate) const fn new(value: T, witness: W) -> Self {
        Self { value, witness }
    }

    /// Returns the admitted value by reference.
    #[must_use]
    pub const fn as_ref(&self) -> &T {
        &self.value
    }

    /// Returns the admission witness by reference.
    #[must_use]
    pub const fn witness(&self) -> &W {
        &self.witness
    }

    /// Consumes this wrapper and returns the admitted value.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.value
    }
}

/// Witness for legacy or intentionally unclassified admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnscopedAdmission;

/// Witness for values admitted against an authenticated principal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalBound {
    principal_subject: String,
}

impl PrincipalBound {
    /// Creates a principal-bound witness.
    #[must_use]
    pub(crate) fn new(principal_subject: impl Into<String>) -> Self {
        Self {
            principal_subject: principal_subject.into(),
        }
    }

    /// Returns the authenticated principal subject used for admission.
    #[must_use]
    pub fn principal_subject(&self) -> &str {
        &self.principal_subject
    }
}

impl<T> Admitted<T, PrincipalBound> {
    /// Creates a principal-bound admitted value after principal checks.
    #[must_use]
    pub(crate) const fn principal_bound(value: T, witness: PrincipalBound) -> Self {
        Self::new(value, witness)
    }

    /// Returns the authenticated principal subject used for admission.
    #[must_use]
    pub fn principal_subject(&self) -> &str {
        self.witness.principal_subject()
    }
}

/// Witness for values admitted against an application workflow run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowBound {
    workflow_run: ApplicationWorkflowRun,
}

impl WorkflowBound {
    /// Creates a workflow-bound witness.
    #[must_use]
    pub(crate) const fn new(workflow_run: ApplicationWorkflowRun) -> Self {
        Self { workflow_run }
    }

    /// Returns the workflow run used for admission.
    #[must_use]
    pub const fn workflow_run(self) -> ApplicationWorkflowRun {
        self.workflow_run
    }
}

impl<T> Admitted<T, WorkflowBound> {
    /// Creates a workflow-bound admitted value after workflow specification checks.
    #[must_use]
    pub(crate) const fn workflow_bound(value: T, witness: WorkflowBound) -> Self {
        Self::new(value, witness)
    }

    /// Returns the workflow run used for admission.
    #[must_use]
    pub const fn workflow_run(&self) -> ApplicationWorkflowRun {
        self.witness.workflow_run()
    }
}
