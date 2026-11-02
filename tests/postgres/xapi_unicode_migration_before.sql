-- Existing-row regression controls for the Unicode/Account migration.
-- These run only in the disposable PostgreSQL verification cluster.
CREATE TEMP TABLE expected_actor_keys (tenant_id text, actor_ifi text, statement_text text);
CREATE TEMP TABLE expected_document_keys (tenant_id text, document_key text, document_id text, agent_text text, content_text text, etag text, updated_at timestamptz);
CREATE TEMP TABLE expected_account_rekey_rows (row_json jsonb);
-- Coexisting names form chains of occupied old keys. Exercise both insertion
-- orders, both resources, JSON/binary payloads, and an unrelated IFI per scope.
CREATE FUNCTION pg_temp.control_key_part(label text, value text)
RETURNS text LANGUAGE SQL IMMUTABLE AS $$
    SELECT label || ':' || octet_length(value)::text || ':' || value || '|'
$$;
DO $$
DECLARE
    kind text;
    reversed boolean;
    family integer;
    item record;
    tenant text;
    prefix text;
    suffix text;
    old_key text;
    new_key text;
BEGIN
    FOREACH kind IN ARRAY ARRAY['state', 'agent-profile'] LOOP
        FOREACH reversed IN ARRAY ARRAY[false, true] LOOP
            FOR family IN 0..1 LOOP
                tenant := 'account-rekey-' || kind || '-' || reversed || '-' || family;
                prefix := pg_temp.control_key_part('kind', kind) ||
                    CASE WHEN kind = 'state' THEN pg_temp.control_key_part('activity_id', 'https://example.org/activity') ELSE '' END;
                suffix := CASE WHEN kind = 'state' THEN pg_temp.control_key_part('registration', 'abcdef01-abcd-4abc-8abc-abcdef012345') ELSE '' END ||
                    pg_temp.control_key_part('document_id', '日本:doc%');
                FOR item IN
                    SELECT * FROM (VALUES
                        (0, 0, 'b:c', 'b%3Ac'), (0, 1, 'b%3Ac', 'b%253Ac'), (0, 2, 'b%253Ac', 'b%25253Ac'),
                        (1, 0, 'b%c', 'b%25c'), (1, 1, 'b%25c', 'b%2525c'), (1, 2, 'b%2525c', 'b%252525c'),
                        (0, 3, NULL, NULL), (1, 3, NULL, NULL)
                    ) AS names(family_id, ordinal, original, escaped)
                    WHERE family_id = family
                    ORDER BY CASE WHEN reversed THEN -ordinal ELSE ordinal END
                LOOP
                    old_key := prefix || pg_temp.control_key_part('agent', CASE WHEN item.original IS NULL THEN 'mbox:mailto:control@example.org' ELSE 'account:https://example.org/a:' || item.original END) || suffix;
                    new_key := prefix || pg_temp.control_key_part('agent', CASE WHEN item.escaped IS NULL THEN 'mbox:mailto:control@example.org' ELSE 'account:https://example.org/a:' || item.escaped END) || suffix;
                    INSERT INTO kiva.xapi_documents
                        (tenant_id, resource_kind, document_key, document_id, activity_id, agent, registration, content_type, content_json, content_bytes, etag, updated_at)
                    VALUES (tenant, kind, old_key, '日本:doc%',
                        CASE WHEN kind = 'state' THEN 'https://example.org/activity' ELSE NULL END,
                        CASE WHEN item.original IS NULL THEN '{"mbox":"mailto:control@example.org"}'::jsonb ELSE jsonb_build_object('account', jsonb_build_object('homePage', 'https://example.org/a', 'name', item.original)) END,
                        CASE WHEN kind = 'state' THEN 'abcdef01-abcd-4abc-8abc-abcdef012345'::uuid ELSE NULL END,
                        CASE WHEN item.ordinal % 2 = 0 THEN 'application/json' ELSE 'application/octet-stream' END,
                        CASE WHEN item.ordinal % 2 = 0 THEN jsonb_build_object('owner', item.original, 'value', item.ordinal) ELSE NULL END,
                        CASE WHEN item.ordinal % 2 = 1 THEN decode('0000ff01', 'hex') ELSE NULL END,
                        '"etag-' || item.ordinal || '"', '2026-01-02T03:04:05Z');
                    INSERT INTO expected_account_rekey_rows
                    SELECT to_jsonb(d) || jsonb_build_object('document_key', convert_to(new_key, 'UTF8'), 'document_id', convert_to(d.document_id, 'UTF8'))
                    FROM kiva.xapi_documents d
                    WHERE tenant_id = tenant AND resource_kind = kind AND document_key = old_key;
                END LOOP;
            END LOOP;
        END LOOP;
    END LOOP;
