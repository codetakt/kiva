-- Persist bounded differential privacy epsilon-budget ledgers.

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
