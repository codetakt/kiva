-- JSON string values and arbitrary document identifiers include U+0000.
-- Keep searchable identifiers as UTF-8 bytes and JSON as JSON text; neither
-- PostgreSQL text nor jsonb can represent all accepted xAPI string values.
-- First disambiguate Account components in keys written by the old format.
-- These old promoted text/jsonb fields could not contain U+0000.
-- Statement JSON can already contain it elsewhere after the preceding
-- migration. PostgreSQL's JSON extraction unescapes even unrelated fields.
-- Mask actual NUL escapes in an extraction copy only, preserving paired
-- backslashes (a literal "\u0000" in an Account name must stay distinct).
-- An old text actor_ifi could not have admitted NUL in the Account itself.
CREATE FUNCTION pg_temp.kiva_existing_account_ifi(document json)
RETURNS text LANGUAGE SQL IMMUTABLE AS $$
    SELECT 'account:' || (actor->'account'->>'homePage') || ':' ||
        replace(replace(actor->'account'->>'name', '%', '%25'), ':', '%3A')
    FROM (SELECT regexp_replace(document::text,
        $pattern$(?<!\\)((?:\\\\)*)\\u0000$pattern$,
        $replacement$\1\\ufffd$replacement$, 'g')::json->'actor' AS actor) AS extraction
$$;

UPDATE kiva.xapi_statements
SET actor_ifi = pg_temp.kiva_existing_account_ifi(statement)
WHERE actor_ifi LIKE 'account:%';

CREATE FUNCTION pg_temp.kiva_document_key_part(label text, value text)
RETURNS text LANGUAGE SQL IMMUTABLE AS $$
    SELECT label || ':' || octet_length(value)::text || ':' || value || '|'
$$;

WITH accounts AS (
    SELECT tenant_id, resource_kind, document_key,
        'account:' || (agent->'account'->>'homePage') || ':' ||
        replace(replace(agent->'account'->>'name', '%', '%25'), ':', '%3A') AS ifi
    FROM kiva.xapi_documents WHERE agent->'account' IS NOT NULL
)
UPDATE kiva.xapi_documents AS documents
SET document_key = pg_temp.kiva_document_key_part('kind', documents.resource_kind) ||
    CASE WHEN documents.resource_kind = 'state'
        THEN pg_temp.kiva_document_key_part('activity_id', documents.activity_id) ELSE '' END ||
    pg_temp.kiva_document_key_part('agent', accounts.ifi) ||
    CASE WHEN documents.resource_kind = 'state'
        THEN pg_temp.kiva_document_key_part('registration', coalesce(documents.registration::text, 'none')) ELSE '' END ||
    pg_temp.kiva_document_key_part('document_id', documents.document_id)
FROM accounts
WHERE documents.tenant_id = accounts.tenant_id
    AND documents.resource_kind = accounts.resource_kind
    AND documents.document_key = accounts.document_key;

ALTER TABLE kiva.xapi_statements
    ALTER COLUMN actor_ifi TYPE bytea USING convert_to(actor_ifi, 'UTF8');

DROP INDEX kiva.xapi_documents_agent_gin_idx;
ALTER TABLE kiva.xapi_documents
    DROP CONSTRAINT xapi_documents_document_key_check,
    DROP CONSTRAINT xapi_documents_document_id_check,
    DROP CONSTRAINT xapi_documents_agent_check,
    ALTER COLUMN document_key TYPE bytea USING convert_to(document_key, 'UTF8'),
    ALTER COLUMN document_id TYPE bytea USING convert_to(document_id, 'UTF8'),
    ALTER COLUMN agent TYPE json USING agent::json,
    ALTER COLUMN content_json TYPE json USING content_json::json,
    ADD CHECK (octet_length(document_key) > 0),
    ADD CHECK (octet_length(document_id) > 0),
    ADD CHECK (agent IS NULL OR json_typeof(agent) = 'object');
