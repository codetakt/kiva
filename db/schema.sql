-- Kiva database schema (managed by Atlas).
-- This file is the desired schema state for the versioned migration workflow.

CREATE SCHEMA IF NOT EXISTS kiva;

-- gen_random_uuid() and digest primitives are useful for future immutable
-- record identifiers and audit evidence.
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS kiva.xapi_statements (
    tenant_id text NOT NULL,
    statement_id uuid NOT NULL,
    actor_ifi bytea,
    verb_id text,
    object_id text,
    authority json,
    statement json NOT NULL,
    opaque_values json NOT NULL DEFAULT '{}'::json,
    stored_at timestamptz NOT NULL DEFAULT now(),
    version text NOT NULL DEFAULT '2.0.0',
    voided_by_statement_id uuid,
    PRIMARY KEY (tenant_id, statement_id),
    CHECK (tenant_id <> ''),
    CHECK (json_typeof(statement) = 'object'),
    CHECK (authority IS NULL OR json_typeof(authority) = 'object')
);

CREATE INDEX IF NOT EXISTS xapi_statements_actor_ifi_idx
    ON kiva.xapi_statements (tenant_id, actor_ifi)
    WHERE actor_ifi IS NOT NULL;

CREATE INDEX IF NOT EXISTS xapi_statements_verb_id_idx
    ON kiva.xapi_statements (tenant_id, verb_id)
    WHERE verb_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS xapi_statements_object_id_idx
    ON kiva.xapi_statements (tenant_id, object_id)
    WHERE object_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS xapi_statements_stored_at_idx
    ON kiva.xapi_statements (tenant_id, stored_at DESC);

CREATE TABLE IF NOT EXISTS kiva.xapi_statement_attachments (
    tenant_id text NOT NULL,
    statement_id uuid NOT NULL,
    sha2 text NOT NULL,
    content_type bytea NOT NULL,
    body bytea NOT NULL,
    PRIMARY KEY (tenant_id, statement_id, sha2),
    FOREIGN KEY (tenant_id, statement_id)
        REFERENCES kiva.xapi_statements (tenant_id, statement_id)
        ON DELETE CASCADE,
    CHECK (tenant_id <> ''),
    CHECK (sha2 ~ '^([0-9a-fA-F]{56}|[0-9a-fA-F]{64}|[0-9a-fA-F]{96}|[0-9a-fA-F]{128})$')
);

