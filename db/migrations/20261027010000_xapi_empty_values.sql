-- Document IDs are Strings with no minimum length. Presence (including an
-- empty value) selects one document; absence selects listing/bulk deletion.
ALTER TABLE kiva.xapi_documents
    DROP CONSTRAINT xapi_documents_document_id_check;

-- Attachment length is a non-negative octet count. An empty MIME part is
-- present raw data and must be retained when its SHA-2/metadata match.
ALTER TABLE kiva.xapi_statement_attachments
    DROP CONSTRAINT xapi_statement_attachments_body_check;
