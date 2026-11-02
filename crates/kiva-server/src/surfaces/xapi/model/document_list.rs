//! The selected document-ID representation shared by reads and scope mutations.

use std::collections::BTreeSet;

use serde_json::Value;
use sha2::{Digest, Sha256};

pub(crate) struct DocumentList {
    pub(crate) body: Vec<u8>,
    pub(crate) etag: String,
}

impl DocumentList {
    pub(crate) fn new<'a>(ids: impl Iterator<Item = &'a str>) -> Self {
        // State without registration spans registrations; duplicate IDs still
        // identify one list entry. Match both repositories' stable byte order.
        let ids = ids.collect::<BTreeSet<_>>();
        let value = Value::Array(
            ids.into_iter()
                .map(|id| Value::String(id.to_owned()))
                .collect(),
        );
        // An array of strings is infallibly JSON-serializable. Hash these exact
        // bytes, including escaping; the empty array is also a representation.
        let body = value.to_string().into_bytes();
        let etag = format!("\"{:x}\"", Sha256::digest(&body));
        Self { body, etag }
    }
}
