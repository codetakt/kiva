//! Confidential workflow family labels.

/// Stable confidential workflow family used by admission evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidentialWorkflowFamily {
    /// Additive secure-aggregation workflows.
    Additive,
    /// Differential-privacy budget and release workflows.
    DifferentialPrivacy,
    /// Confidential job lifecycle workflows.
    Job,
    /// Confidential participant lifecycle workflows.
    Participant,
    /// Shamir threshold reconstruction workflows.
    Shamir,
    /// Threshold release workflows.
    Threshold,
}