CREATE TABLE IF NOT EXISTS kiva.xapi_documents (
    tenant_id text NOT NULL,
    resource_kind text NOT NULL,
    document_key bytea NOT NULL,
    document_id bytea NOT NULL,
    activity_id text,
    agent json,
    registration uuid,
    content_type text NOT NULL,
    content_json json,
    content_bytes bytea,
    etag text NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, resource_kind, document_key),
    CHECK (tenant_id <> ''),
    CHECK (octet_length(document_key) > 0),
    CHECK (resource_kind IN ('state', 'agent-profile', 'activity-profile')),
    CHECK (agent IS NULL OR json_typeof(agent) = 'object'),
    CHECK (
        (content_json IS NOT NULL AND content_bytes IS NULL)
        OR (content_json IS NULL AND content_bytes IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS xapi_documents_activity_idx
    ON kiva.xapi_documents (tenant_id, resource_kind, activity_id)
    WHERE activity_id IS NOT NULL;

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

CREATE TABLE IF NOT EXISTS kiva.confidential_shamir_threshold_reconstructions (
    tenant_id text NOT NULL,
    reconstruction_id bytea NOT NULL,
    transcript_id bytea NOT NULL,
    owner_subject text NOT NULL,
    participant_count bigint NOT NULL,
    threshold bigint NOT NULL,
    share_count bigint NOT NULL,
    reconstructed_value bigint NOT NULL,
    verification_profile text NOT NULL DEFAULT 'kiva-shamir-hash-vss-v1',
    verified_share_count bigint NOT NULL DEFAULT 0,
    complaint_count bigint NOT NULL DEFAULT 0,
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
    CHECK (reconstructed_value >= 0 AND reconstructed_value <= 4294967290),
    CHECK (verification_profile = 'kiva-shamir-hash-vss-v1'),
    CHECK (
        verified_share_count >= threshold
        AND verified_share_count <= share_count
        AND complaint_count >= 0
        AND complaint_count <= share_count
    )
);

CREATE INDEX IF NOT EXISTS confidential_shamir_threshold_reconstructions_owner_idx
    ON kiva.confidential_shamir_threshold_reconstructions
        (tenant_id, owner_subject, created_at);

CREATE INDEX IF NOT EXISTS confidential_shamir_threshold_reconstructions_transcript_idx
    ON kiva.confidential_shamir_threshold_reconstructions
        (tenant_id, transcript_id, created_at);

CREATE TABLE IF NOT EXISTS kiva.confidential_differential_privacy_budgets (
    tenant_id text NOT NULL,
    budget_id bytea NOT NULL,
    owner_subject text NOT NULL,
    total_epsilon_microunits numeric(20, 0) NOT NULL,
    spent_epsilon_microunits numeric(20, 0) NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, budget_id),
    CHECK (tenant_id <> ''),
    CHECK (octet_length(budget_id) = 32),
    CHECK (owner_subject <> ''),
    CHECK (
        total_epsilon_microunits >= 1
        AND total_epsilon_microunits <= 18446744073709551615
    ),
    CHECK (
        spent_epsilon_microunits >= 0
        AND spent_epsilon_microunits <= 18446744073709551615
    ),
    CHECK (spent_epsilon_microunits <= total_epsilon_microunits)
);

CREATE INDEX IF NOT EXISTS confidential_differential_privacy_budgets_owner_idx
    ON kiva.confidential_differential_privacy_budgets (tenant_id, owner_subject);

CREATE TABLE IF NOT EXISTS kiva.confidential_differential_privacy_releases (
    tenant_id text NOT NULL,
    release_id bytea NOT NULL,
    budget_id bytea NOT NULL,
    transcript_id bytea NOT NULL,
    owner_subject text NOT NULL,
    aggregate_value bigint NOT NULL,
    noise_value bigint NOT NULL,
    noised_value bigint NOT NULL,
    mechanism text NOT NULL,
    accounting_scope text NOT NULL,
    epsilon_microunits numeric(20, 0) NOT NULL,
    l1_sensitivity numeric(20, 0) NOT NULL,
    data_subject_count bigint NOT NULL,
    minimum_population bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, release_id),
    CHECK (tenant_id <> ''),
    CHECK (octet_length(release_id) = 32),
    CHECK (octet_length(budget_id) = 32),
    CHECK (octet_length(transcript_id) = 32),
    CHECK (owner_subject <> ''),
    CHECK (aggregate_value >= 0 AND aggregate_value <= 4294967290),
    CHECK (mechanism IN ('discrete-laplace')),
    CHECK (accounting_scope IN ('subject-budget')),
    CHECK (
        epsilon_microunits >= 1
        AND epsilon_microunits <= 18446744073709551615
    ),
    CHECK (
        l1_sensitivity >= 1
        AND l1_sensitivity <= 18446744073709551615
    ),
    CHECK (minimum_population >= 2),
    CHECK (data_subject_count >= minimum_population),
    FOREIGN KEY (tenant_id, budget_id)
        REFERENCES kiva.confidential_differential_privacy_budgets
            (tenant_id, budget_id)
        ON DELETE RESTRICT,
    FOREIGN KEY (tenant_id, transcript_id, aggregate_value)
        REFERENCES kiva.confidential_additive_aggregate_results
            (tenant_id, transcript_id, aggregate_value)
        ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS confidential_differential_privacy_releases_owner_idx
    ON kiva.confidential_differential_privacy_releases (tenant_id, owner_subject, created_at);

CREATE INDEX IF NOT EXISTS confidential_differential_privacy_releases_budget_idx
    ON kiva.confidential_differential_privacy_releases (tenant_id, budget_id, created_at);

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
    CHECK (released_at IS NULL OR released_at >= created_at),
    FOREIGN KEY (tenant_id, transcript_id, aggregate_value)
        REFERENCES kiva.confidential_additive_aggregate_results
            (tenant_id, transcript_id, aggregate_value)
        ON DELETE RESTRICT
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

CREATE TABLE IF NOT EXISTS kiva.confidential_jobs (
    tenant_id text NOT NULL,
    job_id bytea NOT NULL,
    owner_subject text NOT NULL,
    mechanism text NOT NULL,
    mechanism_profile text NOT NULL,
    workload_id text NOT NULL,
    participant_subjects text[] NOT NULL,
    policy_binding_id text NOT NULL,
    status text NOT NULL DEFAULT 'draft',
    retry_count bigint NOT NULL DEFAULT 0,
    max_retry_count bigint NOT NULL DEFAULT 3,
    deadline_at timestamptz NOT NULL,
    admission_quota_key text NOT NULL,
    evidence_refs text[] NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, job_id),
    CHECK (tenant_id <> ''),
    CHECK (octet_length(job_id) = 32),
    CHECK (owner_subject <> ''),
    CHECK (
        mechanism IN (
            'additive-secure-aggregation',
            'differential-privacy-release',
            'shamir-vss-admission',
            'threshold-frost-signing',
            'psi-cohort-hash-token',
            'psi-oprf-dh-voprf',
            'tee-portable-attestation',
            'tee-aws-nitro-attestation',
            'fhe-sum-feasibility',
            'fhe-sum-runtime'
        )
    ),
    CHECK (mechanism_profile <> ''),
    CHECK (workload_id <> ''),
    CHECK (array_length(participant_subjects, 1) >= 2),
    CHECK (array_position(participant_subjects, '') IS NULL),
    CHECK (policy_binding_id <> ''),
    CHECK (
        status IN (
            'draft',
            'admitted',
            'running',
            'awaitingRelease',
            'released',
            'failed',
            'cancelled',
            'expired',
            'retired'
        )
    ),
    CHECK (retry_count >= 0),
    CHECK (max_retry_count >= 1),
    CHECK (retry_count <= max_retry_count),
    CHECK (admission_quota_key <> ''),
    CHECK (array_length(evidence_refs, 1) >= 1),
    CHECK (array_position(evidence_refs, '') IS NULL),
    CHECK (updated_at >= created_at)
);

