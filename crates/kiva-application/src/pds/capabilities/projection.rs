//! Subject projection capabilities for PDS application reads.

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::PdsSubjectAccountNamespace;

use crate::runtime::ApplicationFuture;

use super::super::model::{
    PdsPortabilitySnapshot, PdsReadModelError, PdsSubjectId, PdsSubjectSnapshot,
};
use super::records::{PdsRecordReadPort, XapiStatementReader};

/// Port for mapping a PDS subject into the xAPI actor IFI used by read projections.
pub trait PdsSubjectIdentityMapper: Clone + Send + Sync + 'static {
    /// Returns the xAPI actor IFI for the PDS subject.
    fn actor_ifi(&self, subject: &str) -> String;
}

/// Account-based PDS subject identity mapper.
#[derive(Debug, Clone)]
pub struct AccountHomePagePdsSubjectIdentityMapper {
    namespace: PdsSubjectAccountNamespace,
}

impl AccountHomePagePdsSubjectIdentityMapper {
    /// Creates an account-home-page based subject identity mapper.
    pub fn new(account_home_page: impl Into<String>) -> Self {
        Self {
            namespace: PdsSubjectAccountNamespace::new(account_home_page),
        }
    }
}

impl PdsSubjectIdentityMapper for AccountHomePagePdsSubjectIdentityMapper {
    fn actor_ifi(&self, subject: &str) -> String {
        self.namespace.actor_ifi(subject)
    }
}

/// Subject-scoped read projection capability.
pub trait PdsSubjectProjection: Clone + Send + Sync + 'static {
    /// Builds a subject snapshot for inventory and export workflows.
    fn subject_snapshot<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
    ) -> ApplicationFuture<'a, Result<PdsSubjectSnapshot, PdsReadModelError>>;

    /// Builds one subject-bound portability page without transforming Statement JSON.
    fn portability_snapshot<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        cursor: Option<&'a str>,
        page_size: usize,
    ) -> ApplicationFuture<'a, Result<PdsPortabilitySnapshot, PdsReadModelError>>;
}

/// Subject-scoped read projection across xAPI metadata and PDS records.
#[derive(Debug, Clone)]
pub struct PdsSubjectProjectionPort<S, P, I>
where
    S: XapiStatementReader,
    P: PdsRecordReadPort,
    I: PdsSubjectIdentityMapper,
{
    identity: I,
    statements: S,
    pds_records: P,
}

impl<S, P, I> PdsSubjectProjectionPort<S, P, I>
where
    S: XapiStatementReader,
    P: PdsRecordReadPort,
    I: PdsSubjectIdentityMapper,
{
    /// Creates a subject projection port from explicit identity, xAPI, and PDS ports.
    pub const fn new(identity: I, statements: S, pds_records: P) -> Self {
        Self {
            identity,
            statements,
            pds_records,
        }
    }
}

impl<S, P, I> PdsSubjectProjection for PdsSubjectProjectionPort<S, P, I>
where
    S: XapiStatementReader,
    P: PdsRecordReadPort,
    I: PdsSubjectIdentityMapper,
{
    fn subject_snapshot<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
    ) -> ApplicationFuture<'a, Result<PdsSubjectSnapshot, PdsReadModelError>> {
        Box::pin(async move {
            let statements = self
                .statements
                .list_metadata_by_actor_ifi(&self.identity.actor_ifi(&principal.subject))
                .await
                .map_err(PdsReadModelError::Statements)?;
            let data_products = self
                .pds_records
                .list_data_products(principal)
                .await
                .map_err(PdsReadModelError::Pds)?;
            let metadata_records = self
                .pds_records
                .list_metadata_records(principal)
                .await
                .map_err(PdsReadModelError::Pds)?;
            Ok(PdsSubjectSnapshot {
                data_subject: PdsSubjectId::new(principal.subject.clone()),
                statements,
                data_products,
                consent_grants: metadata_records.consent_grants,
                lifecycle_events: metadata_records.lifecycle_events,
                retention_policies: metadata_records.retention_policies,
                lifecycle_cases: metadata_records.lifecycle_cases,
                governance_profiles: metadata_records.governance_profiles,
            })
        })
    }

    fn portability_snapshot<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        cursor: Option<&'a str>,
        page_size: usize,
    ) -> ApplicationFuture<'a, Result<PdsPortabilitySnapshot, PdsReadModelError>> {
        Box::pin(async move {
            let statement_page = self
                .statements
                .list_portability_page_by_actor_ifi(
                    &self.identity.actor_ifi(&principal.subject),
                    cursor,
                    page_size,
                )
                .await
                .map_err(PdsReadModelError::Statements)?;
            let data_products = self
                .pds_records
                .list_data_products(principal)
                .await
                .map_err(PdsReadModelError::Pds)?;
            let metadata = self
                .pds_records
                .list_metadata_records(principal)
                .await
                .map_err(PdsReadModelError::Pds)?;
            Ok(PdsPortabilitySnapshot {
                data_subject: PdsSubjectId::new(principal.subject.clone()),
                statement_page,
                data_products,
                consent_grants: metadata.consent_grants,
                lifecycle_events: metadata.lifecycle_events,
                retention_policies: metadata.retention_policies,
                lifecycle_cases: metadata.lifecycle_cases,
                governance_profiles: metadata.governance_profiles,
            })
        })
    }
}
