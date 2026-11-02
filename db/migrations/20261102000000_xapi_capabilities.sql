-- Additional capability state is separate from immutable Statement JSON.
-- BYTEA preserves arbitrary Unicode/NUL and opaque JSON without JSONB coercion.
CREATE TABLE IF NOT EXISTS kiva.xapi_capability_records (
    tenant_id TEXT NOT NULL,
    record_key BYTEA NOT NULL,
    body BYTEA NOT NULL,
    etag TEXT NOT NULL,
    PRIMARY KEY (tenant_id, record_key)
);

ALTER TABLE kiva.xapi_statements
    ADD COLUMN IF NOT EXISTS canonical_update_permitted BOOLEAN NOT NULL DEFAULT FALSE;
