-- Add metadata-only operational lifecycle workflows for PDS retention,
-- tombstone publication, correction, dispute, and tombstone cases.
-- These tables do not mutate accepted xAPI Statements.

CREATE TABLE kiva.pds_retention_policies (
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

CREATE INDEX pds_retention_policies_owner_idx
    ON kiva.pds_retention_policies (tenant_id, owner_subject, created_at, policy_id);

CREATE TABLE kiva.pds_lifecycle_cases (
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
        REFERENCES kiva.pds_retention_policies (tenant_id, policy_id)
        NOT VALID,
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

ALTER TABLE kiva.pds_lifecycle_cases
    VALIDATE CONSTRAINT pds_lifecycle_cases_retention_policy_fk;

CREATE INDEX pds_lifecycle_cases_subject_idx
    ON kiva.pds_lifecycle_cases (tenant_id, data_subject, updated_at DESC, case_id);

CREATE INDEX pds_lifecycle_cases_policy_idx
    ON kiva.pds_lifecycle_cases (tenant_id, retention_policy_id, updated_at DESC)
    WHERE retention_policy_id IS NOT NULL;
