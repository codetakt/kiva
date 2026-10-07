# FHE Feasibility Runbook

This runbook is for CC6 feasibility checks only. It does not operate a
production FHE service.

## Admission Checklist

- Use profile `kiva-fhe-sum-bfv-feasibility-v1`.
- Admit only the `sum` operation.
- Require n-of-n participation.
- Reject any parameter set that differs from the profile values in
  `spec/confidential/fhe-sum-feasibility-assurance.yaml`.
- Reject participant plaintext values above `65_000`.
- Reject jobs where the maximum admitted sum can reach or exceed the plaintext
  modulus `1_048_583`.
- Keep plaintext inputs outside public API, audit, and PDS metadata surfaces.

## Evaluation Checklist

- Confirm one ciphertext fixture per participant.
- Confirm all ciphertext fixtures are bound to the same job identifier.
- Confirm every ciphertext fixture uses the exact admitted parameter set.
- Reject duplicate participant indexes.
- Reject any evaluated sum that would overflow or wrap the plaintext modulus.
- Record only evaluated ciphertext digest, operation, participant count, and
  noise-budget metadata for governance review.

## Release Checklist

- Require completed evaluation.
- Require policy-authority approval.
- Preserve governed release evidence for the feasibility result.
- Confirm no plaintext/ciphertext boundary violation was observed.
- Do not register FHE feasibility outputs as PDS data products until a
  dedicated PDS source kind, HTTP contract, and PostgreSQL persistence path are
  implemented.

## Validation Commands

```sh
cargo test -p kiva-confidential fhe -- --nocapture
python scripts/ci/validate_fhe_feasibility_assurance.py \
  --json-report target/fhe-feasibility-assurance-validation.json
nix develop .#verification -c scripts/ci/run_kani_verification.sh
nix develop .#verification -c scripts/ci/run_tamarin_verification.sh
```

## Escalation

Move beyond this runbook only after a production FHE library, ciphertext
serialization format, key lifecycle, parameter-selection policy, and external
cryptographic review are selected. Until then, public language must say
`FHE feasibility profile` rather than `production FHE support`.
