use std::collections::BTreeMap;

use axum::extract::Query;

use super::*;

pub(in crate::tests) async fn pds_portability_exports(
    state: &super::AppState,
    token: &'static str,
    cursor: Option<&str>,
    page_size: usize,
) -> axum::response::Response {
    let mut query = BTreeMap::new();
    query.insert("pageSize".to_owned(), page_size.to_string());
    if let Some(cursor) = cursor {
        query.insert("cursor".to_owned(), cursor.to_owned());
    }
    super::get_pds_data_portability_exports(
        pds_application_state(state),
        confidential_headers_for(token),
        Query(query),
    )
    .await
}
