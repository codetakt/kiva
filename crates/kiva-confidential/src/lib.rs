//! Confidential computation planning model for Kiva.

#![allow(clippy::redundant_pub_crate)]

pub mod mechanism;
pub mod primitive;

pub(crate) use mechanism::additive::SecureAggregateError;
