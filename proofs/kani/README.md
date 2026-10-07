# Kani Harnesses

Kani harnesses currently live next to the Rust code they verify under
`#[cfg(kani)]` modules. This keeps each invariant close to the implementation
it constrains while leaving ordinary builds unchanged.

Run the bounded Rust verification checks with:

```sh
nix develop .#verification -c scripts/ci/run_kani_verification.sh
```

Current targets:

- `kiva-http-core`: bounded checks for xAPI version admission, Bearer header
  classification, ETag matching, ETag preconditions, document PUT decisions,
  document DELETE decisions, immutable Statement PUT decisions, and the pure
  Statement `GET` query-admission boundary for single-Statement versus
  collection-query separation and scalar query validation. It also checks the
  structural contract for JSON document merge without depending on a JSON value
  implementation. Kani checks the Bearer classifier at the byte-offset level;
  the public `&str` token-slicing adapter remains covered by ordinary Rust
  tests.
- `kiva-policy`: exhaustive checks over the finite baseline policy decision
  table plus the stricter PDS consent/delegation model for active consent,
  scope matching, analyst raw-record exclusion, and PDS data-product release
  admission for threshold-release, differential-privacy-release, Shamir
  threshold-reconstruction sources. PSI release evidence is checked in
  `kiva-confidential` and does not add a PDS data-product source kind in the
  current implementation.
  It also checks that optional PDS data-product references are scoped to
  aggregate reads over existing products owned by the delegated data subject,
  and that public-sector PDS governance profile admission requires declared
  partitioning, separated duty subjects, emergency-access policy metadata, and
  runbook references. The OAuth/OIDC resource-server harness checks that Kiva
  scope labels map exactly to resource-server operations and that scopes are
  not treated as hierarchical. The lifecycle-case harness checks the pure M12
  correction/dispute/tombstone status state machine: new cases start `open`,
  `open` cannot jump directly to a terminal state, `under-review` can reach
  terminal statuses, and terminal statuses cannot be reopened.
