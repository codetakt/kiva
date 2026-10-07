-- Kiva xAPI LRS core schema for PostgreSQL.
--
-- The schema keeps xAPI JSON payloads as jsonb while promoting the fields that
-- Kiva must govern, index, and constrain as first-class relational columns.

CREATE SCHEMA IF NOT EXISTS kiva;

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS kiva.xapi_statements (
    tenant_id text NOT NULL,
    statement_id uuid NOT NULL,
    actor_ifi text,
    verb_id text,
    object_id text,
    authority jsonb,
    statement jsonb NOT NULL,
    stored_at timestamptz NOT NULL DEFAULT now(),
    version text NOT NULL DEFAULT '2.0.0',
    voided_by_statement_id uuid,
    PRIMARY KEY (tenant_id, statement_id),
    CHECK (tenant_id <> ''),
    CHECK (jsonb_typeof(statement) = 'object'),
    CHECK (authority IS NULL OR jsonb_typeof(authority) = 'object')
);

CREATE INDEX IF NOT EXISTS xapi_statements_actor_ifi_idx
    ON kiva.xapi_statements (tenant_id, actor_ifi)
    WHERE actor_ifi IS NOT NULL;

CREATE INDEX IF NOT EXISTS xapi_statements_verb_id_idx
    ON kiva.xapi_statements (tenant_id, verb_id)
    WHERE verb_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS xapi_statements_object_id_idx
    ON kiva.xapi_statements (tenant_id, object_id)
    WHERE object_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS xapi_statements_stored_at_idx
    ON kiva.xapi_statements (tenant_id, stored_at DESC);

CREATE INDEX IF NOT EXISTS xapi_statements_statement_gin_idx
    ON kiva.xapi_statements USING gin (statement jsonb_path_ops);

CREATE TABLE IF NOT EXISTS kiva.xapi_documents (
    tenant_id text NOT NULL,
    resource_kind text NOT NULL,
    document_key text NOT NULL,
    document_id text NOT NULL,
    activity_id text,
    agent jsonb,
    registration uuid,
    content_type text NOT NULL,
    content_json jsonb,
    content_bytes bytea,
    etag text NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, resource_kind, document_key),
    CHECK (tenant_id <> ''),
    CHECK (document_key <> ''),
    CHECK (resource_kind IN ('state', 'agent-profile', 'activity-profile')),
    CHECK (document_id <> ''),
    CHECK (agent IS NULL OR jsonb_typeof(agent) = 'object'),
    CHECK (
        (content_json IS NOT NULL AND content_bytes IS NULL)
        OR (content_json IS NULL AND content_bytes IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS xapi_documents_activity_idx
    ON kiva.xapi_documents (tenant_id, resource_kind, activity_id)
    WHERE activity_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS xapi_documents_agent_gin_idx
    ON kiva.xapi_documents USING gin (agent jsonb_path_ops)
    WHERE agent IS NOT NULL;
