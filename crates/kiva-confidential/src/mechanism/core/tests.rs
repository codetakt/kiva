#![allow(clippy::wildcard_imports)]

use super::*;

#[path = "tests/prelude.rs"]
mod prelude;
use prelude::*;

#[path = "tests/helpers.rs"]
mod helpers;
use helpers::*;

#[path = "tests/additive.rs"]
mod additive;
#[path = "tests/differential_privacy.rs"]
mod differential_privacy;
#[path = "tests/fhe.rs"]
mod fhe;
#[path = "tests/frost.rs"]
mod frost;
#[path = "tests/lifecycle.rs"]
mod lifecycle;
#[path = "tests/provider.rs"]
mod provider;
#[path = "tests/psi.rs"]
mod psi;
#[path = "tests/shamir.rs"]
mod shamir;
#[path = "tests/tee.rs"]
mod tee;
#[path = "tests/threshold_release.rs"]
mod threshold_release;
#[path = "tests/vectors.rs"]
mod vectors;