- `kiva-confidential`: bounded checks for release-policy participant and
  threshold validation, additive field closure, and the n-of-n constraint for
  the first additive secure-aggregate mechanism. It also checks that
  authenticated additive-share envelope acceptance matches the expected
  commitment slot, that every commitment slot has exactly one registered
  verification key, and that replay-windowed acceptance requires an in-window
  envelope sequence. The threshold-release transition harnesses check distinct
  authorities, unambiguous authority classification, duplicate approval
  idempotence, two-authority completion, computed-aggregate binding, and final
  release authorization. The signing-key lifecycle harnesses check rotation
  preconditions, old-key revocation, new-key activation, and idempotent
  fail-closed revocation. The differential privacy release-gate harnesses check
  that single-subject publication thresholds are rejected and that release
  requires a computed aggregate, sufficient population, non-zero and
  within-policy epsilon budget, and applied noise. The differential privacy
  budget-ledger harnesses check over-spent ledger rejection, zero-spend
  rejection, remaining-budget enforcement, total-budget preservation, and
  monotonic spent-budget growth. The differential privacy composition harness
  checks that release-attributed epsilon spend cannot exceed the persisted
  budget ledger spend. The advanced-composition harness checks fail-closed
  delta admission for the DRV-style pure-DP audit bound, and the budget-policy
  lifecycle harness checks monotonic draft/active/superseded/retired
  transitions. The differential privacy noise harness checks that integer
  noise application either fails closed on signed overflow or records a noised
  value equal to the raw aggregate plus the sampled noise. The server-side DP
  sampler has ordinary Rust tests and CI profile validation for deterministic
  behavior, numeric parameter rejection, and a distribution smoke check; Kani
  does not prove the sampler distribution or privacy loss bound.
  The confidential mechanism abstraction harness checks that additive plans
  and PSI plans require n-of-n release while Shamir plans require threshold at
  least two. The confidential job lifecycle harnesses check that the CC9
  durable job transition table is exact and that the admission gate fails
  closed on too few participants, missing policy binding, expired deadlines,
  quota exhaustion, scheduler backpressure, and retry-budget exhaustion.
  The confidential participant lifecycle harnesses check that the CC10
  invited/active/suspended/frozen/removed transition table is exact and that
  only active participants can be admitted for new jobs or key registration.
  The CC16 provider-contract harnesses check that the fixed reference
  primitive provider cannot be used for production claims and that provider
  capabilities fail closed when they do not match the selected confidential
  mechanism family.
  The PSI cohort harness checks that the bounded intersection result exposes
  only digests present in every participant set and excludes participant-only
  non-matches. The CC11 DH-OPRF PSI harnesses check that the review-gated
  opaque-element boundary exposes only elements common to all participant
  transcripts and that release fails closed unless intersection computation,
  active participants, external cryptographic review completion, policy
  approval, and governed release treatment are all present.
  The TEE attestation harnesses check that portable evidence admission fails
  closed unless policy approval, supported format, job binding, measurement
  match, image digest match, policy binding match, freshness, minimum
  sequence, digest match, and replay checks all pass, and that secret
  provisioning and result release fail closed without accepted attestation,
  policy approval, and matching release job identity.
  The AWS Nitro TEE harnesses check that CC12 admission fails closed unless
  policy approval, supported Nitro format, job binding, non-debug PCRs, PCR
  policy, image digest, policy binding, nonce, freshness, certificate-chain
  validation, COSE-signature validation, trust-anchor binding, verifier
  artifact binding, external security review, verifier key epoch, sequence,
  digest, and replay checks all pass. They also check that key and result
  release fail closed without accepted attestation, policy approval, external
  review, current verifier key epoch, prior secret provisioning, and matching
  release job identity.
  The CC13 FROST threshold-signing harnesses check that key-ceremony admission
  fails closed unless runtime binding, external cryptographic review,
  participant-count matching, active participants, share-custody evidence,
  complaint resolution, absence of equivocation, and no replay all hold. They
  also check that threshold-signature release fails closed unless the key
  ceremony is complete, the signature transcript is computed, external review
  is complete, the key epoch is active, policy approval exists, and key use is
  governed. Kani does not prove FROST group arithmetic, DKG soundness, or
  malicious-participant robustness.
  The FHE feasibility harnesses check that only the strict
  `kiva-fhe-sum-bfv-feasibility-v1` parameter profile is admitted and that the
  release gate fails closed unless evaluation, policy approval, governed
  release treatment, and plaintext-boundary clearance are all present.
  The CC14 FHE runtime harnesses check that ciphertext transcript admission
  fails closed unless runtime binding, external cryptographic review,
  participant-count matching, active participants, key-custody evidence,
  parameter-set binding, digest matching, and replay checks all pass, and that
  runtime release fails closed unless ciphertext metadata, evaluation,
  external review, active key epoch, policy approval, and governed
  release treatment are all present. Kani does not prove FHE arithmetic,
  parameter security, ciphertext serialization, decryption, or malicious
  participant robustness.
  The Shamir reconstruction harness checks fail-closed rejection for
  insufficient shares and duplicate participant indexes before interpolation.
  The Shamir complaint harness checks that invalid openings produce sustained
  complaints while verified openings reject complaints. Concrete Shamir
  interpolation values and SHA-256 share-opening commitments are covered by
  ordinary Rust vector tests, not by Kani.
  Ed25519 signature verification is covered by Rust tests, not Kani, because
  the cryptographic implementation is delegated to `ed25519-dalek`.

The verification shell packages a pinned Kani 0.66.0 toolchain because the
pinned nixpkgs input does not expose a usable `kani` package. The runner
executes an explicit harness list with a fixed solver, unwind bound, and
per-harness timeout. It does not run `cargo kani -p ...` crate-wide because
crate-wide discovery enters standard-library string and slice internals that
are outside the intended proof surface.
