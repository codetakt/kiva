# Application Operation Kernel

Kiva treats PDS and confidential-computation application calls as the same
shape of operation:

```text
typed operation -> workflow context -> admission policy -> port execution -> audit evidence
```

`ApplicationOperationKernel` owns the internal started/completed/failed
workflow lifecycle and is intentionally module-private. `ApplicationOperationGateway`
is the public application entrypoint used by PDS and confidential workflows
when operation metadata, admission, and execution should be treated as one
boundary. The gateway keeps the compatibility `run_*` methods, while new call
sites can use the `execute_*` spelling that mirrors the kernel boundary.
`ApplicationOperationPlan` binds the operation metadata to the admission policy
used inside that operation.
`ApplicationAdmissionPolicy` owns the narrow synchronous admission step before
a value crosses an application port. PDS uses it to admit `PdsDomainEvent`
values against the authenticated principal through
`PdsDomainEvent::controlling_subject()`. Confidential workflows use it to
create workflow-bound `ConfidentialWorkflowAdmission` values before an
explicit `ConfidentialWorkflowAdmissionRecorder` records admission and
repository work executes.

The surface operation source ledger is the operation review entrypoint. The
generated integrated surface catalog drives both the application operation
catalog projection and the Rust operation registry. Its generated Rust
projection contains the private
`APPLICATION_OPERATION_SPECS` table,
`ApplicationOperationId`, `application_operation_specs()`, and lookup methods
for catalog ids and OpenAPI operationIds. Runtime `ApplicationWorkflowSpec`
values carry only workflow labels; Rust workflow markers, OpenAPI operations,
and OAuth/OIDC scopes stay in the operation-catalog evidence projection.
The same generator also emits
`docs/architecture/generated/application-operation-catalog.md` and
`generated/application-operation-catalog.evidence.json` so human review tables
and machine evidence remain projections of the surface entries.
