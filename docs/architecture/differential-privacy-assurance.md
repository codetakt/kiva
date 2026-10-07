# Differential Privacy Assurance

## Scope

The release profile specifies two-sided geometric discrete-Laplace-family noise
for aggregate outputs under an add/remove-one-subject adjacency model, charged
to a subject-owned epsilon budget ledger. It defines aggregate release,
budget accounting, metadata publication and sampler contracts.

## Mechanism

The current profile is recorded in
`spec/confidential/differential-privacy-assurance.yaml`.

- mechanism label: `discreteLaplace`
- distribution family: two-sided geometric
- sampler: difference of two independent geometric magnitudes
- adjacency model: `addRemoveOneSubject`
- sensitivity unit: aggregate L1 sensitivity
- accounting scope: `subjectBudget`
- epsilon and delta representation: integer microunits
- server sampling source: OS CSPRNG

For deterministic tests and evidence generation, the sampler exposes a pure
entry point that accepts two random `u64` words and returns the corresponding
integer noise sample. Runtime sampled releases use OS randomness and do not
persist or expose the sampled noise value in metadata responses.

## Composition

Kiva enforces the persisted budget ledger with basic sequential composition:
the sum of release epsilon spends cannot exceed the subject-owned budget.

For audit evidence, Kiva also computes a conservative heterogeneous pure-DP
advanced-composition upper bound with `delta = 1e-6`:

```text
sum(eps_i * (exp(eps_i) - 1)) +
sqrt(2 * ln(1 / delta') * sum(eps_i^2))
```

The advanced bound is reported for review. The enforced admission boundary
remains the integer budget ledger because it is deterministic, durable, and
fail-closed.

## Budget Lifecycle

Budget policy lifecycle states are monotonic:

- `draft` may remain `draft`, become `active`, or be `retired`;
- `active` may remain `active`, become `superseded`, or be `retired`;
- `superseded` may remain `superseded` or be `retired`;
- `retired` may only remain `retired`.

The pure lifecycle transition function is covered by ordinary Rust tests and a
Kani harness. Current HTTP budget ledgers remain intentionally small; policy
publication and deployment-specific budget migration can be layered on this
state machine without weakening release admission.

## Validation

CC2 evidence includes:

- Rust unit tests for budget spend, composition, lifecycle transitions, release
  gates, noise arithmetic, and deterministic sampler behavior;
- a deterministic sampler distribution smoke test against the theoretical
  zero probability and mean absolute noise for epsilon `1.0` and sensitivity
  `1`;
- Kani harnesses for budget, composition, advanced-composition delta, lifecycle
  transitions, noise arithmetic, and release-gate fail-closed behavior;
- F* boundary artifacts for budget, noise, and release gate invariants;
- Tamarin budget ownership, spend, and release boundary model;
- OpenAPI metadata fields for `adjacencyModel`,
  `advancedCompositionDeltaMicrounits`, and
  `advancedCompositionEpsilonMicrounits`;
- CI validation through `scripts/ci/validate_differential_privacy_assurance.py`.

## F* Boundary Decision

The current F* boundary remains focused on stable integer gates:

- budget spend admission and monotonicity;
- valid noise evidence requiring an applied-noise flag;
- release-gate admission requiring computed aggregate, population admission,
  epsilon admission, and applied noise.

The DRV-style advanced-composition audit bound uses floating-point arithmetic
to produce a conservative review value. It is covered by Rust tests, Kani delta
admission, OpenAPI projection, and CI profile validation, but it is not an F*
extraction target in CC2. If Kiva later needs a certified advanced-accounting
core, that should be introduced as a separate narrow F*/Everest boundary with
fixed-point arithmetic and explicit theorem statements.

## Non-Claims

CC2 does not prove privacy for arbitrary analytics, arbitrary neighboring
relations, unbounded analyst query composition, or deployment-specific legal
compliance. It does not make PSI, TEE, FHE, VSS, or malicious-robust MPC part
of the DP claim.
