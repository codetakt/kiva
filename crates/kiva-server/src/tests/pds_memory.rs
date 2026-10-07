#![allow(clippy::wildcard_imports)]

use super::*;

#[path = "pds_memory/consent.rs"]
mod consent;
#[path = "pds_memory/data_products.rs"]
mod data_products;
#[path = "pds_memory/governance.rs"]
mod governance;
#[path = "pds_memory/inventory.rs"]
mod inventory;
#[path = "pds_memory/learner_principal.rs"]
mod learner_principal;
#[path = "pds_memory/ledger.rs"]
mod ledger;
#[path = "pds_memory/lifecycle_cases.rs"]
mod lifecycle_cases;
#[path = "pds_memory/lifecycle_events.rs"]
mod lifecycle_events;
#[path = "pds_memory/portability.rs"]
pub(super) mod portability;
#[path = "pds_memory/provider_bindings.rs"]
mod provider_bindings;
