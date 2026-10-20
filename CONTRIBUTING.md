# Contributing to Kiva

Thank you for your interest in contributing to Kiva.

Kiva is an xAPI LRS platform with a PostgreSQL backend, OAuth/OIDC-oriented
authorization boundaries, Personal Data
Store design goals, and internal conformance and formal-assurance evidence.
Its requirements ledger uses the pinned IEEE Std 9274.1.1-2023 source.
The historical `39274-1-1-2025` filenames and machine labels are compatibility
identifiers; equivalence with a separate ISO/2025 edition is not established.
Changes that affect xAPI behavior should cite the relevant requirement ids and
include conformance, persistence, or verification evidence.

## Development Environment

Kiva is Nix-first. The supported local workflow uses the pinned flake
toolchains instead of host-installed Rust, Python, PostgreSQL, F*, Kani, or
Tamarin.

```bash
nix develop
nix flake check --print-build-logs
nix build .#default --print-build-logs
```

For formal-verification work:

```bash
nix develop .#verification
nix build .#checks.x86_64-linux.fstar-boundary --print-build-logs
nix build .#checks.x86_64-linux.kani-boundary --print-build-logs
nix build .#checks.x86_64-linux.tamarin-boundary --print-build-logs
```

Non-Nix setups are not supported as CI-equivalent environments.

## Code Style

- Keep implementation changes narrowly scoped.
- Prefer existing crate boundaries and helper APIs.
- Keep comments and documentation in English.
- Treat unknown, unsupported, or ambiguous protocol inputs as fail-closed unless
  the xAPI requirement explicitly says otherwise.
- Do not weaken conformance, privacy, or formal-evidence claims to make tests
  pass.

Before opening a pull request, run:

```bash
nix flake check --print-build-logs
```

## xAPI Conformance Changes

When changing xAPI behavior, update the relevant artifacts:

- `spec/requirements/ieee-39274-1-1-2025.yaml`
- `spec/conformance/xapi-lrs-discharge.yaml`
- `tests/conformance/http/`
- `spec/openapi/kiva-xapi.yaml`
- `docs/architecture/conformance-claim.md`, when the claim boundary changes

Useful checks:

```bash
nix build .#checks.x86_64-linux.openapi-contract --print-build-logs
nix build .#checks.x86_64-linux.conformance-plan --print-build-logs
nix build .#checks.x86_64-linux.memory-conformance --print-build-logs
nix build .#checks.x86_64-linux.postgres-conformance --print-build-logs
```

The ADL xAPI LRS conformance suite is compatibility evidence, not the normative
source for Kiva's IEEE Std 9274.1.1-2023 requirements ledger. A requirement's
presence in that ledger does not establish implementation conformance.

## PostgreSQL Changes

Database changes must include versioned migrations under `db/migrations/` and
must keep the PostgreSQL execution checks passing:

```bash
nix build .#checks.x86_64-linux.database-migrations --print-build-logs
nix build .#checks.x86_64-linux.postgres-verification --print-build-logs
```

For behavior that depends on transactions, idempotence, retry handling, or
privacy budget accounting, add or update PostgreSQL execution tests rather than
only memory-backend tests.

## Formal Verification Changes

Kiva uses Kani, Tamarin, and F* as bounded internal assurance tools:

- Kani covers selected pure Rust decision logic.
- Tamarin covers symbolic authorization, consent, OAuth/OIDC, and confidential
  workflow boundaries.
- F* covers small stable release-gate and privacy-gate models.

When changing a verified boundary, update the corresponding proof or explain
why the change is outside the current formal scope. The current scope is
documented in `docs/architecture/formal-verification-coverage-review.md`.

## Evidence Bundles

Generate local conformance evidence with:

```bash
scripts/ci/generate_conformance_evidence.sh --require-clean-git
```

Generate formal verification evidence with:

```bash
nix develop .#verification -c \
  scripts/ci/generate_formal_verification_evidence.sh --require-clean-git
```

Validate both release-candidate bundles together with:

```bash
scripts/ci/validate_release_evidence_set.py \
  --conformance target/conformance-evidence/latest \
  --formal target/formal-verification-evidence/latest
```

Generated evidence under `target/` is intentionally not committed.

## Pull Request Checklist

- Run `nix flake check --print-build-logs`.
- Add tests or conformance cases for behavior changes.
- Update OpenAPI, requirement catalogs, or discharge plans when protocol
  behavior changes.
- Update formal verification artifacts when a verified boundary changes.
- Keep conformance and security wording conservative: do not claim IEEE
  certification or complete formal verification.
- Avoid unrelated refactors in protocol, persistence, or verification changes.

## Security Reporting

Do not open public issues for security vulnerabilities. Use GitHub Security
Advisories for private reporting and coordinated disclosure.

## License

By contributing, you agree that your contributions will be licensed under the
same terms as the project. See `LICENSE`.
