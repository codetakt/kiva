# Conformance Suite Contribution Strategy

Kiva should incubate a conformance test suite that can be contributed upstream
as a neutral xAPI LRS conformance aid using the pinned IEEE Std 9274.1.1-2023
requirements baseline.

Historical `39274-1-1-2025` filenames and machine labels are compatibility
identifiers. The proposed repository name below retains the same naming
convention. Equivalence with the 2025 edition has not been established.
Requirement traceability alone does not establish implementation conformance;
see the [xAPI claim boundary](conformance-claim.md).

## Position

Contributing a conformance suite is strategically valuable. The xAPI ecosystem
has an ADL LRS Conformance Test Suite with xAPI 2.0 coverage, but Kiva should
not treat it as the normative source for an IEEE Std 9274.1.1-2023
conformance claim. Kiva should use ADL as an external compatibility gate while
building a clause-indexed, machine-readable suite around the IEEE requirement
catalog in this repository.

The contribution should be framed as a public conformance aid, not as a Kiva
certification program.

## Implementation Model

The contribution should be presented as language-neutral conformance assets with
a Rust reference runner.

The normative project assets should be:

- The IEEE Std 9274.1.1-2023 requirement catalog.
- Clause-indexed YAML test cases and fixtures.
- Endpoint and authentication fixture contracts.
- JSON Schema contracts for machine-readable reports.
- Traceability reports from requirements to executable cases.

The Rust runner should be positioned as a high-assurance reference
implementation of those assets. It should remain responsible for strict case
validation, HTTP execution, capture handling, fixture profile injection, and
JSON/JUnit report output. It should not be the only place where conformance
meaning is expressed.

Python tooling may remain useful for matrix generation, report validation,
schema checks, migration scripts, and CI glue. If an upstream community strongly
prefers Python, the right path is to add a thin Python runner over the same
YAML and JSON Schema contracts, not to make Python the source of truth.

This framing keeps the suite accessible to reviewers who do not read Rust while
preserving the safety, distribution, and execution-quality benefits of Rust.

## Eventual Repository Split

Kiva should incubate the work in this repository only until the conformance aid
has a stable shape. The long-term target is a separate repository, tentatively
named `ieee-39274-1-1-xapi-lrs-conformance-aid`.

The dependency direction must stay one-way:

- Kiva may depend on the conformance aid.
- The conformance aid must not depend on Kiva.
- Kiva may act as a reference target for the suite, but not as a required
  runtime or implementation dependency.

The standalone repository should own:

- The IEEE Std 9274.1.1-2023 xAPI LRS requirement catalog.
- Generic LRS conformance tests as language-neutral YAML assets.
- Fixtures and expected results.
- Test runner code, including the Rust reference runner.
- Authentication adapters.
- Report schemas and report generation.
- ADL compatibility mapping.
- Destructive-test classification, isolation guidance, and execution gates.
- Contribution, licensing, and non-certification notices.

The Kiva repository should keep:

- Kiva implementation code.
- Kiva-specific adapters for running the suite in CI.
- Kiva-specific PDS, BI, governance, OAuth/OIDC policy, and confidential
  computation tests.
- Evidence showing which conformance aid version was run against Kiva.
- Any Kiva-only extensions to the standard LRS behavior.

The split should happen when the generic suite can run against a disposable LRS
without Kiva services, and when every test case can be traced to a stable
requirement id.

## Contribution Boundaries

The upstreamable suite should contain:

- IEEE clause-indexed requirement metadata.
- Destructive black-box LRS HTTP tests for isolated test endpoints.
- JSON fixtures and expected status/header/body assertions.
- Language-neutral test definitions that can be reviewed without reading the
  runner implementation.
- Authentication adapters, including explicit headers for ADL parity and
  OAuth/OIDC bearer tokens read from environment variables for modern
  deployments.
- Machine-readable reports, such as JSON and JUnit.
- A traceability table from test cases to requirement ids.

The upstreamable suite should not contain:

- Kiva-specific PDS, BI, confidential-computation, or governance tests.
- Product-specific policy assumptions.
- Certification language unless IEEE or another authorized body defines that
  program.
- Long verbatim IEEE standard text. Clause references and short paraphrases are
  sufficient for traceability.

## Relationship To ADL

