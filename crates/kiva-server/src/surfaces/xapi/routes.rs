//! xAPI HTTP routes.

use axum::{
    Router,
    extract::{Request, State},
    middleware::{self, Next},
    response::Response,
    routing::get,
};

use crate::core::state::XapiHttpContext;

use super::handlers::{
    about, delete_activity_profile_document, delete_agent_profile_document, delete_state_document,
    get_activity, get_activity_profile_document, get_agent_person, get_agent_profile_document,
    get_more_statements, get_state_document, get_statements, post_activity_profile_document,
    post_agent_profile_document, post_state_document, post_statements,
    put_activity_profile_document, put_agent_profile_document, put_state_document, put_statement,
};

pub(crate) fn router(state: XapiHttpContext) -> Router {
    let statements = Router::new()
        .route("/statements/more", get(get_more_statements))
        .route(
            "/statements",
            get(get_statements).put(put_statement).post(post_statements),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            statement_watermark,
        ));
    Router::new()
        .merge(statements)
        .route("/about", get(about))
        .route("/agents", get(get_agent_person))
        .route("/activities", get(get_activity))
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
        .layer(middleware::from_fn(response_version))
        .with_state(state)
}

async fn response_version(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    // Extractor failures (including the transport body ceiling) and method
    // rejections are xAPI responses too. Preserve any explicitly routed version.
    response
        .headers_mut()
        .entry("x-experience-api-version")
        .or_insert(axum::http::HeaderValue::from_static(
            crate::core::config::XAPI_VERSION,
        ));
    response
}

async fn statement_watermark(
    State(state): State<XapiHttpContext>,
    request: Request,
    next: Next,
) -> Response {
    use crate::core::{
        recording_clock::EMPTY_WATERMARK,
        response::{insert_header, rfc3339},
    };
    // A successful result set was read after this committed watermark. Taking
    // the watermark AFTER the query could promise concurrent rows absent from
    // that result. On storage failure the empty lower bound makes no claim of
    // availability, including error/extractor responses and HEAD.
    let through = state.statement_watermark().await.unwrap_or_else(|error| {
        tracing::error!(error = ?error, "Statement watermark lookup failed");
        EMPTY_WATERMARK
    });
    let mut response = next.run(request).await;
    insert_header(
        response.headers_mut(),
        ::http::header::HeaderName::from_static("x-experience-api-consistent-through"),
        &rfc3339(through),
    );
    response
}
