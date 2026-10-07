use super::{
    BearerHeaderError, DocumentContentShape, DocumentDeleteDecision, DocumentJsonMergeDecision,
    DocumentWriteDecision, StatementBooleanParameter, StatementFormatParameter,
    StatementGetQueryDecision, StatementGetQueryParameter, StatementGetQueryParameterSet,
    StatementGetQueryParameters, StatementLimitParameter, StatementPutDecision,
    StatementTimestampParameter, bearer_token_start, document_delete_decision,
    document_json_merge_decision, document_preconditions_pass, document_put_decision,
    etag_list_matches, statement_get_query_decision, statement_put_decision,
    xapi_version_is_supported,
};

fn any_known_version() -> &'static str {
    match kani::any::<u8>() % 5 {
        0 => "2.0",
        1 => "2.0.0",
        2 => "2",
        3 => "2.0.1",
        _ => "",
    }
}

fn any_authorization_header() -> Option<&'static str> {
    match kani::any::<u8>() % 5 {
        0 => None,
        1 => Some("Bearer token"),
        2 => Some("Bearer "),
        3 => Some("Basic token"),
        _ => Some("Bearer token with spaces"),
    }
}

fn any_etag() -> &'static str {
    match kani::any::<u8>() % 2 {
        0 => "\"a\"",
        _ => "\"b\"",
    }
}

fn any_etag_condition() -> Option<&'static str> {
    match kani::any::<u8>() % 5 {
        0 => None,
        1 => Some("*"),
        2 => Some("\"a\""),
        3 => Some("\"b\""),
        _ => Some("\"a\", \"b\""),
    }
}

fn any_existing_etag() -> Option<&'static str> {
    match kani::any::<u8>() % 3 {
        0 => None,
        1 => Some("\"a\""),
        _ => Some("\"b\""),
    }
}

fn any_existing_content_matches() -> Option<bool> {
    match kani::any::<u8>() % 3 {
        0 => None,
        1 => Some(false),
        _ => Some(true),
    }
}

fn any_content_shape() -> DocumentContentShape {
    match kani::any::<u8>() % 3 {
        0 => DocumentContentShape::JsonObject,
        1 => DocumentContentShape::JsonNonObject,
        _ => DocumentContentShape::Bytes,
    }
}

fn any_existing_content_shape() -> Option<DocumentContentShape> {
    match kani::any::<u8>() % 4 {
        0 => None,
        1 => Some(DocumentContentShape::JsonObject),
        2 => Some(DocumentContentShape::JsonNonObject),
        _ => Some(DocumentContentShape::Bytes),
    }
}

fn any_statement_get_query_parameter_set() -> StatementGetQueryParameterSet {
    let selector = kani::any::<u8>();
    let mut parameters = StatementGetQueryParameterSet::EMPTY;
    if selector & 1 != 0 {
        parameters = parameters.with(StatementGetQueryParameter::Unknown);
    }
    if selector & 2 != 0 {
        parameters = parameters.with(StatementGetQueryParameter::StatementId);
    }
    if selector & 4 != 0 {
        parameters = parameters.with(StatementGetQueryParameter::VoidedStatementId);
    }
    if selector & 8 != 0 {
        parameters = parameters.with(StatementGetQueryParameter::Agent);
    }
    if selector & 16 != 0 {
        parameters = parameters.with(StatementGetQueryParameter::Verb);
    }
    if selector & 32 != 0 {
        parameters = parameters.with(StatementGetQueryParameter::Activity);
    }
    if selector & 64 != 0 {
        parameters = parameters.with(StatementGetQueryParameter::Registration);
    }
    parameters
}

fn any_statement_format_parameter() -> StatementFormatParameter {
    match kani::any::<u8>() % 5 {
        0 => StatementFormatParameter::Missing,
        1 => StatementFormatParameter::Exact,
        2 => StatementFormatParameter::Canonical,
        3 => StatementFormatParameter::Ids,
        _ => StatementFormatParameter::Invalid,
    }
}

fn any_statement_boolean_parameter() -> StatementBooleanParameter {
    match kani::any::<u8>() % 4 {
        0 => StatementBooleanParameter::Missing,
        1 => StatementBooleanParameter::True,
        2 => StatementBooleanParameter::False,
        _ => StatementBooleanParameter::Invalid,
    }
}

fn any_valid_statement_boolean_parameter() -> StatementBooleanParameter {
    match kani::any::<u8>() % 3 {
        0 => StatementBooleanParameter::Missing,
        1 => StatementBooleanParameter::True,
        _ => StatementBooleanParameter::False,
    }
}

fn any_statement_timestamp_parameter() -> StatementTimestampParameter {
    match kani::any::<u8>() % 3 {
        0 => StatementTimestampParameter::Missing,
        1 => StatementTimestampParameter::Valid,
        _ => StatementTimestampParameter::Invalid,
    }
}

fn any_valid_statement_timestamp_parameter() -> StatementTimestampParameter {
    match kani::any::<u8>() % 2 {
        0 => StatementTimestampParameter::Missing,
        _ => StatementTimestampParameter::Valid,
    }
}

