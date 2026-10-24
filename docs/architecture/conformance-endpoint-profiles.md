# Conformance Endpoint Profiles

Kiva's xAPI LRS conformance aid uses the pinned IEEE Std 9274.1.1-2023
requirements baseline. It keeps generic HTTP cases portable by separating
implementation-specific trigger values from the test cases themselves.

Equivalence with the 2025 edition has not been established. Requirement
traceability alone does not establish implementation conformance; see the
[xAPI claim boundary](conformance-claim.md).

## Problem

Some xAPI LRS requirements specify the response status for implementation
limits, but not the concrete trigger threshold:

- Oversized Statements, documents, or attachments must be rejected with `413`.
- Requests rejected because of rate limiting must be rejected with `429`.

A fixed black-box suite cannot choose one universal payload size or burst count
that is correct for every LRS. Large arbitrary payloads can be slow or unsafe,
and timing-dependent rate-limit probes are flaky.

## Fixture Contract

The runner therefore accepts an endpoint profile with `--endpoint-profile PATH`.
The profile is a local conformance fixture declaration for the target LRS, not
an xAPI extension and not a production API contract.

The current profile schema is intentionally small. Its historical `standard`
value remains a compatibility identifier, not a claim about the reviewed edition:

```yaml
schema_version: 1
standard: IEEE/ISO/IEC 39274-1-1-2025
profiles:
  deterministic-too-large-threshold:
    max_request_body_bytes: 4096
    probe_body_bytes: 8192
  deterministic-rate-limit-trigger:
    request_count: 3
    window_seconds: 1
    path: /__kiva/conformance/rate-limit
```

`deterministic-too-large-threshold` says that the target LRS will reject the
runner's generated valid xAPI Statement body when its body length is at least
`probe_body_bytes`, because that exceeds the target's declared
`max_request_body_bytes`.

`deterministic-rate-limit-trigger` says that the target LRS will return `429`
for the final request when the runner sends `request_count` immediate requests
to `path` within `window_seconds`.

The bundled Kiva example uses a dedicated `__kiva` fixture path so the complete
bundled suite can run against one fresh in-memory server without exhausting the
standard `/about` resource. It also sets the Statement body limit high enough
for normal multipart and signed-Statement fixtures while keeping the generated
oversized probe deterministic.

The profile schema also accepts two declarations based on independent fixture
evidence:

- `declared-parameter-vocabulary`: `contexts` records a method, path, evidence
  and nonempty `unrecognized` parameter list. A rejection response cannot
  itself establish that the parameter is unknown to the target.
- `declared-resource-absence`: `case_evidence` maps each exact case ID to
  independent evidence that all negative selectors are absent for the request
  credentials immediately before and throughout that case. A fresh-server
  label or the response under test is insufficient. The runner checks evidence
  availability; fixture validity and integration are separate later work.

The case validator and report format additionally recognize
`signature-validation-context`. It is a requirement-only name: the current
profile schema cannot declare it and the runner does not supply its provider.
Recognizing its name does not make signature-validation cases executable.

The Python oracle controls compare the five prerequisite names in the Rust
validator, Python case-matrix reporter and report schema. They also check that
the four declarable profile types exclude the signature prerequisite. This
guards duplicated vocabularies; it does not add a provider or certify fixtures.

## Runner Behavior

Cases that need these fixtures declare `requires_endpoint_profiles`. Validation
does not require a profile file, so the suite remains statically checkable in
CI. A case with an unavailable required profile is NOT-RUN before sending any
requests; it does not become a failed LRS observation.

The runner exposes profile values as case-local captures:

- `kiva_endpoint_profile_too_large_statement_body`
- `kiva_endpoint_profile_rate_limit_request_count`
- `kiva_endpoint_profile_rate_limit_window_seconds`
- `kiva_endpoint_profile_rate_limit_path`

Profile captures are only valid in cases that declare the corresponding
`requires_endpoint_profiles` entry.

## Isolation

Endpoint-profile-sensitive cases should run against disposable or explicitly
configured conformance targets. They are not expected to mutate accepted xAPI
state, but they deliberately exercise resource limits and rate-limit controls.

The standalone conformance aid should keep this contract generic. Kiva-specific
deployment values, throttling knobs, or server configuration should stay in
Kiva CI adapters and should not become part of the portable suite.
