-- MIME comments, qtext and quoted-pair may contain ASCII NUL. Keep the exact
-- accepted field, including the empty marker for an absent optional header.
-- convert_to preserves every existing text value as its original UTF-8 bytes.
ALTER TABLE kiva.xapi_statement_attachments
    ALTER COLUMN content_type TYPE bytea USING convert_to(content_type, 'UTF8');
