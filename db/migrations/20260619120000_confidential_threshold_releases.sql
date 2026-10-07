-- Persist two-authority governance release requests for confidential aggregates.

CREATE TABLE IF NOT EXISTS kiva.confidential_threshold_release_requests (
    tenant_id text NOT NULL,
    release_id bytea NOT NULL,
    transcript_id bytea NOT NULL,
    aggregate_value bigint NOT NULL,
    requester_subject text NOT NULL,
    left_authority_subject text NOT NULL,
    right_authority_subject text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    released_at timestamptz,
    PRIMARY KEY (tenant_id, release_id),
    CHECK (tenant_id <> ''),
    CHECK (octet_length(release_id) = 32),
    CHECK (octet_length(transcript_id) = 32),
    CHECK (aggregate_value >= 0 AND aggregate_value <= 4294967290),
    CHECK (requester_subject <> ''),
    CHECK (left_authority_subject <> ''),
    CHECK (right_authority_subject <> ''),
    CHECK (left_authority_subject <> right_authority_subject),
    CHECK (released_at IS NULL OR released_at >= created_at)
);

CREATE INDEX IF NOT EXISTS confidential_threshold_release_requests_transcript_idx
    ON kiva.confidential_threshold_release_requests (tenant_id, transcript_id);

CREATE INDEX IF NOT EXISTS confidential_threshold_release_requests_created_at_idx
    ON kiva.confidential_threshold_release_requests (tenant_id, created_at DESC);

CREATE TABLE IF NOT EXISTS kiva.confidential_threshold_release_approvals (
    tenant_id text NOT NULL,
    release_id bytea NOT NULL,
    authority_subject text NOT NULL,
    approved_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, release_id, authority_subject),
    FOREIGN KEY (tenant_id, release_id)
        REFERENCES kiva.confidential_threshold_release_requests (tenant_id, release_id)
        ON DELETE CASCADE,
    CHECK (tenant_id <> ''),
    CHECK (octet_length(release_id) = 32),
    CHECK (authority_subject <> '')
);
