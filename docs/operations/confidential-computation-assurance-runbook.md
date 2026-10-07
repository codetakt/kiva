# Confidential Computation Assurance Runbook

## Scope

This runbook covers operation of Kiva's CC7 selected confidential computation
boundaries. It is not a cryptographic key ceremony specification, legal advice,
or an incident-response substitute for a deployment's own security program.

## Key Lifecycle

Use subject-owned signing-key registration, rotation, and revocation endpoints
for additive-share transport keys. Rotate keys before participant turnover,
after suspected exposure, and before publishing an external review evidence
package. Revoked keys must not be used for aggregate verification.

For DP, Shamir, PSI, TEE, FROST threshold signing, FHE feasibility, and FHE
runtime assurance, treat keys, nonces, commitments, custody digests, parameter
digests, and deployment verifier or runtime material as deployment evidence.
Do not publish secret material, share openings, FROST private shares, raw
signature shares, TEE secrets, FHE plaintexts, ciphertext payloads, or
production decryption keys in metadata exports.

## Participant Lifecycle

Onboard each computation participant with a tenant, subject binding,
participant index, active signing key where required, mechanism profile, and
role assignment. Confirm that participant indexes are one-based, unique inside
the job, and inside the declared participant set.

When a participant leaves, revoke active additive-share keys, close or retire
pending jobs, preserve metadata-only audit evidence, and require a new job id
for any restarted computation.

## Incident Handling

Treat duplicate shares, replay records, invalid signatures, commitment
mismatches, out-of-budget DP releases, stale TEE evidence, PSI duplicate
datasets, FROST unresolved complaints, FROST equivocation evidence, FROST
signing replay, and FHE parameter mismatches as fail-closed incidents.
For the FHE runtime boundary, also treat missing key-custody evidence,
ciphertext transcript replay, evaluation replay, and external-review
regression as fail-closed incidents.
Preserve audit metadata and do not publish raw payloads while the incident is
reviewed.

Escalate incidents that could affect legal privacy claims, deployment
separation of authority, or cryptographic assumptions to external review before
re-enabling affected mechanisms.

## Emergency Access

Emergency access is limited to metadata necessary for investigation and audit.
It must not grant analysts direct raw xAPI Statement access, submitted-share
payloads, reconstructed Shamir secrets, raw DP aggregate or noise evidence, TEE
secrets, PSI non-matches, FROST private shares or raw signature shares, or FHE
plaintext/ciphertext fixture internals, ciphertext payloads, decryption keys,
or evaluation proof payloads.

Require policy-authority approval and auditor review for emergency metadata
access. Record the reason, affected tenant and subject partitions, permitted
metadata operations, time window, and follow-up review.

## Audit Export

Export metadata-only evidence: OpenAPI contract hashes, package validation
reports, conformance evidence manifests, formal verification reports, PDS
evidence manifests, signed-key state, replay metadata, release metadata,
budget ledgers, reconstruction metadata, attestation metadata, and deployment
template identifiers, FROST ceremony digests, FROST signing transcript digests,
FROST share-custody evidence digests, FHE runtime ciphertext transcript
digests, FHE runtime evaluation digests, and FHE key-custody evidence digests.

Do not export raw xAPI Statements, additive share values, signed envelopes,
noise samples, Shamir share values, opening nonces, PSI participant datasets,
FROST private shares, raw FROST signature shares, TEE secret material, FHE
fixture plaintexts, FHE runtime ciphertext payloads, or FHE decryption keys.

## Retirement

Retire a confidential mechanism when its profile is superseded, the deployment
no longer accepts its assumptions, or external review identifies a blocking
gap. Mark the mechanism disabled in deployment templates, preserve historical
metadata, stop accepting new jobs for that profile, and document replacement
or migration guidance.

For FHE feasibility, FHE runtime, portable TEE evidence, and FROST threshold
signing, retirement or promotion review is mandatory before making production
FHE, vendor-hardware TEE, or malicious-robust threshold-cryptography claims
unless new production library, key-management, quote-verification,
runtime-review, and external-review evidence is added.
