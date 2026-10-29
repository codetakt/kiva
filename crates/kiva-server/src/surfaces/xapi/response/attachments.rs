//! Response-only MIME headers for raw bytes shared by multiple Statements.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::surfaces::xapi::{model::AttachmentRecord, request::matching_media_types};

pub(super) fn reconcile_content_types(
    result: &Value,
    attachments: &mut BTreeMap<String, AttachmentRecord>,
) {
    for statement in result["statements"].as_array().into_iter().flatten() {
        let substatement = statement.get("object").filter(|value| {
            value.get("objectType").and_then(Value::as_str) == Some("SubStatement")
        });
        for owner in std::iter::once(statement).chain(substatement) {
            for metadata in owner["attachments"].as_array().into_iter().flatten() {
                let Some(hash) = metadata["sha2"].as_str() else {
                    continue;
                };
                let Some(attachment) = attachments.get_mut(&hash.to_ascii_lowercase()) else {
                    continue;
                };
                // Include fileUrl-only references: the emitted part still has to
                // agree with every metadata reference in this response. A header
                // cleared by one reference must stay absent for all later ones.
                if !attachment.content_type.is_empty()
                    && !metadata["contentType"]
                        .as_str()
                        .is_some_and(|content_type| {
                            matches!(
                                matching_media_types(&attachment.content_type, content_type),
                                Ok(true)
                            )
                        })
                {
                    // Content-Type is optional on raw parts. Keep the bytes and
                    // required hash/encoding headers without asserting a type
                    // whose agreement cannot be established for the whole result.
                    attachment.content_type.clear();
                }
            }
        }
    }
}
