//! Canonical domain newtypes shared by Kiva crates.
//!
//! This crate owns small, server-independent value boundaries for learner
//! experience records. It intentionally contains no HTTP DTOs, SQL mappings,
//! process configuration, authorization policy, or confidential-computation
//! runtime logic. Values admitted here should be strict enough to move between
//! xAPI, PDS, policy, and verification code without reinterpreting raw strings.

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

/// Domain-level validation errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    /// The supplied value is not an absolute IRI.
    #[error("expected an absolute IRI")]
    InvalidIri,

    /// The supplied IRI is syntactically valid only after trimming.
    #[error("IRI must not contain leading or trailing whitespace")]
    NonCanonicalIri,

    /// The supplied identifier is empty after trimming.
    #[error("identifier must not be empty")]
    EmptyIdentifier,

    /// The supplied identifier is syntactically valid only after trimming.
    #[error("identifier must not contain leading or trailing whitespace")]
    NonCanonicalIdentifier,
}

/// Absolute IRI used by xAPI identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Iri(String);

impl Iri {
    /// Parses and stores an absolute IRI.
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        if value.trim() != value {
            return Err(DomainError::NonCanonicalIri);
        }
        Url::parse(&value).map_err(|_error| DomainError::InvalidIri)?;
        Ok(Self(value))
    }

    /// Returns the validated string representation retained by Kiva.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// xAPI Statement identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StatementId(Uuid);

impl StatementId {
    /// Creates a new random Statement id.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wraps an existing UUID.
    #[must_use]
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Returns the underlying UUID.
    #[must_use]
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for StatementId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for StatementId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

/// xAPI Actor subset used by the Kiva core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    account_id: String,
}

impl Actor {
    /// Creates an actor from a stable account identifier.
    pub fn new(account_id: impl Into<String>) -> Result<Self, DomainError> {
        let account_id = account_id.into();
        let trimmed = account_id.trim();
        if trimmed.is_empty() {
            return Err(DomainError::EmptyIdentifier);
        }
        if trimmed != account_id.as_str() {
            return Err(DomainError::NonCanonicalIdentifier);
        }
        Ok(Self { account_id })
    }

    /// Returns the actor account identifier.
    #[must_use]
    pub const fn account_id(&self) -> &str {
        self.account_id.as_str()
    }
}

/// xAPI Activity subset used by the Kiva core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    id: Iri,
}

impl Activity {
    /// Creates an activity from an absolute IRI.
    #[must_use]
    pub const fn new(id: Iri) -> Self {
        Self { id }
    }

    /// Returns the activity IRI.
    #[must_use]
    pub const fn id(&self) -> &Iri {
        &self.id
    }
}

/// Immutable learning experience record accepted at the Kiva core boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Statement {
    id: StatementId,
    actor: Actor,
    verb: Iri,
    object: Activity,
    stored: OffsetDateTime,
}

impl Statement {
    /// Creates a Statement value.
    #[must_use]
    pub const fn new(
        id: StatementId,
        actor: Actor,
        verb: Iri,
        object: Activity,
        stored: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            actor,
            verb,
            object,
            stored,
        }
    }

    /// Returns the Statement id.
    #[must_use]
    pub const fn id(&self) -> StatementId {
        self.id
    }

    /// Returns the actor.
    #[must_use]
    pub const fn actor(&self) -> &Actor {
        &self.actor
    }

    /// Returns the verb IRI.
    #[must_use]
    pub const fn verb(&self) -> &Iri {
        &self.verb
    }

    /// Returns the activity object.
    #[must_use]
    pub const fn object(&self) -> &Activity {
        &self.object
    }

    /// Returns the stored timestamp.
    #[must_use]
    pub const fn stored(&self) -> OffsetDateTime {
        self.stored
    }
}

#[cfg(test)]
mod tests {
    use super::{Actor, DomainError, Iri};

    #[test]
    fn rejects_empty_actor_identifier() {
        assert_eq!(Actor::new("   "), Err(DomainError::EmptyIdentifier));
    }

    #[test]
    fn rejects_non_canonical_actor_identifier() {
        assert_eq!(
            Actor::new(" learner:1 "),
            Err(DomainError::NonCanonicalIdentifier)
        );
    }

    #[test]
    fn rejects_relative_iri() {
        assert_eq!(Iri::parse("/relative"), Err(DomainError::InvalidIri));
    }

    #[test]
    fn rejects_non_canonical_iri() {
        assert_eq!(
            Iri::parse(" https://example.edu/activity/1"),
            Err(DomainError::NonCanonicalIri)
        );
    }
}
