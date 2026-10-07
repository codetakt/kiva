//! Fixed-width confidential-computation primitive types.

use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};

use crate::SecureAggregateError;

/// Prime field modulus used by the first additive secure-aggregate transcript.
///
/// This is the largest 32-bit prime. Keeping the modulus below `u32::MAX`
/// lets the verifier reason about addition using ordinary `u64` arithmetic.
pub const SECURE_AGGREGATE_FIELD_MODULUS: u64 = 4_294_967_291;

/// Byte width for transcript identifiers, commitments, key identifiers, and proof tags.
pub const SECURE_AGGREGATE_ARTIFACT_BYTES: usize = 32;

/// Byte width for Ed25519 signatures carried by signed additive-share envelopes.
pub const SECURE_AGGREGATE_SIGNATURE_BYTES: usize = 64;

#[allow(clippy::redundant_pub_crate)]
pub(crate) fn artifact_is_empty(bytes: &[u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> bool {
    bytes.iter().all(|byte| *byte == 0)
}

#[allow(clippy::redundant_pub_crate)]
pub(crate) fn signature_is_empty(bytes: &[u8; SECURE_AGGREGATE_SIGNATURE_BYTES]) -> bool {
    bytes.iter().all(|byte| *byte == 0)
}

/// Element of the fixed finite field used by additive secure aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct SecureAggregateFieldElement {
    pub(crate) value: u64,
}

impl SecureAggregateFieldElement {
    /// The additive identity.
    pub const ZERO: Self = Self { value: 0 };

    /// The multiplicative identity.
    pub const ONE: Self = Self { value: 1 };

    /// Creates a field element after validating the configured modulus.
    pub const fn new(value: u64) -> Result<Self, SecureAggregateError> {
        if value >= SECURE_AGGREGATE_FIELD_MODULUS {
            return Err(SecureAggregateError::ValueOutsideField);
        }
        Ok(Self { value })
    }

    /// Returns the canonical integer representation.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.value
    }

    /// Adds two field elements modulo the configured modulus.
    #[must_use]
    pub const fn add_mod(self, other: Self) -> Self {
        let sum = self.value + other.value;
        if sum >= SECURE_AGGREGATE_FIELD_MODULUS {
            Self {
                value: sum - SECURE_AGGREGATE_FIELD_MODULUS,
            }
        } else {
            Self { value: sum }
        }
    }

    /// Subtracts two field elements modulo the configured modulus.
    #[must_use]
    pub const fn sub_mod(self, other: Self) -> Self {
        if self.value >= other.value {
            Self {
                value: self.value - other.value,
            }
        } else {
            Self {
                value: SECURE_AGGREGATE_FIELD_MODULUS - (other.value - self.value),
            }
        }
    }

    /// Multiplies two field elements modulo the configured modulus.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn mul_mod(self, other: Self) -> Self {
        let product = u128::from(self.value) * u128::from(other.value);
        let value = (product % u128::from(SECURE_AGGREGATE_FIELD_MODULUS)) as u64;
        Self { value }
    }

    /// Returns the multiplicative inverse for a non-zero field element.
    pub fn inv_mod(self) -> Result<Self, SecureAggregateError> {
        if self.value == 0 {
            return Err(SecureAggregateError::FieldElementHasNoInverse);
        }
        Ok(self.pow_mod(SECURE_AGGREGATE_FIELD_MODULUS - 2))
    }

    fn pow_mod(self, mut exponent: u64) -> Self {
        let mut base = self;
        let mut result = Self::ONE;
        while exponent > 0 {
            if exponent % 2 == 1 {
                result = result.mul_mod(base);
            }
            base = base.mul_mod(base);
            exponent /= 2;
        }
        result
    }
}

impl TryFrom<u64> for SecureAggregateFieldElement {
    type Error = SecureAggregateError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<SecureAggregateFieldElement> for u64 {
    fn from(value: SecureAggregateFieldElement) -> Self {
        value.value()
    }
}

/// Identifier for one additive secure aggregate transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "[u8; 32]", into = "[u8; 32]")]
pub struct SecureAggregateTranscriptId {
    pub(crate) bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES],
}

