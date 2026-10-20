# Kiva LRS requirements ledger

This directory contains the final views of the LRS requirements in the pinned
**IEEE Std 9274.1.1-2023** source. Start with the
[publication record](../lrs-requirements-ledger-publication.md) for scope, source qualifications
and verification, and the
[field semantics](../lrs-requirements-ledger-semantics.md) for strength and polarity.

The [catalog](../ieee-39274-1-1-2025.yaml), under `issued_contract`, is authoritative.
The historical 2025 filename is a compatibility identifier; equivalence with a
separate 2025 edition has not been established.

| View | Content |
| --- | --- |
| [Requirements](requirements.csv) | 673 rules and contextual facets with actor, strength, applicability, exceptions and source links. |
| [Source units](source-units.csv) | 157 clause units with byte/LF/page coordinates, digests, dispositions and rule backlinks. |
| [Issues](ISSUES.md) / [machine records](issues.json) | 20 source questions, five policy holds and separately qualified audit/contract observations. |
| [Dependencies](dependencies.json) | 19 delegation boundaries, owners and required integration evidence. |
| [Families](families.json) / [discharge groups](discharge-groups.json) | 128 stable families and 30 evidence assignments; neither establishes detailed coverage or a passing execution. |
| [Project interpretations](project-interpretations.json) | Six existing local choices, distinguished from source adjudication. |
| [Metadata](ledger.json) | Final file inventory, source identity, counts and assurance limits. |

The source PDF SHA-256 is
`fa57dbaa0fbc0800b8c5a693af884032e782110c1f446ef865797fd0e0811526`.
The PDF and extracted full text are licensed inputs and are not redistributed.
Their hashes and coordinate convention are recorded in the catalog and metadata.

Review of the source interpretations belongs to the PR review. Counts and hashes
do not establish semantic completeness, issuer certification or implementation
conformance.

The 673 records include permissions, recommendations and context, not 673
mandatory tests. The 175 rules without a legacy family link are not 175 proven
missing tests. `related_rules` supplies navigation without propagating strength
or evidence. OPEN/POL questions, dependency integration and later runtime evidence
retain their separate boundaries. Existing local interpretations do not close
source questions.

Change the authoritative catalog first when altering a requirement, then update
its views and publication inventory together. Keep the CSVs as CRLF under
`.gitattributes`. Generate the issue table from its JSON:

```sh
python -B scripts/ci/lrs_ledger_publication.py --write-issues
python -B scripts/ci/lrs_ledger_publication.py --check
```

The write command updates only the table; it does not renew hashes or acceptance.
A semantic change requires source reading and review appropriate to its impact.
