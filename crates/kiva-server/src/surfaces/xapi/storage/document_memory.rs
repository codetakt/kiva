#![allow(clippy::wildcard_imports)]

use super::document_mapping::{
    merged_json_document, preconditions_pass, put_document_decision, scope_preconditions_pass,
};
use super::*;

impl MemoryDocumentRepository {
    pub(crate) async fn get(&self, address: &DocumentAddress) -> Option<DocumentRecord> {
        self.documents
            .read()
            .await
            .get(&address.document_key)
            .cloned()
    }

    pub(crate) async fn list_ids(&self, scope: &DocumentScope) -> Vec<String> {
        self.documents
            .read()
            .await
            .iter()
            .filter(|(key, record)| {
                key.starts_with(&scope.document_key_prefix)
                    && scope.since.is_none_or(|since| record.updated_at > since)
            })
            .map(|(_key, record)| record.document_id.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub(crate) async fn put(
        &self,
        address: DocumentAddress,
        content: DocumentContent,
        preconditions: Preconditions,
    ) -> DocumentWriteOutcome {
        let mut documents = self.documents.write().await;
        let existing = documents.get(&address.document_key);
        let decision = put_document_decision(existing, &preconditions);
        if decision != DocumentWriteDecision::Write {
            return decision.into();
        }

        documents.insert(
            address.document_key,
            DocumentRecord::new(
                address.document_id,
                content,
                crate::core::response::postgres_microsecond_timestamp(self.clock.now()),
            ),
        );
        DocumentWriteOutcome::Stored
    }

    pub(crate) async fn post_json_merge(
        &self,
        address: DocumentAddress,
        content: DocumentContent,
        preconditions: Preconditions,
    ) -> DocumentWriteOutcome {
        let mut documents = self.documents.write().await;
        let existing = documents.get(&address.document_key);
        let Some(merged) = merged_json_document(existing, &content) else {
            return DocumentWriteOutcome::InvalidMerge;
        };
        if preconditions.undefined_combination {
            return DocumentWriteOutcome::UndefinedPreconditions;
        }
        if !preconditions_pass(existing, &preconditions) {
            return DocumentWriteOutcome::PreconditionFailed;
        }

        documents.insert(
            address.document_key,
            DocumentRecord::new(
                address.document_id,
                merged,
                crate::core::response::postgres_microsecond_timestamp(self.clock.now()),
            ),
        );
        DocumentWriteOutcome::Stored
    }

    pub(crate) async fn delete(
        &self,
        selection: &DocumentSelection,
        preconditions: &Preconditions,
    ) -> DocumentDeleteOutcome {
        let mut documents = self.documents.write().await;
        match selection {
            DocumentSelection::Single(address) => {
                let Some(existing) = documents.get(&address.document_key) else {
                    if !preconditions_pass(None, preconditions) {
                        return DocumentDeleteOutcome::PreconditionFailed;
                    }
                    return DocumentDeleteOutcome::NotFound;
                };
                if !preconditions_pass(Some(existing), preconditions) {
                    return DocumentDeleteOutcome::PreconditionFailed;
                }
                documents.remove(&address.document_key);
                DocumentDeleteOutcome::Deleted
            }
            DocumentSelection::Scope(scope) => {
                let keys = documents
                    .keys()
                    .filter(|key| key.starts_with(&scope.document_key_prefix))
                    .cloned()
                    .collect::<Vec<_>>();
                if !scope_preconditions_pass(
                    keys.iter()
                        .filter_map(|key| documents.get(key))
                        .map(|record| record.document_id.as_str()),
                    preconditions,
                ) {
                    return DocumentDeleteOutcome::PreconditionFailed;
                }
                if keys.is_empty() {
                    return DocumentDeleteOutcome::NotFound;
                }
                for key in keys {
                    documents.remove(&key);
                }
                DocumentDeleteOutcome::Deleted
            }
        }
    }
}
