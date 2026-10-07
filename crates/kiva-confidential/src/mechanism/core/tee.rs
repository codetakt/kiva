//! TEE attestation integrated implementation owner.

#![allow(clippy::wildcard_imports)]

use super::*;

#[path = "tee/nitro.rs"]
mod nitro;
#[path = "tee/portable.rs"]
mod portable;

pub use nitro::*;
pub use portable::*;