CREATE INDEX IF NOT EXISTS confidential_jobs_owner_idx
    ON kiva.confidential_jobs (tenant_id, owner_subject, updated_at DESC);

CREATE INDEX IF NOT EXISTS confidential_jobs_status_idx
    ON kiva.confidential_jobs (tenant_id, status, updated_at DESC);

CREATE INDEX IF NOT EXISTS confidential_jobs_quota_idx
    ON kiva.confidential_jobs (tenant_id, admission_quota_key, status);

CREATE TABLE IF NOT EXISTS kiva.confidential_job_audit_events (
    tenant_id text NOT NULL,
    event_id uuid NOT NULL,
    job_id bytea NOT NULL,
    previous_status text,
    new_status text NOT NULL,
    actor_subject text NOT NULL,
    reason text NOT NULL,
    evidence_refs text[] NOT NULL DEFAULT ARRAY[]::text[],
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, event_id),
    FOREIGN KEY (tenant_id, job_id)
        REFERENCES kiva.confidential_jobs (tenant_id, job_id)
        ON DELETE CASCADE,
    CHECK (tenant_id <> ''),
    CHECK (octet_length(job_id) = 32),
    CHECK (
        previous_status IS NULL
        OR previous_status IN (
            'draft',
            'admitted',
            'running',
            'awaitingRelease',
            'released',
            'failed',
            'cancelled',
            'expired',
            'retired'
        )
    ),
    CHECK (
        new_status IN (
            'draft',
            'admitted',
            'running',
            'awaitingRelease',
            'released',
            'failed',
            'cancelled',
            'expired',
            'retired'
        )
    ),
    CHECK (actor_subject <> ''),
    CHECK (reason <> ''),
    CHECK (array_position(evidence_refs, '') IS NULL)
);

