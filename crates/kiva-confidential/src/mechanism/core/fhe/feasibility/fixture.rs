#![allow(clippy::wildcard_imports)]

use super::*;

/// Nonce used by deterministic FHE feasibility ciphertext fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "[u8; 32]", into = "[u8; 32]")]
pub struct FheFeasibilityNonce {
    pub(in crate::mechanism::core) bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES],
}

impl FheFeasibilityNonce {
    /// Creates a fixture nonce after rejecting the reserved empty value.
    pub fn new(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, SecureAggregateError> {
        if artifact_is_empty(&bytes) {
            return Err(SecureAggregateError::EmptyFheNonce);
        }
        Ok(Self { bytes })
    }

    /// Returns the canonical nonce bytes.
    #[must_use]
    pub const fn bytes(self) -> [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
        self.bytes
    }
}

impl TryFrom<[u8; SECURE_AGGREGATE_ARTIFACT_BYTES]> for FheFeasibilityNonce {
    type Error = SecureAggregateError;

    fn try_from(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, Self::Error> {
        Self::new(bytes)
    }
}

impl From<FheFeasibilityNonce> for [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
    fn from(value: FheFeasibilityNonce) -> Self {
        value.bytes()
    }
}

/// Participant plaintext admitted to the deterministic FHE feasibility fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FheFeasibilityPlaintext {
    pub(in crate::mechanism::core) participant_index: NonZeroUsize,
    pub(in crate::mechanism::core) value: u64,
}

impl FheFeasibilityPlaintext {
    /// Creates a plaintext after checking the admitted feasibility range.
    pub fn new(
        parameter_set: FheFeasibilityParameterSet,
        participant_index: NonZeroUsize,
        value: u64,
    ) -> Result<Self, SecureAggregateError> {
        validate_fhe_feasibility_parameters(parameter_set)?;
        if value > parameter_set.max_plaintext_value() {
            return Err(SecureAggregateError::FhePlaintextOutsideRange);
        }
        Ok(Self {
            participant_index,
            value,
        })
    }

    /// Returns the one-based participant index.
    #[must_use]
    pub const fn participant_index(self) -> NonZeroUsize {
        self.participant_index
    }

    /// Returns the fixture plaintext value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.value
    }
}

/// Deterministic ciphertext fixture for the CC6 FHE feasibility boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FheFeasibilityCiphertextFixture {
    job_id: SecureAggregateTranscriptId,
    parameter_set: FheFeasibilityParameterSet,
    participant_index: NonZeroUsize,
    ciphertext_digest: SecureAggregateCommitment,
    slot_count: NonZeroUsize,
    noise_budget_bits: u16,
    fixture_plaintext_value: u64,
}

impl FheFeasibilityCiphertextFixture {
    /// Creates a ciphertext fixture from explicit parts for boundary tests.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn from_fixture_parts(
        job_id: SecureAggregateTranscriptId,
        parameter_set: FheFeasibilityParameterSet,
        participant_index: NonZeroUsize,
        ciphertext_digest: SecureAggregateCommitment,
        slot_count: NonZeroUsize,
        noise_budget_bits: u16,
        fixture_plaintext_value: u64,
    ) -> Self {
        Self {
            job_id,
            parameter_set,
            participant_index,
            ciphertext_digest,
            slot_count,
            noise_budget_bits,
            fixture_plaintext_value,
        }
    }

    /// Returns the job identifier.
    #[must_use]
    pub const fn job_id(self) -> SecureAggregateTranscriptId {
        self.job_id
    }

    /// Returns the parameter set used to create the fixture.
    #[must_use]
    pub const fn parameter_set(self) -> FheFeasibilityParameterSet {
        self.parameter_set
    }

    /// Returns the one-based participant index.
    #[must_use]
    pub const fn participant_index(self) -> NonZeroUsize {
        self.participant_index
    }

    /// Returns the deterministic ciphertext digest.
    #[must_use]
    pub const fn ciphertext_digest(self) -> SecureAggregateCommitment {
        self.ciphertext_digest
    }

    /// Returns the encoded slot count.
    #[must_use]
    pub const fn slot_count(self) -> NonZeroUsize {
        self.slot_count
    }

    /// Returns the fixture noise budget in bits.
    #[must_use]
    pub const fn noise_budget_bits(self) -> u16 {
        self.noise_budget_bits
    }
}

/// Evaluated ciphertext fixture for an FHE sum feasibility job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FheFeasibilityEvaluation {
    job_id: SecureAggregateTranscriptId,
    parameter_set: FheFeasibilityParameterSet,
    operation: FheFeasibilityOperation,
    participant_count: NonZeroUsize,
    evaluated_ciphertext_digest: SecureAggregateCommitment,
    aggregate_value_fixture: u64,
    noise_budget_bits: u16,
}

impl FheFeasibilityEvaluation {
    /// Returns the job identifier.
    #[must_use]
    pub const fn job_id(self) -> SecureAggregateTranscriptId {
        self.job_id
    }

    /// Returns the parameter set used for evaluation.
    #[must_use]
    pub const fn parameter_set(self) -> FheFeasibilityParameterSet {
        self.parameter_set
    }

