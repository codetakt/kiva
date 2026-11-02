-- Preserve the full JSON string domain, including escaped U+0000. PostgreSQL
-- jsonb cannot represent this code point. Kiva compares/filter Statements in
-- Rust; its relational metadata indexes remain available.
DROP INDEX kiva.xapi_statements_statement_gin_idx;
ALTER TABLE kiva.xapi_statements
    DROP CONSTRAINT xapi_statements_statement_check,
    DROP CONSTRAINT xapi_statements_authority_check,
    ALTER COLUMN statement TYPE json USING statement::json,
    ALTER COLUMN authority TYPE json USING authority::json,
    ADD CHECK (json_typeof(statement) = 'object'),
    ADD CHECK (authority IS NULL OR json_typeof(authority) = 'object');

-- FIPS 180-2 with Change Notice 1: full SHA-224/256/384/512 digests.
ALTER TABLE kiva.xapi_statement_attachments
    DROP CONSTRAINT xapi_statement_attachments_sha2_check,
    ADD CHECK (sha2 ~ '^([0-9a-fA-F]{56}|[0-9a-fA-F]{64}|[0-9a-fA-F]{96}|[0-9a-fA-F]{128})$');