CREATE INDEX IF NOT EXISTS confidential_job_audit_events_job_idx
    ON kiva.confidential_job_audit_events (tenant_id, job_id, created_at DESC);

CREATE TABLE IF NOT EXISTS kiva.pds_consent_grants (
    tenant_id text NOT NULL,
    grant_id uuid NOT NULL,
    data_subject text NOT NULL,
    grantee_subject text NOT NULL,
    scope text NOT NULL,
    state text NOT NULL DEFAULT 'active',
    purpose text NOT NULL,
    legal_basis text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz,
    revoked_at timestamptz,
    renewed_from_grant_id uuid,
    PRIMARY KEY (tenant_id, grant_id),
    FOREIGN KEY (tenant_id, renewed_from_grant_id)
        REFERENCES kiva.pds_consent_grants (tenant_id, grant_id),
    CHECK (tenant_id <> ''),
    CHECK (data_subject <> ''),
    CHECK (grantee_subject <> ''),
    CHECK (scope IN ('raw-records', 'personal-data', 'aggregate-insights')),
    CHECK (state IN ('active', 'revoked')),
    CHECK (purpose <> ''),
    CHECK (
        legal_basis IN (
            'consent',
            'public-task',
            'legitimate-interest',
            'contract',
            'legal-obligation'
        )
    ),
    CHECK (
        (state = 'active' AND revoked_at IS NULL)
        OR (state = 'revoked' AND revoked_at IS NOT NULL)
    ),
    CHECK (expires_at IS NULL OR expires_at > created_at),
    CHECK (revoked_at IS NULL OR revoked_at >= created_at)
);

CREATE INDEX IF NOT EXISTS pds_consent_grants_owner_idx
    ON kiva.pds_consent_grants (tenant_id, data_subject, created_at);

CREATE INDEX IF NOT EXISTS pds_consent_grants_grantee_idx
    ON kiva.pds_consent_grants (tenant_id, grantee_subject, created_at);

CREATE TABLE IF NOT EXISTS kiva.pds_data_products (
    tenant_id text NOT NULL,
    data_product_id uuid NOT NULL,
    owner_subject text NOT NULL,
    source_kind text NOT NULL,
    release_id bytea NOT NULL,
    threshold_release_id bytea GENERATED ALWAYS AS (
        CASE
            WHEN source_kind = 'threshold-release' THEN release_id
            ELSE NULL::bytea
        END
    ) STORED,
    differential_privacy_release_id bytea GENERATED ALWAYS AS (
        CASE
            WHEN source_kind = 'differential-privacy-release' THEN release_id
            ELSE NULL::bytea
        END
    ) STORED,
    shamir_threshold_reconstruction_id bytea GENERATED ALWAYS AS (
        CASE
            WHEN source_kind = 'shamir-threshold-reconstruction' THEN release_id
            ELSE NULL::bytea
        END
    ) STORED,
    title text NOT NULL,
    purpose text NOT NULL,
    legal_basis text NOT NULL,
    data_classes jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, data_product_id),
    FOREIGN KEY (tenant_id, threshold_release_id)
        REFERENCES kiva.confidential_threshold_release_requests (tenant_id, release_id),
    FOREIGN KEY (tenant_id, differential_privacy_release_id)
        REFERENCES kiva.confidential_differential_privacy_releases (tenant_id, release_id),
    FOREIGN KEY (tenant_id, shamir_threshold_reconstruction_id)
        REFERENCES kiva.confidential_shamir_threshold_reconstructions
            (tenant_id, reconstruction_id),
    CHECK (tenant_id <> ''),
    CHECK (owner_subject <> ''),
    CHECK (
        source_kind IN (
            'threshold-release',
            'differential-privacy-release',
            'shamir-threshold-reconstruction'
        )
    ),
    CHECK (octet_length(release_id) = 32),
    CHECK (title <> ''),
    CHECK (purpose <> ''),
    CHECK (
        legal_basis IN (
            'consent',
            'public-task',
            'legitimate-interest',
            'contract',
            'legal-obligation'
        )
    ),
    CHECK (
        jsonb_typeof(data_classes) = 'array'
        AND jsonb_array_length(data_classes) > 0
        AND data_classes <@ '[
            "statement-metadata",
            "consent-metadata",
            "data-product-metadata",
            "access-audit-metadata",
            "lifecycle-metadata"
        ]'::jsonb
    )
);

