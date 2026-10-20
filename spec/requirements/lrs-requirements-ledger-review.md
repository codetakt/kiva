# Reviewing the LRS requirements ledger

Start with the [publication record](lrs-requirements-ledger-publication.md) for
the scope and source qualifications, then read the
[field semantics](lrs-requirements-ledger-semantics.md).
The source is **IEEE Std 9274.1.1-2023**. The historical 2025 filenames and machine
labels are compatibility identifiers, not evidence of edition equivalence.

## Reading the final artifacts

1. Browse [requirements.csv](lrs-requirements-ledger/requirements.csv), which has
   one row per rule or contextual facet. Check actor, strength, condition, effect
   and exceptions together. An optional enclosing object does not become mandatory
   merely because a child is required when present.
2. Follow `source_units` into
   [source-units.csv](lrs-requirements-ledger/source-units.csv) and the
   [catalog](ieee-39274-1-1-2025.yaml), whose `issued_contract` is authoritative.
3. Read [issues.json](lrs-requirements-ledger/issues.json),
   [dependencies.json](lrs-requirements-ledger/dependencies.json) and
   [project-interpretations.json](lrs-requirements-ledger/project-interpretations.json)
   for qualifications and the distinction between source and local policy.
4. Check [ledger.json](lrs-requirements-ledger/ledger.json) and the
   [publication inventory](lrs-requirements-ledger-publication.json) for final file
   identities, counts and assurance boundaries.

The ledger covers LRS requirements and supporting role boundaries, not every
requirement for every actor. Family links and discharge assignments identify
traceability, not complete condition coverage or successful execution. The PR's
OpenAPI and two case-reference repairs change annotations and generated bindings,
not runtime code or case expectations.

## Reproducible checks

The publication check needs only Python's standard library. It checks file hashes,
the exact generated issue table and the final ledger inventory, with no network,
private packet or Git-history dependency:

```sh
python -B scripts/ci/lrs_ledger_publication.py --check
```

The following checks use the repository's Python dependencies:

```sh
python -B scripts/ci/issued_contract.py
python -B scripts/ci/audit_requirement_catalog.py --no-reference-root --json
python -B scripts/ci/validate_openapi.py spec/openapi/kiva-xapi.yaml
python -B scripts/ci/validate_conformance.py spec/conformance/xapi-lrs-discharge.yaml
python -B scripts/ci/audit_lrs_source_projection.py --json
git -c core.whitespace=trailing-space,space-before-tab,cr-at-eol diff --check
```

`validate_openapi.py` validates the xAPI contract; applying it to the PDS or
confidential contracts is not a valid check of those contracts. The projection
audit uses the pinned public IEEE Markdown checkout when available and reports
whether source bytes were verified. Private PDF span verification is separate:

```sh
python -B scripts/ci/issued_contract.py --source-text /path/to/pinned/ieee-layout.txt
```

The PDF and extracted text are licensed inputs identified by their catalog hashes,
not committed files. Their names are descriptive input names; pass the actual
authorized text path explicitly for a fresh span check.

On the supported Linux host, run the complete contract/definition gates with:

```sh
nix build --no-link .#checks.x86_64-linux.openapi-contract
nix build --no-link .#checks.x86_64-linux.conformance-plan
nix build --no-link .#checks.x86_64-linux.pre-commit-check
```

These gates include generators and static evidence validation. The definition
gate does not run all 6,415 cases against an LRS. Hosted workflows perform their
own required checks before merge.

## Maintenance

Keep both CSVs as CRLF under `.gitattributes`. Regenerate the issue view from its
JSON, rather than editing rows independently:

```sh
python -B scripts/ci/lrs_ledger_publication.py --write-issues
```

This updates only `ISSUES.md`; it does not renew publication hashes or acceptance.
Changes to meanings require source reading and review appropriate to their impact.
The final publication does not resolve OPEN/POL questions, discharge dependency
integration or establish Kiva implementation conformance.
