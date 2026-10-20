//! xAPI HTTP handler module owner.

mod about;
mod agents_activities;
mod common;
mod documents;
mod statements;

pub(crate) use about::about;
pub(crate) use agents_activities::{get_activity, get_agent_person};
pub(crate) use documents::{
    delete_activity_profile_document, delete_agent_profile_document, delete_state_document,
    get_activity_profile_document, get_agent_profile_document, get_state_document,
    post_activity_profile_document, post_agent_profile_document, post_state_document,
    put_activity_profile_document, put_agent_profile_document, put_state_document,
};
pub(crate) use statements::{get_more_statements, get_statements, post_statements, put_statement};
