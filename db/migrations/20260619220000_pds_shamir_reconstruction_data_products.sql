-- Allow PDS data products to reference persisted Shamir reconstruction metadata.
-- The reconstructed value remains outside PDS data-product responses.

ALTER TABLE kiva.pds_data_products
    DROP CONSTRAINT IF EXISTS pds_data_products_source_kind_check;

ALTER TABLE kiva.pds_data_products
    ADD CONSTRAINT pds_data_products_source_kind_check
    CHECK (
        source_kind IN (
            'threshold-release',
            'differential-privacy-release',
            'shamir-threshold-reconstruction'
        )
    );

ALTER TABLE kiva.pds_data_products
    ADD COLUMN shamir_threshold_reconstruction_id bytea GENERATED ALWAYS AS (
        CASE
            WHEN source_kind = 'shamir-threshold-reconstruction' THEN release_id
            ELSE NULL::bytea
        END
    ) STORED;

ALTER TABLE kiva.pds_data_products
    ADD CONSTRAINT pds_data_products_shamir_threshold_reconstruction_fk
    FOREIGN KEY (tenant_id, shamir_threshold_reconstruction_id)
    REFERENCES kiva.confidential_shamir_threshold_reconstructions
        (tenant_id, reconstruction_id)
    NOT VALID;

ALTER TABLE kiva.pds_data_products
    VALIDATE CONSTRAINT pds_data_products_shamir_threshold_reconstruction_fk;

CREATE INDEX pds_data_products_shamir_threshold_reconstruction_id_idx
    ON kiva.pds_data_products (tenant_id, shamir_threshold_reconstruction_id)
    WHERE shamir_threshold_reconstruction_id IS NOT NULL;
