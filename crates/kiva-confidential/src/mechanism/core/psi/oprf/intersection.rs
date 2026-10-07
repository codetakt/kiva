#![allow(clippy::wildcard_imports)]

use super::*;

/// Metadata-only result of an accepted DH-OPRF PSI intersection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsiOprfIntersectionResult {
    record: PsiOprfTranscriptRecord,
    participant_count: NonZeroUsize,
    intersection_elements: Vec<PsiOprfElement>,
}

impl PsiOprfIntersectionResult {
    /// Creates a DH-OPRF PSI intersection result.
    #[must_use]
    pub const fn new(
        record: PsiOprfTranscriptRecord,
        participant_count: NonZeroUsize,
        intersection_elements: Vec<PsiOprfElement>,
    ) -> Self {
        Self {
            record,
            participant_count,
            intersection_elements,
        }
    }

    /// Returns the replay record shape retained by the admitted evidence boundary.
    #[must_use]
    pub const fn record(&self) -> PsiOprfTranscriptRecord {
        self.record
    }

    /// Returns the configured participant count.
    #[must_use]
    pub const fn participant_count(&self) -> NonZeroUsize {
        self.participant_count
    }

    /// Returns only elements present in every participant transcript.
    #[must_use]
    pub fn intersection_elements(&self) -> &[PsiOprfElement] {
        &self.intersection_elements
    }
}

/// Computes a metadata-only intersection over opaque DH-OPRF runtime elements.
pub fn compute_psi_oprf_intersection(
    plan: PsiOprfPlan,
    job_id: SecureAggregateTranscriptId,
    existing_records: &[PsiOprfTranscriptRecord],
    transcripts: &[PsiOprfParticipantTranscript],
) -> Result<PsiOprfIntersectionResult, SecureAggregateError> {
    validate_psi_oprf_transcripts(plan, job_id, existing_records, transcripts)?;

    let mut intersection = transcripts[0]
        .evaluated_elements()
        .iter()
        .copied()
        .filter(|candidate| {
            transcripts[1..]
                .iter()
                .all(|transcript| transcript.evaluated_elements().contains(candidate))
        })
        .collect::<Vec<_>>();
    intersection.sort_unstable();

    let transcript_digest = derive_psi_oprf_transcript_digest(plan, job_id, transcripts)?;
    Ok(PsiOprfIntersectionResult::new(
        PsiOprfTranscriptRecord::new(job_id, transcript_digest),
        plan.participant_count(),
        intersection,
    ))
}

fn validate_psi_oprf_transcripts(
    plan: PsiOprfPlan,
    job_id: SecureAggregateTranscriptId,
    existing_records: &[PsiOprfTranscriptRecord],
    transcripts: &[PsiOprfParticipantTranscript],
) -> Result<(), SecureAggregateError> {
    validate_psi_oprf_runtime_binding(plan.runtime_binding())?;
    if transcripts.len() != plan.participant_count().get() {
        return Err(SecureAggregateError::PsiOprfParticipantTranscriptCountMismatch);
    }
    if existing_records
        .iter()
        .any(|record| record.job_id() == job_id)
    {
        return Err(SecureAggregateError::PsiOprfTranscriptReplayDetected);
    }

    for (offset, transcript) in transcripts.iter().enumerate() {
        let participant_index = transcript.participant_index().get();
        if participant_index > plan.participant_count().get() {
            return Err(SecureAggregateError::ParticipantIndexOutOfRange);
        }
        if transcripts[..offset]
            .iter()
            .any(|previous| previous.participant_index() == transcript.participant_index())
        {
            return Err(SecureAggregateError::DuplicateParticipantIndex);
        }
        if transcript.status() == PsiOprfParticipantTranscriptStatus::Aborted {
            return Err(SecureAggregateError::PsiOprfTranscriptAborted);
        }
        if transcript.evaluated_elements().is_empty() {
            return Err(SecureAggregateError::PsiOprfParticipantTranscriptEmpty);
        }
        for (element_offset, element) in transcript.evaluated_elements().iter().enumerate() {
            if transcript.evaluated_elements()[..element_offset].contains(element) {
                return Err(SecureAggregateError::DuplicatePsiOprfElement);
            }
        }
    }

    let transcript_digest = derive_psi_oprf_transcript_digest(plan, job_id, transcripts)?;
    if existing_records
        .iter()
        .any(|record| record.conflicts_with(job_id, transcript_digest))
    {
        return Err(SecureAggregateError::PsiOprfTranscriptReplayDetected);
    }
    Ok(())
}

fn derive_psi_oprf_transcript_digest(
    plan: PsiOprfPlan,
    job_id: SecureAggregateTranscriptId,
    transcripts: &[PsiOprfParticipantTranscript],
) -> Result<SecureAggregateCommitment, SecureAggregateError> {
    let mut ordered = transcripts
        .iter()
        .map(|transcript| {
            let mut elements = transcript
                .evaluated_elements()
                .iter()
                .map(|element| element.bytes())
                .collect::<Vec<_>>();
            elements.sort_unstable();
            (transcript.participant_index().get(), elements)
        })
        .collect::<Vec<_>>();
    ordered.sort_unstable_by_key(|(participant_index, _elements)| *participant_index);

    let mut hasher = Sha256::new();
    hasher.update(PSI_OPRF_DH_VOPRF_TRANSCRIPT_DOMAIN);
    update_len_prefixed(&mut hasher, PSI_OPRF_DH_VOPRF_PROFILE.as_bytes());
    update_len_prefixed(
        &mut hasher,
        plan.runtime_binding().suite().label().as_bytes(),
    );
    hasher.update(job_id.bytes());
    hasher.update((plan.participant_count().get() as u128).to_be_bytes());
    hasher.update(plan.runtime_binding().runtime_artifact_digest().bytes());
    hasher.update(plan.runtime_binding().review_evidence_digest().bytes());
    for (participant_index, elements) in ordered {
        hasher.update((participant_index as u128).to_be_bytes());
        hasher.update((elements.len() as u128).to_be_bytes());
        for element in elements {
            hasher.update(element);
        }
    }
    let digest = hasher.finalize();
    let mut bytes = [0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES];
    bytes.copy_from_slice(&digest);
    SecureAggregateCommitment::new(bytes)
}
