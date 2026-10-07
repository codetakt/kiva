#![allow(clippy::wildcard_imports)]

use super::*;

/// Result of admitting an xAPI Statement batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionReport {
    statement_ids: Vec<StatementId>,
}

impl AdmissionReport {
    /// Creates an admission report from ordered Statement ids.
    #[must_use]
    pub const fn new(statement_ids: Vec<StatementId>) -> Self {
        Self { statement_ids }
    }

    /// Returns the admitted Statement ids in request order.
    #[must_use]
    pub const fn statement_ids(&self) -> &[StatementId] {
        self.statement_ids.as_slice()
    }
}

/// Promoted metadata extracted from a Statement JSON object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementMetadata {
    /// Canonical actor IFI, when present.
    pub actor_ifi: Option<String>,
    /// Verb IRI, when present.
    pub verb_id: Option<String>,
    /// Object activity IRI, when present.
    pub object_id: Option<String>,
    /// Statement authority object, when present.
    pub authority: Option<Value>,
    /// xAPI version promoted from the Statement or supplied default.
    pub version: String,
}

impl StatementMetadata {
    /// Extracts promoted metadata from a Statement JSON object.
    #[must_use]
    pub fn from_statement(statement: &Value, default_version: &str) -> Self {
        Self {
            actor_ifi: statement.get("actor").and_then(canonical_actor_ifi),
            verb_id: statement
                .pointer("/verb/id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            object_id: statement
                .pointer("/object/id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            authority: statement.get("authority").cloned(),
            version: statement
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or(default_version)
                .to_owned(),
        }
    }
}

/// Admits a Statement batch according to Kiva's structural boundary.
pub fn admit_statement_batch(
    statements: impl IntoIterator<Item = Statement>,
) -> Result<AdmissionReport, XapiError> {
    let (statement_ids, _seen) = statements.into_iter().try_fold(
        (Vec::new(), HashSet::new()),
        |(mut statement_ids, mut seen), statement| {
            let id = statement.id();
            if !seen.insert(id) {
                return Err(XapiError::DuplicateStatementId(id));
            }
            statement_ids.push(id);
            Ok((statement_ids, seen))
        },
    )?;

    if statement_ids.is_empty() {
        return Err(XapiError::EmptyBatch);
    }

    Ok(AdmissionReport::new(statement_ids))
}

/// Performs a shallow JSON admission check for an xAPI object payload.
///
/// Full xAPI conformance is intentionally not encoded in this helper. This
/// function exists so request handling can fail closed before typed decoding is
/// implemented for each resource family.
pub fn admit_json_object(value: &Value) -> Result<(), XapiError> {
    value
        .as_object()
        .map(|_object| ())
        .ok_or(XapiError::InvalidJsonPayload)
}
