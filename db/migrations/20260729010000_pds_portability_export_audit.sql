-- Extend the append-only PDS ledger with learner portability export audit events.

ALTER TABLE kiva.pds_ledger_events
    DROP CONSTRAINT pds_ledger_events_event_kind_check;

ALTER TABLE kiva.pds_ledger_events
    ADD CONSTRAINT pds_ledger_events_event_kind_check CHECK (
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
    );

ALTER TABLE kiva.pds_ledger_events
    DROP CONSTRAINT pds_ledger_events_resource_kind_check;

ALTER TABLE kiva.pds_ledger_events
    ADD CONSTRAINT pds_ledger_events_resource_kind_check CHECK (
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
    );