CREATE INDEX IF NOT EXISTS pds_data_products_owner_idx
    ON kiva.pds_data_products (tenant_id, owner_subject, created_at);

CREATE INDEX IF NOT EXISTS pds_data_products_release_idx
    ON kiva.pds_data_products (tenant_id, source_kind, release_id);

CREATE INDEX IF NOT EXISTS pds_data_products_threshold_release_id_idx
    ON kiva.pds_data_products (tenant_id, threshold_release_id)
    WHERE threshold_release_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS pds_data_products_differential_privacy_release_id_idx
    ON kiva.pds_data_products (tenant_id, differential_privacy_release_id)
    WHERE differential_privacy_release_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS pds_data_products_shamir_threshold_reconstruction_id_idx
    ON kiva.pds_data_products (tenant_id, shamir_threshold_reconstruction_id)
    WHERE shamir_threshold_reconstruction_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS kiva.pds_access_audit_events (
    tenant_id text NOT NULL,
    event_id uuid NOT NULL,
    requester_subject text NOT NULL,
    role text NOT NULL,
    resource text NOT NULL,
    action text NOT NULL,
    grant_id uuid,
    data_product_id uuid,
    decision text NOT NULL,
    deny_reason text,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, event_id),
    CHECK (tenant_id <> ''),
    CHECK (requester_subject <> ''),
    CHECK (
        role IN (
            'data-subject',
            'data-custodian',
            'policy-authority',
            'analyst',
            'auditor',
            'computation-participant'
        )
    ),
    CHECK (
        resource IN (
            'raw-learning-record',
            'personal-data-store',
            'aggregate-insight',
            'audit-evidence',
            'confidential-computation'
        )
    ),
    CHECK (
        action IN (
            'write',
            'read',
            'delegate',
            'revoke',
            'compute',
            'release',
            'audit'
        )
    ),
    CHECK (decision IN ('allow', 'deny')),
    CHECK (
        (decision = 'allow' AND deny_reason IS NULL)
        OR (decision = 'deny' AND deny_reason IS NOT NULL)
    ),
    CONSTRAINT pds_access_audit_events_deny_reason_check CHECK (
        deny_reason IS NULL
        OR deny_reason IN (
            'rawRecordBoundary',
            'releaseRequiresAuthority',
            'consentRequired',
            'consentRevoked',
            'consentExpired',
            'delegationScopeMismatch',
            'dataProductUnavailable',
            'unsupportedRoleAction'
        )
    )
);

CREATE INDEX IF NOT EXISTS pds_access_audit_events_requester_idx
    ON kiva.pds_access_audit_events (tenant_id, requester_subject, created_at DESC);

