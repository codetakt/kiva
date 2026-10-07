#![allow(clippy::wildcard_imports)]

use super::*;

/// Pure fact bitset evaluated before AWS Nitro key or result release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAwsNitroReleaseFacts {
    bits: u8,
}

impl TeeAwsNitroReleaseFacts {
    /// Fact bit for accepted Nitro attestation.
    pub const ATTESTATION_ACCEPTED: u8 = 1 << 0;
    /// Fact bit for policy-authority approval.
    pub const POLICY_APPROVED: u8 = 1 << 1;
    /// Fact bit for completed external security review.
    pub const EXTERNAL_REVIEW_COMPLETED: u8 = 1 << 2;
    /// Fact bit for verifier key epoch at or above policy.
    pub const VERIFIER_KEY_CURRENT: u8 = 1 << 3;
    /// Fact bit for result release bound to the accepted job.
    pub const JOB_MATCHES_RELEASE_POLICY: u8 = 1 << 4;
    /// Fact bit for completed secret provisioning before result release.
    pub const SECRETS_PROVISIONED: u8 = 1 << 5;

    /// Creates a fact set from raw bits.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    /// Returns whether a fact bit is present.
    #[must_use]
    pub const fn contains(self, bit: u8) -> bool {
        self.bits & bit == bit
    }
}

/// Runtime facts evaluated before AWS Nitro key or result release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAwsNitroReleaseContext {
    facts: TeeAwsNitroReleaseFacts,
}

impl TeeAwsNitroReleaseContext {
    /// Creates a release context from normalized facts.
    #[must_use]
    pub const fn new(facts: TeeAwsNitroReleaseFacts) -> Self {
        Self { facts }
    }

    /// Returns the release fact set.
    #[must_use]
    pub const fn facts(self) -> TeeAwsNitroReleaseFacts {
        self.facts
    }

    /// Returns whether Nitro attestation was accepted.
    #[must_use]
    pub const fn attestation_accepted(self) -> bool {
        self.facts()
            .contains(TeeAwsNitroReleaseFacts::ATTESTATION_ACCEPTED)
    }

    /// Returns whether policy authority approved release.
    #[must_use]
    pub const fn policy_approved(self) -> bool {
        self.facts()
            .contains(TeeAwsNitroReleaseFacts::POLICY_APPROVED)
    }

    /// Returns whether external security review has completed.
    #[must_use]
    pub const fn external_review_completed(self) -> bool {
        self.facts()
            .contains(TeeAwsNitroReleaseFacts::EXTERNAL_REVIEW_COMPLETED)
    }

    /// Returns whether verifier key rotation policy is satisfied.
    #[must_use]
    pub const fn verifier_key_current(self) -> bool {
        self.facts()
            .contains(TeeAwsNitroReleaseFacts::VERIFIER_KEY_CURRENT)
    }

    /// Returns whether result release is bound to the accepted job.
    #[must_use]
    pub const fn job_matches_release_policy(self) -> bool {
        self.facts()
            .contains(TeeAwsNitroReleaseFacts::JOB_MATCHES_RELEASE_POLICY)
    }

    /// Returns whether secret provisioning already completed.
    #[must_use]
    pub const fn secrets_provisioned(self) -> bool {
        self.facts()
            .contains(TeeAwsNitroReleaseFacts::SECRETS_PROVISIONED)
    }
}

/// Validates that AWS Nitro key release is gated by reviewed attestation.
pub const fn validate_tee_aws_nitro_key_release(
    context: TeeAwsNitroReleaseContext,
) -> Result<(), SecureAggregateError> {
    if !context.attestation_accepted() {
        return Err(SecureAggregateError::TeeAttestationRequired);
    }
    if !context.policy_approved() {
        return Err(SecureAggregateError::TeePolicyApprovalMissing);
    }
    if !context.external_review_completed() {
        return Err(SecureAggregateError::TeeNitroExternalReviewRequired);
    }
    if !context.verifier_key_current() {
        return Err(SecureAggregateError::TeeNitroVerifierKeyRotationRequired);
    }
    Ok(())
}

/// Validates that AWS Nitro result release is bound to accepted attestation and keys.
pub const fn validate_tee_aws_nitro_result_release(
    context: TeeAwsNitroReleaseContext,
) -> Result<(), SecureAggregateError> {
    if let Err(error) = validate_tee_aws_nitro_key_release(context) {
        return Err(error);
    }
    if !context.secrets_provisioned() {
        return Err(SecureAggregateError::TeeNitroSecretProvisioningRequired);
    }
    if !context.job_matches_release_policy() {
        return Err(SecureAggregateError::TeeJobBindingMismatch);
    }
    Ok(())
}
