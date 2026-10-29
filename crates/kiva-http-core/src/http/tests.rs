use super::{
    BearerHeaderError, DocumentContentShape, DocumentDeleteDecision, DocumentJsonMergeDecision,
    DocumentWriteDecision, StatementBooleanParameter, StatementFormatParameter,
    StatementGetQueryDecision, StatementGetQueryParameter, StatementGetQueryParameterSet,
    StatementGetQueryParameters, StatementLimitParameter, StatementPutDecision,
    StatementTimestampParameter, bearer_token, document_delete_decision,
    document_json_merge_decision, document_preconditions_pass, document_put_decision,
    etag_list_matches, etag_list_matches_weak, statement_get_query_decision,
    statement_put_decision, xapi_version_is_supported,
};

#[test]
fn supports_normal_xapi_2_0_patches_without_integer_narrowing() {
    assert!(xapi_version_is_supported("2.0"));
    assert!(xapi_version_is_supported("2.0.0"));
    for value in [
        "2.0.1",
        "2.0.999999",
        "2.0.184467440737095516160000000000000000000000000000001",
    ] {
        assert!(xapi_version_is_supported(value), "{value}");
    }
    for value in [
        "",
        "2",
        "2.0.",
        "2.0.-1",
        "2.0.01",
        "2.0.1-beta",
        "2.0.1+build",
        "2.0.１",
        "2.1.0",
        "1.0.3",
        " 2.0.0",
        "2.0.0 ",
    ] {
        assert!(!xapi_version_is_supported(value), "{value}");
    }
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
    for (header, tag, expected) in [
        (r#""stale,*,tag""#, r#""a""#, false),
        (r#""stale\",*,tag""#, r#""a""#, false),
        (r#""*""#, r#""a""#, false),
        (r#""stale,*,tag", "a""#, r#""a""#, true),
        (r#""stale\",*,tag", "a""#, r#""a""#, true),
        (r#""a,b""#, r#""a,b""#, true),
        (r#""\a""#, r#""a""#, true),
        (r#"W/"a""#, r#""a""#, false),
        (r#""a""#, r#"W/"a""#, false),
        (r#"W/"a""#, r#"W/"a""#, false),
        (r#""a", *"#, r#""a""#, false),
        (r#""a", "unterminated"#, r#""a""#, false),
        (r#""a"junk"#, r#""a""#, false),
        (", \t\"a\", ,", r#""a""#, true),
    ] {
        assert_eq!(
            etag_list_matches(header, tag),
            expected,
            "{header} vs {tag}"
        );
    }
    for (header, tag, expected) in [
        (r#"W/"a""#, r#""a""#, true),
        (r#""a""#, r#"W/"a""#, true),
        (r#"W/"a""#, r#"W/"a""#, true),
        (r#"W/"a,b""#, r#""a,b""#, true),
        (r#"W/"stale,*,tag""#, r#""a""#, false),
        (r#""stale\",*,tag", W/"a""#, r#""a""#, true),
        (r#""a", *"#, r#""a""#, false),
        (r#"W/"a", "unterminated"#, r#""a""#, false),
    ] {
        assert_eq!(
            etag_list_matches_weak(header, tag),
            expected,
            "{header} vs {tag}"
        );
    }
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
        document_put_decision(false, true, true),
        DocumentWriteDecision::PreconditionFailed
    );
    assert_eq!(
        document_put_decision(true, false, true),
        DocumentWriteDecision::Conflict
    );
    assert_eq!(
        document_put_decision(true, true, true),
        DocumentWriteDecision::Write
    );
    assert_eq!(
        document_put_decision(true, false, false),
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
