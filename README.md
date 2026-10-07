# Kiva

Kiva is a learner experience data platform that exposes an xAPI Learning
Record Store (LRS) interface targeting **IEEE/ISO/IEC 39274-1-1-2025**.
Around that interface, Kiva provides separate governed
surfaces for Personal Data Store (PDS) workflows, confidential-computation
provider integration, and aggregate analytics.

## Positioning

Kiva is designed around four deliberately separate responsibilities:

1. **Standards-based xAPI access**
   - Expose an xAPI LRS interface aligned with IEEE/ISO/IEC
     39274-1-1-2025.
   - Preserve interoperability for statements, activities, agents, verbs, documents, and RESTful access patterns.
   - Define the interface through OpenAPI and portable conformance cases.

2. **Identity, consent, and authorization**
   - Connect to production OAuth/OIDC identity providers.
   - Support a learner-principal deployment profile in which an authenticated
     data subject exercises consent grant and revocation over their own PDS
     metadata.
   - Support learner-initiated data portability through a subject-bound,
     audited export of accepted Statement JSON and PDS metadata.

3. **Privacy-aware Personal Data Store**
   - Govern PDS metadata for consent, access audit, lifecycle, retention,
     public-sector governance, approved data products, and provider-binding
     registry metadata.
   - Keep the PDS extension separate from the IEEE xAPI LRS conformance
     surface.
   - Treat the learner-facing capabilities as self-sovereign-oriented user
     agency, not as conformance with a self-sovereign identity standard.

4. **Governed analytics and confidential computation**
   - Provide analytics-ready governed aggregate serving for institutional
     reporting over approved data products and admitted confidential releases.
   - Provide reference reporting deployments for schools and
     corporate training as external consumers of the governed analytics API;
     Kiva does not host a BI product.
   - Admit secret/confidential-compute provider metadata through a typed
     provider contract and review gates before PDS registry persistence.
   - Keep raw Statements and individual-level data out of the analytics
     surface.

## Compatibility Testing

The repository includes an ADL compatibility runner and language-neutral xAPI
conformance cases. Run-specific observations and assessments are retained locally
under `target/`. See [the conformance aid](spec/conformance/README.md) for usage.

## Target Use Cases

- A school or public institution records learning activities through the
  standards-aligned xAPI interface while governing PDS metadata through
  consent and lifecycle workflows.
- Schools and corporate training programs consume approved aggregate products
  through an external reporting deployment.
- Public-sector deployments separate operational administration from the ability to inspect or exploit personal learning data.
- Institutional reporting users retrieve approved aggregate insights through
  the governed analytics surface.
- Learners exercise subject-bound consent and request audited portability
  exports for their own records.

## Design Principles

- **Interoperability first:** the standards-aligned xAPI interface is the LRS
  contract.
- **Privacy by architecture:** personal data handling should be constrained by identity, policy, audit, and computation boundaries.
- **Explicit governance:** schools and public organizations require stronger
  governance than ordinary application telemetry.
- **Multi-party by default:** analytics should not assume a single trusted data owner.
- **Conformance must be testable:** claims about IEEE/ISO/IEC 39274-1-1-2025 support should be backed by explicit tests, examples, and traceable references.
- **Conformance assets should be portable:** the xAPI conformance suite should keep requirement metadata,
  YAML cases, fixture contracts, schemas, and reports language-neutral, with Rust used as a strict reference runner.

## Local Verification

Verification commands and output formats are documented in
[Release Evidence](docs/operations/release-evidence.md) and
[CI Gates](docs/operations/ci-gates.md). Store execution results and review
assessments under the Git-ignored `target/` directory.

## Reference Material

Kiva uses the following IEEE Open Source xAPI reference repositories:

- [xAPI Base Standard documentation](https://opensource.ieee.org/xapi/xapi-base-standard-documentation)
- [xAPI Base Standard examples](https://opensource.ieee.org/xapi/xapi-base-standard-examples)
- [xAPI Profiles](https://opensource.ieee.org/xapi/xapi-profiles)

See `AGENTS.md` for repository-specific development notes and reference paths.

## Documentation

- [Documentation map](docs/README.md)
- [xAPI Conformance Claim](docs/architecture/conformance-claim.md)
- [PDS Extension Compatibility](docs/architecture/pds-extension-compatibility.md)
- [Personal Data Store Milestones](docs/architecture/pds-milestones.md)
- [Learner Data Portability Export](docs/architecture/learner-data-portability-export.md)
- [Analytics Claim Boundary](docs/architecture/analytics-claim-boundary.md)
- [Reference Reporting Deployments](docs/architecture/reference-reporting-deployments.md)
- [Confidential Computation Milestones](docs/architecture/confidential-computation-milestones.md)
- [Formal Verification Roadmap](docs/architecture/formal-verification-roadmap.md)
- [CI Gates](docs/operations/ci-gates.md)
- [Release Evidence](docs/operations/release-evidence.md)
- [Kiva xAPI OpenAPI contract](spec/openapi/kiva-xapi.yaml)
- [Kiva PDS OpenAPI contract](spec/openapi/kiva-pds.yaml)
- [Kiva confidential-computation OpenAPI contract](spec/openapi/kiva-confidential.yaml)
- [Kiva analytics OpenAPI contract](spec/openapi/kiva-analytics.yaml)
- [PDS examples](examples/pds)
- [PDS publication artifact schemas](spec/pds)
- [IEEE/ISO/IEC 39274-1-1-2025 requirement catalog](spec/requirements/ieee-39274-1-1-2025.yaml)
- [xAPI LRS conformance discharge plan](spec/conformance/xapi-lrs-discharge.yaml)
- [xAPI LRS conformance aid runner](tools/xapi-lrs-conformance)