impl SecureAggregateTranscriptId {
    /// Creates a transcript identifier after rejecting the reserved empty value.
    pub fn new(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, SecureAggregateError> {
        if artifact_is_empty(&bytes) {
            return Err(SecureAggregateError::EmptyTranscriptIdentifier);
        }
        Ok(Self { bytes })
    }

    /// Returns the canonical bytes for this transcript identifier.
    #[must_use]
    pub const fn bytes(self) -> [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
        self.bytes
    }
}

impl TryFrom<[u8; SECURE_AGGREGATE_ARTIFACT_BYTES]> for SecureAggregateTranscriptId {
    type Error = SecureAggregateError;

    fn try_from(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, Self::Error> {
        Self::new(bytes)
    }
}

impl From<SecureAggregateTranscriptId> for [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
    fn from(value: SecureAggregateTranscriptId) -> Self {
        value.bytes()
    }
}

/// Commitment identifier for a participant share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "[u8; 32]", into = "[u8; 32]")]
pub struct SecureAggregateCommitment {
    pub(crate) bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES],
}

impl SecureAggregateCommitment {
    /// Creates a share commitment after rejecting the reserved empty value.
    pub fn new(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, SecureAggregateError> {
        if artifact_is_empty(&bytes) {
            return Err(SecureAggregateError::EmptyShareCommitment);
        }
        Ok(Self { bytes })
    }

    /// Returns the canonical bytes for this commitment identifier.
    #[must_use]
    pub const fn bytes(self) -> [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
        self.bytes
    }
}

impl TryFrom<[u8; SECURE_AGGREGATE_ARTIFACT_BYTES]> for SecureAggregateCommitment {
    type Error = SecureAggregateError;

    fn try_from(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, Self::Error> {
        Self::new(bytes)
    }
}

impl From<SecureAggregateCommitment> for [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
    fn from(value: SecureAggregateCommitment) -> Self {
        value.bytes()
    }
}

/// Identifier for a participant's share-signing key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "[u8; 32]", into = "[u8; 32]")]
pub struct SecureAggregateSigningKeyId {
    pub(crate) bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES],
}

impl SecureAggregateSigningKeyId {
    /// Creates a signing-key identifier after rejecting the reserved empty value.
    pub fn new(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, SecureAggregateError> {
        if artifact_is_empty(&bytes) {
            return Err(SecureAggregateError::EmptySigningKeyIdentifier);
        }
        Ok(Self { bytes })
    }

    /// Returns the canonical bytes for this signing-key identifier.
    #[must_use]
    pub const fn bytes(self) -> [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
        self.bytes
    }
}

impl TryFrom<[u8; SECURE_AGGREGATE_ARTIFACT_BYTES]> for SecureAggregateSigningKeyId {
    type Error = SecureAggregateError;

    fn try_from(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, Self::Error> {
        Self::new(bytes)
    }
}

impl From<SecureAggregateSigningKeyId> for [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
    fn from(value: SecureAggregateSigningKeyId) -> Self {
        value.bytes()
    }
}

/// Ed25519 public key used to verify participant share signatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "[u8; 32]", into = "[u8; 32]")]
pub struct SecureAggregateSigningPublicKey {
    pub(crate) bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES],
}

impl SecureAggregateSigningPublicKey {
    /// Creates a signing public key after rejecting the reserved empty value.
    pub fn new(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, SecureAggregateError> {
        if artifact_is_empty(&bytes) {
            return Err(SecureAggregateError::EmptySigningPublicKey);
        }
        Ok(Self { bytes })
    }

    /// Returns the canonical bytes for this signing public key.
    #[must_use]
    pub const fn bytes(self) -> [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
        self.bytes
    }
}

impl TryFrom<[u8; SECURE_AGGREGATE_ARTIFACT_BYTES]> for SecureAggregateSigningPublicKey {
    type Error = SecureAggregateError;

    fn try_from(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, Self::Error> {
        Self::new(bytes)
    }
}

impl From<SecureAggregateSigningPublicKey> for [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
    fn from(value: SecureAggregateSigningPublicKey) -> Self {
        value.bytes()
    }
}

/// Ed25519 signature carried by a signed additive-share envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecureAggregateSignature {
    pub(crate) bytes: [u8; SECURE_AGGREGATE_SIGNATURE_BYTES],
}

impl SecureAggregateSignature {
    /// Creates a signature after rejecting the reserved empty value.
    pub fn new(
        bytes: [u8; SECURE_AGGREGATE_SIGNATURE_BYTES],
    ) -> Result<Self, SecureAggregateError> {
        if signature_is_empty(&bytes) {
            return Err(SecureAggregateError::EmptySignature);
        }
        Ok(Self { bytes })
    }

