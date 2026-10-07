#![allow(clippy::wildcard_imports)]

use super::super::*;

/// Plan for the first concrete confidential aggregate mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "AdditiveSecureAggregatePlanUnchecked",
    into = "AdditiveSecureAggregatePlanUnchecked"
)]
pub struct AdditiveSecureAggregatePlan {
    release_policy: ReleasePolicy,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct AdditiveSecureAggregatePlanUnchecked {
    release_policy: ReleasePolicy,
}

impl AdditiveSecureAggregatePlan {
    /// Creates an additive secure aggregate plan.
    ///
    /// The initial mechanism is deliberately n-of-n. Threshold release needs a
    /// different share construction and should not be represented by this plan.
    pub const fn new(release_policy: ReleasePolicy) -> Result<Self, SecureAggregateError> {
        if release_policy.release_threshold().get() != release_policy.participant_count().get() {
            return Err(SecureAggregateError::ThresholdMustRequireAllParticipants);
        }
        Ok(Self { release_policy })
    }

    /// Returns the release policy.
    #[must_use]
    pub const fn release_policy(self) -> ReleasePolicy {
        self.release_policy
    }
}

impl TryFrom<AdditiveSecureAggregatePlanUnchecked> for AdditiveSecureAggregatePlan {
    type Error = SecureAggregateError;

    fn try_from(value: AdditiveSecureAggregatePlanUnchecked) -> Result<Self, Self::Error> {
        Self::new(value.release_policy)
    }
}

impl From<AdditiveSecureAggregatePlan> for AdditiveSecureAggregatePlanUnchecked {
    fn from(value: AdditiveSecureAggregatePlan) -> Self {
        Self {
            release_policy: value.release_policy(),
        }
    }
}
