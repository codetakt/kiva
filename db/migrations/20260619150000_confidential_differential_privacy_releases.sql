-- Persist noised differential privacy aggregate release evidence.

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
