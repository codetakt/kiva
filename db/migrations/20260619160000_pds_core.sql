-- Persist Kiva Personal Data Store consent, data-product, and audit metadata.

CREATE TABLE IF NOT EXISTS kiva.pds_consent_grants (
    tenant_id text NOT NULL,
    grant_id uuid NOT NULL,
    data_subject text NOT NULL,
    grantee_subject text NOT NULL,
    scope text NOT NULL,
    state text NOT NULL DEFAULT 'active',
    created_at timestamptz NOT NULL DEFAULT now(),
    revoked_at timestamptz,
    PRIMARY KEY (tenant_id, grant_id),
    CHECK (tenant_id <> ''),
    CHECK (data_subject <> ''),
    CHECK (grantee_subject <> ''),
    CHECK (scope IN ('raw-records', 'personal-data', 'aggregate-insights')),
    CHECK (state IN ('active', 'revoked')),
    CHECK (
        (state = 'active' AND revoked_at IS NULL)
        OR (state = 'revoked' AND revoked_at IS NOT NULL)
    ),
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
    title text NOT NULL,
    purpose text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, data_product_id),
    CHECK (tenant_id <> ''),
    CHECK (owner_subject <> ''),
    CHECK (source_kind IN ('threshold-release', 'differential-privacy-release')),
    CHECK (octet_length(release_id) = 32),
    CHECK (title <> ''),
    CHECK (purpose <> '')
);

CREATE INDEX IF NOT EXISTS pds_data_products_owner_idx
    ON kiva.pds_data_products (tenant_id, owner_subject, created_at);

CREATE INDEX IF NOT EXISTS pds_data_products_release_idx
    ON kiva.pds_data_products (tenant_id, source_kind, release_id);

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
    CHECK (
        deny_reason IS NULL
        OR deny_reason IN (
            'rawRecordBoundary',
            'releaseRequiresAuthority',
            'consentRequired',
            'consentRevoked',
            'delegationScopeMismatch',
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