CREATE INDEX IF NOT EXISTS pds_access_audit_events_grant_idx
    ON kiva.pds_access_audit_events (tenant_id, grant_id, created_at DESC)
    WHERE grant_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS pds_access_audit_events_data_product_idx
    ON kiva.pds_access_audit_events (tenant_id, data_product_id, created_at DESC)
    WHERE data_product_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS kiva.pds_lifecycle_events (
    tenant_id text NOT NULL,
    event_id uuid NOT NULL,
    data_subject text NOT NULL,
    event_type text NOT NULL,
    resource_kind text NOT NULL,
    resource_id text NOT NULL,
    reason text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, event_id),
    CHECK (tenant_id <> ''),
    CHECK (data_subject <> ''),
    CHECK (
        event_type IN (
            'exported',
            'retention-reviewed',
            'correction-requested',
            'dispute-opened',
            'tombstone-recorded'
        )
    ),
    CHECK (
        resource_kind IN (
            'xapi-statement',
            'consent-grant',
            'data-product',
            'metadata-export'
        )
    ),
    CHECK (resource_id <> ''),
    CHECK (reason <> '')
);

CREATE INDEX IF NOT EXISTS pds_lifecycle_events_subject_idx
    ON kiva.pds_lifecycle_events (tenant_id, data_subject, created_at DESC);

CREATE TABLE IF NOT EXISTS kiva.pds_retention_policies (
    tenant_id text NOT NULL,
    policy_id uuid NOT NULL,
    owner_subject text NOT NULL,
    resource_kind text NOT NULL,
    data_class text NOT NULL,
    retention_period_days integer NOT NULL,
    review_interval_days integer NOT NULL,
    tombstone_after_days integer,
    tombstone_publication text NOT NULL,
    legal_basis text NOT NULL,
    description text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, policy_id),
    CHECK (tenant_id <> ''),
    CHECK (owner_subject <> ''),
    CHECK (
        resource_kind IN (
            'xapi-statement',
            'consent-grant',
            'data-product',
            'metadata-export'
        )
    ),
    CHECK (
        data_class IN (
            'statement-metadata',
            'consent-metadata',
            'data-product-metadata',
            'access-audit-metadata',
            'lifecycle-metadata'
        )
    ),
    CHECK (retention_period_days > 0),
    CHECK (review_interval_days > 0),
    CHECK (
        tombstone_after_days IS NULL
        OR tombstone_after_days >= retention_period_days
    ),
    CHECK (
        tombstone_publication IN (
            'metadata-only',
            'subject-visible',
            'public-redacted'
        )
    ),
    CHECK (
        legal_basis IN (
            'consent',
            'public-task',
            'legitimate-interest',
            'contract',
            'legal-obligation'
        )
    ),
    CHECK (description <> '')
);

CREATE INDEX IF NOT EXISTS pds_retention_policies_owner_idx
    ON kiva.pds_retention_policies (tenant_id, owner_subject, created_at, policy_id);

CREATE TABLE IF NOT EXISTS kiva.pds_lifecycle_cases (
    tenant_id text NOT NULL,
    case_id uuid NOT NULL,
    data_subject text NOT NULL,
    case_type text NOT NULL,
    resource_kind text NOT NULL,
    resource_id text NOT NULL,
    status text NOT NULL,
    retention_policy_id uuid,
    summary text NOT NULL,
    resolution text,
    tombstone_publication text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, case_id),
    CONSTRAINT pds_lifecycle_cases_retention_policy_fk
        FOREIGN KEY (tenant_id, retention_policy_id)
        REFERENCES kiva.pds_retention_policies (tenant_id, policy_id),
    CHECK (tenant_id <> ''),
    CHECK (data_subject <> ''),
    CHECK (case_type IN ('correction', 'dispute', 'tombstone')),
    CHECK (
        resource_kind IN (
            'xapi-statement',
            'consent-grant',
            'data-product',
            'metadata-export'
        )
    ),
    CHECK (resource_id <> ''),
    CHECK (
        status IN (
            'open',
            'under-review',
            'resolved',
            'rejected',
            'tombstoned'
        )
    ),
    CHECK (summary <> ''),
    CHECK (resolution IS NULL OR resolution <> ''),
    CHECK (
        status NOT IN ('resolved', 'rejected', 'tombstoned')
        OR resolution IS NOT NULL
    ),
    CHECK (status <> 'tombstoned' OR case_type = 'tombstone'),
    CHECK (case_type <> 'tombstone' OR tombstone_publication IS NOT NULL),
    CHECK (
        tombstone_publication IS NULL
        OR tombstone_publication IN (
            'metadata-only',
            'subject-visible',
            'public-redacted'
        )
    ),
    CHECK (updated_at >= created_at)
);

