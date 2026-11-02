-- Exercise an upgrade containing real legacy text rows, including an absent
-- optional header, literal escapes and non-ASCII text. No production data.
CREATE TEMP TABLE expected_attachment_media AS
SELECT ordinal, media_type, convert_to(media_type, 'UTF8') AS expected_bytes,
       decode('000d0aff', 'hex') AS body
FROM unnest(ARRAY[
    '',
    'application/octet-stream',
    'application/octet-stream; p="literal\u0000"',
    'application/octet-stream (日本語)',
    E'application/octet-stream; p="a\nb"'
]) WITH ORDINALITY AS controls(media_type, ordinal);

INSERT INTO kiva.xapi_statements (tenant_id, statement_id, actor_ifi, statement)
VALUES ('attachment-media-migration', '00000000-0000-4000-8000-000000000001',
        convert_to('mbox:mailto:legacy@example.org', 'UTF8'), '{}');
INSERT INTO kiva.xapi_statement_attachments (tenant_id, statement_id, sha2, content_type, body)
SELECT 'attachment-media-migration', '00000000-0000-4000-8000-000000000001',
       lpad(ordinal::text, 64, '0'), media_type, body
FROM expected_attachment_media;
