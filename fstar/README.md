# F* Verification Boundary

F* is reserved for narrow, stable, security-critical Kiva components where
proof and extraction cost is justified.

Current artifacts:

- `Kiva.Confidential.PrivacyBudget.fst` models the minimal differential
  privacy budget spend boundary. It proves that spend admission requires a
  valid ledger and non-zero request, preserves the configured total budget,
  monotonically increases spent budget, and stays within total budget.
- `Kiva.Confidential.PrivacyNoise.fst` models the minimal differential
  privacy noise evidence boundary. It proves that valid noise evidence requires
  an applied-noise flag and binds the released value to the raw aggregate plus
  the recorded noise sample.
- `Kiva.Confidential.ReleaseGate.fst` models the minimal two-authority release
  gate used by the confidential-computation roadmap. It proves that release
  requires a computed aggregate plus left and right approvals, that duplicate
  left approval alone is insufficient, and that no release is possible without
  a computed aggregate.
- `Kiva.Confidential.PrivacyGate.fst` models the minimal differential-privacy
  release gate. It proves that release requires a computed aggregate,
  population admission, epsilon-budget admission, and an applied noise
  mechanism.
- `Kiva.Confidential.TranscriptGate.fst` models the minimal pre-computation
  transcript gate for future confidential workflows. It proves that aggregate
  computation is admitted only when commitment slots, verification keys,
  persisted replay records, and authenticated shares are all complete.
- `Kiva.Confidential.ShamirGate.fst` models the minimal Shamir reconstruction
  gate. It proves that reconstruction requires participant-count admission,
  threshold at least two, threshold within participants, enough shares, and
  unique participant indexes, and verified share openings. It also proves no
  reconstruction through the modeled gate with duplicate participant indexes,
  insufficient shares, or unverified openings.

Near-term non-targets:

- xAPI HTTP routing and ordinary JSON CRUD behavior.
- PostgreSQL persistence code.
- BI query orchestration.
- Fast-changing policy glue.
- Concrete finite-field arithmetic, Shamir polynomial security, hash
  computation, distributed key generation, proactive share refresh, and
  malicious dealer robustness.

Candidate future targets:

- canonical Statement hashing and signature preconditions;
- deeper transcript validators for confidential workflows;
- threshold-release or PSI helper logic with explicit cryptographic
  assumptions;
- EverParse/F* boundaries if Kiva introduces security-sensitive binary or
  canonical encodings.

Run the pinned F* checks with:

```sh
nix develop .#verification -c scripts/ci/run_fstar_verification.sh
```
