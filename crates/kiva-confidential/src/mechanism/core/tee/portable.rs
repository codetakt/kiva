//! Deployment-bound portable TEE attestation boundary.

#![allow(clippy::wildcard_imports)]

use super::*;

#[path = "portable/claims.rs"]
mod claims;
#[path = "portable/evidence.rs"]
mod evidence;
#[path = "portable/gate.rs"]
mod gate;
#[path = "portable/policy.rs"]
mod policy;

pub use claims::*;
pub use evidence::*;
pub use gate::*;
pub use policy::*;
