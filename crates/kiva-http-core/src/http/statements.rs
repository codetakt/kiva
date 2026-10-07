/// Pure PUT decision for immutable Statement resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementPutDecision {
    /// Store a new Statement.
    Store,
    /// Existing Statement is equivalent to the submitted Statement.
    Unchanged,
    /// Existing Statement differs from the submitted Statement.
    Conflict,
}

/// Decides the immutable Statement PUT outcome from the existing-record comparison.
#[must_use]
pub const fn statement_put_decision(
    existing_statement_matches: Option<bool>,
) -> StatementPutDecision {
    match existing_statement_matches {
        None => StatementPutDecision::Store,
        Some(true) => StatementPutDecision::Unchanged,
        Some(false) => StatementPutDecision::Conflict,
    }
}

/// Statement `GET` query parameter names modeled by the pure validation gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementGetQueryParameter {
    /// A parameter outside the xAPI Statement `GET` contract.
    Unknown,
    /// `statementId`.
    StatementId,
    /// `voidedStatementId`.
    VoidedStatementId,
    /// `agent`.
    Agent,
    /// `verb`.
    Verb,
    /// `activity`.
    Activity,
    /// `registration`.
    Registration,
}

impl StatementGetQueryParameter {
    const fn bit(self) -> u16 {
        match self {
            Self::Unknown => 1 << 0,
            Self::StatementId => 1 << 1,
            Self::VoidedStatementId => 1 << 2,
            Self::Agent => 1 << 3,
            Self::Verb => 1 << 4,
            Self::Activity => 1 << 5,
            Self::Registration => 1 << 6,
        }
    }
}

/// Compact set of non-scalar Statement `GET` query parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatementGetQueryParameterSet {
    bits: u16,
}

impl StatementGetQueryParameterSet {
    /// Empty parameter set.
    pub const EMPTY: Self = Self { bits: 0 };

    /// Returns a new set containing `parameter`.
    #[must_use]
    pub const fn with(self, parameter: StatementGetQueryParameter) -> Self {
        Self {
            bits: self.bits | parameter.bit(),
        }
    }

    /// Returns whether `parameter` is present.
    #[must_use]
    pub const fn contains(self, parameter: StatementGetQueryParameter) -> bool {
        self.bits & parameter.bit() != 0
    }
}

/// `format` parameter state for Statement `GET`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementFormatParameter {
    /// Parameter is absent.
    Missing,
    /// `exact`.
    Exact,
    /// `canonical`.
    Canonical,
    /// `ids`.
    Ids,
    /// Parameter is present but not one of the allowed values.
    Invalid,
}

impl StatementFormatParameter {
    const fn is_invalid(self) -> bool {
        matches!(self, Self::Invalid)
    }
}

/// Boolean query parameter state for Statement `GET`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementBooleanParameter {
    /// Parameter is absent.
    Missing,
    /// Parameter is present with `true`.
    True,
    /// Parameter is present with `false`.
    False,
    /// Parameter is present but not a supported boolean literal.
    Invalid,
}

impl StatementBooleanParameter {
    const fn is_present(self) -> bool {
        !matches!(self, Self::Missing)
    }

    const fn is_invalid(self) -> bool {
        matches!(self, Self::Invalid)
    }
}

/// Timestamp query parameter state for Statement `GET`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementTimestampParameter {
    /// Parameter is absent.
    Missing,
    /// Parameter is present and parses as an RFC3339 timestamp.
    Valid,
    /// Parameter is present but does not parse as an RFC3339 timestamp.
    Invalid,
}

impl StatementTimestampParameter {
    const fn is_present(self) -> bool {
        !matches!(self, Self::Missing)
    }

    const fn is_invalid(self) -> bool {
        matches!(self, Self::Invalid)
    }
}

/// `limit` query parameter state for Statement `GET`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementLimitParameter {
    /// Parameter is absent.
    Missing,
    /// Parameter is present and parses as a non-negative integer.
    Valid,
    /// Parameter is present but does not parse as a non-negative integer.
    Invalid,
}

