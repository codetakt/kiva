//! PDS subject identity projection helpers.

use super::responses::{PdsSubjectAccount, PdsSubjectAgent};

/// Account-home-page namespace used to represent a PDS subject as an xAPI Agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdsSubjectAccountNamespace {
    account_home_page: String,
}

impl PdsSubjectAccountNamespace {
    /// Creates a PDS subject account namespace.
    pub fn new(account_home_page: impl Into<String>) -> Self {
        Self {
            account_home_page: account_home_page.into(),
        }
    }

    /// Returns the account home page for this subject namespace.
    #[must_use]
    pub fn account_home_page(&self) -> &str {
        &self.account_home_page
    }

    /// Returns the canonical xAPI actor IFI for a PDS subject in this namespace.
    #[must_use]
    pub fn actor_ifi(&self, subject: &str) -> String {
        pds_subject_actor_ifi(&self.account_home_page, subject)
    }

    /// Returns the xAPI Agent DTO for a PDS subject in this namespace.
    #[must_use]
    pub fn agent(&self, subject: &str) -> PdsSubjectAgent {
        pds_subject_agent(&self.account_home_page, subject)
    }
}

/// Returns the canonical xAPI actor IFI for a PDS account subject.
#[must_use]
pub fn pds_subject_actor_ifi(account_home_page: &str, subject: &str) -> String {
    format!("account:{account_home_page}:{subject}")
}

/// Returns the xAPI Agent DTO for a PDS account subject.
#[must_use]
pub fn pds_subject_agent(account_home_page: &str, subject: &str) -> PdsSubjectAgent {
    PdsSubjectAgent {
        object_type: "Agent".to_owned(),
        account: PdsSubjectAccount {
            home_page: account_home_page.to_owned(),
            name: subject.to_owned(),
        },
    }
}
