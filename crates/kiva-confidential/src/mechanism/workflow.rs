//! Confidential workflow and participant lifecycle facade.

pub use kiva_secret_provider_contract::{
    REFERENCE_PRIMITIVE_PROVIDER_PROFILE, SECRET_COMPUTE_PROVIDER_CONTRACT_PROFILE,
};

pub use super::core::{
    ConfidentialMechanismKind, ConfidentialMechanismPlan, ConfidentialParticipantStatus,
    ConfidentialParticipantTransitionError, ReleasePolicy, SECRET_COMPUTE_PROVIDER_BINDING_DOMAIN,
    Technique, WorkflowError, confidential_participant_accepts_jobs,
    confidential_participant_accepts_new_keys, transition_confidential_participant_status,
};
