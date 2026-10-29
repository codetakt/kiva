DO $$ BEGIN
    IF (SELECT count(*) FROM expected_attachment_media) <> 5 THEN
        RAISE EXCEPTION 'incomplete attachment migration controls';
    END IF;
    IF EXISTS (
        SELECT 1 FROM expected_attachment_media AS expected
        LEFT JOIN kiva.xapi_statement_attachments AS actual
            ON actual.tenant_id = 'attachment-media-migration'
            AND actual.statement_id = '00000000-0000-4000-8000-000000000001'
            AND actual.sha2 = lpad(expected.ordinal::text, 64, '0')
        WHERE actual.content_type IS DISTINCT FROM expected.expected_bytes
            OR actual.body IS DISTINCT FROM expected.body
    ) THEN
        RAISE EXCEPTION 'attachment migration lost a header or payload byte';
    END IF;
END $$;
DELETE FROM kiva.xapi_statements WHERE tenant_id = 'attachment-media-migration';
