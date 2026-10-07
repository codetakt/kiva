-- Persist xAPI Statement attachment bodies separately from Statement JSON.

CREATE TABLE IF NOT EXISTS kiva.xapi_statement_attachments (
    tenant_id text NOT NULL,
    statement_id uuid NOT NULL,
    sha2 text NOT NULL,
    content_type text NOT NULL,
    body bytea NOT NULL,
    PRIMARY KEY (tenant_id, statement_id, sha2),
    FOREIGN KEY (tenant_id, statement_id)
        REFERENCES kiva.xapi_statements (tenant_id, statement_id)
        ON DELETE CASCADE,
    CHECK (tenant_id <> ''),
    CHECK (sha2 ~ '^[0-9a-f]{64}$'),
    CHECK (content_type <> ''),
    CHECK (octet_length(body) > 0)
);
