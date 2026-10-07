-- Add PDS metadata registry for secret/confidential-compute provider bindings.

CREATE TABLE kiva.pds_provider_bindings (
    tenant_id text NOT NULL,
    binding_id uuid NOT NULL,
    owner_subject text NOT NULL,
    mechanism text NOT NULL,
    provider_kind text NOT NULL,
    capability text NOT NULL,
    profile text NOT NULL,
    provider_id_digest text NOT NULL,
    artifact_digest text NOT NULL,
    parameter_set_digest text,
    review_evidence_digest text,
    production_claim_requested boolean NOT NULL,
    binding_digest text NOT NULL,
    evidence_gate text NOT NULL,
    release_gate text NOT NULL,
    runtime_class text NOT NULL,
    production_ready boolean NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, binding_id),
    CHECK (tenant_id <> ''),
    CHECK (owner_subject <> ''),
    CHECK (
        mechanism IN (
            'additive-n-of-n',
            'private-set-intersection',
            'private-set-intersection-oprf-dh',
            'shamir-threshold',
            'threshold-frost-signing',
            'trusted-execution-environment',
            'trusted-execution-environment-aws-nitro',
            'fhe-sum-feasibility',
            'fhe-sum-runtime'
        )
    ),
    CHECK (
        provider_kind IN (
            'reference-primitive',
            'secret-custody',
            'trusted-execution-environment',
            'fully-homomorphic-encryption',
            'private-set-intersection',
            'threshold-cryptography'
        )
    ),
    CHECK (
        capability IN (
            'secret-custody',
            'tee-attestation',
            'fhe-evaluation',
            'psi-evaluation',
            'threshold-signing',
            'reference-fixtures'
        )
    ),
    CHECK (
        profile IN (
            'kiva-reference-primitives-v1',
            'kiva-secret-compute-provider-contract-v1'
        )
    ),
    CHECK (provider_id_digest ~ '^[0-9a-f]{64}$'),
    CHECK (artifact_digest ~ '^[0-9a-f]{64}$'),
    CHECK (parameter_set_digest IS NULL OR parameter_set_digest ~ '^[0-9a-f]{64}$'),
    CHECK (review_evidence_digest IS NULL OR review_evidence_digest ~ '^[0-9a-f]{64}$'),
    CHECK (binding_digest ~ '^[0-9a-f]{64}$'),
    CHECK (
        evidence_gate IN (
            'reference-fixture',
            'external-non-production',
            'external-reviewed'
        )
    ),
    CHECK (release_gate IN ('fixture-only', 'non-production', 'production')),
    CHECK (
        runtime_class IN (
            'reference',
            'external-non-production',
            'external-reviewed-production'
        )
    )
);

CREATE INDEX pds_provider_bindings_owner_idx
    ON kiva.pds_provider_bindings (tenant_id, owner_subject, created_at ASC);

CREATE INDEX pds_provider_bindings_runtime_class_idx
    ON kiva.pds_provider_bindings (tenant_id, runtime_class, production_ready);