impl StatementLimitParameter {
    const fn is_present(self) -> bool {
        !matches!(self, Self::Missing)
    }

    const fn is_invalid(self) -> bool {
        matches!(self, Self::Invalid)
    }
}

/// Modeled Statement `GET` query surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatementGetQueryParameters {
    /// Non-scalar parameter presence.
    pub parameters: StatementGetQueryParameterSet,
    /// `related_activities` value state.
    pub related_activities: StatementBooleanParameter,
    /// `related_agents` value state.
    pub related_agents: StatementBooleanParameter,
    /// `format` value state.
    pub format: StatementFormatParameter,
    /// `attachments` value state.
    pub attachments: StatementBooleanParameter,
    /// `ascending` value state.
    pub ascending: StatementBooleanParameter,
    /// `since` timestamp state.
    pub since: StatementTimestampParameter,
    /// `until` timestamp state.
    pub until: StatementTimestampParameter,
    /// `limit` value state.
    pub limit: StatementLimitParameter,
}

impl StatementGetQueryParameters {
    const fn has_single_statement_id(self) -> bool {
        self.parameters
            .contains(StatementGetQueryParameter::StatementId)
            || self
                .parameters
                .contains(StatementGetQueryParameter::VoidedStatementId)
    }

    const fn has_search_parameter(self) -> bool {
        self.parameters.contains(StatementGetQueryParameter::Agent)
            || self.parameters.contains(StatementGetQueryParameter::Verb)
            || self
                .parameters
                .contains(StatementGetQueryParameter::Activity)
            || self
                .parameters
                .contains(StatementGetQueryParameter::Registration)
            || self.related_activities.is_present()
            || self.related_agents.is_present()
            || self.ascending.is_present()
            || self.since.is_present()
            || self.until.is_present()
            || self.limit.is_present()
    }
}

/// Pure validation decision for Statement `GET` query parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementGetQueryDecision {
    /// Query can proceed to resource lookup or filtering.
    Allow,
    /// At least one query parameter is outside the Statement `GET` contract.
    UnknownParameter,
    /// `statementId` and `voidedStatementId` were both supplied.
    ConflictingStatementIds,
    /// Single-Statement retrieval was combined with collection filter options.
    SingleStatementParameterConflict,
    /// `format` was supplied with an unsupported value.
    InvalidFormat,
    /// A boolean parameter was supplied with an unsupported value.
    InvalidBoolean,
    /// A timestamp parameter was supplied with a non-RFC3339 value.
    InvalidTimestamp,
    /// `limit` was supplied with a value that is not a non-negative integer.
    InvalidLimit,
}

/// Decides whether modeled Statement `GET` query parameters are admissible.
#[must_use]
pub const fn statement_get_query_decision(
    query: StatementGetQueryParameters,
) -> StatementGetQueryDecision {
    if query
        .parameters
        .contains(StatementGetQueryParameter::Unknown)
    {
        return StatementGetQueryDecision::UnknownParameter;
    }
    if query
        .parameters
        .contains(StatementGetQueryParameter::StatementId)
        && query
            .parameters
            .contains(StatementGetQueryParameter::VoidedStatementId)
    {
        return StatementGetQueryDecision::ConflictingStatementIds;
    }
    if query.has_single_statement_id() && query.has_search_parameter() {
        return StatementGetQueryDecision::SingleStatementParameterConflict;
    }
    if query.format.is_invalid() {
        return StatementGetQueryDecision::InvalidFormat;
    }
    if query.attachments.is_invalid()
        || query.related_activities.is_invalid()
        || query.related_agents.is_invalid()
        || query.ascending.is_invalid()
    {
        return StatementGetQueryDecision::InvalidBoolean;
    }
    if query.since.is_invalid() || query.until.is_invalid() {
        return StatementGetQueryDecision::InvalidTimestamp;
    }
    if query.limit.is_invalid() {
        return StatementGetQueryDecision::InvalidLimit;
    }
    StatementGetQueryDecision::Allow
}
