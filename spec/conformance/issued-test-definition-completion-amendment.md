# Source residual policy for test definitions

The pinned IEEE Std 9274.1.1-2023 ledger defines the responsibility set.
[Interpretation contracts](issued-definition-contracts.json) and the
[source residual register](issued-test-definition-source-residuals.json)
separate concrete Kiva test expectations from unresolved source propositions.
This policy permits complete project test definitions without declaring source
ambiguities or contradictions resolved.

## Requirements

- Distinguish settled source requirements, permitted choices, unspecified behavior,
  ambiguity and contradiction. Link each to pinned text, including competing
  clauses. Do not reinterpret a contradiction as unrestricted choice.
- Where the source does not establish a unique answer, state the scoped Kiva
  policy, rationale, alternatives and compatibility, implementation and testing
  impacts. Do not infer a policy from the current implementation's behavior.
- Define inputs, prerequisites, operations, observations, allowed/prohibited
  results and state changes, including positive, negative, boundary and exception
  cases. Observation without an expectation is not a complete test definition.
- Separate Kiva-policy outcomes from IEEE outcomes. Preserve disputed source
  propositions as indeterminate; policy PASS is not IEEE PASS. Continue to detect
  independent violations with established normative force.
- Retain source residuals and reassessment conditions for official corrections,
  updates or new evidence. An external answer is not a prerequisite to defining
  project behavior. Where the source establishes a complete allowed set, no
  additional Kiva choice is necessary.

The residual register contains two topics and three branches. Neither the policies
nor their tests constitute official IEEE adjudication or resolve those conflicts.

## Semantic and execution boundaries

Review must compare the whole responsibility set with its tests, preserving actors,
force, applicability and exceptions, and assess discrimination, excessive
constraints and the impact of shared oracles. Matching counts and hashes establish
identity and structural correspondence, not semantic correctness.

The pinned source ledger is unchanged. Primitive delegation for date/time,
Duration and floating point retains xAPI-specific calling conditions, precision
and preservation duties. Missing provider capabilities leave execution incomplete;
they do not waive duties or establish a passing result.

Official review is the GitHub PR merge. Interim review receipts, progress and
adoption decisions are not repository inputs or a separate acceptance gate.
Runtime input/output receipts remain required for execution evidence. Definition
correspondence, execution completion and product conformance are distinct claims.
