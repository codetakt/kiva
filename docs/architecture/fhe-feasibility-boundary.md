# FHE Sum Feasibility Boundary

Kiva's CC6 boundary is a feasibility profile for one exact aggregate operation:
an integer sum over a small number of bounded participant values. It is not a
production fully homomorphic encryption implementation.

The profile is `kiva-fhe-sum-bfv-feasibility-v1`. It uses a BFV-style parameter
shape so Kiva can model API, policy, benchmark, and proof boundaries before
selecting a production FHE library.

## Implemented Boundary

- operation: exact integer `sum`;
- participant rule: n-of-n, with two or more participants;
- maximum participants: `16`;
- maximum participant plaintext: `65_000`;
- plaintext modulus: `1_048_583`, so the maximum admitted sum `1_040_000`
  stays below the modulus and cannot wrap;
- polynomial modulus degree: `4096`;
- ciphertext modulus budget: `109` bits;
- documented security level: `128` bits;
- deterministic ciphertext and evaluation digests for reproducible fixtures;
- release gate requiring completed evaluation, policy approval, governed release
  treatment, and a clear plaintext/ciphertext boundary.

The Rust implementation lives in `crates/kiva-confidential/src/lib.rs` and
exposes `FheFeasibilityParameterSet`, `FheFeasibilityPlan`,
`encrypt_fhe_feasibility_plaintext_fixture`, `evaluate_fhe_feasibility_sum`,
`validate_fhe_feasibility_release`, and
`release_fhe_feasibility_sum_fixture`.

## Plaintext and Ciphertext Boundary

The deterministic fixture carries a private plaintext value inside the Rust
test structure so the repository can test homomorphic-style evaluation and
release policy without linking a production FHE runtime. Public getters expose
job identity, parameter metadata, ciphertext digest, slot count, and noise
budget only. The fixture type is deliberately not a persisted server contract.

Production persistence remains out of scope until Kiva selects a real FHE
library and ciphertext serialization format. The current repository does not
expose a dedicated FHE PDS data-product source kind, HTTP contract, or
PostgreSQL result store. Future PDS and BI integrations must treat any released
result as governed aggregate metadata, not as raw xAPI data or reusable
ciphertext material.

## Evidence

- deterministic vectors:
  `spec/confidential/fhe-sum-feasibility-vectors.yaml`;
- assurance profile:
  `spec/confidential/fhe-sum-feasibility-assurance.yaml`;
- job contract:
  `spec/confidential/fhe-sum-feasibility-job-contract.yaml`;
- benchmark fixtures:
  `spec/confidential/fhe-sum-feasibility-benchmarks.yaml`;
- CI validator:
  `scripts/ci/validate_fhe_feasibility_assurance.py`;
- Kani harnesses for strict parameter admission and release-gate fail-closed
  behavior;
- Tamarin model:
  `proofs/tamarin/kiva_fhe_feasibility_boundary.spthy`.

## Non-Claims

Kiva does not yet claim production BFV, BGV, CKKS, bootstrapping, FHE key
management, ciphertext wire-format stability, malicious participant robustness,
side-channel resistance, or regulatory privacy guarantees from this profile.
The CC6 claim is limited to feasibility modeling for a governed small-sum
workflow.
