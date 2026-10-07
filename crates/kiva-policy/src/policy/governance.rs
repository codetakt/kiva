#![allow(clippy::wildcard_imports)]

use super::*;

/// Structural evidence bits required before admitting a public-sector PDS
/// governance profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicSectorGovernanceProfile {
    evidence_bits: u8,
}

impl PublicSectorGovernanceProfile {
    /// Evidence bit for declared tenant partitioning.
    pub const TENANT_PARTITION_DECLARED: u8 = 1 << 0;
    /// Evidence bit for declared subject partitioning.
    pub const SUBJECT_PARTITION_DECLARED: u8 = 1 << 1;
    /// Evidence bit for present required role subjects.
    pub const REQUIRED_ROLE_SUBJECTS_DECLARED: u8 = 1 << 2;
    /// Evidence bit for pairwise distinct required role subjects.
    pub const REQUIRED_ROLE_SUBJECTS_DISTINCT: u8 = 1 << 3;
    /// Evidence bit for absent or separated optional identity authority.
    pub const IDENTITY_AUTHORITY_ABSENT_OR_DISTINCT: u8 = 1 << 4;
    /// Evidence bit for declared emergency access policy metadata.
    pub const EMERGENCY_ACCESS_POLICY_DECLARED: u8 = 1 << 5;
    /// Evidence bit for declared operational runbook references.
    pub const RUNBOOKS_DECLARED: u8 = 1 << 6;
    /// Complete evidence bitset required by the strict Kiva PDS boundary.
    pub const STRICT_EVIDENCE_BITS: u8 = Self::TENANT_PARTITION_DECLARED
        | Self::SUBJECT_PARTITION_DECLARED
        | Self::REQUIRED_ROLE_SUBJECTS_DECLARED
        | Self::REQUIRED_ROLE_SUBJECTS_DISTINCT
        | Self::IDENTITY_AUTHORITY_ABSENT_OR_DISTINCT
        | Self::EMERGENCY_ACCESS_POLICY_DECLARED
        | Self::RUNBOOKS_DECLARED;

    /// Creates public-sector governance profile admission metadata from bits.
    #[must_use]
    pub const fn from_evidence_bits(evidence_bits: u8) -> Self {
        Self { evidence_bits }
    }

    /// Returns the raw evidence bitset.
    #[must_use]
    pub const fn evidence_bits(self) -> u8 {
        self.evidence_bits
    }
}
