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

ALTER TABLE kiva.xapi_statements
    ALTER COLUMN actor_ifi TYPE bytea USING convert_to(actor_ifi, 'UTF8');

DROP INDEX kiva.xapi_documents_agent_gin_idx;
ALTER TABLE kiva.xapi_documents
    DROP CONSTRAINT xapi_documents_document_key_check,
    DROP CONSTRAINT xapi_documents_document_id_check,
    DROP CONSTRAINT xapi_documents_agent_check,
    -- Rebuild keys during the table rewrite, before the primary-key index is
    -- rebuilt. In-place UPDATE can collide with another Account's OLD key
    -- (b:c -> b%3Ac, while b%3Ac -> b%253Ac), despite distinct final keys.
    ALTER COLUMN document_key TYPE bytea USING convert_to(
        CASE WHEN agent->'account' IS NOT NULL THEN
            pg_temp.kiva_document_key_part('kind', resource_kind) ||
            CASE WHEN resource_kind = 'state'
                THEN pg_temp.kiva_document_key_part('activity_id', activity_id) ELSE '' END ||
            pg_temp.kiva_document_key_part('agent',
                'account:' || (agent->'account'->>'homePage') || ':' ||
                replace(replace(agent->'account'->>'name', '%', '%25'), ':', '%3A')) ||
            CASE WHEN resource_kind = 'state'
                THEN pg_temp.kiva_document_key_part('registration', coalesce(registration::text, 'none')) ELSE '' END ||
            pg_temp.kiva_document_key_part('document_id', document_id)
        ELSE document_key END, 'UTF8'),
    ALTER COLUMN document_id TYPE bytea USING convert_to(document_id, 'UTF8'),
    ALTER COLUMN agent TYPE json USING agent::json,
    ALTER COLUMN content_json TYPE json USING content_json::json,
    ADD CHECK (octet_length(document_key) > 0),
    ADD CHECK (octet_length(document_id) > 0),
    ADD CHECK (agent IS NULL OR json_typeof(agent) = 'object');
