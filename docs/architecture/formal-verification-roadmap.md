# Formal Verification Roadmap

## Scope

The verification plan assigns pure Rust invariants to Kani, symbolic protocol
and role-separation models to Tamarin, and selected stable computation gates
to F*. Run-specific proof and coverage assessments are kept locally in `target/`.

## Tool Responsibilities

### Kani

Kani is the first implementation-level verification target because current
Kiva code already contains small, pure, finite-state Rust functions.

Kani targets:

- `kiva-http-core`
  - xAPI version admission accepts only the supported version forms.
  - Bearer header classification rejects unsupported schemes and empty tokens
    through a byte-offset classifier that Kani can verify.
  - ETag matching and document precondition decisions fail closed.
  - Document PUT and DELETE decisions preserve fail-closed precondition
    behavior.
  - JSON document merge accepts only object patches and empty/object merge
    bases.
  - Immutable Statement PUT decisions preserve idempotency and conflict rules.
  - Statement `GET` query admission rejects unknown parameters, mutually
    exclusive single-Statement identifiers, single-Statement requests mixed
    with collection filters, invalid `format` values, invalid boolean values,
    invalid RFC3339 timestamps, and invalid `limit` values.
- `kiva-policy::decide`
  - The finite baseline allow-set is exactly the policy table.
  - Raw learning record reads are custodian-only.
  - Confidential computation result release requires policy authority.
  - The stricter PDS delegation model requires active, unrevoked consent and
    matching delegated scope for custodian raw reads and analyst aggregate
    reads.
  - PDS data-product release admission is source-kind-specific for completed
    threshold releases, persisted differential privacy releases, and persisted
    Shamir threshold reconstruction metadata.
- `kiva-confidential`
  - participant count must be at least two.
  - release threshold must not exceed participant count.
  - accepted policies preserve those invariants.
  - additive secure aggregation keeps field addition inside the configured
    field.
  - additive secure aggregation accepts only n-of-n release policies.
  - additive share transport artifacts reject reserved empty identifiers.
  - authenticated additive-share envelopes must match their commitment slot
    before reconstruction.
  - additive-share verification-key sets must provide exactly one registered
    key for each commitment slot.
  - replay-windowed additive-share acceptance requires the envelope sequence to
    fall inside the configured window.
  - share-signing-key rotation requires matching owner, matching participant,
    an active old key, and an absent new key.
  - share-signing-key rotation revokes the old key and activates the new key.
  - share-signing-key revocation is idempotent and revoked keys fail closed for
    new submissions.
  - differential privacy release policies reject single-subject publication
    thresholds.
  - differential privacy budget ledgers reject over-spent persisted state.
  - differential privacy budget spending fails closed for zero or over-remaining
    requests, preserves the configured total budget, and increases spent budget
    monotonically.
  - differential privacy basic sequential composition summaries fail closed
    when release-attributed epsilon spend exceeds the persisted budget ledger
    spend.
  - differential privacy advanced-composition audit bounds reject invalid
    delta values.
  - differential privacy budget-policy lifecycle transitions are monotonic.
  - differential privacy integer-noise application fails closed on signed
    overflow and records noised values that match checked arithmetic.
  - differential privacy deterministic sampler entry points reject unsupported
    numeric parameter ranges and remain reproducible for fixed uniform inputs
    under ordinary Rust tests; CI profile validation adds a deterministic
    distribution smoke check. Kani does not claim distributional privacy for
    the sampler.
  - differential privacy aggregate release fails closed unless it is bound to a
    computed aggregate, satisfies the population threshold, uses a non-zero
    epsilon budget within policy, and has an applied noise mechanism.
  - signed additive-share envelope verification is covered by ordinary Rust
    tests; Kani does not attempt to prove Ed25519 cryptography.
  - confidential mechanism selection enforces additive n-of-n and Shamir
    threshold-specific policy rules, and now also enforces the current TEE
    evidence gate, FHE feasibility profile, and FHE runtime profile as n-of-n.
  - Shamir reconstruction fails closed before interpolation when shares are
    insufficient or participant indexes are duplicated.
  - Shamir complaint resolution sustains invalid openings and rejects
    complaints against verified openings.
  - Shamir interpolation arithmetic and SHA-256 share-opening commitments are
    covered by ordinary Rust vectors; Kani does not prove full finite-field
    arithmetic or hash-function equivalence.
  - portable TEE attestation admission fails closed unless policy approval,
    supported format, job binding, measurement match, image digest match,
    policy binding match, freshness, minimum sequence, digest match, and
    replay checks all pass.
  - TEE secret provisioning and result release fail closed without accepted
    attestation, policy approval, and matching release job identity.
  - FHE feasibility parameter admission accepts only the strict
    `kiva-fhe-sum-bfv-feasibility-v1` profile.
  - FHE feasibility release fails closed unless evaluation is complete, policy
    approval is present, the output has governed release treatment, and the
    plaintext boundary is clear.
  - FHE runtime ciphertext transcript admission and release fail closed unless
    runtime binding, external review, participant-count matching, active
    participants, key-custody evidence, parameter-set binding, digest matching,
    replay checks, active key epoch, policy approval, and governed release
    treatment are present as appropriate.
  - CC16 secret/confidential-compute provider bindings reject reference-provider
    production claims and fail closed when provider capability does not match the
    selected confidential mechanism family. Kani does not prove external
    provider security, external review correctness, full PDS provider registry
    HTTP/PostgreSQL behavior, or provider-binding persistence semantics.

