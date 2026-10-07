# Security Policy

## Supported Versions

Kiva is pre-release software. Security fixes target the current `main` branch
until the project publishes versioned releases.

| Version | Supported |
| ------- | --------- |
| main    | Yes       |

## Reporting a Vulnerability

Do not open public issues for security vulnerabilities.

Please report security vulnerabilities through GitHub Security Advisories for
the active Kiva repository. For the current pre-publication CI mirror, use:

https://github.com/codetakt/kiva-server-ci/security/advisories/new

### What to include

- A description of the vulnerability and its potential impact
- Steps to reproduce or a proof of concept
- Affected commit, version, deployment profile, and configuration
- Any suggested mitigation or fix

### Response timeline

| Stage                         | Target    |
| ----------------------------- | --------- |
| Acknowledgement               | 72 hours  |
| Initial assessment            | 7 days    |
| Patch for critical issues     | 30 days   |
| Patch for non-critical issues | 90 days   |
| Public disclosure             | After fix is released, or 90 days, whichever comes first |

We follow coordinated disclosure. We will credit reporters in the advisory
unless they prefer to remain anonymous.

## Scope

The following are considered security issues:

- Authentication or authorization bypass
- OAuth/OIDC bearer-token validation failures
- Cross-tenant or cross-subject data access
- Exposure of raw learning records through aggregate, audit, or conformance APIs
- Bypasses of Personal Data Store consent, delegation, or revocation checks
- PostgreSQL transaction or isolation bugs that permit duplicate release,
  replay, over-spend, or inconsistent audit state
- Confidential-computation boundary failures in additive aggregate,
  threshold-release, differential-privacy budget, or release-gate logic
- Injection, request smuggling, SSRF, CSRF, XSS, or unsafe deserialization
- Supply-chain compromise or dependency policy bypass

The following are not security issues by themselves:

- Feature requests or usability improvements
- Non-security bugs or ordinary test failures
- Performance issues without a confidentiality, integrity, or availability impact
- Documentation errors that do not change security expectations

## Security Posture

Kiva is an IEEE/ISO/IEC 39274-1-1-2025-aligned xAPI LRS implementation with a
PostgreSQL backend, conformance evidence generation, and bounded formal
assurance artifacts. The current assurance boundary is defined in:

- `docs/architecture/conformance-claim.md`
- `docs/architecture/formal-verification-coverage-review.md`
- `docs/architecture/assurance-and-confidential-computation.md`

The xAPI conformance claim is internal evidence-based alignment, not IEEE
certification. The formal artifacts are also internal assurance evidence. They
cover selected pure Rust logic and symbolic boundary properties with Kani,
Tamarin, and F*, but they do not prove the complete HTTP server, PostgreSQL
execution semantics, OAuth/OIDC deployment profile, cryptographic primitives,
or production confidential-computation protocols.

## Required CI Gates

Protected `main` branches for pre-publication CI must require:

- `Core Checks & Server Build`
- `xAPI Conformance Evidence`
- `F*, Kani, and Tamarin`
- `Evidence Set Contract`

Release-candidate claims should be based on clean-revision evidence bundles
generated with the documented `--require-clean-git` paths.

## Contact

For security matters, use GitHub Security Advisories. For ordinary development
questions, use issues or discussions in the active repository.
