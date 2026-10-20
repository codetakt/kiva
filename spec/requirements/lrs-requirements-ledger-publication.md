# LRS requirements ledger

The ledger describes requirements applicable to a Learning Record Store and the
supporting data definitions, conditions, exceptions and role boundaries needed
to interpret them. Its source is **IEEE Std 9274.1.1-2023**, identified by the
PDF and extracted-text hashes in the [catalog](ieee-39274-1-1-2025.yaml).

Historical 2025 filenames and machine labels are compatibility identifiers.
Equivalence with the separate 2025 edition has not been established. See the
[claim boundary](../../docs/architecture/conformance-claim.md).

## Contents and authority

The catalog's `issued_contract` contains 673 rules and contextual facets, 157
source units, 128 stable families, 25 OPEN/POL questions and 19 dependency
boundaries. Detailed rules retain actor, strength, applicability, effect,
exceptions and source links. The [field semantics](lrs-requirements-ledger-semantics.md)
define strength and polarity and distinguish source obligations from local policy.

The ten [ledger files](lrs-requirements-ledger/README.md) provide CSV/JSON views,
an issue register, six existing local interpretations and metadata. The catalog
is authoritative; views do not override its rule meanings. Family classifications
are coarse indices, not replacements for detailed strengths and conditions.

The [publication inventory](lrs-requirements-ledger-publication.json) records the
hashes of the files supplied with this change. It excludes itself to avoid a
circular digest; Git binds the inventory. `published` describes availability of
the source projection and its views. It does not mean that a review has approved
its source interpretation or that an LRS implementation is conformant.

## Conditions that must survive projection

`IP-multipart-accept` retains OPEN-PUT because general PUT/POST batch wording
coexists with the single-Statement PUT resource contract. The POST acceptance
requirement remains explicit. `IP-person-fallback` retains both the absence of
additional information and the restriction to information associated with the
requested Agent.

Hosted metadata optionality belongs to the metadata host, including an LRS acting
in that role. PUT body/query ID matching is an LRP obligation; it does not by
itself specify an LRS error response. Other producer duties, recommendations and
permissions likewise must not become mandatory receiver rejection rules.

The ledger includes recording-time, common-type and field-value definitions,
characterstring conditions and source links needed to interpret the rules.
Optional enclosing objects do not become mandatory merely because a child is
required when present. The source questions and dependency boundaries remain
explicit where an unconditional interpretation is not justified.

## Contract and case references

GET `/statements` adds the `statement-format-ids` and
`statement-format-canonical-agents` requirement-family references. The operation
kernel and HTTP operation catalog expose the same references.
The two `statement-unknown-parameter-with-valid-selectors` cases and their generator
cite the existing `xapi-lrs-4.1.5-unknown-parameters` family. Their whole-case hashes
match those definitions in the assertion manifest. These changes repair static
traceability without changing case expectations or runtime behavior.

The documentation anchor for the historical edition label is retained within a
compatibility explanation. It is not a positive claim of edition equivalence.

## Validation and limits

The [review guide](lrs-requirements-ledger-review.md) provides commands for the
publication hashes, generated issue table, catalog structure, OpenAPI/discharge
references, source projection and Nix contract/definition gates.
Source-span verification requires authorized access to the pinned extracted text;
the licensed PDF and full text are not redistributed.

Hashes, counts and structural checks do not prove semantic completeness. Review
of source interpretations belongs to the PR review. Twenty source questions,
five policy holds, 19 dependency integrations and later Suite/implementation
evidence remain separate. Family evidence does not automatically cover each
detailed condition or exception. The `conformance-plan` gate checks definitions
and evidence bindings; it does not execute all 6,415 cases against an LRS.
