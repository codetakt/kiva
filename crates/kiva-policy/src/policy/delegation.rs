#![allow(clippy::wildcard_imports)]

use super::*;

/// Access request evaluated by the policy core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessRequest {
    pub(super) role: PrincipalRole,
    pub(super) resource: ResourceClass,
    pub(super) action: Action,
}

impl AccessRequest {
    /// Creates a policy request.
    #[must_use]
    pub const fn new(role: PrincipalRole, resource: ResourceClass, action: Action) -> Self {
        Self {
            role,
            resource,
            action,
        }
    }

    /// Returns the principal role.
    #[must_use]
    pub const fn role(self) -> PrincipalRole {
        self.role
    }

    /// Returns the target resource class.
    #[must_use]
    pub const fn resource(self) -> ResourceClass {
        self.resource
    }

    /// Returns the requested action.
    #[must_use]
    pub const fn action(self) -> Action {
        self.action
    }
}

/// Lifecycle state of a subject consent/delegation grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegationState {
    /// The grant is active and can be evaluated for the delegated scope.
    Active,
    /// The grant was revoked and must fail closed.
    Revoked,
    /// The grant expired and must fail closed.
    Expired,
}

/// Resource scope covered by a subject consent/delegation grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegationScope {
    /// Raw learning-record read access.
    RawRecords,
    /// Personal data-store read access.
    PersonalData,
    /// Approved aggregate-insight read access.
    AggregateInsights,
}

/// Subject consent/delegation grant evaluated by the stricter PDS policy model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentDelegation {
    state: DelegationState,
    scope: DelegationScope,
}

impl ConsentDelegation {
    /// Creates a consent/delegation grant.
    #[must_use]
    pub const fn new(state: DelegationState, scope: DelegationScope) -> Self {
        Self { state, scope }
    }

    /// Creates an active consent/delegation grant for a scope.
    #[must_use]
    pub const fn active(scope: DelegationScope) -> Self {
        Self {
            state: DelegationState::Active,
            scope,
        }
    }

    /// Creates a revoked consent/delegation grant for a scope.
    #[must_use]
    pub const fn revoked(scope: DelegationScope) -> Self {
        Self {
            state: DelegationState::Revoked,
            scope,
        }
    }

    /// Returns the grant lifecycle state.
    #[must_use]
    pub const fn state(self) -> DelegationState {
        self.state
    }

    /// Returns the delegated resource scope.
    #[must_use]
    pub const fn scope(self) -> DelegationScope {
        self.scope
    }
}
