DO $$ BEGIN
    IF (SELECT count(*) FROM expected_actor_keys) <> 6
       OR (SELECT count(*) FROM expected_document_keys) <> 6 THEN
        RAISE EXCEPTION 'incomplete migration controls';
    END IF;
    IF EXISTS (
        SELECT 1 FROM expected_actor_keys AS expected
        LEFT JOIN kiva.xapi_statements AS actual USING (tenant_id)
        WHERE actual.actor_ifi IS DISTINCT FROM convert_to(expected.actor_ifi, 'UTF8')
            OR actual.statement::text IS DISTINCT FROM expected.statement_text
    ) THEN
        RAISE EXCEPTION 'existing Statement/Account migration changed payloads or keys';
    END IF;
    IF EXISTS (
        SELECT 1 FROM expected_document_keys AS expected
        LEFT JOIN kiva.xapi_documents AS actual USING (tenant_id)
        WHERE actual.document_key IS DISTINCT FROM convert_to(expected.document_key, 'UTF8')
            OR actual.document_id IS DISTINCT FROM convert_to(expected.document_id, 'UTF8')
            OR actual.agent::text IS DISTINCT FROM expected.agent_text
            OR actual.content_json::text IS DISTINCT FROM expected.content_text
            OR actual.etag IS DISTINCT FROM expected.etag
            OR actual.updated_at IS DISTINCT FROM expected.updated_at
    ) THEN
        RAISE EXCEPTION 'existing Document migration changed payloads, metadata or keys';
    END IF;
END $$;

DELETE FROM kiva.xapi_statements WHERE tenant_id IN (SELECT tenant_id FROM expected_actor_keys);
DELETE FROM kiva.xapi_documents WHERE tenant_id IN (SELECT tenant_id FROM expected_document_keys);