    /// Returns the evaluated operation.
    #[must_use]
    pub const fn operation(self) -> FheFeasibilityOperation {
        self.operation
    }

    /// Returns the number of contributing participants.
    #[must_use]
    pub const fn participant_count(self) -> NonZeroUsize {
        self.participant_count
    }

    /// Returns the deterministic digest of the evaluated ciphertext fixture.
    #[must_use]
    pub const fn evaluated_ciphertext_digest(self) -> SecureAggregateCommitment {
        self.evaluated_ciphertext_digest
    }

    /// Returns the deterministic aggregate value fixture.
    #[must_use]
    pub const fn aggregate_value_fixture(self) -> u64 {
        self.aggregate_value_fixture
    }

    /// Returns the post-evaluation fixture noise budget.
    #[must_use]
    pub const fn noise_budget_bits(self) -> u16 {
        self.noise_budget_bits
    }
}

/// Derives the deterministic ciphertext digest for an FHE feasibility fixture.
pub fn derive_fhe_feasibility_ciphertext_digest(
    job_id: SecureAggregateTranscriptId,
    parameter_set: FheFeasibilityParameterSet,
    plaintext: FheFeasibilityPlaintext,
    nonce: FheFeasibilityNonce,
) -> Result<SecureAggregateCommitment, SecureAggregateError> {
    validate_fhe_feasibility_parameters(parameter_set)?;
    if plaintext.value() > parameter_set.max_plaintext_value() {
        return Err(SecureAggregateError::FhePlaintextOutsideRange);
    }

    let mut hasher = Sha256::new();
    hasher.update(FHE_FEASIBILITY_CIPHERTEXT_DOMAIN);
    update_len_prefixed(&mut hasher, FHE_FEASIBILITY_PROFILE.as_bytes());
    update_len_prefixed(
        &mut hasher,
        parameter_set.scheme_family().label().as_bytes(),
    );
    update_len_prefixed(&mut hasher, parameter_set.operation().label().as_bytes());
    hasher.update(job_id.bytes());
    hasher.update(parameter_set.polynomial_modulus_degree().to_be_bytes());
    hasher.update(parameter_set.plaintext_modulus().to_be_bytes());
    hasher.update(parameter_set.ciphertext_modulus_bits().to_be_bytes());
    hasher.update(parameter_set.security_level_bits().to_be_bytes());
    hasher.update((parameter_set.max_participant_count().get() as u128).to_be_bytes());
    hasher.update(parameter_set.max_plaintext_value().to_be_bytes());
    hasher.update((plaintext.participant_index().get() as u128).to_be_bytes());
    hasher.update(plaintext.value().to_be_bytes());
    hasher.update(nonce.bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES];
    bytes.copy_from_slice(&digest);
    SecureAggregateCommitment::new(bytes)
}

/// Creates a deterministic FHE feasibility ciphertext fixture.
///
/// This is a test and planning fixture, not production encryption. The fixture
/// hides the participant plaintext from public getters and serialization
/// boundaries, but it does not provide cryptographic confidentiality.
pub fn encrypt_fhe_feasibility_plaintext_fixture(
    plan: FheFeasibilityPlan,
    job_id: SecureAggregateTranscriptId,
    plaintext: FheFeasibilityPlaintext,
    nonce: FheFeasibilityNonce,
) -> Result<FheFeasibilityCiphertextFixture, SecureAggregateError> {
    if plaintext.participant_index().get() > plan.participant_count().get() {
        return Err(SecureAggregateError::ParticipantIndexOutOfRange);
    }
    let ciphertext_digest =
        derive_fhe_feasibility_ciphertext_digest(job_id, plan.parameter_set(), plaintext, nonce)?;
    Ok(FheFeasibilityCiphertextFixture::from_fixture_parts(
        job_id,
        plan.parameter_set(),
        plaintext.participant_index(),
        ciphertext_digest,
        NonZeroUsize::MIN,
        FHE_FEASIBILITY_INITIAL_NOISE_BUDGET_BITS,
        plaintext.value(),
    ))
}

/// Verifies that a deterministic FHE ciphertext fixture matches its opening.
pub fn verify_fhe_feasibility_ciphertext_fixture(
    job_id: SecureAggregateTranscriptId,
    parameter_set: FheFeasibilityParameterSet,
    plaintext: FheFeasibilityPlaintext,
    nonce: FheFeasibilityNonce,
    ciphertext: FheFeasibilityCiphertextFixture,
) -> Result<(), SecureAggregateError> {
    if ciphertext.job_id() != job_id {
        return Err(SecureAggregateError::FheCiphertextJobMismatch);
    }
    if ciphertext.parameter_set() != parameter_set {
        return Err(SecureAggregateError::FheCiphertextParameterMismatch);
    }
    if ciphertext.participant_index() != plaintext.participant_index() {
        return Err(SecureAggregateError::ParticipantIndexOutOfRange);
    }
    let expected =
        derive_fhe_feasibility_ciphertext_digest(job_id, parameter_set, plaintext, nonce)?;
    if ciphertext.ciphertext_digest() != expected {
        return Err(SecureAggregateError::FheCiphertextDigestMismatch);
    }
    Ok(())
}

