# Historical LRS interpretation policy and judgment reliability

Pin adopted interpretations and make expectations, execution and results traceable from them. This supports a reproducible Suite even where source text is inconsistent. Project adoption of an interpretation differs from official IEEE interpretation.

The historical profile described here is **`xapi-lrs-ieee-2023-interpretation-3`**. Consult the [definition contracts](issued-definition-contracts.json) and machine-readable policy for effective successor interpretations; this transcription preserves the earlier narrative and its scope. [lrs-interpretation-policy.yaml](lrs-interpretation-policy.yaml) records adopted rules, reasons, rejected readings and inspection targets. Retain source facts/coordinates in [lrs-published-source.md](lrs-published-source.md).

| Topic | Adopted interpretation | Judgment treatment |
| --- | --- | --- |
| State listing | stateId presence selects single/list; omit it for lists and use since as a filter. | Preserves explicit dispatch and table descriptions. Display dependence in applicable GET/HEAD expectations. |
| Activity Profile listing | activityId selects Activity scope; omit profileId for lists. | Read the clause 4 cell using clause 5's required activityId/optional since table. Withdraw conditional exclusion and display GET/HEAD dependence. |
| IRI/UUID | Validate required format; resolution, existence and real-world meaning are not required. | Preserve both data format, not values and format-rejection duties. Separate invalid-format expectations from target type errors; UUID variant recommendations remain nonmandatory. |
| PUT batches | Test mandatory single-Statement PUT and single/batch POST. Do not assert a duty to reject PUT arrays. | Do not classify valid single PUT as ambiguous. Display dependence for PUT-array expectations; without cases, the scope is untested. |
| Case | JSON key/enumeration case is strict; IRIs follow permitted RFC comparison methods. | Move the signature mbox Case-conflict status leaf to diagnostics and record actual status. Other header/crypto assertions remain mandatory. Do not derive unconditional lowercasing of IRI paths or whole mail addresses. |
| related_activities | Use the duplicated clause 5 PDF table to filter Activities in Object/contextActivities/SubStatement. | Test separately from related_agents. Present the clause 4 typo beside explicit clause 5 wording; require no external overlay. |

Do not fit expectations to observed Kiva behavior. Record source reasons/diffs when correcting expectations, preserving old candidates/results. Add neither violation-ignoring skips nor failure-to-success conversions.

Establish reliability through these checks:

1. **Basis and expectation:** preserve source roles, shall/should/may, conditions and exceptions; link evidence to concrete response checks. Distinguish table type violations, mandatory conditions and invalid formats.
2. **Result interpretation:** retain raw success/failure/not-run and state known interpretation dependencies, including preparation/later observations within a case. A case-level dependency does not prove its failure was caused by interpretation.
3. **Oracle reliability:** use positives, negatives, unchanged-state and boundary controls, plus altered responses/judgments to detect false passes. Do not pass by deleting expectations.
4. **Reproducibility:** pin source/profile/cases/executor/target configuration. Reuse past execution only after receipt-based input/result comparison, distinguishing it from fresh execution.

`scripts/ci/report_lrs_interpretations.py` maps six known topics and judgments added after cross-review to concrete case/expectation coordinates and digests. Existing source auditing also binds policy, mapping logic and results into the assertion manifest. This prevents silently reusing old results as equivalent evidence after interpretation changes; it does not mechanically prove source meaning.

```sh
python scripts/ci/report_lrs_interpretations.py --json
python scripts/ci/report_lrs_interpretations.py \
  --run-report target/source-projection-work/memory.json \
  --evidence-receipt target/source-projection-work/final-receipt.json \
  --require-complete-report --json
```

Using execution reports requires a receipt with input_sha256/artifacts_sha256 binding the report, case files and executor contract. Report self-description alone is not current-case execution evidence. Interpret target authentication, permissions, clocks and visibility delays under the original fixture. Exit 0 means successful mapping/evidence comparison, not LRS conformance.

no_registered_dependency means no dependency on registered judgments was identified. This finite case category does not prove interpretation/completeness of an entire clause. interpretation_dependent does not cancel raw failures. Bind list-table interpretations to locations expecting successful list responses. Negative cases rejected under both readings, such as argument-name case errors, retain ordinary judgment.

The contribution claim is that enumerated requirements of this LRS edition can be checked under this interpretation profile and target configuration. It requires semantic comparison from each duty to actual assertions, incorporated normative-reference boundaries and procedures for applicable clocks/delays/policies. Case counts, hashes and all-HTTP-case success alone do not prove complete projection. Review relevant interpretation/expectation differences and version them if an official IEEE response arrives; obtaining that response must not stop currently implementable checks.

The machine-readable policy's additional_judgments defines:

| Judgment | Source and disposition |
| --- | --- |
| Multiple State DELETE | Supplied registration limits deletion to it; absence covers all registrations for Activity/Agent. Observe other Agent/Activity survival through bytes. |
| Attachment order | Distinguish logical-equivalence Any attachments exclusion from exact/default Ordered array output. Compare two distinct metadata entries in order, check completeness of both MIME data parts without prescribing part order. |
| Empty relevantTypes | Zero in prose covers absence; one-or-more in the table applies when present. Require 400 for [], retaining independent type/null/key-case duties. |
| Other empty arrays | Accept arrays without a minimum. Compare attachments, Group member, Context arrays, Context Activities and interaction lists against their respective tables. Distinguish missing mandatory properties/IFIs. |
| Canonical information | Do not assume submitted metadata persists. Direct cases check IDs and returned-map language count; language-content selection uses a procedure comparing an independently declared canonical inventory. |
| Interaction structure | Common null/duplicate-key/key-case violations are prohibited. Restore 48 cases to mandatory 400, separate from optional interaction-specific validation. |

These judgments have source anchors and actual-case dependency indicators and remain reviewable. Withdraw the older profile's four empty-array cases allowing both outcomes. Contrast acceptance of omission/nonempty values separately with empty-array rejection.

Missing fixtures mean not-run, counted as neither target success nor failure. Do not use not-applicable without evidence that the condition is absent.

unknown-local-offset-boundary permits 200 or 400 for -00:00. Interpret the ISO 8601/RFC 3339 boundary together with RFC9557 §§2.1–2.3. Only if accepted, read back the same instant in UTC. This is not official adjudication. Store interpretation checks and execution results outside Git under target/.
