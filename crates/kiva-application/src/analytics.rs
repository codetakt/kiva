//! HTTP-independent governed analytics serving workflows.

use kiva_identity::AuthenticatedPrincipal;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    operation_catalog::{self, ApplicationOperationId},
    runtime::{
        ApplicationFuture, ApplicationOperationGateway, ApplicationRuntimeError,
        ApplicationRuntimePort, ApplicationWorkflowFailure, ApplicationWorkflowKind,
        ApplicationWorkflowSpec, DefaultApplicationRuntime,
    },
};

/// Stable analytics application workflow kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyticsWorkflowKind {
    /// Lists governed aggregate products.
    ListProducts,
    /// Reads governed aggregate product metadata.
    ReadProduct,
    /// Reads an admitted aggregate release payload.
    ReadAggregate,
}

impl AnalyticsWorkflowKind {
    /// Returns the generated workflow specification.
    #[must_use]
    pub const fn workflow_spec(self) -> ApplicationWorkflowSpec {
        operation_catalog::analytics_workflow_spec(self)
    }
}

/// Governed aggregate product metadata. No owner identifier or raw record is exposed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyticsProduct {
    /// Stable PDS data-product identifier.
    pub data_product_id: Uuid,
    /// Human-readable product title.
    pub title: String,
    /// Approved aggregate use purpose.
    pub purpose: String,
    /// Aggregate release mechanism label.
    pub release_kind: String,
    /// Product approval timestamp.
    pub approved_at: OffsetDateTime,
}

/// Admitted aggregate payload that is safe for the analytics boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyticsAggregate {
    /// Stable PDS data-product identifier.
    pub data_product_id: Uuid,
    /// Aggregate release mechanism label.
    pub release_kind: String,
    /// Released aggregate value. Differential-privacy releases expose only the noised value.
    pub value: i64,
    /// Timestamp at which the aggregate became available.
    pub released_at: OffsetDateTime,
}

/// Audit record requested for every analytics read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyticsReadAudit {
    /// Unique audit event identifier.
    pub event_id: Uuid,
    /// Authenticated analytics reader.
    pub requester_subject: String,
    /// Optional product associated with the read.
    pub data_product_id: Option<Uuid>,
    /// Stable operation label.
    pub operation: &'static str,
    /// Whether the governed resource was visible to the reader.
    pub outcome: AnalyticsReadOutcome,
    /// Read timestamp.
    pub occurred_at: OffsetDateTime,
}

/// Result recorded for a governed analytics read attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyticsReadOutcome {
    /// The requested governed output was returned.
    Allowed,
    /// The requested output was not visible at this boundary.
    NotFound,
}

/// Store errors at the analytics application port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyticsStoreError {
    /// Persistence failed.
    Store,
    /// Persisted data violated the analytics boundary.
    InvalidStoredData,
}

/// Analytics query errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyticsQueryError {
    /// The product or admitted aggregate is not visible at this boundary.
    NotFound,
    /// A storage adapter failed.
    Store(AnalyticsStoreError),
    /// The shared application runtime failed.
    Runtime(ApplicationRuntimeError),
}

impl From<ApplicationRuntimeError> for AnalyticsQueryError {
    fn from(error: ApplicationRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

/// Port exposing only approved product metadata, admitted aggregates, and audit writes.
pub trait AnalyticsServingPort: Clone + Send + Sync + 'static {
    /// Lists currently admitted aggregate products owned by the principal.
    fn list_products<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
    ) -> ApplicationFuture<'a, Result<Vec<AnalyticsProduct>, AnalyticsStoreError>>;

    /// Reads one currently admitted aggregate product owned by the principal.
    fn read_product<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        data_product_id: Uuid,
    ) -> ApplicationFuture<'a, Result<Option<AnalyticsProduct>, AnalyticsStoreError>>;

    /// Reads the admitted aggregate payload without exposing source records.
    fn read_aggregate<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        data_product_id: Uuid,
    ) -> ApplicationFuture<'a, Result<Option<AnalyticsAggregate>, AnalyticsStoreError>>;

    /// Persists one analytics read audit record.
    fn record_read(
        &self,
        audit: AnalyticsReadAudit,
    ) -> ApplicationFuture<'_, Result<(), AnalyticsStoreError>>;
}

/// Governed analytics application service.
#[derive(Debug, Clone)]
pub struct AnalyticsApplication<P, A = DefaultApplicationRuntime>
where
    P: AnalyticsServingPort,
    A: ApplicationRuntimePort,
{
    port: P,
    runtime: A,
}

impl<P> AnalyticsApplication<P>
where
    P: AnalyticsServingPort,
{
    /// Creates an analytics service with the default runtime.
    #[must_use]
    pub fn new(port: P) -> Self {
        Self {
            port,
            runtime: DefaultApplicationRuntime::default(),
        }
    }
}