CREATE INDEX IF NOT EXISTS pds_lifecycle_cases_subject_idx
    ON kiva.pds_lifecycle_cases (tenant_id, data_subject, updated_at DESC, case_id);

CREATE INDEX IF NOT EXISTS pds_lifecycle_cases_policy_idx
    ON kiva.pds_lifecycle_cases (tenant_id, retention_policy_id, updated_at DESC)
    WHERE retention_policy_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS kiva.pds_governance_profiles (
    tenant_id text NOT NULL,
    profile_id uuid NOT NULL,
    profile_owner_subject text NOT NULL,
    profile_kind text NOT NULL,
    tenant_partition text NOT NULL,
    subject_partition text NOT NULL,
    data_custodian_subject text NOT NULL,
    platform_operator_subject text NOT NULL,
    policy_authority_subject text NOT NULL,
    analyst_subject text NOT NULL,
    auditor_subject text NOT NULL,
    identity_authority_subject text,
    emergency_access_policy text NOT NULL,
    runbook_refs text[] NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, profile_id),
    CHECK (tenant_id <> ''),
    CHECK (profile_owner_subject <> ''),
    CHECK (profile_kind IN ('single-operator', 'school-district', 'multi-agency')),
    CHECK (tenant_partition <> ''),
    CHECK (subject_partition <> ''),
    CHECK (data_custodian_subject <> ''),
    CHECK (platform_operator_subject <> ''),
    CHECK (policy_authority_subject <> ''),
    CHECK (analyst_subject <> ''),
    CHECK (auditor_subject <> ''),
    CHECK (identity_authority_subject IS NULL OR identity_authority_subject <> ''),
    CHECK (emergency_access_policy <> ''),
    CHECK (array_length(runbook_refs, 1) > 0),
    CHECK (array_position(runbook_refs, '') IS NULL),
    CHECK (data_custodian_subject <> platform_operator_subject),
    CHECK (data_custodian_subject <> policy_authority_subject),
    CHECK (data_custodian_subject <> analyst_subject),
    CHECK (data_custodian_subject <> auditor_subject),
    CHECK (platform_operator_subject <> policy_authority_subject),
    CHECK (platform_operator_subject <> analyst_subject),
    CHECK (platform_operator_subject <> auditor_subject),
    CHECK (policy_authority_subject <> analyst_subject),
    CHECK (policy_authority_subject <> auditor_subject),
    CHECK (analyst_subject <> auditor_subject),
    CHECK (
        identity_authority_subject IS NULL
        OR (
            identity_authority_subject <> data_custodian_subject
            AND identity_authority_subject <> platform_operator_subject
            AND identity_authority_subject <> policy_authority_subject
            AND identity_authority_subject <> analyst_subject
            AND identity_authority_subject <> auditor_subject
        )
    )
);

CREATE INDEX IF NOT EXISTS pds_governance_profiles_owner_idx
    ON kiva.pds_governance_profiles (tenant_id, profile_owner_subject, created_at);

