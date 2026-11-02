//! Deployment-owned Statement read permissions, keyed by verified subjects.

use std::{
    collections::{BTreeMap, BTreeSet},
    io,
};

use kiva_identity::AuthenticatedPrincipal;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Selection {
    All(All),
    Ids(BTreeSet<Uuid>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
enum All {
    All,
}

impl Default for Selection {
    fn default() -> Self {
        Self::Ids(BTreeSet::new())
    }
}

impl Selection {
    fn permits(&self, id: Uuid) -> bool {
        match self {
            Self::All(All::All) => true,
            Self::Ids(ids) => ids.contains(&id),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StatementVisibility {
    #[serde(default)]
    default: Selection,
    #[serde(default)]
    subjects: BTreeMap<String, Selection>,
}

impl StatementVisibility {
    pub(crate) const fn unrestricted() -> Self {
        Self {
            default: Selection::All(All::All),
            subjects: BTreeMap::new(),
        }
    }

    pub(crate) fn from_json(value: &str) -> Result<Self, io::Error> {
        serde_json::from_str(value).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("KIVA_XAPI_STATEMENT_VISIBILITY_JSON: {error}"),
            )
        })
    }

    pub(crate) fn permits(&self, principal: Option<&AuthenticatedPrincipal>, id: Uuid) -> bool {
        principal
            .and_then(|principal| self.subjects.get(&principal.subject))
            .unwrap_or(&self.default)
            .permits(id)
    }
}
