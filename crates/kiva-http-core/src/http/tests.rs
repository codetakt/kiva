use super::{
    BearerHeaderError, DocumentContentShape, DocumentDeleteDecision, DocumentJsonMergeDecision,
    DocumentWriteDecision, StatementBooleanParameter, StatementFormatParameter,
    StatementGetQueryDecision, StatementGetQueryParameter, StatementGetQueryParameterSet,
    StatementGetQueryParameters, StatementLimitParameter, StatementPutDecision,
    StatementTimestampParameter, bearer_token, document_delete_decision,
    document_json_merge_decision, document_preconditions_pass, document_put_decision,
    etag_list_matches, statement_get_query_decision, statement_put_decision,
    xapi_version_is_supported,
};

#[test]
fn supports_xapi_2_0_version_forms_only() {
    assert!(xapi_version_is_supported("2.0"));
    assert!(xapi_version_is_supported("2.0.0"));
    assert!(!xapi_version_is_supported("2.0.1"));
}

#[test]
fn extracts_bearer_token() {
    assert_eq!(bearer_token(None), Ok(None));
    assert_eq!(bearer_token(Some("Bearer token")), Ok(Some("token")));
    assert_eq!(
        bearer_token(Some("Bearer ")),
        Err(BearerHeaderError::EmptyToken)
    );
    assert_eq!(
        bearer_token(Some("Basic token")),
        Err(BearerHeaderError::UnsupportedScheme)
    );
}

#[test]
fn matches_etag_lists() {
    assert!(etag_list_matches("*", "\"a\""));
    assert!(etag_list_matches("\"a\", \"b\"", "\"b\""));
    assert!(!etag_list_matches("\"a\"", "\"b\""));
}

#[test]
fn evaluates_document_preconditions() {
    assert!(document_preconditions_pass(None, None, None));
    assert!(!document_preconditions_pass(None, Some("*"), None));
    assert!(document_preconditions_pass(Some("\"a\""), Some("*"), None));
    assert!(!document_preconditions_pass(Some("\"a\""), None, Some("*")));
    assert!(document_preconditions_pass(
        Some("\"a\""),
        None,
        Some("\"b\"")
    ));
}

#[test]
fn decides_document_put_contract() {
    assert_eq!(
        document_put_decision(false, true, Some(true)),
        DocumentWriteDecision::PreconditionFailed
    );
    assert_eq!(
        document_put_decision(true, false, Some(false)),
        DocumentWriteDecision::Conflict
    );
    assert_eq!(
        document_put_decision(true, false, Some(true)),
        DocumentWriteDecision::Noop
    );
    assert_eq!(
        document_put_decision(true, true, Some(false)),
        DocumentWriteDecision::Write
    );
    assert_eq!(
        document_put_decision(true, false, None),
        DocumentWriteDecision::Write
    );
}

#[test]
fn decides_document_delete_contract() {
    assert_eq!(
        document_delete_decision(false, true),
        DocumentDeleteDecision::PreconditionFailed
    );
    assert_eq!(
        document_delete_decision(true, true),
        DocumentDeleteDecision::Delete
    );
    assert_eq!(
        document_delete_decision(true, false),
        DocumentDeleteDecision::NotFound
    );
}

#[test]
fn decides_json_document_merge_contract() {
    assert_eq!(
        document_json_merge_decision(None, DocumentContentShape::JsonObject),
        DocumentJsonMergeDecision::Merge
    );
    assert_eq!(
        document_json_merge_decision(
            Some(DocumentContentShape::JsonObject),
            DocumentContentShape::JsonObject,
        ),
        DocumentJsonMergeDecision::Merge
    );
    assert_eq!(
        document_json_merge_decision(None, DocumentContentShape::JsonNonObject),
        DocumentJsonMergeDecision::InvalidPatch
    );
    assert_eq!(
        document_json_merge_decision(
            Some(DocumentContentShape::Bytes),
            DocumentContentShape::JsonObject,
        ),
        DocumentJsonMergeDecision::InvalidExisting
    );
}

#[test]
fn decides_statement_put_contract() {
    assert_eq!(statement_put_decision(None), StatementPutDecision::Store);
    assert_eq!(
        statement_put_decision(Some(true)),
        StatementPutDecision::Unchanged
    );
    assert_eq!(
        statement_put_decision(Some(false)),
        StatementPutDecision::Conflict
    );
}

#[test]
fn decides_statement_get_query_contract() {
    let base = StatementGetQueryParameters {
        parameters: StatementGetQueryParameterSet::EMPTY,
        related_activities: StatementBooleanParameter::Missing,
        related_agents: StatementBooleanParameter::Missing,
        format: StatementFormatParameter::Missing,
        attachments: StatementBooleanParameter::Missing,
        ascending: StatementBooleanParameter::Missing,
        since: StatementTimestampParameter::Missing,
        until: StatementTimestampParameter::Missing,
        limit: StatementLimitParameter::Missing,
    };

    assert_eq!(
        statement_get_query_decision(base),
        StatementGetQueryDecision::Allow
    );
    assert_eq!(
        statement_get_query_decision(StatementGetQueryParameters {
            parameters: base
                .parameters
                .with(StatementGetQueryParameter::StatementId)
                .with(StatementGetQueryParameter::VoidedStatementId),
            ..base
        }),
        StatementGetQueryDecision::ConflictingStatementIds
    );
    assert_eq!(
        statement_get_query_decision(StatementGetQueryParameters {
            parameters: base
                .parameters
                .with(StatementGetQueryParameter::StatementId),
            limit: StatementLimitParameter::Valid,
            ..base
        }),
        StatementGetQueryDecision::SingleStatementParameterConflict
    );
    assert_eq!(
        statement_get_query_decision(StatementGetQueryParameters {
            format: StatementFormatParameter::Invalid,
            ..base
        }),
        StatementGetQueryDecision::InvalidFormat
    );
    assert_eq!(
        statement_get_query_decision(StatementGetQueryParameters {
            related_agents: StatementBooleanParameter::Invalid,
            ..base
        }),
        StatementGetQueryDecision::InvalidBoolean
    );
    assert_eq!(
        statement_get_query_decision(StatementGetQueryParameters {
            since: StatementTimestampParameter::Invalid,
            ..base
        }),
        StatementGetQueryDecision::InvalidTimestamp
    );
    assert_eq!(
        statement_get_query_decision(StatementGetQueryParameters {
            limit: StatementLimitParameter::Invalid,
            ..base
        }),
        StatementGetQueryDecision::InvalidLimit
    );
}