END $$;
INSERT INTO kiva.xapi_documents (tenant_id,resource_kind,document_key,document_id,activity_id,agent,registration,content_type,content_json,etag,updated_at) VALUES ('legacy-0','state','kind:5:state|activity_id:28:https://example.org/activity|agent:33:account:https://example.org/a:b:c|registration:4:none|document_id:11:日本:doc%|','日本:doc%','https://example.org/activity','{"account": {"homePage": "https://example.org/a", "name": "b:c"}}',NULL,'application/json','{"literal": "\\u0000", "unicode": "\u65e5\u672c\u8a9e"}','"old-etag"','2026-01-02T03:04:05Z');
INSERT INTO kiva.xapi_statements (tenant_id,statement_id,actor_ifi,statement) VALUES ('legacy-0','00000000-0000-4000-8000-000000000001','account:https://example.org/a:b:c','{"id": "00000000-0000-4000-8000-000000000001", "actor": {"account": {"homePage": "https://example.org/a", "name": "b:c"}, "name": "\u0000\u0000"}, "result": {"response": "\u0000"}}');
INSERT INTO kiva.xapi_documents (tenant_id,resource_kind,document_key,document_id,activity_id,agent,registration,content_type,content_json,etag,updated_at) VALUES ('legacy-1','state','kind:5:state|activity_id:28:https://example.org/activity|agent:33:account:https://example.org/a:b:c|registration:36:00000000-0000-4000-8000-000000000002|document_id:11:日本:doc%|','日本:doc%','https://example.org/activity','{"account": {"homePage": "https://example.org/a:b", "name": "c"}}','00000000-0000-4000-8000-000000000002','application/json','{"literal": "\\u0000", "unicode": "\u65e5\u672c\u8a9e"}','"old-etag"','2026-01-02T03:04:05Z');
INSERT INTO kiva.xapi_statements (tenant_id,statement_id,actor_ifi,statement) VALUES ('legacy-1','00000000-0000-4000-8000-000000000002','account:https://example.org/a:b:c','{"id": "00000000-0000-4000-8000-000000000002", "actor": {"account": {"homePage": "https://example.org/a:b", "name": "c"}, "name": "\u0000\u0000"}, "result": {"response": "\u0000"}}');
INSERT INTO kiva.xapi_documents (tenant_id,resource_kind,document_key,document_id,activity_id,agent,registration,content_type,content_json,etag,updated_at) VALUES ('legacy-2','agent-profile','kind:13:agent-profile|agent:35:account:https://example.org/a:b%3Ac|document_id:11:日本:doc%|','日本:doc%',NULL,'{"account": {"homePage": "https://example.org/a", "name": "b%3Ac"}}',NULL,'application/json','{"literal": "\\u0000", "unicode": "\u65e5\u672c\u8a9e"}','"old-etag"','2026-01-02T03:04:05Z');
INSERT INTO kiva.xapi_statements (tenant_id,statement_id,actor_ifi,statement) VALUES ('legacy-2','00000000-0000-4000-8000-000000000003','account:https://example.org/a:b%3Ac','{"id": "00000000-0000-4000-8000-000000000003", "actor": {"account": {"homePage": "https://example.org/a", "name": "b%3Ac"}, "name": "\u0000\u0000"}, "result": {"response": "\u0000"}}');
INSERT INTO kiva.xapi_documents (tenant_id,resource_kind,document_key,document_id,activity_id,agent,registration,content_type,content_json,etag,updated_at) VALUES ('legacy-3','agent-profile','kind:13:agent-profile|agent:41:account:https://example.org/a:日本%:語|document_id:11:日本:doc%|','日本:doc%',NULL,'{"account": {"homePage": "https://example.org/a", "name": "\u65e5\u672c%:\u8a9e"}}',NULL,'application/json','{"literal": "\\u0000", "unicode": "\u65e5\u672c\u8a9e"}','"old-etag"','2026-01-02T03:04:05Z');
INSERT INTO kiva.xapi_statements (tenant_id,statement_id,actor_ifi,statement) VALUES ('legacy-3','00000000-0000-4000-8000-000000000004','account:https://example.org/a:日本%:語','{"id": "00000000-0000-4000-8000-000000000004", "actor": {"account": {"homePage": "https://example.org/a", "name": "\u65e5\u672c%:\u8a9e"}, "name": "\u0000\u0000"}, "result": {"response": "\u0000"}}');
INSERT INTO kiva.xapi_documents (tenant_id,resource_kind,document_key,document_id,activity_id,agent,registration,content_type,content_json,etag,updated_at) VALUES ('legacy-4','state','kind:5:state|activity_id:28:https://example.org/activity|agent:43:account:https://example.org/a:literal\u0000|registration:4:none|document_id:11:日本:doc%|','日本:doc%','https://example.org/activity','{"account": {"homePage": "https://example.org/a", "name": "literal\\u0000"}}',NULL,'application/json','{"literal": "\\u0000", "unicode": "\u65e5\u672c\u8a9e"}','"old-etag"','2026-01-02T03:04:05Z');
INSERT INTO kiva.xapi_statements (tenant_id,statement_id,actor_ifi,statement) VALUES ('legacy-4','00000000-0000-4000-8000-000000000005','account:https://example.org/a:literal\u0000','{"id": "00000000-0000-4000-8000-000000000005", "actor": {"account": {"homePage": "https://example.org/a", "name": "literal\\u0000"}, "name": "\u0000\u0000"}, "result": {"response": "\u0000"}}');
INSERT INTO kiva.xapi_documents (tenant_id,resource_kind,document_key,document_id,activity_id,agent,registration,content_type,content_json,etag,updated_at) VALUES ('legacy-5','agent-profile','kind:13:agent-profile|agent:45:account:https://example.org/a:literal\\\u0000|document_id:11:日本:doc%|','日本:doc%',NULL,'{"account": {"homePage": "https://example.org/a", "name": "literal\\\\\\u0000"}}',NULL,'application/json','{"literal": "\\u0000", "unicode": "\u65e5\u672c\u8a9e"}','"old-etag"','2026-01-02T03:04:05Z');
INSERT INTO kiva.xapi_statements (tenant_id,statement_id,actor_ifi,statement) VALUES ('legacy-5','00000000-0000-4000-8000-000000000006','account:https://example.org/a:literal\\\u0000','{"id": "00000000-0000-4000-8000-000000000006", "actor": {"account": {"homePage": "https://example.org/a", "name": "literal\\\\\\u0000"}, "name": "\u0000\u0000"}, "result": {"response": "\u0000"}}');

