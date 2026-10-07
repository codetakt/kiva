-- Add Shamir VSS-admission metadata without persisting shares or openings.

ALTER TABLE kiva.confidential_shamir_threshold_reconstructions
    ADD COLUMN IF NOT EXISTS verification_profile text NOT NULL
        DEFAULT 'kiva-shamir-hash-vss-v1',
    ADD COLUMN IF NOT EXISTS verified_share_count bigint NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS complaint_count bigint NOT NULL DEFAULT 0;

UPDATE kiva.confidential_shamir_threshold_reconstructions
SET verified_share_count = share_count
WHERE verified_share_count = 0;

ALTER TABLE kiva.confidential_shamir_threshold_reconstructions
    ADD CONSTRAINT confidential_shamir_vss_profile_check
        CHECK (verification_profile = 'kiva-shamir-hash-vss-v1'),
    ADD CONSTRAINT confidential_shamir_vss_counts_check
        CHECK (
            verified_share_count >= threshold
            AND verified_share_count <= share_count
            AND complaint_count >= 0
            AND complaint_count <= share_count
        );
