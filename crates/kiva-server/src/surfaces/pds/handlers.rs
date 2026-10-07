//! PDS HTTP handler bodies grouped by resource family.

mod access;
mod application;
mod consent;
mod data_products;
mod governance;
mod lifecycle;
mod portability;
mod prelude;
mod provider_bindings;
mod request;

pub(crate) use access::*;
pub(crate) use consent::*;
pub(crate) use data_products::*;
pub(crate) use governance::*;
pub(crate) use lifecycle::*;
pub(crate) use portability::*;
pub(crate) use provider_bindings::*;
