//! xAPI HTTP routes.

use axum::{Router, routing::get};

use crate::core::state::XapiHttpContext;

use super::handlers::{
    about, delete_activity_profile_document, delete_agent_profile_document, delete_state_document,
    get_activity, get_activity_profile_document, get_agent_person, get_agent_profile_document,
    get_more_statements, get_state_document, get_statements, post_activity_profile_document,
    post_agent_profile_document, post_state_document, post_statements,
    put_activity_profile_document, put_agent_profile_document, put_state_document, put_statement,
};

pub(crate) fn router(state: XapiHttpContext) -> Router {
    Router::new()
        .route("/about", get(about))
        .route("/agents", get(get_agent_person))
        .route("/activities", get(get_activity))
        .route("/statements/more", get(get_more_statements))
        .route(
            "/statements",
            get(get_statements).put(put_statement).post(post_statements),
        )
        .route(
            "/activities/state",
            get(get_state_document)
                .put(put_state_document)
                .post(post_state_document)
                .delete(delete_state_document),
        )
        .route(
            "/agents/profile",
            get(get_agent_profile_document)
                .put(put_agent_profile_document)
                .post(post_agent_profile_document)
                .delete(delete_agent_profile_document),
        )
        .route(
            "/activities/profile",
            get(get_activity_profile_document)
                .put(put_activity_profile_document)
                .post(post_activity_profile_document)
                .delete(delete_activity_profile_document),
        )
        .with_state(state)
}
