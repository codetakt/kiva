-- Persist registered Ed25519 signing keys for confidential additive-share envelopes.

CREATE TABLE IF NOT EXISTS kiva.confidential_additive_share_signing_keys (
    tenant_id text NOT NULL,
    signing_key_id bytea NOT NULL,
    participant_index bigint NOT NULL,
    public_key bytea NOT NULL,
    oauth_subject text NOT NULL,
    registered_at timestamptz NOT NULL DEFAULT now(),
    revoked_at timestamptz,
    PRIMARY KEY (tenant_id, signing_key_id),
    CHECK (tenant_id <> ''),
    CHECK (octet_length(signing_key_id) = 32),
    CHECK (participant_index > 0),
    CHECK (octet_length(public_key) = 32),
    CHECK (oauth_subject <> ''),
    CHECK (revoked_at IS NULL OR revoked_at >= registered_at)
);

CREATE INDEX IF NOT EXISTS confidential_additive_share_signing_keys_subject_idx
    ON kiva.confidential_additive_share_signing_keys (tenant_id, oauth_subject)
    WHERE revoked_at IS NULL;

CREATE INDEX IF NOT EXISTS confidential_additive_share_signing_keys_participant_idx
    ON kiva.confidential_additive_share_signing_keys (tenant_id, participant_index)
    WHERE revoked_at IS NULL;