impl<P, A> AnalyticsApplication<P, A>
where
    P: AnalyticsServingPort,
    A: ApplicationRuntimePort,
{
    async fn record_read(
        &self,
        principal: &AuthenticatedPrincipal,
        data_product_id: Option<Uuid>,
        operation: &'static str,
        outcome: AnalyticsReadOutcome,
    ) -> Result<(), AnalyticsQueryError> {
        self.port
            .record_read(AnalyticsReadAudit {
                event_id: self.runtime.next_workflow_id(),
                requester_subject: principal.subject.clone(),
                data_product_id,
                operation,
                outcome,
                occurred_at: self.runtime.now_utc(),
            })
            .await
            .map_err(AnalyticsQueryError::Store)
    }

    /// Lists admitted products and records the read.
    pub async fn list_products(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<Vec<AnalyticsProduct>, AnalyticsQueryError> {
        ApplicationOperationGateway::new(&self.runtime)
            .execute(
                ApplicationOperationId::AnalyticsListProducts,
                |_context| async {
                    let products = self.port.list_products(principal).await.map_err(|error| {
                        ApplicationWorkflowFailure::new(
                            "analytics product listing failed",
                            AnalyticsQueryError::Store(error),
                        )
                    })?;
                    self.record_read(
                        principal,
                        None,
                        "analytics.list_products",
                        AnalyticsReadOutcome::Allowed,
                    )
                    .await
                    .map_err(|error| {
                        ApplicationWorkflowFailure::new(
                            "analytics product listing audit failed",
                            error,
                        )
                    })?;
                    Ok(products)
                },
            )
            .await
    }

    /// Reads admitted product metadata and records the read.
    pub async fn read_product(
        &self,
        principal: &AuthenticatedPrincipal,
        data_product_id: Uuid,
    ) -> Result<AnalyticsProduct, AnalyticsQueryError> {
        ApplicationOperationGateway::new(&self.runtime)
            .execute(
                ApplicationOperationId::AnalyticsReadProduct,
                |_context| async {
                    let product = self
                        .port
                        .read_product(principal, data_product_id)
                        .await
                        .map_err(|error| {
                            ApplicationWorkflowFailure::new(
                                "analytics product read failed",
                                AnalyticsQueryError::Store(error),
                            )
                        })?;
                    let outcome = if product.is_some() {
                        AnalyticsReadOutcome::Allowed
                    } else {
                        AnalyticsReadOutcome::NotFound
                    };
                    self.record_read(
                        principal,
                        Some(data_product_id),
                        "analytics.read_product",
                        outcome,
                    )
                    .await
                    .map_err(|error| {
                        ApplicationWorkflowFailure::new(
                            "analytics product read audit failed",
                            error,
                        )
                    })?;
                    product.ok_or_else(|| {
                        ApplicationWorkflowFailure::new(
                            "analytics product was not visible",
                            AnalyticsQueryError::NotFound,
                        )
                    })
                },
            )
            .await
    }

    /// Reads an admitted aggregate payload and records the read.
    pub async fn read_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        data_product_id: Uuid,
    ) -> Result<AnalyticsAggregate, AnalyticsQueryError> {
        ApplicationOperationGateway::new(&self.runtime)
            .execute(
                ApplicationOperationId::AnalyticsReadAggregate,
                |_context| async {
                    let aggregate = self
                        .port
                        .read_aggregate(principal, data_product_id)
                        .await
                        .map_err(|error| {
                            ApplicationWorkflowFailure::new(
                                "analytics aggregate read failed",
                                AnalyticsQueryError::Store(error),
                            )
                        })?;
                    let outcome = if aggregate.is_some() {
                        AnalyticsReadOutcome::Allowed
                    } else {
                        AnalyticsReadOutcome::NotFound
                    };
                    self.record_read(
                        principal,
                        Some(data_product_id),
                        "analytics.read_aggregate",
                        outcome,
                    )
                    .await
                    .map_err(|error| {
                        ApplicationWorkflowFailure::new(
                            "analytics aggregate read audit failed",
                            error,
                        )
                    })?;
                    aggregate.ok_or_else(|| {
                        ApplicationWorkflowFailure::new(
                            "analytics aggregate was not visible",
                            AnalyticsQueryError::NotFound,
                        )
                    })
                },
            )
            .await
    }
}

impl From<AnalyticsWorkflowKind> for ApplicationWorkflowSpec {
    fn from(kind: AnalyticsWorkflowKind) -> Self {
        kind.workflow_spec()
    }
}

const _: ApplicationWorkflowKind = ApplicationWorkflowKind::new("analytics.boundary");
