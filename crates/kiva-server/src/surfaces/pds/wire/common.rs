//! Shared PDS wire helpers.

use kiva_pds::{PdsSubjectAgent, pds_subject_agent};
use serde::Serialize;
use serde_json::Value;
#[cfg(test)]
use serde_json::json;

use crate::{
    core::config::PDS_SUBJECT_ACCOUNT_HOME_PAGE, surfaces::pds::model::pds_subject_actor_ifi,
};

pub(super) fn dto_value<T: Serialize>(value: &T) -> Result<Value, serde_json::Error> {
    serde_json::to_value(value)
}

pub(super) fn subject_agent_dto(subject: &str) -> PdsSubjectAgent {
    pds_subject_agent(PDS_SUBJECT_ACCOUNT_HOME_PAGE, subject)
}

#[cfg(test)]
pub(crate) fn subject_agent(subject: &str) -> Value {
    json!({
        "objectType": "Agent",
        "account": {
            "homePage": PDS_SUBJECT_ACCOUNT_HOME_PAGE,
            "name": subject,
        },
    })
}

pub(crate) fn subject_actor_ifi(subject: &str) -> String {
    pds_subject_actor_ifi(subject)
}
