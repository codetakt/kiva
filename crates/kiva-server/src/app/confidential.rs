//! Confidential-computation application adapter assembly.

mod additive;
mod differential_privacy;
mod jobs;
mod participants;
mod shamir;
mod threshold;

use kiva_application::confidential as confidential_app;
use kiva_identity::AuthenticatedPrincipal;

use crate::surfaces::confidential::repository::ConfidentialRepository;

#[derive(Debug, Clone)]
pub(crate) struct ConfidentialApplicationAdapter {
    application: confidential_app::ConfidentialApplicationCapabilities<ConfidentialWorkflowPort>,
}

#[derive(Debug, Clone)]
struct ConfidentialWorkflowPort {
    repository: ConfidentialRepository,
}

impl ConfidentialApplicationAdapter {
    pub(crate) fn new(repository: ConfidentialRepository) -> Self {
        let workflow_port = ConfidentialWorkflowPort::from_server_adapter(repository);
        Self {
            application: confidential_app::ConfidentialApplicationCapabilities::new(workflow_port),
        }
    }

    pub(crate) fn from_server_adapters(repository: ConfidentialRepository) -> Self {
        Self::new(repository)
    }

    fn begin_workflow(
        &self,
    ) -> confidential_app::ConfidentialApplicationWorkflow<ConfidentialWorkflowPort> {
        self.application.begin_workflow()
    }
}

impl ConfidentialWorkflowPort {
    const fn from_server_adapter(repository: ConfidentialRepository) -> Self {
        Self { repository }
    }
}

impl confidential_app::ConfidentialWorkflowRepository for ConfidentialWorkflowPort {
    fn boundary_name(&self) -> &'static str {
        match &self.repository {
            ConfidentialRepository::Memory(_) | ConfidentialRepository::Postgres(_) => {
                "kiva.confidential.application"
            }
        }
    }
}

impl confidential_app::ConfidentialWorkflowAdmissionRecorder for ConfidentialWorkflowPort {
    fn record_workflow_event<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        event: confidential_app::ConfidentialDomainEvent,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<
            confidential_app::ConfidentialDomainEvent,
            confidential_app::ConfidentialApplicationError,
        >,
    > {
        Box::pin(async move {
            if event.principal_subject() == principal.subject {
                Ok(event)
            } else {
                Err(confidential_app::ConfidentialApplicationError::new(
                    "confidential workflow admission subject must match the authenticated principal",
                ))
            }
        })
    }
}