INSERT INTO expected_actor_keys SELECT tenant_id, 'account:https://example.org/a:b%3Ac', statement::text FROM kiva.xapi_statements WHERE tenant_id = 'legacy-0';
INSERT INTO expected_actor_keys SELECT tenant_id, 'account:https://example.org/a:b:c', statement::text FROM kiva.xapi_statements WHERE tenant_id = 'legacy-1';
INSERT INTO expected_actor_keys SELECT tenant_id, 'account:https://example.org/a:b%253Ac', statement::text FROM kiva.xapi_statements WHERE tenant_id = 'legacy-2';
INSERT INTO expected_actor_keys SELECT tenant_id, 'account:https://example.org/a:日本%25%3A語', statement::text FROM kiva.xapi_statements WHERE tenant_id = 'legacy-3';
INSERT INTO expected_actor_keys SELECT tenant_id, 'account:https://example.org/a:literal\u0000', statement::text FROM kiva.xapi_statements WHERE tenant_id = 'legacy-4';
INSERT INTO expected_actor_keys SELECT tenant_id, 'account:https://example.org/a:literal\\\u0000', statement::text FROM kiva.xapi_statements WHERE tenant_id = 'legacy-5';
INSERT INTO expected_document_keys SELECT tenant_id, 'kind:5:state|activity_id:28:https://example.org/activity|agent:35:account:https://example.org/a:b%3Ac|registration:4:none|document_id:11:日本:doc%|', document_id, agent::text, content_json::text, etag, updated_at FROM kiva.xapi_documents WHERE tenant_id = 'legacy-0';
INSERT INTO expected_document_keys SELECT tenant_id, 'kind:5:state|activity_id:28:https://example.org/activity|agent:33:account:https://example.org/a:b:c|registration:36:00000000-0000-4000-8000-000000000002|document_id:11:日本:doc%|', document_id, agent::text, content_json::text, etag, updated_at FROM kiva.xapi_documents WHERE tenant_id = 'legacy-1';
INSERT INTO expected_document_keys SELECT tenant_id, 'kind:13:agent-profile|agent:37:account:https://example.org/a:b%253Ac|document_id:11:日本:doc%|', document_id, agent::text, content_json::text, etag, updated_at FROM kiva.xapi_documents WHERE tenant_id = 'legacy-2';
INSERT INTO expected_document_keys SELECT tenant_id, 'kind:13:agent-profile|agent:45:account:https://example.org/a:日本%25%3A語|document_id:11:日本:doc%|', document_id, agent::text, content_json::text, etag, updated_at FROM kiva.xapi_documents WHERE tenant_id = 'legacy-3';
INSERT INTO expected_document_keys SELECT tenant_id, 'kind:5:state|activity_id:28:https://example.org/activity|agent:43:account:https://example.org/a:literal\u0000|registration:4:none|document_id:11:日本:doc%|', document_id, agent::text, content_json::text, etag, updated_at FROM kiva.xapi_documents WHERE tenant_id = 'legacy-4';
INSERT INTO expected_document_keys SELECT tenant_id, 'kind:13:agent-profile|agent:45:account:https://example.org/a:literal\\\u0000|document_id:11:日本:doc%|', document_id, agent::text, content_json::text, etag, updated_at FROM kiva.xapi_documents WHERE tenant_id = 'legacy-5';
