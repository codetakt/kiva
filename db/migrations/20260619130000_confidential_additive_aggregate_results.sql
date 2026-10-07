-- Persist computed additive aggregate results before threshold-governed release.

CREATE TABLE IF NOT EXISTS kiva.confidential_additive_aggregate_results (
    tenant_id text NOT NULL,
    transcript_id bytea NOT NULL,
    aggregate_value bigint NOT NULL,
    participant_count bigint NOT NULL,
    computed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, transcript_id),
    UNIQUE (tenant_id, transcript_id, aggregate_value),
    CHECK (tenant_id <> ''),
    CHECK (octet_length(transcript_id) = 32),
    CHECK (aggregate_value >= 0 AND aggregate_value <= 4294967290),
    CHECK (participant_count >= 2)
);

ALTER TABLE kiva.confidential_threshold_release_requests
    ADD CONSTRAINT confidential_threshold_release_requests_aggregate_result_fk
    FOREIGN KEY (tenant_id, transcript_id, aggregate_value)
    REFERENCES kiva.confidential_additive_aggregate_results
        (tenant_id, transcript_id, aggregate_value)
    ON DELETE RESTRICT;
