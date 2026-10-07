-- Persist Shamir threshold reconstruction metadata and reconstructed values.

CREATE TABLE IF NOT EXISTS kiva.confidential_shamir_threshold_reconstructions (
    tenant_id text NOT NULL,
    reconstruction_id bytea NOT NULL,
    transcript_id bytea NOT NULL,
    owner_subject text NOT NULL,
    participant_count bigint NOT NULL,
    threshold bigint NOT NULL,
    share_count bigint NOT NULL,
    reconstructed_value bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, reconstruction_id),
    CHECK (tenant_id <> ''),
    CHECK (octet_length(reconstruction_id) = 32),
    CHECK (octet_length(transcript_id) = 32),
    CHECK (owner_subject <> ''),
    CHECK (participant_count >= 2),
    CHECK (threshold >= 2),
    CHECK (threshold <= participant_count),
    CHECK (share_count >= threshold),
    CHECK (share_count <= participant_count),
    CHECK (reconstructed_value >= 0 AND reconstructed_value <= 4294967290)
);

CREATE INDEX IF NOT EXISTS confidential_shamir_threshold_reconstructions_owner_idx
    ON kiva.confidential_shamir_threshold_reconstructions
        (tenant_id, owner_subject, created_at);

CREATE INDEX IF NOT EXISTS confidential_shamir_threshold_reconstructions_transcript_idx
    ON kiva.confidential_shamir_threshold_reconstructions
        (tenant_id, transcript_id, created_at);
