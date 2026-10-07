//! Mechanism-oriented namespaces for confidential-computation boundaries.
//!
//! Narrow mechanism namespaces own the public facades. `core` owns the
//! remaining tightly coupled implementation bodies.

pub mod additive;
pub(crate) mod core;
pub mod differential_privacy;
pub mod fhe;
pub mod frost;
pub mod job;
pub mod provider;
pub mod psi;
pub mod registry;
pub mod shamir;
pub mod tee;
pub mod threshold;
pub mod workflow;
