//! Request parsing helpers for PDS HTTP handlers.

use super::prelude::{Bytes, HttpError};
use crate::core::extract::parse_json_request;

pub(super) fn parse_pds_openapi_body<T>(body: &Bytes, label: &'static str) -> Result<T, HttpError>
where
    T: serde::de::DeserializeOwned,
{
    parse_json_request::<T>(body, label)
}
