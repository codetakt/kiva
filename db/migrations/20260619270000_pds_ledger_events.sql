-- Add append-only PDS lifecycle ledger projection events.

CREATE TABLE kiva.pds_ledger_events (
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
            'governance-profile-registered'
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
            'governance-profile'
        )
    ),
    CHECK (resource_id <> '')
);

CREATE INDEX pds_ledger_events_owner_idx
    ON kiva.pds_ledger_events (tenant_id, owner_subject, occurred_at DESC);

CREATE INDEX pds_ledger_events_resource_idx
    ON kiva.pds_ledger_events (tenant_id, resource_kind, resource_id);
