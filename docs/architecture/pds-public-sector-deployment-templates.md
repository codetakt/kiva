# PDS Public-Sector Deployment Templates

## Scope

M13 publishes deployment templates, legal-policy examples, and operator
runbooks for public-sector and school PDS deployments. M15 adds a
learner-principal profile over the existing M11 resource-server scope boundary.

The templates are operational guidance artifacts for Kiva's PDS extension.
They are not legal advice, not a certification claim, and not part of the
[xAPI LRS claim boundary](conformance-claim.md).

## Artifacts

- Catalog: `spec/pds/deployment-templates.yaml`
- Catalog schema:
  `spec/pds/schemas/pds-deployment-template-catalog.schema.json`
- Validator: `scripts/ci/validate_pds_deployment_templates.py`
- Publication validator integration:
  `scripts/ci/validate_pds_publication.py`
- Evidence bundle integration:
  `scripts/ci/generate_pds_evidence.sh`

## Published Templates

The catalog includes three concrete deployment templates:

- `school-district-public-task-v1`
  - school-district profile
  - public-task legal-basis example
  - statement metadata retention schedule
  - metadata-only tombstone publication
  - school operations runbooks
- `multi-agency-research-aggregate-v1`
  - multi-agency profile
  - aggregate-insight research example
  - data-product metadata retention schedule
  - public-redacted tombstone publication
  - collaboration operations runbooks
- `learner-principal-v1`
  - learner is the validated OAuth/OIDC token subject
  - existing `pds:read`, `pds:write`, `pds:delegate`, and `pds:revoke` scopes
  - subject-owned consent grant and revocation
  - subject-facing record inventory and metadata export
  - no new authorization semantics or production identity-provider integration

Each template includes:

- separated governance role subjects;
- tenant and subject partition labels;
- emergency access policy metadata;
- enabled and disabled confidential-computation mechanisms for the template;
- an optional practical workload profile reference when a template is selected
  for CC8;
- legal-policy examples with purpose, legal basis, data classes, consent
  scope, retention policy, permitted PDS operations, and prohibited payloads;
- operator runbooks for key rotation, policy changes, consent incidents, audit
  exports, and emergency access;
- explicit claim-boundary text.

## Validation Boundary

The validator requires:

- at least one learner-principal, school-district, and multi-agency template;
- pairwise-distinct governance role subjects;
- complete and unique runbook references;
- exact consistency between governance `runbook_refs` and published operator
  runbooks;
- confidential mechanisms are classified exactly once as enabled or disabled;
- practical workload profile references resolve to repository files;
- no runbook owner self-approval;
- legal-policy examples that reference existing PDS OpenAPI operation IDs;
- retention review intervals not exceeding retention periods;
- tombstone timing not before the retention period;
- explicit prohibition of raw xAPI Statement JSON.

## Non-Goals

These templates do not implement production identity-provider federation,
legal advice, physical deletion/redaction execution, legal-hold automation,
administrative UI, or complete emergency-access workflow execution. They
publish validated deployment guidance for the current metadata-only PDS
boundary.