Run:

```sh
nix develop .#verification -c scripts/ci/run_kani_verification.sh
```

`nix flake check` runs the same Kani regression harness set as
`checks.kani-boundary` and stores the generated Kani report in the check output.
The verification shell packages Kani 0.66.0 with its own nightly toolchain. The
Kiva application toolchain remains the stable toolchain pinned in
`rust-toolchain.toml`. The Kani runner executes explicit harnesses with
`cadical`, `--unwind 16`, and a per-harness timeout; it intentionally avoids
crate-wide `cargo kani -p ...` runs because those enter standard-library
string/slice internals outside Kiva's intended pure proof surface.

Formal evidence bundles collect the same Kani report under
`reports/kani.json`:

```sh
nix develop .#verification -c \
  scripts/ci/generate_formal_verification_evidence.sh --skip-fstar --skip-tamarin
```

For a fixed Nix-store artifact that is cheap enough to build on demand, Kiva
also exposes a smoke evidence bundle:

```sh
nix build .#formal-verification-evidence-smoke
```

That package runs F*, Tamarin, and the Kani smoke harness set, validates the
bundle manifest, reports, artifact lists, and checksums, then places the bundle
under `result/`. Full Kani regression evidence should still be generated with
`generate_formal_verification_evidence.sh` or checked through
`nix flake check`.

Near-term PDS Kani candidates:

- Extract a bounded helper for PDS subject identifiers if Kiva adds stricter
  subject canonicalization beyond the current authenticated-principal string.
  The present `PdsSubjectId` is a typed application boundary, not a parser or
  validator.
- Extract a pure, bounded PDS projection core if admitted-event to durable-state
  equivalence needs proof beyond the current Rust regression tests in
  `surfaces/pds/model/projection.rs`. The proof surface should avoid server
  `String`, `Vec`, `Uuid`, and timestamp-heavy adapters where possible.
- Keep `PdsSubjectProjectionPort`, `PdsSubjectIdentityMapper`, and
  `PdsApplicationPorts` under ordinary Rust and PostgreSQL tests unless a small
  pure subject-snapshot decision core is introduced. The current read path is
  asynchronous repository composition, which is outside Kani's intended proof
  surface.

### Tamarin

Tamarin is the right tool for Kiva's protocol and separation-of-authority
questions. It should model the shape of authorization and confidential
workflows, not Rust implementation behavior.

Current models:

- `proofs/tamarin/kiva_authorization_boundary.spthy`
  - raw record reads require an issued raw-read token;
  - raw record reads trace back to custodian registration;
  - aggregate-read tokens do not imply raw-record reads in the initial model.
- `proofs/tamarin/kiva_oauth_oidc_boundary.spthy`
  - LRS token acceptance requires prior token issue;
  - accepted tokens trace to a trusted issuer and registered LRS audience;
  - token issue and acceptance require active, unrevoked delegation;
  - accepted tokens require active introspection;
  - accepted tokens require a registered client binding and token-bound key;
  - accepted tokens are single-use in the model, covering replay resistance at
    this boundary;
  - access tokens issued through refresh-token exchange require a prior refresh
    token;
  - raw-read acceptance requires raw-read delegation;
  - grants and refresh tokens are modeled as linear resources so proof search
    remains finite.
- `proofs/tamarin/kiva_confidential_workflow_boundary.spthy`
  - raw record reads require data-custodian registration;
  - analysts cannot read raw learning records;
  - computation-participant admission requires participant registration;
  - aggregate computation requires admitted custodian input and computation
    participation;
  - aggregate release requires policy-authority registration and prior
    computation;
  - analyst aggregate reads require prior release and analyst registration.
  - a representative two-authority threshold release path assigns distinct
    policy authorities, requires both approvals after computation, emits an
    audit event at release, and permits analyst reads only after threshold
    release.
