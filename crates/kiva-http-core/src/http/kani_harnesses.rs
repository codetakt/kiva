use super::{
    BearerHeaderError, DocumentContentShape, DocumentDeleteDecision, DocumentJsonMergeDecision,
    DocumentWriteDecision, StatementBooleanParameter, StatementFormatParameter,
    StatementGetQueryDecision, StatementGetQueryParameter, StatementGetQueryParameterSet,
    StatementGetQueryParameters, StatementLimitParameter, StatementPutDecision,
    StatementTimestampParameter, bearer_token_start, document_delete_decision,
    document_json_merge_decision, document_preconditions_pass, document_put_decision,
    etag_list_matches, etag_list_matches_weak, statement_get_query_decision,
    statement_put_decision, xapi_version_is_supported,
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

// Enumerate every finite input explicitly so the verifier can specialize the
// parser loops to each literal, instead of exploring symbolic slice aliases.
// Independent masks use bit 0 for "a" and bit 1 for "b", with separate
// strong/weak results. None is compared as "a" by the matching proof.
macro_rules! each_etag_condition {
    ($check:ident $(, $argument:expr)*) => {
        $check(None, 1, 1 $(, $argument)*);
        $check(Some("*"), 3, 3 $(, $argument)*);
        $check(Some("\"a\""), 1, 1 $(, $argument)*);
        $check(Some("\"b\""), 2, 2 $(, $argument)*);
        $check(Some("\"a\", \"b\""), 3, 3 $(, $argument)*);
        $check(Some(r#""a,*,b""#), 0, 0 $(, $argument)*);
        $check(Some(r#""a\",*,b""#), 0, 0 $(, $argument)*);
        $check(Some(r#""a,*,b", "a""#), 1, 1 $(, $argument)*);
        $check(Some(r#"W/"a""#), 0, 1 $(, $argument)*);
    };
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

#[kani::proof]
fn xapi_version_support_is_exact_for_known_inputs() {
    let value = any_known_version();
    assert!(
        xapi_version_is_supported(value)
            == (value == "2.0" || value == "2.0.0" || value == "2.0.1")
    );
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
    // Same 2 x 9 Cartesian product for both strong and weak comparison.
    each_etag_condition!(check_etag_matching);
}

fn check_etag_matching(condition: Option<&str>, strong: u8, weak: u8) {
    let condition = condition.unwrap_or("\"a\"");
    assert!(etag_list_matches(condition, "\"a\"") == (strong & 1 != 0));
    assert!(etag_list_matches(condition, "\"b\"") == (strong & 2 != 0));
    assert!(etag_list_matches_weak(condition, "\"a\"") == (weak & 1 != 0));
    assert!(etag_list_matches_weak(condition, "\"b\"") == (weak & 2 != 0));
}

#[kani::proof]
fn document_preconditions_follow_if_match_and_if_none_match_rules() {
    // Together these three proofs cover all 3 x 9 x 9 combinations. Keeping
    // the existence states separate bounds each CI verification job's cost.
    each_etag_condition!(check_document_if_match, None, 0);
}

#[kani::proof]
fn document_preconditions_with_existing_etag_a() {
    each_etag_condition!(check_document_if_match, Some("\"a\""), 1);
}

#[kani::proof]
fn document_preconditions_with_existing_etag_b() {
    each_etag_condition!(check_document_if_match, Some("\"b\""), 2);
}

fn check_document_if_match(
    if_match: Option<&str>,
    if_match_mask: u8,
    _weak: u8,
    existing: Option<&str>,
    existing_mask: u8,
) {
    each_etag_condition!(
        check_document_conditions,
        if_match,
        if_match_mask,
        existing,
        existing_mask
    );
}

fn check_document_conditions(
    if_none_match: Option<&str>,
    if_none_match_mask: u8,
    _weak: u8,
    if_match: Option<&str>,
    if_match_mask: u8,
    existing: Option<&str>,
    existing_mask: u8,
) {
    let if_match_passes =
        if_match.is_none() || (existing.is_some() && if_match_mask & existing_mask != 0);
    let if_none_match_passes =
        if_none_match.is_none() || existing.is_none() || if_none_match_mask & existing_mask == 0;
    let expected = if_match_passes && if_none_match_passes;

    assert!(document_preconditions_pass(existing, if_match, if_none_match) == expected);
}

#[kani::proof]
fn document_put_decision_fails_closed_on_precondition_failure() {
    let has_preconditions = kani::any();
    let existing_content_present = kani::any();

    assert_eq!(
        document_put_decision(false, has_preconditions, existing_content_present),
        DocumentWriteDecision::PreconditionFailed
    );
}

#[kani::proof]
fn document_put_decision_conflicts_only_without_preconditions_on_existing_content() {
    let has_preconditions = kani::any();
    let existing_content_present = kani::any();
    let decision = document_put_decision(true, has_preconditions, existing_content_present);

    assert!(
        matches!(decision, DocumentWriteDecision::Conflict)
            == (existing_content_present && !has_preconditions)
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
