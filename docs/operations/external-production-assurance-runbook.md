# External Production Assurance Runbook

## Scope

This runbook covers the repository-side operational controls required before a
Kiva deployment can move from internal confidential-computation assurance to an
external production assurance review. It is not legal advice, privacy
certification, or evidence that an external review has already been completed.

## Vulnerability Intake

Route confidential-computation vulnerabilities through the deployment security
contact and the Kiva maintainer responsible for the release candidate. Intake
records must include the affected deployment profile, mechanism profile,
runtime or verifier artifact digest when applicable, suspected impact, reporter
contact, disclosure constraints, and whether raw personal data or secret
material may have been exposed.

Do not attach raw xAPI Statements, private shares, FHE ciphertext payloads,
TEE secret material, OPRF private keys, FROST private shares, or decryption
keys to vulnerability records. Use metadata digests and controlled evidence
storage.

## Patch SLAs

Critical issues that can expose raw learner data, secret key material, private
shares, or production release authority require triage within 24 hours. High
severity issues that can bypass release gates, participant gates, budget
checks, replay checks, or review gates require a remediation plan within 7
days. Medium severity issues that affect audit completeness, documentation, or
non-production fixtures require a remediation plan within 30 days.

If a fix changes cryptographic assumptions, OpenAPI contracts, database
migrations, release evidence structure, or public claim language, regenerate
the release-candidate evidence bundle before continuing review.

## Cryptographic Deprecation Policy

Deprecate a mechanism profile when a cryptographic assumption is weakened, a
runtime dependency loses maintenance, a verifier trust anchor is withdrawn, a
parameter set no longer meets the deployment's security target, or external
review identifies a blocking issue.

Deprecation requires disabling new jobs for the affected mechanism, preserving
metadata-only audit evidence, updating deployment templates, documenting the
replacement plan, and preventing public production claims until the replacement
profile has its own review evidence.

## Key Compromise Response

On suspected key compromise, freeze affected participants and key epochs,
revoke additive signing keys where applicable, stop new confidential jobs for
the affected mechanism, preserve replay and audit metadata, and require policy
authority plus auditor approval before resuming release.

For OPRF, TEE, FROST, or FHE runtime artifacts, bind the incident to the
runtime or verifier artifact digest, review evidence digest, key epoch, and
deployment profile. Rotate or retire affected artifacts before a production
claim can resume.

## Audit Export Retention

Retain metadata-only release-candidate evidence for at least 540 days or the
deployment's longer legal retention period. Retained evidence must include
manifest files, checksum files, toolchain versions, OpenAPI contract digests,
database migration state, conformance reports, formal verification reports,
PDS evidence manifests, confidential-assurance validation reports, external
review outcomes, and incident records.

Do not retain raw xAPI payloads, submitted shares, Shamir openings, FROST
private shares, OPRF secrets, TEE secret material, FHE ciphertext payloads, or
decryption keys inside public release evidence bundles.

## Incident Disclosure Workflow

Before disclosure, classify whether the incident affects xAPI conformance,
PDS governance, confidential-computation release gates, cryptographic
assumptions, deployment operations, or legal/privacy obligations. Notify the
deployment owner, policy authority, auditor, and external reviewer according to
that classification.

Public disclosure must state the affected deployment profile and mechanism
set, the release candidate or deployed version, the impact boundary, the
mitigation, and whether any prior public claim language must be withdrawn or
replaced.

## Release Candidate Evidence

Generate release-candidate evidence only from a clean git tree. The evidence
set must include live PostgreSQL conformance execution, PostgreSQL execution
verification, formal verification regression evidence, PDS publication
evidence, signed or otherwise integrity-protected checksums, toolchain
versions, database migration state, OpenAPI contracts, and release evidence
set validation.

The release candidate is not production-claimable until external review
outcomes are bound to the exact commit, deployment profile, selected mechanism
set, runtime artifacts, verifier artifacts, key epochs, and operational
runbooks.

## External Review Outcomes

Store external review outcomes as release-candidate evidence. Each outcome
must name the reviewer, review scope, reviewed commit or artifact digest,
threat model, selected mechanism set, deployment template, unresolved findings,
accepted residual risks, and claim language approved for publication.

If the review excludes legal privacy certification, IEEE xAPI certification,
full formal verification, production MPC, malicious-secure PSI, production
FHE, hardware-backed TEE security, or production threshold cryptography, the
public claim language must keep those exclusions explicit.
