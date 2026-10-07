-- Add subject-facing PDS lifecycle metadata while keeping accepted xAPI
-- Statements immutable. Existing rows receive migration defaults; new writes
-- are expected to provide explicit purpose, legal basis, and data classes.

ALTER TABLE kiva.pds_consent_grants
    ADD COLUMN purpose text NOT NULL DEFAULT 'unspecified migration purpose',
    ADD COLUMN legal_basis text NOT NULL DEFAULT 'consent',
    ADD COLUMN expires_at timestamptz,
    ADD COLUMN renewed_from_grant_id uuid,
    ADD CONSTRAINT pds_consent_grants_legal_basis_check
        CHECK (
            legal_basis IN (
                'consent',
                'public-task',
                'legitimate-interest',
                'contract',
                'legal-obligation'
            )
        ),
    ADD CONSTRAINT pds_consent_grants_expiry_check
        CHECK (expires_at IS NULL OR expires_at > created_at),
    ADD CONSTRAINT pds_consent_grants_renewal_fk
        FOREIGN KEY (tenant_id, renewed_from_grant_id)
        REFERENCES kiva.pds_consent_grants (tenant_id, grant_id)
        NOT VALID;

ALTER TABLE kiva.pds_consent_grants
    VALIDATE CONSTRAINT pds_consent_grants_renewal_fk;

ALTER TABLE kiva.pds_data_products
    ADD COLUMN legal_basis text NOT NULL DEFAULT 'consent',
    ADD COLUMN data_classes jsonb NOT NULL DEFAULT '["data-product-metadata"]'::jsonb,
    ADD CONSTRAINT pds_data_products_legal_basis_check
        CHECK (
            legal_basis IN (
                'consent',
                'public-task',
                'legitimate-interest',
                'contract',
                'legal-obligation'
            )
        ),
    ADD CONSTRAINT pds_data_products_data_classes_check
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
        );

ALTER TABLE kiva.pds_access_audit_events
    DROP CONSTRAINT IF EXISTS pds_access_audit_events_deny_reason_check;

ALTER TABLE kiva.pds_access_audit_events
    ADD CONSTRAINT pds_access_audit_events_deny_reason_check
    CHECK (
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
    );

CREATE TABLE kiva.pds_lifecycle_events (
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

CREATE INDEX pds_lifecycle_events_subject_idx
    ON kiva.pds_lifecycle_events (tenant_id, data_subject, created_at DESC);
