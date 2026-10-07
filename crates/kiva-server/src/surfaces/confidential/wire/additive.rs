//! Additive confidential-computation response projection.

use serde_json::Value;

use kiva_confidential::mechanism::additive::{
    AcceptedAdditiveShareRecord, ReplayWindowedAdditiveAggregate,
};
use kiva_confidential_contract::{
    AdditiveAggregateResponse, ReplayRecordMetadata, ReplayRecordMetadataList,
    ShareSigningKeyMetadata, ShareSigningKeyMetadataList, SubmittedShareMetadata,
    SubmittedShareMetadataList,
};

use crate::{
    core::response::hex_lower,
    surfaces::confidential::model::{ConfidentialShareSigningKey, ConfidentialSubmittedShare},
};

use super::dto_value;

pub(crate) fn aggregate_response(
    result: &ReplayWindowedAdditiveAggregate,
) -> Result<Value, serde_json::Error> {
    dto_value(&AdditiveAggregateResponse {
        aggregate: result.aggregate().value(),
        accepted_records: result
            .accepted_records()
            .iter()
            .map(replay_record_metadata)
            .collect(),
    })
}

pub(crate) fn share_signing_key_list_response(
    signing_keys: &[ConfidentialShareSigningKey],
) -> Result<Value, serde_json::Error> {
    dto_value(&ShareSigningKeyMetadataList {
        signing_keys: signing_keys
            .iter()
            .map(share_signing_key_metadata)
            .collect(),
    })
}

pub(crate) fn submitted_share_list_response(
    submissions: &[ConfidentialSubmittedShare],
) -> Result<Value, serde_json::Error> {
    dto_value(&SubmittedShareMetadataList {
        submissions: submissions.iter().map(submitted_share_metadata).collect(),
    })
}

pub(crate) fn replay_record_list_response(
    records: &[AcceptedAdditiveShareRecord],
) -> Result<Value, serde_json::Error> {
    dto_value(&ReplayRecordMetadataList {
        records: records.iter().map(replay_record_metadata).collect(),
    })
}

fn share_signing_key_metadata(
    signing_key: &ConfidentialShareSigningKey,
) -> ShareSigningKeyMetadata {
    ShareSigningKeyMetadata {
        signing_key_id: hex_lower(&signing_key.signing_key_id.bytes()),
        participant_index: signing_key.participant_index.get(),
        public_key: hex_lower(&signing_key.public_key.bytes()),
        oauth_subject: signing_key.oauth_subject.clone(),
        status: if signing_key.revoked_at.is_some() {
            "revoked".to_owned()
        } else {
            "active".to_owned()
        },
        revoked_at: signing_key.revoked_at.map(crate::core::response::rfc3339),
    }
}

fn submitted_share_metadata(submission: &ConfidentialSubmittedShare) -> SubmittedShareMetadata {
    let envelope = submission.signed_envelope.envelope();
    SubmittedShareMetadata {
        transcript_id: hex_lower(&envelope.transcript_id().bytes()),
        participant_index: envelope.share().participant_index().get(),
        sequence: envelope.sequence().value().get(),
        signing_key_id: hex_lower(&envelope.signing_key_id().bytes()),
        oauth_subject: submission.oauth_subject.clone(),
    }
}

fn replay_record_metadata(record: &AcceptedAdditiveShareRecord) -> ReplayRecordMetadata {
    ReplayRecordMetadata {
        transcript_id: hex_lower(&record.transcript_id().bytes()),
        participant_index: record.participant_index().get(),
        sequence: record.sequence().value().get(),
        signing_key_id: hex_lower(&record.signing_key_id().bytes()),
    }
}
