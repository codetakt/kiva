//! Approved data-product PDS wire projection.

use kiva_pds::{
    DataClass, DataProduct as DataProductDto, DataProductList,
    DataProductReadResponse as DataProductReadResponseDto, LegalBasis, ReleaseArtifactRef,
};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    app::pds as pds_app,
    core::response::rfc3339,
    surfaces::pds::{
        model::{PdsDataProductReadOutcome, PdsDataProductState},
        model::{pds_decision_label, pds_deny_reason, pds_deny_reason_label},
    },
};

use super::common::dto_value;

pub(crate) fn data_product_list_response(
    data_products: &[PdsDataProductState],
) -> Result<Value, serde_json::Error> {
    dto_value(&DataProductList {
        data_products: data_products.iter().map(data_product_dto).collect(),
    })
}

pub(crate) fn data_product_read_response(
    outcome: &PdsDataProductReadOutcome,
) -> Result<Value, serde_json::Error> {
    dto_value(&DataProductReadResponseDto {
        event_id: outcome.event.event_id.to_string(),
        decision: pds_decision_label(outcome.event.decision).to_owned(),
        deny_reason: pds_deny_reason(outcome.event.decision)
            .map(pds_deny_reason_label)
            .map(str::to_owned),
        data_product: outcome.data_product.as_ref().map(data_product_dto),
    })
}

pub(super) trait DataProductProjection {
    fn data_product_id(&self) -> Uuid;
    fn owner_subject(&self) -> &str;
    fn release_artifact(&self) -> ReleaseArtifactRef;
    fn title(&self) -> &str;
    fn purpose(&self) -> &str;
    fn legal_basis(&self) -> LegalBasis;
    fn data_classes(&self) -> &[DataClass];
    fn created_at(&self) -> OffsetDateTime;
}

impl DataProductProjection for PdsDataProductState {
    fn data_product_id(&self) -> Uuid {
        self.data_product_id
    }

    fn owner_subject(&self) -> &str {
        &self.owner_subject
    }

    fn release_artifact(&self) -> ReleaseArtifactRef {
        self.release_artifact()
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn purpose(&self) -> &str {
        &self.purpose
    }

    fn legal_basis(&self) -> LegalBasis {
        self.legal_basis
    }

    fn data_classes(&self) -> &[DataClass] {
        &self.data_classes
    }

    fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

impl DataProductProjection for pds_app::DataProductRecord {
    fn data_product_id(&self) -> Uuid {
        self.data_product_id
    }

    fn owner_subject(&self) -> &str {
        &self.owner_subject
    }

    fn release_artifact(&self) -> ReleaseArtifactRef {
        self.release_artifact
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn purpose(&self) -> &str {
        &self.purpose
    }

    fn legal_basis(&self) -> LegalBasis {
        self.legal_basis
    }

    fn data_classes(&self) -> &[DataClass] {
        &self.data_classes
    }

    fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

pub(super) fn data_product_dto<T: DataProductProjection>(product: &T) -> DataProductDto {
    let release_artifact = product.release_artifact();
    DataProductDto {
        data_product_id: product.data_product_id().to_string(),
        owner_subject: product.owner_subject().to_owned(),
        source_kind: release_artifact.source_kind(),
        release_id: release_artifact.artifact_id_hex(),
        release_artifact,
        title: product.title().to_owned(),
        purpose: product.purpose().to_owned(),
        legal_basis: product.legal_basis(),
        data_classes: product.data_classes().to_vec(),
        created_at: rfc3339(product.created_at()),
    }
}
