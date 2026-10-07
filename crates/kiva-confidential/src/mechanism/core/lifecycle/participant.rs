#![allow(clippy::wildcard_imports)]

use super::super::*;

/// Lifecycle state for one production confidential-computation participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfidentialParticipantStatus {
    /// Participant identity and policy metadata exist, but the participant is
    /// not admitted for jobs or key registration yet.
    Invited,
    /// Participant is admitted for jobs and mechanism-specific key ceremonies.
    Active,
    /// Participant is temporarily unavailable and cannot join new work.
    Suspended,
    /// Emergency freeze state for suspected compromise.
    Frozen,
    /// Participant is retired from future confidential jobs.
    Removed,
}

impl ConfidentialParticipantStatus {
    /// Returns the stable API label for this status.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Invited => "invited",
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Frozen => "frozen",
            Self::Removed => "removed",
        }
    }

    /// Parses a stable API participant status label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "invited" => Some(Self::Invited),
            "active" => Some(Self::Active),
            "suspended" => Some(Self::Suspended),
            "frozen" => Some(Self::Frozen),
            "removed" => Some(Self::Removed),
            _ => None,
        }
    }
}

/// Participant lifecycle transition validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ConfidentialParticipantTransitionError {
    /// Participant lifecycle transition is outside the strict transition table.
    #[error("confidential participant lifecycle transition is invalid")]
    InvalidTransition,
}

/// Returns whether a participant may be admitted to new confidential jobs.
#[must_use]
pub const fn confidential_participant_accepts_jobs(status: ConfidentialParticipantStatus) -> bool {
    matches!(status, ConfidentialParticipantStatus::Active)
}

/// Returns whether a participant may register new mechanism keys.
#[must_use]
pub const fn confidential_participant_accepts_new_keys(
    status: ConfidentialParticipantStatus,
) -> bool {
    matches!(status, ConfidentialParticipantStatus::Active)
}

/// Returns whether a participant lifecycle transition is allowed.
#[allow(clippy::match_same_arms)]
#[must_use]
pub const fn confidential_participant_transition_is_allowed(
    current: ConfidentialParticipantStatus,
    requested: ConfidentialParticipantStatus,
) -> bool {
    use ConfidentialParticipantStatus as Status;
    match (current, requested) {
        (Status::Invited, Status::Active | Status::Suspended | Status::Removed) => true,
        (Status::Active, Status::Suspended | Status::Frozen | Status::Removed) => true,
        (Status::Suspended, Status::Active | Status::Frozen | Status::Removed) => true,
        (Status::Frozen, Status::Active | Status::Removed) => true,
        (Status::Removed, _) => false,
        _ => false,
    }
}

/// Applies a strict participant lifecycle transition.
pub fn transition_confidential_participant_status(
    current: ConfidentialParticipantStatus,
    requested: ConfidentialParticipantStatus,
) -> Result<ConfidentialParticipantStatus, ConfidentialParticipantTransitionError> {
    if current == requested {
        Ok(current)
    } else if confidential_participant_transition_is_allowed(current, requested) {
        Ok(requested)
    } else {
        Err(ConfidentialParticipantTransitionError::InvalidTransition)
    }
}