- `proofs/tamarin/kiva_additive_share_transport_boundary.spthy`
  - additive-share acceptance requires participant registration;
  - accepted shares require a required transcript slot;
  - accepted shares require an active registered signing key;
  - accepted shares require a subject-bound signing key and subject-bound
    submission event;
  - accepted shares require a prior participant commitment;
  - accepted shares require a signature-verification event;
  - accepted shares require a persisted replay-record event;
  - submitted-share audit metadata requires a prior subject-bound submission;
  - submitted-share values and signatures are not published through the
    representative audit path;
  - revoked signing keys cannot be accepted after revocation;
  - each transcript participant has at most one accepted share in the
    representative model;
  - additive aggregate computation requires accepted shares from both
    transcript participants and their persisted replay records.
- `proofs/tamarin/kiva_additive_collusion_boundary.spthy`
  - additive workflows use distinct two-party participants;
  - aggregate computation requires workflow scheduling and both participant
    shares;
  - coordinators cannot submit additive shares;
  - analysts and auditors cannot release additive aggregates;
  - release requires policy-authority registration and prior aggregate
    computation;
  - accepting a share does not bypass release before analyst aggregate reads.
- `proofs/tamarin/kiva_pds_consent_boundary.spthy`
  - delegated PDS reads require a matching consent grant;
  - delegated PDS reads require unrevoked consent;
  - aggregate reads require aggregate-read scope;
  - raw-record reads require raw-read consent or explicit custodial authority;
  - analysts cannot read raw records through the modeled PDS boundary;
  - raw-read and aggregate-read scopes do not substitute for each other.
- `proofs/tamarin/kiva_public_sector_separation_boundary.spthy`
  - public-sector workflows assign separated custodian, operator, analyst,
    policy-authority, and auditor duties;
  - operators and analysts cannot admit raw records;
  - operators cannot approve aggregate releases;
  - aggregate release approval requires policy-authority registration;
  - aggregate release requires prior approval and emits an audit event;
  - aggregate reads require prior release;
  - auditor evidence access does not grant raw-data admission.
- `proofs/tamarin/kiva_threshold_release_failure_boundary.spthy`
  - persisted threshold-release requests require a computed aggregate;
  - rejected threshold-release requests without a computed aggregate are not
    persisted under the same request identity;
  - non-authority threshold approvals are rejected rather than accepted;
  - duplicate left-authority approval does not complete the threshold release
    without a later right-authority approval;
  - release requires both left and right approvals.
- `proofs/tamarin/kiva_differential_privacy_budget_boundary.spthy`
  - differential privacy budget registration traces to a registered owner;
  - accepted budget spends trace to the registered subject-owned budget;
  - foreign-subject spend rejection is not accepted as a spend;
  - differential privacy aggregate release requires a prior persisted budget
    spend and an applied-noise event.
- `proofs/tamarin/kiva_pds_data_product_release_boundary.spthy`
  - registered PDS data products trace to an available release for the same
    owner and source kind;
  - threshold data products require completed threshold release;
  - differential privacy data products require persisted DP release metadata;
  - Shamir threshold reconstruction data products require persisted
    reconstruction metadata;
  - source kinds do not substitute for threshold-release admission.
- `proofs/tamarin/kiva_shamir_threshold_boundary.spthy`
  - representative Shamir reconstruction requires a configured transcript;
  - reconstruction requires a distinct threshold share set;
  - reconstruction requires verified share openings;
  - duplicate-share rejection is not accepted as a share;
  - sustained complaints and rejected openings are not accepted as shares for
    reconstruction;
  - insufficient-share rejection does not become reconstruction without a later
    distinct threshold share set.
- `proofs/tamarin/kiva_psi_cohort_boundary.spthy`
  - representative PSI release requires a configured job;
  - intersection computation requires both participant submissions for the
    released digest;
  - non-matching left-only and right-only digests cannot be released.
- `proofs/tamarin/kiva_tee_attestation_boundary.spthy`
  - accepted attestation requires a configured and approved TEE policy;
  - accepted attestation records measurement, image digest, and policy binding
    matches;
  - secret provisioning requires accepted attestation;
  - result release requires accepted attestation, secret provisioning, and job
    binding;
  - stale, replayed, and rollback evidence rejection does not become accepted
    or released evidence.
- `proofs/tamarin/kiva_fhe_feasibility_boundary.spthy`
  - accepted ciphertext fixtures require a configured FHE feasibility job;
  - sum evaluation requires left and right ciphertext submissions;
  - release requires a configured job, prior evaluation, and policy approval;
  - released outputs are treated as governed release metadata;
  - public plaintext publication is absent from the representative boundary.
