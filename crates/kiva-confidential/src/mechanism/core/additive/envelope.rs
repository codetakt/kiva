#![allow(clippy::wildcard_imports)]

use super::super::*;

/// Inclusive replay window for authenticated additive-share envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "AdditiveShareReplayWindowUnchecked",
    into = "AdditiveShareReplayWindowUnchecked"
)]
pub struct AdditiveShareReplayWindow {
    pub(in crate::mechanism::core) first_sequence: SecureAggregateEnvelopeSequence,
    pub(in crate::mechanism::core) last_sequence: SecureAggregateEnvelopeSequence,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct AdditiveShareReplayWindowUnchecked {
    first_sequence: SecureAggregateEnvelopeSequence,
    last_sequence: SecureAggregateEnvelopeSequence,
}

impl AdditiveShareReplayWindow {
    /// Creates an inclusive replay window.
    pub const fn new(
        first_sequence: SecureAggregateEnvelopeSequence,
        last_sequence: SecureAggregateEnvelopeSequence,
    ) -> Result<Self, SecureAggregateError> {
        if first_sequence.value().get() > last_sequence.value().get() {
            return Err(SecureAggregateError::ReplayWindowLowerBoundExceedsUpperBound);
        }
        Ok(Self {
            first_sequence,
            last_sequence,
        })
    }

    /// Returns the first accepted sequence.
    #[must_use]
    pub const fn first_sequence(self) -> SecureAggregateEnvelopeSequence {
        self.first_sequence
    }

    /// Returns the last accepted sequence.
    #[must_use]
    pub const fn last_sequence(self) -> SecureAggregateEnvelopeSequence {
        self.last_sequence
    }

    /// Returns whether a sequence is inside this inclusive replay window.
    #[must_use]
    pub const fn contains(self, sequence: SecureAggregateEnvelopeSequence) -> bool {
        self.first_sequence.value().get() <= sequence.value().get()
            && sequence.value().get() <= self.last_sequence.value().get()
    }
}

impl TryFrom<AdditiveShareReplayWindowUnchecked> for AdditiveShareReplayWindow {
    type Error = SecureAggregateError;

    fn try_from(value: AdditiveShareReplayWindowUnchecked) -> Result<Self, Self::Error> {
        Self::new(value.first_sequence, value.last_sequence)
    }
}

impl From<AdditiveShareReplayWindow> for AdditiveShareReplayWindowUnchecked {
    fn from(value: AdditiveShareReplayWindow) -> Self {
        Self {
            first_sequence: value.first_sequence(),
            last_sequence: value.last_sequence(),
        }
    }
}

/// One participant's additive share for a secure aggregate transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecureAggregateShare {
    participant_index: NonZeroUsize,
    value: SecureAggregateFieldElement,
}

impl SecureAggregateShare {
    /// Creates a share for a one-based participant index.
    #[must_use]
    pub const fn new(participant_index: NonZeroUsize, value: SecureAggregateFieldElement) -> Self {
        Self {
            participant_index,
            value,
        }
    }

    /// Returns the one-based participant index.
    #[must_use]
    pub const fn participant_index(self) -> NonZeroUsize {
        self.participant_index
    }

    /// Returns the share value.
    #[must_use]
    pub const fn value(self) -> SecureAggregateFieldElement {
        self.value
    }
}

/// Expected commitment slot for one participant in an additive transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdditiveShareCommitmentSlot {
    transcript_id: SecureAggregateTranscriptId,
    participant_index: NonZeroUsize,
    commitment: SecureAggregateCommitment,
    signing_key_id: SecureAggregateSigningKeyId,
}

impl AdditiveShareCommitmentSlot {
    /// Creates a commitment slot for one participant.
    #[must_use]
    pub const fn new(
        transcript_id: SecureAggregateTranscriptId,
        participant_index: NonZeroUsize,
        commitment: SecureAggregateCommitment,
        signing_key_id: SecureAggregateSigningKeyId,
    ) -> Self {
        Self {
            transcript_id,
            participant_index,
            commitment,
            signing_key_id,
        }
    }

    /// Returns the transcript identifier.
    #[must_use]
    pub const fn transcript_id(self) -> SecureAggregateTranscriptId {
        self.transcript_id
    }

    /// Returns the one-based participant index.
    #[must_use]
    pub const fn participant_index(self) -> NonZeroUsize {
        self.participant_index
    }

    /// Returns the expected share commitment.
    #[must_use]
    pub const fn commitment(self) -> SecureAggregateCommitment {
        self.commitment
    }

