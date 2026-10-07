//! Confidential-computation application boundary.
//!
//! This module keeps confidential-computation workflows behind a pure
//! application capability handle. Server adapters may still expose legacy
//! service methods while individual use cases migrate into this boundary.

mod additive;
mod boundary;
mod differential_privacy;
mod family;
mod jobs;
mod operation;
mod participants;
mod shamir;
mod threshold;
mod workflow_kind;

pub use crate::runtime::{ApplicationFuture, ApplicationRuntimePort};
pub use additive::ConfidentialAdditiveRepository;
pub use boundary::{
    ConfidentialApplicationCapabilities, ConfidentialApplicationError,
    ConfidentialApplicationWorkflow, ConfidentialWorkflowAdmissionRecorder,
    ConfidentialWorkflowRepository,
};
pub use differential_privacy::ConfidentialDifferentialPrivacyRepository;
pub use family::ConfidentialWorkflowFamily;
pub use jobs::{
    ConfidentialJobAuditEventFilter, ConfidentialJobFilter, ConfidentialJobRegistrationCommand,
    ConfidentialJobRepository, ConfidentialJobTransitionCommand,
};
pub use kiva_confidential_core::{ConfidentialJobStatus, SecureAggregateTranscriptId};
pub use operation::{
    ConfidentialCommand, ConfidentialDomainEvent, ConfidentialOutcome,
    ConfidentialWorkflowAdmission,
};
pub use participants::ConfidentialParticipantRepository;
pub use shamir::ConfidentialShamirRepository;
pub use threshold::ConfidentialThresholdRepository;
pub use workflow_kind::ConfidentialWorkflowKind;
