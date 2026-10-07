//! Record projections used by PDS server adapters.

use crate::surfaces::{
    confidential::outcome::ConfidentialAggregateStoreError,
    pds::{
        model::{
            PdsConsentGrantState, PdsDataProductState, PdsGovernanceProfileState,
            PdsLifecycleCaseState, PdsLifecycleEventState, PdsRetentionPolicyState,
        },
        outcome::PdsRepositoryError,
    },
    xapi::{model::StoredStatementMetadata, outcome::StatementStoreError},
};

use super::super::{
    ConsentGrantRecord, DataProductRecord, GovernanceProfileRecord, LifecycleCaseRecord,
    LifecycleEventRecord, PdsApplicationStoreError, ReleaseArtifactAvailabilityError,
    RetentionPolicyRecord, StatementReadError, StatementRecord,
};

pub(super) fn statement_error(error: StatementStoreError) -> StatementReadError {
    let StatementStoreError { source } = error;
    tracing::error!(error = ?source, "xAPI statement repository operation failed");
    StatementReadError::Store
}

pub(super) fn pds_error(error: PdsRepositoryError) -> PdsApplicationStoreError {
    match error {
        PdsRepositoryError::Runtime(message) => {
            tracing::error!(error = %message, "PDS repository operation runtime failed");
            PdsApplicationStoreError::Store
        }
        PdsRepositoryError::InvalidRequest(message) => {
            PdsApplicationStoreError::InvalidRequest(message)
        }
        PdsRepositoryError::Store { source } => {
            tracing::error!(error = ?source, "PDS repository operation failed");
            PdsApplicationStoreError::Store
        }
        PdsRepositoryError::InvalidStoredData => {
            tracing::error!("PDS repository returned invalid stored data");
            PdsApplicationStoreError::InvalidStoredData
        }
    }
}

pub(super) fn release_error(
    error: ConfidentialAggregateStoreError,
) -> ReleaseArtifactAvailabilityError {
    if let ConfidentialAggregateStoreError::Store { source } = error {
        tracing::error!(
            error = ?source,
            "confidential release-artifact repository operation failed"
        );
        ReleaseArtifactAvailabilityError::Store
    } else {
        tracing::error!(
            error = ?error,
            "confidential release-artifact repository returned invalid stored data"
        );
        ReleaseArtifactAvailabilityError::InvalidStoredData
    }
}

pub(super) fn statement_record(metadata: StoredStatementMetadata) -> StatementRecord {
    StatementRecord {
        statement_id: metadata.statement_id.to_string(),
        actor_ifi: metadata.actor_ifi,
        verb_id: metadata.verb_id,
        object_id: metadata.object_id,
        stored_at: metadata.stored_at,
        version: metadata.version,
    }
}

pub(super) fn consent_grant_record(state: PdsConsentGrantState) -> ConsentGrantRecord {
    ConsentGrantRecord {
        grant_id: state.grant_id,
        data_subject: state.data_subject,
        grantee_subject: state.grantee_subject,
        scope: state.scope,
        state: state.state,
        purpose: state.purpose,
        legal_basis: state.legal_basis,
        created_at: state.created_at,
        expires_at: state.expires_at,
        revoked_at: state.revoked_at,
        renewed_from_grant_id: state.renewed_from_grant_id,
    }
}

pub(super) fn data_product_record(state: PdsDataProductState) -> DataProductRecord {
    DataProductRecord {
        data_product_id: state.data_product_id,
        owner_subject: state.owner_subject,
        release_artifact: state.release_artifact,
        title: state.title,
        purpose: state.purpose,
        legal_basis: state.legal_basis,
        data_classes: state.data_classes,
        created_at: state.created_at,
    }
}

pub(super) fn lifecycle_event_record(state: PdsLifecycleEventState) -> LifecycleEventRecord {
    LifecycleEventRecord {
        event_id: state.event_id,
        data_subject: state.data_subject,
        event_type: state.event_type,
        resource_kind: state.resource_kind,
        resource_id: state.resource_id,
        reason: state.reason,
        created_at: state.created_at,
    }
}

pub(super) fn retention_policy_record(state: PdsRetentionPolicyState) -> RetentionPolicyRecord {
    RetentionPolicyRecord {
        policy_id: state.policy_id,
        owner_subject: state.owner_subject,
        resource_kind: state.resource_kind,
        data_class: state.data_class,
        retention_period_days: state.retention_period_days,
        review_interval_days: state.review_interval_days,
        tombstone_after_days: state.tombstone_after_days,
        tombstone_publication: state.tombstone_publication,
        legal_basis: state.legal_basis,
        description: state.description,
        created_at: state.created_at,
    }
}

pub(super) fn lifecycle_case_record(state: PdsLifecycleCaseState) -> LifecycleCaseRecord {
    LifecycleCaseRecord {
        case_id: state.case_id,
        data_subject: state.data_subject,
        case_type: state.case_type,
        resource_kind: state.resource_kind,
        resource_id: state.resource_id,
        status: state.status,
        retention_policy_id: state.retention_policy_id,
        summary: state.summary,
        resolution: state.resolution,
        tombstone_publication: state.tombstone_publication,
        created_at: state.created_at,
        updated_at: state.updated_at,
    }
}

pub(super) fn governance_profile_record(
    state: PdsGovernanceProfileState,
) -> GovernanceProfileRecord {
    GovernanceProfileRecord {
        profile_id: state.profile_id,
        profile_owner_subject: state.profile_owner_subject,
        profile_kind: state.profile_kind,
        tenant_partition: state.tenant_partition,
        subject_partition: state.subject_partition,
        data_custodian_subject: state.data_custodian_subject,
        platform_operator_subject: state.platform_operator_subject,
        policy_authority_subject: state.policy_authority_subject,
        analyst_subject: state.analyst_subject,
        auditor_subject: state.auditor_subject,
        identity_authority_subject: state.identity_authority_subject,
        emergency_access_policy: state.emergency_access_policy,
        runbook_refs: state.runbook_refs,
        created_at: state.created_at,
    }
}