- `proofs/tamarin/kiva_fhe_runtime_boundary.spthy`
  - accepted ciphertext transcripts require a configured reviewed runtime;
  - key-custody evidence, parameter-set binding, digest matching, and replay
    evidence are recorded before acceptance;
  - accepted evaluations require an accepted ciphertext transcript and
    evaluation proof digest;
  - release requires accepted ciphertext metadata, accepted evaluation, active
    key epoch, policy approval, and governed release treatment.

Run:

```sh
nix develop .#verification -c scripts/ci/run_tamarin_verification.sh
```

`nix flake check` also runs the Tamarin boundary models as
`checks.tamarin-boundary`. The runner checks each lemma individually so the
representative workflow model remains tractable while still failing the check on
any unproved lemma.

Formal evidence bundles collect the same Tamarin report under
`reports/tamarin.json`.

Next Tamarin targets:

- production-mechanism assumptions for the next confidential-computation
  backend, such as malicious-secure PSI, concrete vendor TEE quote
  verification, production FHE, or a production-reviewed threshold
  cryptography runtime.
- PDS subject-snapshot release assumptions if inventory/export evolves from a
  subject-owned introspection feature into a delegated release flow. The model
  should require authenticated subject binding, subject-indexed xAPI metadata,
  same-subject PDS metadata, and explicit delegated-consent facts before a
  third-party snapshot release.
- strengthening the representative threshold release model after production
  mechanism selection. The current runtime API maps the modeled two-authority
  approval boundary for an already-computed aggregate and requires a persisted
  computed aggregate result before a threshold-release request; it does not yet
  model or implement raw threshold-secret material. CC13 now adds a separate
  review-gated FROST threshold-signing boundary model for metadata admission
  and release gates.

### F*

F* should not be used for the xAPI REST interface itself. The xAPI boundary is
better captured by OpenAPI, conformance fixtures, and HTTP integration tests.

Kiva currently has five minimal F* boundary artifacts:

- `fstar/Kiva.Confidential.PrivacyBudget.fst`
  - spend admission requires a valid budget ledger;
  - spend admission requires a non-zero epsilon request;
  - accepted spends preserve the configured total budget;
  - accepted spends increase spent budget monotonically;
  - accepted spends keep spent budget within total budget.
- `fstar/Kiva.Confidential.PrivacyNoise.fst`
  - valid noise evidence requires an applied-noise flag;
  - valid noise evidence binds the released value to the raw aggregate plus the
    recorded noise sample;
  - missing noise evidence is invalid.
- `fstar/Kiva.Confidential.PrivacyGate.fst`
  - release requires a persisted computed aggregate flag;
  - release requires population-threshold admission;
  - release requires epsilon-budget admission;
  - release requires an applied noise mechanism.
- `fstar/Kiva.Confidential.ReleaseGate.fst`
  - release requires a persisted computed aggregate flag;
  - release requires left-authority approval;
  - release requires right-authority approval;
  - duplicate left approval alone cannot satisfy the two-authority release
    gate;
  - no release is possible without a computed aggregate.
- `fstar/Kiva.Confidential.TranscriptGate.fst`
  - computation requires complete commitment slots;
  - computation requires complete verification keys;
  - computation requires persisted replay records;
  - computation requires authenticated shares;
  - no computation is possible without persisted replay records.

F* becomes useful when Kiva introduces additional narrow security-critical cores
whose contracts are stable enough to justify extraction or proof maintenance:

- Statement canonicalization and signed Statement preconditions;
- transcript validators for confidential workflows;
- PDS admitted-event to lifecycle-ledger projection if that logic is extracted
  into a server-independent, finite algebraic model;
- PSI, threshold-release, or commitment helpers;
- EverParse boundaries for security-sensitive encodings.

Run:

```sh
nix develop .#verification -c scripts/ci/run_fstar_verification.sh
```

F* boundary verification is also part of `nix flake check` as
`checks.fstar-boundary`.

Formal evidence bundles collect the same F* report under `reports/fstar.json`.

Generate a full bundle with:

```sh
nix develop .#verification -c \
  scripts/ci/generate_formal_verification_evidence.sh --require-clean-git
```

The bundle contains schema-validated Kani, Tamarin, and F* reports, a
schema-validated manifest, logs, and checksums. The bundle validator also
checks report-to-manifest totals, artifact lists, and checksum coverage. It is
internal assurance evidence only; it is not an IEEE certification record and
does not establish complete formal verification of Kiva.

## Next Implementation Steps

1. Use the CC2 DP release profile as the baseline for deployment
   privacy review. Future work should focus on concrete BI-query sensitivity,
   cross-system composition policy, and external privacy review rather than
   changing the core release gate without a deployment need.