    /// Returns the canonical bytes for this signature.
    #[must_use]
    pub const fn bytes(self) -> [u8; SECURE_AGGREGATE_SIGNATURE_BYTES] {
        self.bytes
    }
}

impl TryFrom<[u8; SECURE_AGGREGATE_SIGNATURE_BYTES]> for SecureAggregateSignature {
    type Error = SecureAggregateError;

    fn try_from(bytes: [u8; SECURE_AGGREGATE_SIGNATURE_BYTES]) -> Result<Self, Self::Error> {
        Self::new(bytes)
    }
}

impl From<SecureAggregateSignature> for [u8; SECURE_AGGREGATE_SIGNATURE_BYTES] {
    fn from(value: SecureAggregateSignature) -> Self {
        value.bytes()
    }
}

impl TryFrom<Vec<u8>> for SecureAggregateSignature {
    type Error = SecureAggregateError;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        if bytes.len() != SECURE_AGGREGATE_SIGNATURE_BYTES {
            return Err(SecureAggregateError::SignatureLengthMismatch);
        }

        let mut signature_bytes = [0_u8; SECURE_AGGREGATE_SIGNATURE_BYTES];
        signature_bytes.copy_from_slice(&bytes);
        Self::new(signature_bytes)
    }
}

impl Serialize for SecureAggregateSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.bytes.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecureAggregateSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        Self::try_from(bytes).map_err(serde::de::Error::custom)
    }
}

/// Verification key registered for one additive-share signer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdditiveShareVerificationKey {
    pub(crate) signing_key_id: SecureAggregateSigningKeyId,
    pub(crate) public_key: SecureAggregateSigningPublicKey,
}

impl AdditiveShareVerificationKey {
    /// Creates a verification key entry for one signing-key identifier.
    #[must_use]
    pub const fn new(
        signing_key_id: SecureAggregateSigningKeyId,
        public_key: SecureAggregateSigningPublicKey,
    ) -> Self {
        Self {
            signing_key_id,
            public_key,
        }
    }

    /// Returns the signing-key identifier.
    #[must_use]
    pub const fn signing_key_id(self) -> SecureAggregateSigningKeyId {
        self.signing_key_id
    }

    /// Returns the Ed25519 public key.
    #[must_use]
    pub const fn public_key(self) -> SecureAggregateSigningPublicKey {
        self.public_key
    }
}

/// Proof tag carried by an authenticated share transport envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "[u8; 32]", into = "[u8; 32]")]
pub struct SecureAggregateTransportProof {
    pub(crate) bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES],
}

impl SecureAggregateTransportProof {
    /// Creates a transport proof tag after rejecting the reserved empty value.
    pub fn new(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, SecureAggregateError> {
        if artifact_is_empty(&bytes) {
            return Err(SecureAggregateError::EmptyTransportProof);
        }
        Ok(Self { bytes })
    }

    /// Returns the canonical bytes for this transport proof tag.
    #[must_use]
    pub const fn bytes(self) -> [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
        self.bytes
    }
}

impl TryFrom<[u8; SECURE_AGGREGATE_ARTIFACT_BYTES]> for SecureAggregateTransportProof {
    type Error = SecureAggregateError;

    fn try_from(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, Self::Error> {
        Self::new(bytes)
    }
}

impl From<SecureAggregateTransportProof> for [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
    fn from(value: SecureAggregateTransportProof) -> Self {
        value.bytes()
    }
}

/// Monotonic envelope sequence used by replay-window checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct SecureAggregateEnvelopeSequence {
    pub(crate) value: NonZeroU64,
}

impl SecureAggregateEnvelopeSequence {
    /// Creates an envelope sequence.
    #[must_use]
    pub const fn new(value: NonZeroU64) -> Self {
        Self { value }
    }

    /// Returns the canonical integer representation.
    #[must_use]
    pub const fn value(self) -> NonZeroU64 {
        self.value
    }
}

impl TryFrom<u64> for SecureAggregateEnvelopeSequence {
    type Error = SecureAggregateError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let Some(value) = NonZeroU64::new(value) else {
            return Err(SecureAggregateError::EnvelopeSequenceMustNotBeZero);
        };
        Ok(Self::new(value))
    }
}

impl From<SecureAggregateEnvelopeSequence> for u64 {
    fn from(value: SecureAggregateEnvelopeSequence) -> Self {
        value.value().get()
    }
}
