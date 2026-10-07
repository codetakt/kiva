#![allow(clippy::wildcard_imports)]

use super::super::*;

#[path = "errors/additive.rs"]
mod additive;
#[path = "errors/compat.rs"]
mod compat;
#[path = "errors/fhe.rs"]
mod fhe;
#[path = "errors/frost.rs"]
mod frost;
#[path = "errors/provider.rs"]
mod provider;
#[path = "errors/psi.rs"]
mod psi;
#[path = "errors/shamir.rs"]
mod shamir;
#[path = "errors/tee.rs"]
mod tee;

pub use additive::*;
pub use compat::*;
pub use fhe::*;
pub use frost::*;
pub use provider::*;
pub use psi::*;
pub use shamir::*;
pub use tee::*;

/// Confidential workflow validation errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkflowError {
    /// A multi-party workflow requires at least two participants.
    #[error("confidential workflow requires at least two participants")]
    TooFewParticipants,

    /// The release threshold exceeds the participant count.
    #[error("release threshold must not exceed participant count")]
    ThresholdExceedsParticipants,
}
