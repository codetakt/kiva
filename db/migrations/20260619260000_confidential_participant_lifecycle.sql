CREATE TABLE IF NOT EXISTS kiva.confidential_participants (
    tenant_id text NOT NULL,
    owner_subject text NOT NULL,
    participant_subject text NOT NULL,
    participant_index bigint NOT NULL,
    status text NOT NULL DEFAULT 'invited',
    mechanism_roles text[] NOT NULL,
    authorization_scope text NOT NULL,
    key_ceremony_ref text NOT NULL,
    recovery_contact_ref text NOT NULL,
    evidence_refs text[] NOT NULL DEFAULT ARRAY[]::text[],
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, owner_subject, participant_subject, participant_index),
    CHECK (tenant_id <> ''),
    CHECK (owner_subject <> ''),
    CHECK (participant_subject <> ''),
    CHECK (participant_index > 0),
    CHECK (status IN ('invited', 'active', 'suspended', 'frozen', 'removed')),
    CHECK (array_length(mechanism_roles, 1) IS NOT NULL),
    CHECK (array_position(mechanism_roles, '') IS NULL),
    CHECK (authorization_scope <> ''),
    CHECK (key_ceremony_ref <> ''),
    CHECK (recovery_contact_ref <> ''),
    CHECK (array_position(evidence_refs, '') IS NULL),
    CHECK (updated_at >= created_at)
);

CREATE INDEX IF NOT EXISTS confidential_participants_status_idx
    ON kiva.confidential_participants (tenant_id, owner_subject, status);

CREATE INDEX IF NOT EXISTS confidential_participants_subject_idx
    ON kiva.confidential_participants (tenant_id, owner_subject, participant_subject);

CREATE TABLE IF NOT EXISTS kiva.confidential_participant_audit_events (
    tenant_id text NOT NULL,
    event_id uuid NOT NULL,
    owner_subject text NOT NULL,
    participant_subject text NOT NULL,
    participant_index bigint NOT NULL,
    previous_status text,
    new_status text NOT NULL,
    actor_subject text NOT NULL,
    reason text NOT NULL,
    evidence_refs text[] NOT NULL DEFAULT ARRAY[]::text[],
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, event_id),
    FOREIGN KEY (tenant_id, owner_subject, participant_subject, participant_index)
        REFERENCES kiva.confidential_participants (
            tenant_id,
            owner_subject,
            participant_subject,
            participant_index
        )
        ON DELETE CASCADE,
    CHECK (tenant_id <> ''),
    CHECK (owner_subject <> ''),
    CHECK (participant_subject <> ''),
    CHECK (participant_index > 0),
    CHECK (
        previous_status IS NULL
        OR previous_status IN ('invited', 'active', 'suspended', 'frozen', 'removed')
    ),
    CHECK (new_status IN ('invited', 'active', 'suspended', 'frozen', 'removed')),
    CHECK (actor_subject <> ''),
    CHECK (reason <> ''),
    CHECK (array_position(evidence_refs, '') IS NULL)
);

CREATE INDEX IF NOT EXISTS confidential_participant_audit_events_participant_idx
    ON kiva.confidential_participant_audit_events (
        tenant_id,
        owner_subject,
        participant_subject,
        participant_index,
        created_at DESC
    );
