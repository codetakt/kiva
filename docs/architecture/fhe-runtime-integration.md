# FHE Runtime Integration Boundary

Kiva's CC14 boundary promotes the CC6 FHE feasibility work into a
review-gated runtime integration contract. Kiva still does not implement FHE
cryptography in process. A selected external runtime is responsible for key
generation, parameter security, encryption, ciphertext serialization,
homomorphic evaluation, decryption, and any cryptographic proof material.

Kiva admits metadata only:

- runtime artifact digest;
- parameter-set digest;
- external cryptographic review evidence digest;
- key epoch;
- one ciphertext digest per participant;
- per-participant key-custody evidence digest;
- ciphertext transcript digest;
- evaluated ciphertext digest;
- evaluation proof digest;
- evaluation digest.

The admitted profile is `kiva-fhe-sum-runtime-review-gated-v1`, scoped to
BFV-style exact integer sums. The mechanism identifier is `fhe-sum-runtime`.
The current repository defines and validates these records in
`crates/kiva-confidential` and carries deployment evidence through the generic
confidential job lifecycle. It does not add a dedicated FHE transcript HTTP API
or PostgreSQL FHE transcript table.

## Admission Rules

Ciphertext transcript admission fails closed unless all of the following are
true:

- the runtime artifact, parameter set, and review evidence are bound by digest;
- external cryptographic review is marked complete;
- the release policy is n-of-n;
- the ciphertext count exactly matches the participant count;
- all participants are active and submitted exactly one ciphertext record;
- every ciphertext is bound to the selected parameter-set digest;
- every ciphertext carries key-custody evidence;
- the canonical ciphertext transcript digest matches the supplied digest;
- no replay record conflicts with the candidate transcript.

Evaluation admission fails closed unless it is bound to an accepted ciphertext
transcript, carries evaluated ciphertext and evaluation-proof digests, matches
the canonical evaluation digest, and does not replay a prior evaluation.

Release fails closed unless an accepted ciphertext transcript and accepted
evaluation exist, external review is complete, the key epoch is active, policy
authority approved release, and the output has governed release treatment.
At the current repository boundary, "accepted" means accepted by the Rust
metadata gate and represented through generic confidential-job evidence; it does
not imply dedicated FHE transcript persistence or a dedicated FHE PDS
data-product source kind.

## Non-Claims

This boundary does not claim:

- an in-process FHE implementation;
- FHE key management by Kiva;
- a dedicated FHE transcript HTTP API or PostgreSQL FHE transcript table;
- a dedicated FHE PDS data-product source kind or PDS registration path;
- ciphertext payload persistence or wire-format standardization;
- runtime parameter security;
- malicious participant robustness;
- production FHE before external cryptographic review is complete and bound to
  the runtime artifact digest.

## Evidence

- `spec/confidential/fhe-runtime-assurance.yaml`
- `spec/confidential/fhe-runtime-job-contract.yaml`
- `spec/confidential/fhe-runtime-vectors.yaml`
- `scripts/ci/validate_fhe_runtime_assurance.py`
- `crates/kiva-confidential/src/lib.rs`
- `proofs/tamarin/kiva_fhe_runtime_boundary.spthy`
- `scripts/ci/run_kani_verification.sh`