    /// Returns the expected signing-key identifier.
    #[must_use]
    pub const fn signing_key_id(self) -> SecureAggregateSigningKeyId {
        self.signing_key_id
    }
}

/// Authenticated transport envelope for one additive share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticatedAdditiveShareEnvelope {
    transcript_id: SecureAggregateTranscriptId,
    share: SecureAggregateShare,
    commitment: SecureAggregateCommitment,
    signing_key_id: SecureAggregateSigningKeyId,
    sequence: SecureAggregateEnvelopeSequence,
    transport_proof: SecureAggregateTransportProof,
}

impl AuthenticatedAdditiveShareEnvelope {
    /// Creates an authenticated additive-share envelope.
    #[must_use]
    pub const fn new(
        transcript_id: SecureAggregateTranscriptId,
        share: SecureAggregateShare,
        commitment: SecureAggregateCommitment,
        signing_key_id: SecureAggregateSigningKeyId,
        sequence: SecureAggregateEnvelopeSequence,
        transport_proof: SecureAggregateTransportProof,
    ) -> Self {
        Self {
            transcript_id,
            share,
            commitment,
            signing_key_id,
            sequence,
            transport_proof,
        }
    }

    /// Returns the transcript identifier.
    #[must_use]
    pub const fn transcript_id(self) -> SecureAggregateTranscriptId {
        self.transcript_id
    }

    /// Returns the enclosed additive share.
    #[must_use]
    pub const fn share(self) -> SecureAggregateShare {
        self.share
    }

    /// Returns the commitment claimed by the envelope.
    #[must_use]
    pub const fn commitment(self) -> SecureAggregateCommitment {
        self.commitment
    }

    /// Returns the signing-key identifier claimed by the envelope.
    #[must_use]
    pub const fn signing_key_id(self) -> SecureAggregateSigningKeyId {
        self.signing_key_id
    }

    /// Returns the envelope sequence used for replay-window checks.
    #[must_use]
    pub const fn sequence(self) -> SecureAggregateEnvelopeSequence {
        self.sequence
    }

    /// Returns the transport proof tag carried by the envelope.
    #[must_use]
    pub const fn transport_proof(self) -> SecureAggregateTransportProof {
        self.transport_proof
    }
}

/// Signed authenticated transport envelope for one additive share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedAuthenticatedAdditiveShareEnvelope {
    envelope: AuthenticatedAdditiveShareEnvelope,
    signature: SecureAggregateSignature,
}

impl SignedAuthenticatedAdditiveShareEnvelope {
    /// Creates a signed authenticated additive-share envelope.
    #[must_use]
    pub const fn new(
        envelope: AuthenticatedAdditiveShareEnvelope,
        signature: SecureAggregateSignature,
    ) -> Self {
        Self {
            envelope,
            signature,
        }
    }

    /// Returns the signed envelope body.
    #[must_use]
    pub const fn envelope(self) -> AuthenticatedAdditiveShareEnvelope {
        self.envelope
    }

    /// Returns the Ed25519 signature.
    #[must_use]
    pub const fn signature(self) -> SecureAggregateSignature {
        self.signature
    }
}

/// Builds the canonical payload that additive-share envelope signatures cover.
///
/// The payload is a domain-separated binary projection of the envelope fields.
/// Signers and verifiers must use this exact byte sequence.
#[must_use]
pub fn additive_share_envelope_signature_payload(
    envelope: AuthenticatedAdditiveShareEnvelope,
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(
        ADDITIVE_SHARE_ENVELOPE_SIGNATURE_DOMAIN.len()
            + SECURE_AGGREGATE_ARTIFACT_BYTES
            + 16
            + 8
            + SECURE_AGGREGATE_ARTIFACT_BYTES
            + SECURE_AGGREGATE_ARTIFACT_BYTES
            + 8
            + SECURE_AGGREGATE_ARTIFACT_BYTES,
    );
    payload.extend_from_slice(ADDITIVE_SHARE_ENVELOPE_SIGNATURE_DOMAIN);
    payload.extend_from_slice(&envelope.transcript_id().bytes());
    payload.extend_from_slice(&(envelope.share().participant_index().get() as u128).to_be_bytes());
    payload.extend_from_slice(&envelope.share().value().value().to_be_bytes());
    payload.extend_from_slice(&envelope.commitment().bytes());
    payload.extend_from_slice(&envelope.signing_key_id().bytes());
    payload.extend_from_slice(&envelope.sequence().value().get().to_be_bytes());
    payload.extend_from_slice(&envelope.transport_proof().bytes());
    payload
}
