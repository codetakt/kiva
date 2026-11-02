-- RFC 2046/xAPI permit absent Content-Type on nonfirst MIME parts. Empty is
-- the internal absence marker, distinct from an explicitly invalid empty
-- header. Existing attachment type/body values remain unchanged.
ALTER TABLE kiva.xapi_statement_attachments
    DROP CONSTRAINT xapi_statement_attachments_content_type_check;
