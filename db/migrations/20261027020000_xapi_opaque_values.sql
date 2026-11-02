-- Keep the exact JSON member sequence of owner-defined extension values.
-- The typed Statement remains available for indexing/validation. Raw JSON is
-- stored as strings keyed by JSON pointer, never as lossy object members.
ALTER TABLE kiva.xapi_statements
    ADD COLUMN opaque_values json NOT NULL DEFAULT '{}'::json;