CREATE TABLE IF NOT EXISTS kiva.pds_provider_bindings (
    tenant_id text NOT NULL,
    binding_id uuid NOT NULL,
    owner_subject text NOT NULL,
    mechanism text NOT NULL,
    provider_kind text NOT NULL,
    capability text NOT NULL,
    profile text NOT NULL,
    provider_id_digest text NOT NULL,
    artifact_digest text NOT NULL,
    parameter_set_digest text,
    review_evidence_digest text,
    production_claim_requested boolean NOT NULL,
    binding_digest text NOT NULL,
    evidence_gate text NOT NULL,
    release_gate text NOT NULL,
    runtime_class text NOT NULL,
    production_ready boolean NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, binding_id),
    CHECK (tenant_id <> ''),
    CHECK (owner_subject <> ''),
    CHECK (
        mechanism IN (
            'additive-n-of-n',
            'private-set-intersection',
            'private-set-intersection-oprf-dh',
            'shamir-threshold',
            'threshold-frost-signing',
            'trusted-execution-environment',
            'trusted-execution-environment-aws-nitro',
            'fhe-sum-feasibility',
            'fhe-sum-runtime'
        )
    ),
    CHECK (
        provider_kind IN (
            'reference-primitive',
            'secret-custody',
            'trusted-execution-environment',
            'fully-homomorphic-encryption',
            'private-set-intersection',
            'threshold-cryptography'
        )
    ),
    CHECK (
        capability IN (
            'secret-custody',
            'tee-attestation',
            'fhe-evaluation',
            'psi-evaluation',
            'threshold-signing',
            'reference-fixtures'
        )
    ),
    CHECK (
        profile IN (
            'kiva-reference-primitives-v1',
            'kiva-secret-compute-provider-contract-v1'
        )
    ),
    CHECK (provider_id_digest ~ '^[0-9a-f]{64}$'),
    CHECK (artifact_digest ~ '^[0-9a-f]{64}$'),
    CHECK (parameter_set_digest IS NULL OR parameter_set_digest ~ '^[0-9a-f]{64}$'),
    CHECK (review_evidence_digest IS NULL OR review_evidence_digest ~ '^[0-9a-f]{64}$'),
    CHECK (binding_digest ~ '^[0-9a-f]{64}$'),
    CHECK (
        evidence_gate IN (
            'reference-fixture',
            'external-non-production',
            'external-reviewed'
        )
    ),
    CHECK (release_gate IN ('fixture-only', 'non-production', 'production')),
    CHECK (
        runtime_class IN (
            'reference',
            'external-non-production',
            'external-reviewed-production'
        )
    )
);

CREATE INDEX IF NOT EXISTS pds_provider_bindings_owner_idx
    ON kiva.pds_provider_bindings (tenant_id, owner_subject, created_at ASC);

CREATE INDEX IF NOT EXISTS pds_provider_bindings_runtime_class_idx
    ON kiva.pds_provider_bindings (tenant_id, runtime_class, production_ready);

CREATE TABLE IF NOT EXISTS kiva.pds_ledger_events (
    tenant_id text NOT NULL,
    event_kind text NOT NULL,
    owner_subject text NOT NULL,
    resource_kind text NOT NULL,
    resource_id text NOT NULL,
    occurred_at timestamptz NOT NULL,
    PRIMARY KEY (tenant_id, event_kind, resource_id),
    CHECK (tenant_id <> ''),
    CHECK (
        event_kind IN (
            'consent-grant-registered',
            'consent-grant-revoked',
            'data-product-registered',
            'access-decision-recorded',
            'lifecycle-event-registered',
            'retention-policy-registered',
            'lifecycle-case-opened',
            'lifecycle-case-transitioned',
            'governance-profile-registered',
            'portability-export-completed',
            'portability-export-rejected'
        )
    ),
    CHECK (owner_subject <> ''),
    CHECK (
        resource_kind IN (
            'consent-grant',
            'data-product',
            'access-audit-event',
            'lifecycle-event',
            'retention-policy',
            'lifecycle-case',
            'governance-profile',
            'data-portability-export'
        )
    ),
    CHECK (resource_id <> '')
);

CREATE INDEX IF NOT EXISTS pds_ledger_events_owner_idx
    ON kiva.pds_ledger_events (tenant_id, owner_subject, occurred_at DESC);

CREATE INDEX IF NOT EXISTS pds_ledger_events_resource_idx
    ON kiva.pds_ledger_events (tenant_id, resource_kind, resource_id);