fn any_statement_limit_parameter() -> StatementLimitParameter {
    match kani::any::<u8>() % 3 {
        0 => StatementLimitParameter::Missing,
        1 => StatementLimitParameter::Valid,
        _ => StatementLimitParameter::Invalid,
    }
}

fn any_valid_statement_limit_parameter() -> StatementLimitParameter {
    match kani::any::<u8>() % 2 {
        0 => StatementLimitParameter::Missing,
        _ => StatementLimitParameter::Valid,
    }
}

fn any_statement_get_query_parameters() -> StatementGetQueryParameters {
    StatementGetQueryParameters {
        parameters: any_statement_get_query_parameter_set(),
        related_activities: any_statement_boolean_parameter(),
        related_agents: any_statement_boolean_parameter(),
        format: any_statement_format_parameter(),
        attachments: any_statement_boolean_parameter(),
        ascending: any_statement_boolean_parameter(),
        since: any_statement_timestamp_parameter(),
        until: any_statement_timestamp_parameter(),
        limit: any_statement_limit_parameter(),
    }
}

fn known_condition_matches(condition: &str, etag: &str) -> bool {
    condition == "*"
        || condition == etag
        || (condition == "\"a\", \"b\"" && (etag == "\"a\"" || etag == "\"b\""))
}

#[kani::proof]
fn xapi_version_support_is_exact_for_known_inputs() {
    let value = any_known_version();
    assert!(xapi_version_is_supported(value) == (value == "2.0" || value == "2.0.0"));
}

#[kani::proof]
fn bearer_token_classification_is_exact_for_known_inputs() {
    let value = any_authorization_header();
    let result = bearer_token_start(value);
    match value {
        None => assert!(matches!(result, Ok(None))),
        Some("Bearer token") => assert!(matches!(result, Ok(Some(7)))),
        Some("Bearer ") => assert!(matches!(result, Err(BearerHeaderError::EmptyToken))),
        Some("Basic token") => {
            assert!(matches!(result, Err(BearerHeaderError::UnsupportedScheme)));
        }
        Some("Bearer token with spaces") => {
            assert!(matches!(result, Ok(Some(7))));
        }
        Some(_) => assert!(false),
    }
}

#[kani::proof]
fn etag_matching_honors_star_exact_values_and_comma_lists() {
    let condition = any_etag_condition().unwrap_or("\"a\"");
    let etag = any_etag();

    assert!(etag_list_matches(condition, etag) == known_condition_matches(condition, etag));
}

#[kani::proof]
fn document_preconditions_follow_if_match_and_if_none_match_rules() {
    let existing = any_existing_etag();
    let if_match = any_etag_condition();
    let if_none_match = any_etag_condition();

    let if_match_passes = if_match.is_none_or(|condition| {
        existing.is_some_and(|etag| known_condition_matches(condition, etag))
    });
    let if_none_match_passes = match (if_none_match, existing) {
        (Some(condition), Some(etag)) => !known_condition_matches(condition, etag),
        (None | Some(_), None) | (None, Some(_)) => true,
    };
    let expected = if_match_passes && if_none_match_passes;

    assert!(document_preconditions_pass(existing, if_match, if_none_match) == expected);
}

#[kani::proof]
fn document_put_decision_fails_closed_on_precondition_failure() {
    let has_preconditions = kani::any();
    let existing_content_matches = any_existing_content_matches();

    assert_eq!(
        document_put_decision(false, has_preconditions, existing_content_matches),
        DocumentWriteDecision::PreconditionFailed
    );
}

#[kani::proof]
fn document_put_decision_conflicts_only_without_preconditions_on_changed_existing_content() {
    let has_preconditions = kani::any();
    let existing_content_matches = any_existing_content_matches();
    let decision = document_put_decision(true, has_preconditions, existing_content_matches);

    assert!(
        matches!(decision, DocumentWriteDecision::Conflict)
            == (existing_content_matches == Some(false) && !has_preconditions)
    );
}

#[kani::proof]
fn document_delete_decision_fails_closed_on_precondition_failure() {
    let existing_content_present = kani::any();

    assert_eq!(
        document_delete_decision(false, existing_content_present),
        DocumentDeleteDecision::PreconditionFailed
    );
}

#[kani::proof]
fn document_delete_decision_deletes_only_existing_content_after_preconditions_pass() {
    let existing_content_present = kani::any();
    let decision = document_delete_decision(true, existing_content_present);

    assert!(matches!(decision, DocumentDeleteDecision::Delete) == existing_content_present);
    assert!(matches!(decision, DocumentDeleteDecision::NotFound) == !existing_content_present);
}

#[kani::proof]
fn document_json_merge_requires_object_patch() {
    let existing_shape = any_existing_content_shape();
    let patch_shape = any_content_shape();
    let decision = document_json_merge_decision(existing_shape, patch_shape);

    assert!(
        matches!(decision, DocumentJsonMergeDecision::InvalidPatch)
            == (patch_shape != DocumentContentShape::JsonObject)
    );
}

