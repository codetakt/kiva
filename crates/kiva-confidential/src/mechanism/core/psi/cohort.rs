#![allow(clippy::wildcard_imports)]

use super::*;

/// One participant's pre-tokenized identifier set for a PSI cohort job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsiParticipantDataset {
    participant_index: NonZeroUsize,
    identifier_digests: Vec<PsiIdentifierDigest>,
}

impl PsiParticipantDataset {
    /// Creates a participant dataset for a one-based participant index.
    #[must_use]
    pub const fn new(
        participant_index: NonZeroUsize,
        identifier_digests: Vec<PsiIdentifierDigest>,
    ) -> Self {
        Self {
            participant_index,
            identifier_digests,
        }
    }

    /// Returns the participant index.
    #[must_use]
    pub const fn participant_index(&self) -> NonZeroUsize {
        self.participant_index
    }

    /// Returns the submitted identifier digests.
    #[must_use]
    pub fn identifier_digests(&self) -> &[PsiIdentifierDigest] {
        &self.identifier_digests
    }
}

/// Replay record for one accepted PSI cohort job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsiJobRecord {
    job_id: SecureAggregateTranscriptId,
}

impl PsiJobRecord {
    /// Creates a PSI job replay record.
    #[must_use]
    pub const fn new(job_id: SecureAggregateTranscriptId) -> Self {
        Self { job_id }
    }

    /// Returns the job identifier.
    #[must_use]
    pub const fn job_id(self) -> SecureAggregateTranscriptId {
        self.job_id
    }
}

/// Metadata-only output of a bounded PSI cohort job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsiCohortIntersectionResult {
    job_record: PsiJobRecord,
    participant_count: NonZeroUsize,
    intersection_digests: Vec<PsiIdentifierDigest>,
}

impl PsiCohortIntersectionResult {
    /// Creates a PSI cohort intersection result.
    #[must_use]
    pub const fn new(
        job_record: PsiJobRecord,
        participant_count: NonZeroUsize,
        intersection_digests: Vec<PsiIdentifierDigest>,
    ) -> Self {
        Self {
            job_record,
            participant_count,
            intersection_digests,
        }
    }

    /// Returns the replay record that must be persisted.
    #[must_use]
    pub const fn job_record(&self) -> PsiJobRecord {
        self.job_record
    }

    /// Returns the number of required participants.
    #[must_use]
    pub const fn participant_count(&self) -> NonZeroUsize {
        self.participant_count
    }

    /// Returns only the identifier digests present in every participant set.
    #[must_use]
    pub fn intersection_digests(&self) -> &[PsiIdentifierDigest] {
        &self.intersection_digests
    }
}

/// Derives a domain-separated digest for one bounded PSI cohort identifier.
pub fn derive_psi_cohort_identifier_digest(
    job_id: SecureAggregateTranscriptId,
    namespace: &str,
    identifier: &str,
) -> Result<PsiIdentifierDigest, SecureAggregateError> {
    let namespace = namespace.trim();
    validate_psi_label(namespace, SecureAggregateError::PsiNamespaceEmpty)?;
    let identifier = canonicalize_psi_cohort_identifier(identifier)?;

    let mut hasher = Sha256::new();
    hasher.update(PSI_COHORT_IDENTIFIER_DOMAIN);
    hasher.update(job_id.bytes());
    update_len_prefixed(&mut hasher, namespace.as_bytes());
    update_len_prefixed(&mut hasher, identifier.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES];
    bytes.copy_from_slice(&digest);
    PsiIdentifierDigest::new(bytes)
}

/// Computes an n-of-n intersection over pre-tokenized PSI cohort identifiers.
///
/// The result exposes only digests present in every participant set. Non-matches
/// and submitted per-participant datasets are excluded from the output.
pub fn compute_psi_cohort_intersection(
    participant_count: NonZeroUsize,
    job_id: SecureAggregateTranscriptId,
    existing_jobs: &[PsiJobRecord],
    datasets: &[PsiParticipantDataset],
) -> Result<PsiCohortIntersectionResult, SecureAggregateError> {
    validate_psi_datasets(participant_count, existing_jobs, job_id, datasets)?;

    let mut intersection = datasets[0]
        .identifier_digests()
        .iter()
        .copied()
        .filter(|candidate| {
            datasets[1..]
                .iter()
                .all(|dataset| dataset.identifier_digests().contains(candidate))
        })
        .collect::<Vec<_>>();
    intersection.sort_unstable();

    Ok(PsiCohortIntersectionResult::new(
        PsiJobRecord::new(job_id),
        participant_count,
        intersection,
    ))
}

fn validate_psi_datasets(
    participant_count: NonZeroUsize,
    existing_jobs: &[PsiJobRecord],
    job_id: SecureAggregateTranscriptId,
    datasets: &[PsiParticipantDataset],
) -> Result<(), SecureAggregateError> {
    if existing_jobs.iter().any(|record| record.job_id() == job_id) {
        return Err(SecureAggregateError::PsiReplayDetected);
    }
    if datasets.len() != participant_count.get() {
        return Err(SecureAggregateError::PsiParticipantDatasetCountMismatch);
    }

    for (offset, dataset) in datasets.iter().enumerate() {
        let participant_index = dataset.participant_index().get();
        if participant_index > participant_count.get() {
            return Err(SecureAggregateError::ParticipantIndexOutOfRange);
        }
        if datasets[..offset]
            .iter()
            .any(|previous| previous.participant_index() == dataset.participant_index())
        {
            return Err(SecureAggregateError::DuplicateParticipantIndex);
        }
        if dataset.identifier_digests().is_empty() {
            return Err(SecureAggregateError::PsiParticipantDatasetEmpty);
        }
        for (digest_offset, digest) in dataset.identifier_digests().iter().enumerate() {
            if dataset.identifier_digests()[..digest_offset].contains(digest) {
                return Err(SecureAggregateError::DuplicatePsiIdentifierDigest);
            }
        }
    }

    Ok(())
}