/// Evaluates the deterministic FHE feasibility sum over one ciphertext per participant.
pub fn evaluate_fhe_feasibility_sum(
    plan: FheFeasibilityPlan,
    job_id: SecureAggregateTranscriptId,
    ciphertexts: &[FheFeasibilityCiphertextFixture],
) -> Result<FheFeasibilityEvaluation, SecureAggregateError> {
    if ciphertexts.len() != plan.participant_count().get() {
        return Err(SecureAggregateError::FheCiphertextCountMismatch);
    }
    let Some(first_ciphertext) = ciphertexts.first() else {
        return Err(SecureAggregateError::FheCiphertextCountMismatch);
    };

    let mut total = 0_u64;
    let mut minimum_noise_budget_bits = first_ciphertext.noise_budget_bits();
    for (offset, ciphertext) in ciphertexts.iter().enumerate() {
        if ciphertext.job_id() != job_id {
            return Err(SecureAggregateError::FheCiphertextJobMismatch);
        }
        if ciphertext.parameter_set() != plan.parameter_set() {
            return Err(SecureAggregateError::FheCiphertextParameterMismatch);
        }
        if ciphertext.participant_index().get() > plan.participant_count().get() {
            return Err(SecureAggregateError::ParticipantIndexOutOfRange);
        }
        if ciphertexts[..offset]
            .iter()
            .any(|previous| previous.participant_index() == ciphertext.participant_index())
        {
            return Err(SecureAggregateError::DuplicateParticipantIndex);
        }
        if ciphertext.fixture_plaintext_value > plan.parameter_set().max_plaintext_value() {
            return Err(SecureAggregateError::FhePlaintextOutsideRange);
        }
        total = total
            .checked_add(ciphertext.fixture_plaintext_value)
            .ok_or(SecureAggregateError::FheEvaluationOverflow)?;
        if total >= plan.parameter_set().plaintext_modulus() {
            return Err(SecureAggregateError::FheEvaluationOverflow);
        }
        minimum_noise_budget_bits = minimum_noise_budget_bits.min(ciphertext.noise_budget_bits());
    }

    let Some(noise_budget_bits) =
        minimum_noise_budget_bits.checked_sub(FHE_FEASIBILITY_SUM_NOISE_COST_BITS)
    else {
        return Err(SecureAggregateError::FheEvaluationOverflow);
    };
    let evaluated_ciphertext_digest =
        derive_fhe_feasibility_evaluation_digest(job_id, plan.parameter_set(), ciphertexts, total)?;

    Ok(FheFeasibilityEvaluation {
        job_id,
        parameter_set: plan.parameter_set(),
        operation: FheFeasibilityOperation::Sum,
        participant_count: plan.participant_count(),
        evaluated_ciphertext_digest,
        aggregate_value_fixture: total,
        noise_budget_bits,
    })
}

fn derive_fhe_feasibility_evaluation_digest(
    job_id: SecureAggregateTranscriptId,
    parameter_set: FheFeasibilityParameterSet,
    ciphertexts: &[FheFeasibilityCiphertextFixture],
    aggregate_value: u64,
) -> Result<SecureAggregateCommitment, SecureAggregateError> {
    let mut ordered_ciphertexts = ciphertexts
        .iter()
        .map(|ciphertext| {
            (
                ciphertext.participant_index().get(),
                ciphertext.ciphertext_digest().bytes(),
            )
        })
        .collect::<Vec<_>>();
    ordered_ciphertexts.sort_unstable_by_key(|(participant_index, _digest)| *participant_index);

    let mut hasher = Sha256::new();
    hasher.update(FHE_FEASIBILITY_EVALUATION_DOMAIN);
    update_len_prefixed(&mut hasher, FHE_FEASIBILITY_PROFILE.as_bytes());
    update_len_prefixed(
        &mut hasher,
        parameter_set.scheme_family().label().as_bytes(),
    );
    update_len_prefixed(&mut hasher, parameter_set.operation().label().as_bytes());
    hasher.update(job_id.bytes());
    hasher.update(parameter_set.polynomial_modulus_degree().to_be_bytes());
    hasher.update(parameter_set.plaintext_modulus().to_be_bytes());
    hasher.update(parameter_set.ciphertext_modulus_bits().to_be_bytes());
    hasher.update(parameter_set.security_level_bits().to_be_bytes());
    hasher.update((parameter_set.max_participant_count().get() as u128).to_be_bytes());
    hasher.update(parameter_set.max_plaintext_value().to_be_bytes());
    hasher.update((ordered_ciphertexts.len() as u128).to_be_bytes());
    for (participant_index, digest) in ordered_ciphertexts {
        hasher.update((participant_index as u128).to_be_bytes());
        hasher.update(digest);
    }
    hasher.update(aggregate_value.to_be_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES];
    bytes.copy_from_slice(&digest);
    SecureAggregateCommitment::new(bytes)
}
