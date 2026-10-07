//! Shared repository outcomes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StoreOutcome {
    Stored,
    Unchanged,
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InsertOutcome {
    Inserted,
    Existing,
}

impl InsertOutcome {
    pub(crate) const fn from_rows_affected(rows_affected: u64) -> Self {
        if rows_affected == 0 {
            Self::Existing
        } else {
            Self::Inserted
        }
    }
}
