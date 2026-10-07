-- Enforce that PDS data products reference an existing confidential release
-- for their source kind. Subject ownership and "released" status remain
-- application policy checks because they depend on request principal context.

ALTER TABLE kiva.pds_data_products
    ADD COLUMN threshold_release_id bytea GENERATED ALWAYS AS (
        CASE
            WHEN source_kind = 'threshold-release' THEN release_id
            ELSE NULL::bytea
        END
    ) STORED;

ALTER TABLE kiva.pds_data_products
    ADD COLUMN differential_privacy_release_id bytea GENERATED ALWAYS AS (
        CASE
            WHEN source_kind = 'differential-privacy-release' THEN release_id
            ELSE NULL::bytea
        END
    ) STORED;

ALTER TABLE kiva.pds_data_products
    ADD CONSTRAINT pds_data_products_threshold_release_fk
    FOREIGN KEY (tenant_id, threshold_release_id)
    REFERENCES kiva.confidential_threshold_release_requests (tenant_id, release_id)
    NOT VALID;

ALTER TABLE kiva.pds_data_products
    ADD CONSTRAINT pds_data_products_differential_privacy_release_fk
    FOREIGN KEY (tenant_id, differential_privacy_release_id)
    REFERENCES kiva.confidential_differential_privacy_releases (tenant_id, release_id)
    NOT VALID;

ALTER TABLE kiva.pds_data_products
    VALIDATE CONSTRAINT pds_data_products_threshold_release_fk;

ALTER TABLE kiva.pds_data_products
    VALIDATE CONSTRAINT pds_data_products_differential_privacy_release_fk;

CREATE INDEX pds_data_products_threshold_release_id_idx
    ON kiva.pds_data_products (tenant_id, threshold_release_id)
    WHERE threshold_release_id IS NOT NULL;

CREATE INDEX pds_data_products_differential_privacy_release_id_idx
    ON kiva.pds_data_products (tenant_id, differential_privacy_release_id)
    WHERE differential_privacy_release_id IS NOT NULL;
