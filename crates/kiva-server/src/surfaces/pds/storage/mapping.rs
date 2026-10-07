//! PDS `PostgreSQL` row mapping module owner.

#![allow(clippy::wildcard_imports)]

use super::*;

mod access;
mod consent;
mod data_products;
mod governance;
mod lifecycle;
mod provider_bindings;

pub(super) use access::*;
pub(super) use consent::*;
pub(super) use data_products::pds_data_product_from_row;
pub(super) use governance::*;
pub(super) use lifecycle::*;
pub(super) use provider_bindings::*;

pub(crate) use data_products::pds_data_class_labels;

pub(super) fn pds_fixed_32_bytes(bytes: Vec<u8>) -> Result<[u8; 32], PdsRepositoryError> {
    bytes
        .try_into()
        .map_err(|_bytes| PdsRepositoryError::InvalidStoredData)
}

pub(super) fn pds_days_to_i32(days: u32) -> Result<i32, PdsRepositoryError> {
    i32::try_from(days).map_err(|_error| {
        PdsRepositoryError::InvalidRequest(
            "PDS retention policy day count exceeds PostgreSQL integer range".to_owned(),
        )
    })
}

pub(super) fn pds_i32_days_to_u32(days: i32) -> Result<u32, PdsRepositoryError> {
    u32::try_from(days).map_err(|_error| PdsRepositoryError::InvalidStoredData)
}
