# Conformance Authentication Fixtures

Kiva's xAPI LRS conformance aid needs explicit authentication fixtures for
IEEE/ISO/IEC 39274-1-1-2025 requirements that are credential-sensitive rather
than purely syntactic.

## Scope

The IEEE xAPI LRS interface requires an LRS to handle or delegate authentication
decisions, permission decisions, and authority assignment based on credentials.
Those properties cannot be fully projected into OpenAPI and cannot be verified
by a single anonymous HTTP client. The conformance aid therefore defines a small
fixture contract for disposable LRS targets.

This fixture contract is not an xAPI extension. It is a test harness convention
for running black-box conformance cases against an isolated endpoint.

## Required Profiles

An isolated conformance target that runs credential-sensitive cases must expose
the following bearer-token profiles through the runner:

- `statement_writer_a`: a valid credential allowed to write and read Statements
  in the conformance namespace. The LRS must treat this credential as a regular
  non-strong-trust Learning Record Provider credential, so submitted Statement
  `authority` values are overwritten by the LRS.
- `statement_reader`: a valid credential that is not allowed to write
  Statements. A Statement write attempted with this credential must be rejected
  with `403 Forbidden`, not accepted and not treated as an invalid credential.
- `invalid_credential`: a credential that the target LRS refuses as invalid.
  A request made with this credential must be rejected with `401 Unauthorized`,
  not treated as a valid read-only credential and not evaluated as a normal
  permission denial.

The runner receives these credentials with an environment-backed auth profile
file:

```sh
cargo run -p xapi-lrs-conformance -- \
  --endpoint http://localhost:8080/xapi \
  --allow-destructive \
  --auth-profile-file spec/conformance/auth-profile.example.yaml
```

For harnesses that prefer direct CLI wiring, the runner also accepts repeated
`--auth-profile NAME=ENV` bearer-token arguments.

Kiva's built-in smoke scripts export `KIVA_CONFORMANCE_WRITER_A_TOKEN`,
`KIVA_CONFORMANCE_READER_TOKEN`, and `KIVA_CONFORMANCE_INVALID_TOKEN`. The
server reads those same variables when it builds its authentication
configuration, so overriding a fixture token changes the bearer value without
changing the required writer, reader, or refused-credential semantics.
The smoke scripts leave `KIVA_REQUIRE_AUTHORIZATION` unset because many generic
xAPI conformance cases intentionally exercise requests without credential
fixtures.

## Authority Mapping

For the `statement_writer_a` profile, the test fixture authority is:

```json
{
  "objectType": "Agent",
  "account": {
    "homePage": "https://example.org/kiva/conformance/auth",
    "name": "statement-writer-a"
  }
}
```

The deterministic-authority cases submit different Statement `authority` values
and then retrieve the stored Statements. A conforming fixture target must return
the fixture authority above for each Statement written by `statement_writer_a`.

This verifies the conditional IEEE requirement that, when the LRS overwrites
authority, credential-to-authority mapping is deterministic. A target that keeps
submitted authority because it has configured a strong trust relationship should
not run this fixture profile as `statement_writer_a`; it should provide a
separate regular writer credential for the conformance aid.

## Isolation

Credential-sensitive cases are marked destructive and require an isolated LRS.
The target may persist rejected write attempts for audit purposes, but it must
not let those attempts create accepted xAPI Statements.

The fixture namespace is limited to deterministic IDs, actors, activities, and
accounts under `https://example.org/kiva/conformance/` and
`mailto:kiva-conformance-...@example.org`.
