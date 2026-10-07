//! Record-inventory and metadata-export PDS wire projection.

use kiva_pds::{
    MetadataExport as MetadataExportDto, RecordInventory as RecordInventoryDto,
    StatementInventoryRecord as StatementInventoryRecordDto,
};
use kiva_pds_openapi_contract::{
    PortabilityExport as PortabilityExportDto, PortabilityStatement as PortabilityStatementDto,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use time::OffsetDateTime;

use crate::{
    app::pds as pds_app, core::response::rfc3339, surfaces::pds::model::pds_subject_actor_ifi,
};

use super::{
    common::{dto_value, subject_actor_ifi, subject_agent_dto},
    consent::consent_grant_dto,
    data_products::data_product_dto,
    governance::governance_profile_dto,
    lifecycle::{lifecycle_case_dto, lifecycle_event_dto, retention_policy_dto},
};

#[derive(Clone, Copy)]
pub(crate) struct RecordInventoryView<'a> {
    data_subject: &'a str,
    statements: &'a [pds_app::StatementRecord],
    data_products: &'a [pds_app::DataProductRecord],
}

impl<'a> RecordInventoryView<'a> {
    pub(crate) fn from_inventory(inventory: &'a pds_app::RecordInventory) -> Self {
        Self {
            data_subject: inventory.data_subject.as_str(),
            statements: &inventory.statements,
            data_products: &inventory.data_products,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct MetadataExportView<'a> {
    data_subject: &'a str,
    statements: &'a [pds_app::StatementRecord],
    data_products: &'a [pds_app::DataProductRecord],
    consent_grants: &'a [pds_app::ConsentGrantRecord],
    lifecycle_events: &'a [pds_app::LifecycleEventRecord],
    retention_policies: &'a [pds_app::RetentionPolicyRecord],
    lifecycle_cases: &'a [pds_app::LifecycleCaseRecord],
    governance_profiles: &'a [pds_app::GovernanceProfileRecord],
}

impl<'a> MetadataExportView<'a> {
    pub(crate) fn from_export(export: &'a pds_app::MetadataExport) -> Self {
        Self {
            data_subject: export.data_subject.as_str(),
            statements: &export.statements,
            data_products: &export.data_products,
            consent_grants: &export.consent_grants,
            lifecycle_events: &export.lifecycle_events,
            retention_policies: &export.retention_policies,
            lifecycle_cases: &export.lifecycle_cases,
            governance_profiles: &export.governance_profiles,
        }
    }

    const fn inventory_view(&self) -> RecordInventoryView<'a> {
        RecordInventoryView {
            data_subject: self.data_subject,
            statements: self.statements,
            data_products: self.data_products,
        }
    }
}

pub(crate) fn record_inventory_response(
    view: &RecordInventoryView<'_>,
) -> Result<Value, serde_json::Error> {
    dto_value(&record_inventory_dto(view))
}

pub(crate) fn metadata_export_response(
    view: &MetadataExportView<'_>,
) -> Result<Value, serde_json::Error> {
    dto_value(&MetadataExportDto {
        data_subject: view.data_subject.to_owned(),
        actor: subject_agent_dto(view.data_subject),
        actor_ifi: subject_actor_ifi(view.data_subject),
        generated_at: rfc3339(OffsetDateTime::now_utc()),
        record_inventory: record_inventory_dto(&view.inventory_view()),
        consent_grants: view.consent_grants.iter().map(consent_grant_dto).collect(),
        data_products: view.data_products.iter().map(data_product_dto).collect(),
        lifecycle_events: view
            .lifecycle_events
            .iter()
            .map(lifecycle_event_dto)
            .collect(),
        retention_policies: view
            .retention_policies
            .iter()
            .map(retention_policy_dto)
            .collect(),
        lifecycle_cases: view
            .lifecycle_cases
            .iter()
            .map(lifecycle_case_dto)
            .collect(),
        governance_profiles: view
            .governance_profiles
            .iter()
            .map(governance_profile_dto)
            .collect(),
    })
}

pub(crate) fn portability_export_response(
    export: &pds_app::PortabilityExport,
) -> Result<Value, serde_json::Error> {
    dto_value(&PortabilityExportDto {
        data_subject: export.data_subject.as_str().to_owned(),
        actor: contract_dto(&subject_agent_dto(export.data_subject.as_str()))?,
        actor_ifi: subject_actor_ifi(export.data_subject.as_str()),
        generated_at: rfc3339(export.generated_at),
        audit_reference: export.audit_reference.to_string(),
        statements: export
            .statements
            .iter()
            .map(|record| PortabilityStatementDto {
                statement_id: record.statement_id.to_string(),
                stored_at: rfc3339(record.stored_at),
                voided: record.voided,
                statement: record.statement.clone(),
            })
            .collect(),
        next_cursor: export.next_cursor.clone(),
        consent_grants: export
            .consent_grants
            .iter()
            .map(|grant| contract_dto(&consent_grant_dto(grant)))
            .collect::<Result<_, _>>()?,
        data_products: export
            .data_products
            .iter()
            .map(|product| contract_dto(&data_product_dto(product)))
            .collect::<Result<_, _>>()?,
        lifecycle_events: export
            .lifecycle_events
            .iter()
            .map(|event| contract_dto(&lifecycle_event_dto(event)))
            .collect::<Result<_, _>>()?,
        retention_policies: export
            .retention_policies
            .iter()
            .map(|policy| contract_dto(&retention_policy_dto(policy)))
            .collect::<Result<_, _>>()?,
        lifecycle_cases: export
            .lifecycle_cases
            .iter()
            .map(|case| contract_dto(&lifecycle_case_dto(case)))
            .collect::<Result<_, _>>()?,
        governance_profiles: export
            .governance_profiles
            .iter()
            .map(|profile| contract_dto(&governance_profile_dto(profile)))
            .collect::<Result<_, _>>()?,
    })
}

fn contract_dto<T, U>(value: &U) -> Result<T, serde_json::Error>
where
    T: DeserializeOwned,
    U: Serialize,
{
    serde_json::from_value(serde_json::to_value(value)?)
}

fn record_inventory_dto(view: &RecordInventoryView<'_>) -> RecordInventoryDto {
    RecordInventoryDto {
        data_subject: view.data_subject.to_owned(),
        actor: subject_agent_dto(view.data_subject),
        actor_ifi: subject_actor_ifi(view.data_subject),
        statement_records: statement_inventory_records(view.data_subject, view.statements),
        data_products: view.data_products.iter().map(data_product_dto).collect(),
    }
}

fn statement_inventory_records(
    data_subject: &str,
    statements: &[pds_app::StatementRecord],
) -> Vec<StatementInventoryRecordDto> {
    let subject_ifi = pds_subject_actor_ifi(data_subject);
    statements
        .iter()
        .filter_map(|statement| {
            if statement.actor_ifi != subject_ifi {
                return None;
            }
            let verb_id = statement.verb_id.clone()?;
            let object_id = statement.object_id.clone()?;
            Some(StatementInventoryRecordDto {
                statement_id: statement.statement_id.clone(),
                actor_ifi: statement.actor_ifi.clone(),
                verb_id,
                object_id,
                stored_at: rfc3339(statement.stored_at),
                version: statement.version.clone(),
            })
        })
        .collect()
}
