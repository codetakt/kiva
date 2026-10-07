-- Persist accepted additive-share replay records for confidential aggregate transcripts.

CREATE TABLE IF NOT EXISTS kiva.confidential_additive_share_replay_records (
    tenant_id text NOT NULL,
    transcript_id bytea NOT NULL,
    participant_index bigint NOT NULL,
    sequence numeric(20, 0) NOT NULL,
    signing_key_id bytea NOT NULL,
    accepted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, transcript_id, participant_index),
    CHECK (tenant_id <> ''),
    CHECK (octet_length(transcript_id) = 32),
    CHECK (participant_index > 0),
    CHECK (
        sequence >= 1
        AND sequence <= 18446744073709551615
    ),
    CHECK (octet_length(signing_key_id) = 32)
);

CREATE INDEX IF NOT EXISTS confidential_additive_share_replay_records_transcript_idx
    ON kiva.confidential_additive_share_replay_records (tenant_id, transcript_id);

CREATE INDEX IF NOT EXISTS confidential_additive_share_replay_records_accepted_at_idx
    ON kiva.confidential_additive_share_replay_records (tenant_id, accepted_at DESC);
