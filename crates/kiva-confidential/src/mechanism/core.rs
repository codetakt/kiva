//! Core confidential-computation mechanism implementation.

#![allow(dead_code, unreachable_pub, unused_imports)]
#![allow(clippy::wildcard_imports)]

use std::num::NonZeroUsize;

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::primitive::{artifact_is_empty, *};

#[cfg(any(test, kani))]
use super::{differential_privacy::*, job::*, provider::*};
#[cfg(any(test, kani))]
use kiva_secret_provider_contract::{
    SecretComputeProviderCapability, SecretComputeProviderKind, SecretComputeProviderProfile,
};

fn update_len_prefixed(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u128).to_be_bytes());
    hasher.update(bytes);
}

#[path = "core/additive.rs"]
mod additive;
#[path = "core/fhe.rs"]
mod fhe;
#[path = "core/frost.rs"]
mod frost;
#[path = "core/kernel.rs"]
mod kernel;
#[path = "core/lifecycle.rs"]
mod lifecycle;
#[path = "core/profiles.rs"]
mod profiles;
#[path = "core/psi.rs"]
mod psi;
#[path = "core/shamir.rs"]
mod shamir;
#[path = "core/tee.rs"]
mod tee;

pub use additive::*;
pub use fhe::*;
pub use frost::*;
pub(crate) use kernel::*;
pub use lifecycle::*;
pub use profiles::*;
pub use psi::*;
pub use shamir::*;
pub use tee::*;

#[cfg(test)]
#[path = "core/tests.rs"]
mod tests;

#[cfg(kani)]
#[path = "verification/kani_harnesses.rs"]
mod kani_harnesses;
