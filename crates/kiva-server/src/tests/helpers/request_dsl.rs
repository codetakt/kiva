#![allow(clippy::wildcard_imports)]

use super::*;

pub(in crate::tests) fn empty_query() -> Query<BTreeMap<String, String>> {
    Query(BTreeMap::new())
}

pub(in crate::tests) fn expect_status(
    response: axum::response::Response,
    expected: StatusCode,
    label: &str,
) -> Result<axum::response::Response, String> {
    let actual = response.status();
    if actual == expected {
        Ok(response)
    } else {
        Err(format!("{label} should return {expected}, got {actual}"))
    }
}

pub(in crate::tests) struct JsonHandlerRequest<T> {
    subject: &'static str,
    query: BTreeMap<String, String>,
    body: T,
}

impl<T> JsonHandlerRequest<T>
where
    T: serde::Serialize,
{
    pub(in crate::tests) fn new(subject: &'static str, body: T) -> Self {
        Self {
            subject,
            query: BTreeMap::new(),
            body,
        }
    }

    pub(in crate::tests) fn with_query(mut self, key: &str, value: impl Into<String>) -> Self {
        self.query.insert(key.to_owned(), value.into());
        self
    }

    pub(in crate::tests) fn headers(&self) -> HeaderMap {
        confidential_headers_for(self.subject)
    }

    pub(in crate::tests) fn query(&self) -> Query<BTreeMap<String, String>> {
        Query(self.query.clone())
    }

    pub(in crate::tests) fn body(&self, label: &str) -> Result<Bytes, String> {
        json_body(&self.body, label)
    }
}
