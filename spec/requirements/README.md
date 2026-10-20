# Kiva Requirement Catalogs

This directory contains machine-readable requirement catalogs used to trace Kiva
contracts, tests, implementation code, and formal models back to standards.

The **LRS requirements ledger** records requirements applicable to a Learning
Record Store, together with the data definitions, conditions, exceptions and
role boundaries needed to interpret them. Its source is IEEE Std
9274.1.1-2023; its scope is LRS requirements and their supporting context, not
every requirement for every role in the standard.

Start with the [publication record](lrs-requirements-ledger-publication.md) for
the scope and source qualifications, and the
[review guide](lrs-requirements-ledger-review.md) for reading order and reproduction.
The [ledger views](lrs-requirements-ledger/README.md) and
[file inventory](lrs-requirements-ledger-publication.json) contain the final
artifacts and their references. Source interpretations are subject to PR review;
structural checks do not establish semantic completeness.

- `ieee-39274-1-1-2025.yaml`: stable requirement-family IDs and the detailed
  `issued_contract` paraphrased from the fixed **IEEE Std 9274.1.1-2023 issued
  PDF**. The historical filename and `standard` label remain compatibility
  identifiers; this work does not establish an independent ISO-text comparison.

The detailed contract records actor, strength, applicability, exceptions, source
unit, interpretation questions and delegated normative dependencies for each
rule. Requiredness, supplied-value type and value restrictions are separate
facets. A field rule applies only when its enclosing object is applicable;
optional objects do not become required because their children have requirements.
Value-meaning definitions are separate facets too. Optional `related_rules`
links resolve definitions and neighboring processing rules; these links do not
transfer test evidence or turn a data meaning into an LRS rejection obligation.
Source units cover the body of chapters 1–5; headings, role context and repeated
sender material have explicit dispositions. Informative annex examples are not
an independent normative requirement set.

`family_projection` maps each existing family to detailed rules. Its links do
not mean existing tests exercise every condition or exception. `new_rule_ids`
records rules without an existing family link. A `shall` family classification
must not override a detailed recommendation, permission, role restriction or
unresolved source conflict. Interpretations in `lrs-interpretation-policy.yaml`
remain project choices and do not adjudicate the issued standard.

The [field semantics guide](lrs-requirements-ledger-semantics.md) defines every
strength category, negative polarity, source/context links and the distinction
between family classification and detailed obligations.

Runtime/library delegation retains the xAPI requirement and its integration
evidence obligation. For example, IEEE 754 contributes a minimum processing and
storage precision obligation, not a requirement to implement or independently
certify the entire floating-point standard. Dated ISO 8601:2004 duration formats
remain cited even after withdrawal. Delegated dependencies are currently
unverified; a represented requirement is not a passed implementation test.

`scripts/ci/audit_requirement_catalog.py` checks the detailed contract's reference
integrity along with the existing catalog checks. To additionally verify private
source digests and coordinates, run:

```sh
nix develop -c python scripts/ci/issued_contract.py --source-text \
  /path/to/pinned/ieee-layout.txt
```

This check does not prove the meaning of paraphrases or semantic completeness.

The ledger includes recording-time and common-type definitions, characterstring
conditions and producer role boundaries. The
[publication record](lrs-requirements-ledger-publication.md) describes the source
qualifications. OPEN/POL questions, dependency integration, Suite coverage and
implementation conformance remain separate from the source projection.

OpenAPI files should reference catalog entries through `x-kiva-requirements`.
Requirements that OpenAPI cannot fully express remain linked from the contract
and must be discharged by conformance tests, property tests, Kani harnesses,
Tamarin models, or narrow F*/Low* models where appropriate.