#[kani::proof]
fn document_json_merge_allows_only_empty_or_object_existing_content() {
    let existing_shape = any_existing_content_shape();
    let decision = document_json_merge_decision(existing_shape, DocumentContentShape::JsonObject);

    assert!(
        matches!(decision, DocumentJsonMergeDecision::Merge)
            == matches!(
                existing_shape,
                None | Some(DocumentContentShape::JsonObject)
            )
    );
}

#[kani::proof]
fn statement_put_decision_matches_immutability_contract() {
    let existing_statement_matches = any_existing_content_matches();
    let decision = statement_put_decision(existing_statement_matches);

    match existing_statement_matches {
        None => assert_eq!(decision, StatementPutDecision::Store),
        Some(true) => assert_eq!(decision, StatementPutDecision::Unchanged),
        Some(false) => assert_eq!(decision, StatementPutDecision::Conflict),
    }
}

#[kani::proof]
fn statement_get_query_rejects_conflicting_statement_ids() {
    let query = StatementGetQueryParameters {
        parameters: StatementGetQueryParameterSet::EMPTY
            .with(StatementGetQueryParameter::StatementId)
            .with(StatementGetQueryParameter::VoidedStatementId),
        related_activities: any_statement_boolean_parameter(),
        related_agents: any_statement_boolean_parameter(),
        format: any_statement_format_parameter(),
        attachments: any_statement_boolean_parameter(),
        ascending: any_statement_boolean_parameter(),
        since: any_statement_timestamp_parameter(),
        until: any_statement_timestamp_parameter(),
        limit: any_statement_limit_parameter(),
    };

    assert_eq!(
        statement_get_query_decision(query),
        StatementGetQueryDecision::ConflictingStatementIds
    );
}

#[kani::proof]
fn statement_get_query_single_statement_rejects_search_parameters() {
    let query = StatementGetQueryParameters {
        parameters: StatementGetQueryParameterSet::EMPTY
            .with(StatementGetQueryParameter::StatementId)
            .with(StatementGetQueryParameter::Agent),
        related_activities: any_valid_statement_boolean_parameter(),
        related_agents: any_valid_statement_boolean_parameter(),
        format: StatementFormatParameter::Exact,
        attachments: any_valid_statement_boolean_parameter(),
        ascending: any_valid_statement_boolean_parameter(),
        since: any_valid_statement_timestamp_parameter(),
        until: any_valid_statement_timestamp_parameter(),
        limit: any_valid_statement_limit_parameter(),
    };

    assert_eq!(
        statement_get_query_decision(query),
        StatementGetQueryDecision::SingleStatementParameterConflict
    );
}

#[kani::proof]
fn statement_get_query_rejects_invalid_scalar_parameters() {
    let mut query = any_statement_get_query_parameters();
    query.parameters = StatementGetQueryParameterSet::EMPTY;

    let decision = statement_get_query_decision(query);
    let has_invalid_boolean = matches!(query.attachments, StatementBooleanParameter::Invalid)
        || matches!(query.related_activities, StatementBooleanParameter::Invalid)
        || matches!(query.related_agents, StatementBooleanParameter::Invalid)
        || matches!(query.ascending, StatementBooleanParameter::Invalid);
    let has_invalid_timestamp = matches!(query.since, StatementTimestampParameter::Invalid)
        || matches!(query.until, StatementTimestampParameter::Invalid);

    if matches!(query.format, StatementFormatParameter::Invalid) {
        assert_eq!(decision, StatementGetQueryDecision::InvalidFormat);
    } else if has_invalid_boolean {
        assert_eq!(decision, StatementGetQueryDecision::InvalidBoolean);
    } else if has_invalid_timestamp {
        assert_eq!(decision, StatementGetQueryDecision::InvalidTimestamp);
    } else if matches!(query.limit, StatementLimitParameter::Invalid) {
        assert_eq!(decision, StatementGetQueryDecision::InvalidLimit);
    } else {
        assert_eq!(decision, StatementGetQueryDecision::Allow);
    }
}

#[kani::proof]
fn statement_get_query_allows_collection_filters_when_scalars_are_valid() {
    let query = StatementGetQueryParameters {
        parameters: StatementGetQueryParameterSet::EMPTY
            .with(StatementGetQueryParameter::Agent)
            .with(StatementGetQueryParameter::Verb)
            .with(StatementGetQueryParameter::Activity)
            .with(StatementGetQueryParameter::Registration),
        related_activities: any_valid_statement_boolean_parameter(),
        related_agents: any_valid_statement_boolean_parameter(),
        format: StatementFormatParameter::Canonical,
        attachments: any_valid_statement_boolean_parameter(),
        ascending: any_valid_statement_boolean_parameter(),
        since: any_valid_statement_timestamp_parameter(),
        until: any_valid_statement_timestamp_parameter(),
        limit: any_valid_statement_limit_parameter(),
    };

    assert_eq!(
        statement_get_query_decision(query),
        StatementGetQueryDecision::Allow
    );
}
