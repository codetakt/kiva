-- Persist validated additive-share submissions before aggregate release.

CREATE TABLE IF NOT EXISTS kiva.confidential_additive_share_submissions (
    tenant_id text NOT NULL,
    transcript_id bytea NOT NULL,
    participant_index bigint NOT NULL,
    signing_key_id bytea NOT NULL,
    oauth_subject text NOT NULL,
    signed_envelope jsonb NOT NULL,
    submitted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, transcript_id, participant_index),
    CHECK (tenant_id <> ''),
    CHECK (octet_length(transcript_id) = 32),
    CHECK (participant_index > 0),
    CHECK (octet_length(signing_key_id) = 32),
    CHECK (oauth_subject <> ''),
    CHECK (jsonb_typeof(signed_envelope) = 'object')
);

CREATE INDEX IF NOT EXISTS confidential_additive_share_submissions_subject_idx
    ON kiva.confidential_additive_share_submissions (tenant_id, oauth_subject);

CREATE INDEX IF NOT EXISTS confidential_additive_share_submissions_signing_key_idx
    ON kiva.confidential_additive_share_submissions (tenant_id, signing_key_id);
