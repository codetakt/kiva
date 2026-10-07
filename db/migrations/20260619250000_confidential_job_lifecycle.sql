-- Add durable confidential job lifecycle metadata and metadata-only audit events.

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