ADL's suite remains useful as a compatibility baseline. Kiva should preserve an
adapter for running it against Kiva and record its results as external evidence.

Kiva's own suite should differ in three ways:

- The requirement catalog is the source of truth.
- Each test has a stable requirement id and clause reference.
- The runner supports modern authentication, destructive-test gates, and
  structured evidence output from the beginning.

Where ADL tests already cover a requirement well, Kiva can either wrap them as
external evidence or propose targeted upstream improvements. Where ADL coverage
is absent, ambiguous, or tied to older assumptions, Kiva should implement new
tests in a separate suite that can later be donated.

## Legal And Governance Considerations

Before donation, Kiva maintainers should confirm:

- The target upstream home: IEEE Open Source xAPI project, ADL, or a jointly
  maintained neutral repository.
- The applicable contribution agreement or CLA.
- The license expected by the target project.
- Whether MIT-licensed ADL code can be reused directly or should remain an
  optional external dependency.
- That no non-public or restricted IEEE text is copied into the repository.

The safest initial shape is to keep Kiva's suite independent, with clean-room
test cases derived from the public requirement catalog and explicit external
links to ADL where useful.

## Kiva Incubation Plan

1. Maintain `spec/requirements/ieee-39274-1-1-2025.yaml` as the normative
   traceability source.
2. Maintain `spec/conformance/xapi-lrs-discharge.yaml` as the discharge plan.
3. Maintain `spec/conformance/adl-compatibility-map.yaml` as the conservative
   ADL compatibility and gap map.
4. Maintain the incubating conformance runner under `tools/xapi-lrs-conformance`
   so it can execute black-box HTTP tests against an ephemeral LRS endpoint,
   inject authentication headers, and gate destructive cases.
5. Keep conformance meaning in requirement metadata, YAML cases, fixture
   contracts, and JSON Schemas; treat Rust as the reference execution engine.
6. Add JSON fixtures for Statement, Document, Agent, Activity, Attachment,
   versioning, and error-condition tests.
7. Generate machine-readable reports keyed by requirement id.
8. Keep Kiva-specific privacy, PDS, and confidential-computation tests in a
   separate suite.
9. Propose the generic suite upstream once it has stable requirement IDs, clear
   reports, and documented destructive-test behavior.

## Split Plan

When the suite is ready to leave Kiva, move or copy these paths into the
standalone repository:

- `spec/requirements/ieee-39274-1-1-2025.yaml`
- `spec/conformance/xapi-lrs-discharge.yaml`, after removing Kiva-specific
  evidence paths.
- `spec/conformance/adl-compatibility-map.yaml`, after rewriting local
  reference paths for the standalone repository.
- Generic fixtures under the future `tests/conformance/fixtures` tree.
- Generic HTTP test cases under the future `tests/conformance/http` tree.
- Generic runner and report tooling under the future `tools` or `crates` tree,
  with the Rust runner documented as a reference implementation.
- Documentation from this strategy document, rewritten to remove Kiva as the
  center of gravity.

Keep these paths in Kiva:

- Kiva CI wrappers.
- Kiva-specific OAuth/OIDC deployment configuration.
- Kiva PDS, BI, governance, and confidential-computation tests.
- Kiva evidence records and implementation-specific discharge notes.

After the split, Kiva should pin the conformance aid by git revision, release
tag, or flake input, and record the exact version used for each conformance run.

## Initial Acceptance Criteria

The contribution candidate is ready for upstream discussion when:

- Every IEEE catalog requirement has a planned or active discharge mechanism.
- OpenAPI, ADL, Kiva HTTP tests, property tests, Kani, and Tamarin roles are
  explicit.
- The black-box runner can validate YAML cases, target a local disposable LRS,
  inject authentication headers, and refuse destructive cases unless explicitly
  enabled.
- Results can be consumed by CI without human interpretation.
- Test cases and report contracts can be reviewed as language-neutral assets,
  without treating Rust source code as the specification.
- The Rust reference runner can be installed or invoked without requiring
  contributors to understand Rust internals.
- The test suite can run without Kiva-specific services.
- Endpoint-profile-sensitive requirements, such as deterministic `413`
  too-large responses and `429` rate-limit responses, have portable fixture
  profiles or are explicitly reported as non-executable black-box gaps.
