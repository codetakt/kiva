# Practical Confidential Deployment Selection

## Scope

CC8 selects the first practical confidential computation deployment profile for
Kiva. The selected profile is `multi-agency-research-aggregate-v1`, described
in `spec/pds/deployment-templates.yaml` and fixed by
`spec/confidential/practical-deployment-workload-selection.yaml`.

This is a deployment and workload selection milestone. It does not make a
production cryptographic security claim by itself. The selected profile is now
bound to the completed CC9 durable job lifecycle, CC10 metadata-only
participant/key lifecycle, and CC15 repository-side release-candidate assurance
gate. External review outcomes, deployment evidence, and selected provider or
runtime bindings remain necessary before Kiva should make a stronger public
production-security claim.

## Selected Workloads

The first practical profile selects two aggregate BI workloads:

- monthly and term-level multi-agency participation counts;
- term-level program completion rate from released eligible and completed
  learner counts.

Raw xAPI Statements remain with custodians. Custodians locally derive bounded
counts, Kiva combines those counts through the selected confidential mechanisms,
and PDS publishes only governed aggregate-insight metadata and release
evidence.

## Mechanism Selection

The first practical workload uses:

- additive secure aggregation;
- differential privacy release;
- Shamir VSS-admission metadata for threshold reconstruction data products.

PSI cohort matching and portable TEE attestation remain enabled only as bounded
assurance capabilities for this profile. They are not promoted to production
PSI or hardware-backed confidential execution until a deployment binds external
review evidence to the CC11 runtime or CC12 verifier boundary. FHE remains
disabled for the first practical profile because additive aggregation plus DP is
sufficient for the selected workloads.

## Success Metrics

The selected profile fixes baseline targets for privacy, utility, latency, cost,
operator effort, and auditability. These targets are intentionally operational:
minimum population thresholds, epsilon budget limits, dashboard freshness,
operator effort, key-rotation drills, incident triage, and complete release
evidence coverage.

These metrics are tracked by the CC9 and CC10 lifecycle gates, but they are not
proof that every future analytical query is safe.

## Validation

Run the CC8 validator with:

```sh
scripts/ci/validate_practical_deployment_selection.py
```

The validator checks that the selected profile exists, names concrete
workloads, classifies mechanisms, documents threat assumptions and residual
risks, declares success metrics, updates the selected deployment template, and
is referenced by the CC7 assurance package.
