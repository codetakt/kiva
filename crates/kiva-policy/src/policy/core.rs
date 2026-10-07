#![allow(clippy::wildcard_imports)]

use super::*;

/// Actor role evaluated by the policy layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrincipalRole {
    /// The learner or person associated with the data.
    DataSubject,
    /// Institutional custodian, such as a school.
    DataCustodian,
    /// Public-interest or statutory policy authority.
    PolicyAuthority,
    /// Approved analyst or BI service.
    Analyst,
    /// Auditor reviewing evidence without direct raw-data access.
    Auditor,
    /// Computation node participating in a confidential workflow.
    ComputationParticipant,
}

/// Resource class evaluated by policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceClass {
    /// Raw xAPI Statement or document data.
    RawLearningRecord,
    /// Subject-managed personal-data view.
    PersonalDataStore,
    /// Approved aggregate or derived analytical view.
    AggregateInsight,
    /// Audit evidence or policy trace.
    AuditEvidence,
    /// Confidential computation input, share, transcript, or result.
    ConfidentialComputation,
}

/// Action requested by a principal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Store a record or document.
    Write,
    /// Read a record or approved view.
    Read,
    /// Delegate access to another party.
    Delegate,
    /// Revoke access or consent.
    Revoke,
    /// Run an approved computation.
    Compute,
    /// Release a computation result.
    Release,
    /// Audit an action or evidence trail.
    Audit,
}
