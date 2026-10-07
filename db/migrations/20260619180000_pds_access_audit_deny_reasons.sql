-- Extend PDS access-audit denial reasons for data-product access gating.

DO $$
DECLARE
    existing_constraint text;
BEGIN
    SELECT conname
    INTO existing_constraint
    FROM pg_constraint
    WHERE conrelid = 'kiva.pds_access_audit_events'::regclass
      AND contype = 'c'
      AND pg_get_constraintdef(oid) LIKE '%deny_reason%'
      AND pg_get_constraintdef(oid) LIKE '%rawRecordBoundary%'
    LIMIT 1;

    IF existing_constraint IS NOT NULL THEN
        EXECUTE format(
            'ALTER TABLE kiva.pds_access_audit_events DROP CONSTRAINT %I',
            existing_constraint
        );
    END IF;
END $$;

ALTER TABLE kiva.pds_access_audit_events
    ADD CONSTRAINT pds_access_audit_events_deny_reason_check
    CHECK (
        deny_reason IS NULL
        OR deny_reason IN (
            'rawRecordBoundary',
            'releaseRequiresAuthority',
            'consentRequired',
            'consentRevoked',
            'delegationScopeMismatch',
            'dataProductUnavailable',
            'unsupportedRoleAction'
        )
    );
