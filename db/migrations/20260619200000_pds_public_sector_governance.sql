-- Add public-sector governance profile metadata for PDS deployments.
-- These records describe deployable separation-of-authority arrangements;
-- they do not change the IEEE xAPI LRS conformance surface.

CREATE TABLE kiva.pds_governance_profiles (
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

CREATE INDEX pds_governance_profiles_owner_idx
    ON kiva.pds_governance_profiles (tenant_id, profile_owner_subject, created_at);
