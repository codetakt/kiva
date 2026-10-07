//! Personal Data Store domain boundary types for Kiva.
//!
//! This crate is intentionally small at first. It holds stable PDS boundary
//! concepts that should not depend on Axum, `SQLx`, or server runtime state.

mod pds;

pub use pds::*;
