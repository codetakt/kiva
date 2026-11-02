# Kiva

Kiva is a learner experience data platform that exposes an xAPI Learning
Record Store (LRS) interface. Its current requirements baseline is the pinned
**IEEE Std 9274.1.1-2023**. [ISO/IEC/IEEE 39274-1-1:2025 is its international
adoption](https://standards.ieee.org/ieee/39274-1-1/12268/). Implementation and
test traceability use the pinned IEEE 2023 requirements.
Around that interface, Kiva provides separate governed
surfaces for Personal Data Store (PDS) workflows, confidential-computation
provider integration, and aggregate analytics.

## Positioning

Kiva is designed around four deliberately separate responsibilities:

1. **Standards-based xAPI access**
   - Use the pinned IEEE Std 9274.1.1-2023 as the current xAPI LRS
     requirements baseline.
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

## LRS capabilities

Kiva supports canonical Activity/Verb metadata, authorized small corrections,
external Activity definitions, Activity metadata hosting, Person associations,
trusted-provider authority retention, detailed interaction validation and both
live and snapshot pagination. Original Statement content remains immutable.
The [capability specification](spec/conformance/lrs-product-capabilities.yaml)
links these features to IEEE requirements, implementation files and required
positive HTTP controls on both storage backends.

Configure these deployment choices using `KIVA_XAPI_CAPABILITIES_JSON`. For example:

```json
{
  "canonical_inventory": true,
  "activity_hosting": true,
  "person_enrichment": true,
  "detailed_interactions": true,
  "rollback_voided_metadata": true,
  "mailbox_hash_queries": true,
  "canonical_writers": ["metadata-administrator"],
  "directory_writers": ["identity-administrator"],
  "trusted_authority_subjects": ["trusted-learning-provider"],
  "metadata_origins": ["https://activities.example.org"],
  "metadata_html": true,
  "public_base_url": "https://lrs.example.org",
  "pagination": "snapshot",
  "cursor_ttl_seconds": 3600,
  "legacy_extension_aliases": false
}
```

Writer and trusted-provider values are verified identity-provider subjects.
Ordinary writers retain their existing admission permissions. An empty trusted
provider list keeps deterministic authority replacement. Metadata fetching uses
only configured HTTP(S) origins, including redirects, without user credentials.
It accepts Activity Definition JSON and HTML containing
`<script type="application/json" data-xapi-activity-definition>` or an alternate
JSON link. Failed retrieval is retried on subsequent metadata access or admission;
pending work and successful definitions persist in PostgreSQL. The default fetch
timeout is two seconds and the response limit is 1 MiB.

`PUT /extensions/activities/{uuid}` publishes a Definition at the corresponding
Activity identifier under `public_base_url`; its GET representation is public.
Hosting requires `activity_hosting`, `canonical_inventory` and `public_base_url`
together, so the published document remains the canonical information source.
`public_base_url` alone configures the deployment prefix without enabling hosting.
`PUT /extensions/people/{uuid}` registers a Person with arrays of associated IFIs
and names. Person reads require xAPI read permission; writes to both resources
require the configured administrative subject and xAPI write permission.
Use `If-None-Match: *` to create and the current ETag in `If-Match` to update or
delete. Conflicting Person associations fail atomically.

Both pagination modes return short opaque URLs and recheck permissions and
voiding on every request. Snapshot mode fixes the initial membership; live mode
includes eligible subsequent arrivals. Expired or foreign continuation tokens
return 404. Cursor capacity defaults to 10,000 records and 64 MiB per tenant.
`self_contained_live_cursors: true` additionally enables stateless live links
for short filters, with automatic token fallback above 2,048 URL bytes. Those
inline links expose their filters and have no storage expiry; every request
still authenticates and reevaluates current visibility and voiding.
Additional application routes are available under `/extensions`; legacy root
aliases remain enabled by default for migration and can be disabled as above.
`/about` reports the active configuration without disclosing subject lists.

Apply all database migrations before starting the updated server. Existing rows
remain immutable and receive no retrospective permission to correct canonical
metadata. Configuration defaults preserve existing canonical and Person fallback
behavior; the positive capability gate explicitly enables and tests the features.

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
- **Conformance must be testable:** claims should identify the reviewed standard
  edition and be backed by explicit tests, examples, and traceable references.
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

The detailed requirement ledger uses the pinned IEEE 2023 publication.
Historical `39274-1-1-2025` filenames and machine labels remain compatibility
identifiers. Source traceability does not establish implementation conformance;
see the [xAPI claim boundary](docs/architecture/conformance-claim.md).

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
- [IEEE Std 9274.1.1-2023 requirement catalog](spec/requirements/ieee-39274-1-1-2025.yaml)
- [xAPI LRS conformance discharge plan](spec/conformance/xapi-lrs-discharge.yaml)
- [xAPI LRS conformance aid runner](tools/xapi-lrs-conformance)
