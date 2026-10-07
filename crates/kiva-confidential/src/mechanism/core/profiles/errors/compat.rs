//! Compatibility facade for legacy confidential-computation error users.

use thiserror::Error;

/// Compatibility error facade for the historical additive aggregate API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SecureAggregateCompatibilityError {
    /// The value is not a member of the configured finite field.
    #[error("secure aggregate field element must be below the field modulus")]
    ValueOutsideField,

    /// The additive mechanism requires all participants for release.
    #[error("additive secure aggregation requires an n-of-n release threshold")]
    ThresholdMustRequireAllParticipants,

    /// The PSI mechanism requires all participants for computation.
    #[error("private set intersection requires an n-of-n participant threshold")]
    PsiRequiresAllParticipants,

    /// A PSI namespace is empty after canonicalization.
    #[error("private set intersection namespace must not be empty")]
    PsiNamespaceEmpty,

    /// A PSI namespace contains an ASCII control character.
    #[error("private set intersection namespace contains a control character")]
    PsiNamespaceContainsControl,

    /// A PSI identifier is empty after canonicalization.
    #[error("private set intersection identifier must not be empty")]
    PsiIdentifierEmpty,

    /// A PSI identifier contains an ASCII control character.
    #[error("private set intersection identifier contains a control character")]
    PsiIdentifierContainsControl,

    /// A PSI identifier digest is the reserved all-zero value.
    #[error("private set intersection identifier digest must not be empty")]
    EmptyPsiIdentifierDigest,

    /// A PSI dataset contains no identifier digests.
    #[error("private set intersection participant dataset must not be empty")]
    PsiParticipantDatasetEmpty,

    /// A PSI job does not contain exactly one dataset per participant.
    #[error("private set intersection participant dataset count does not match participant count")]
    PsiParticipantDatasetCountMismatch,

    /// A PSI participant dataset contains duplicate identifier digests.
    #[error("private set intersection participant dataset contains duplicate identifier digests")]
    DuplicatePsiIdentifierDigest,

    /// A PSI job identifier has already been accepted.
    #[error("private set intersection job was already accepted")]
    PsiReplayDetected,

    /// A DH-OPRF PSI runtime suite is unsupported for the CC11 boundary.
    #[error("DH-OPRF PSI runtime suite is unsupported")]
    PsiOprfUnsupportedSuite,

    /// A DH-OPRF PSI runtime binding is missing.
    #[error("DH-OPRF PSI runtime binding is missing")]
    PsiOprfRuntimeBindingMissing,

    /// External cryptographic review is required before production DH-OPRF PSI release.
    #[error("DH-OPRF PSI external cryptographic review is required")]
    PsiOprfExternalReviewRequired,

    /// A DH-OPRF PSI element is the reserved all-zero value.
    #[error("DH-OPRF PSI element must not be empty")]
    EmptyPsiOprfElement,

    /// A DH-OPRF PSI participant transcript contains no elements.
    #[error("DH-OPRF PSI participant transcript must not be empty")]
    PsiOprfParticipantTranscriptEmpty,

    /// A DH-OPRF PSI job does not contain exactly one transcript per participant.
    #[error("DH-OPRF PSI participant transcript count does not match participant count")]
    PsiOprfParticipantTranscriptCountMismatch,

    /// A DH-OPRF PSI participant transcript contains duplicate elements.
    #[error("DH-OPRF PSI participant transcript contains duplicate elements")]
    DuplicatePsiOprfElement,

    /// A DH-OPRF PSI job identifier has already been accepted.
    #[error("DH-OPRF PSI transcript replay was detected")]
    PsiOprfTranscriptReplayDetected,

    /// A DH-OPRF PSI participant aborted the protocol.
    #[error("DH-OPRF PSI transcript was aborted")]
    PsiOprfTranscriptAborted,

    /// A DH-OPRF PSI release requires a computed intersection.
    #[error("DH-OPRF PSI release requires a computed intersection")]
    PsiOprfReleaseRequiresIntersection,

    /// A DH-OPRF PSI release requires policy-authority approval.
    #[error("DH-OPRF PSI release requires policy approval")]
    PsiOprfReleaseRequiresPolicyApproval,

    /// A DH-OPRF PSI release requires governed release treatment.
    #[error("DH-OPRF PSI release requires governed release treatment")]
    PsiOprfReleaseRequiresGovernedDataProduct,

    /// A DH-OPRF PSI release requires active admitted participants.
    #[error("DH-OPRF PSI release requires active participants")]
    PsiOprfReleaseRequiresActiveParticipants,

    /// The TEE evidence gate requires all configured participants.
    #[error("TEE attestation gate requires an n-of-n participant threshold")]
    TeeRequiresAllParticipants,

    /// The TEE attestation family or format is unsupported for this policy.
    #[error("TEE attestation evidence format is unsupported")]
    TeeUnsupportedAttestationFormat,

    /// The configured policy authority has not approved attested execution.
    #[error("TEE attestation policy approval is missing")]
    TeePolicyApprovalMissing,

    /// The attestation evidence is not bound to the configured job.
    #[error("TEE attestation job binding does not match")]
    TeeJobBindingMismatch,

    /// The attested measurement does not match the configured measurement.
    #[error("TEE attestation measurement does not match")]
    TeeMeasurementMismatch,

    /// The attested image digest does not match the configured image provenance.
    #[error("TEE attestation image digest does not match")]
    TeeImageDigestMismatch,

    /// The attested policy binding does not match the configured policy.
    #[error("TEE attestation policy binding does not match")]
    TeePolicyBindingMismatch,

    /// The attestation evidence validity window is structurally invalid.
    #[error("TEE attestation validity window is invalid")]
    TeeEvidenceWindowInvalid,

    /// The attestation evidence is not yet valid for the configured clock.
    #[error("TEE attestation evidence is not yet valid")]
    TeeEvidenceNotYetValid,

    /// The attestation evidence is stale for the configured clock.
    #[error("TEE attestation evidence is expired")]
    TeeEvidenceExpired,

    /// The attestation evidence sequence is below the accepted minimum.
    #[error("TEE attestation rollback was detected")]
    TeeRollbackDetected,

    /// The attestation evidence has already been accepted.
    #[error("TEE attestation replay was detected")]
    TeeReplayDetected,

    /// The attestation evidence digest does not match its canonical claims.
    #[error("TEE attestation evidence digest does not match")]
    TeeEvidenceDigestMismatch,

    /// A secret or result release requires an accepted attestation first.
    #[error("TEE attestation is required before secret provisioning or release")]
    TeeAttestationRequired,

    /// The AWS Nitro verifier binding is missing or stale.
    #[error("AWS Nitro TEE verifier binding is missing or stale")]
    TeeNitroVerifierBindingMissing,

    /// External security review is required before AWS Nitro production release.
    #[error("AWS Nitro TEE external security review is required")]
    TeeNitroExternalReviewRequired,

    /// The AWS Nitro attestation certificate chain was not verified.
    #[error("AWS Nitro attestation certificate chain is invalid")]
    TeeNitroCertificateChainInvalid,

    /// The AWS Nitro attestation COSE signature was not verified.
    #[error("AWS Nitro attestation COSE signature is invalid")]
    TeeNitroCoseSignatureInvalid,

    /// The AWS Nitro trust anchor does not match policy.
    #[error("AWS Nitro attestation trust anchor does not match")]
    TeeNitroTrustAnchorMismatch,

    /// AWS Nitro debug-mode evidence is not admitted for key or result release.
    #[error("AWS Nitro debug-mode attestation is rejected")]
    TeeNitroDebugModeRejected,

    /// AWS Nitro PCR measurements do not match the configured policy.
    #[error("AWS Nitro attestation PCR policy does not match")]
    TeeNitroPcrMismatch,

    /// The AWS Nitro attestation nonce does not match the configured challenge.
    #[error("AWS Nitro attestation nonce does not match")]
    TeeNitroNonceMismatch,

    /// AWS Nitro all-zero PCR measurements are rejected.
    #[error("AWS Nitro attestation PCR values must not be all zero")]
    TeeNitroPcrAllZeroRejected,

    /// The AWS Nitro verifier key epoch is below the configured minimum.
    #[error("AWS Nitro verifier key rotation is required")]
    TeeNitroVerifierKeyRotationRequired,

    /// AWS Nitro result release requires prior secret provisioning.
    #[error("AWS Nitro TEE secret provisioning is required before result release")]
    TeeNitroSecretProvisioningRequired,

    /// The FHE feasibility profile requires all configured participants.
    #[error("FHE feasibility profile requires an n-of-n participant threshold")]
    FheRequiresAllParticipants,

    /// The FHE feasibility profile or operation is unsupported.
    #[error("FHE feasibility profile or operation is unsupported")]
    FheUnsupportedProfile,

    /// The FHE feasibility parameter set is outside the admitted profile.
    #[error("FHE feasibility parameter set is rejected")]
    FheParameterSetRejected,

    /// A participant plaintext is outside the admitted feasibility range.
    #[error("FHE feasibility plaintext is outside the admitted range")]
    FhePlaintextOutsideRange,

    /// A FHE feasibility nonce is the reserved all-zero value.
    #[error("FHE feasibility nonce must not be empty")]
    EmptyFheNonce,

    /// The submitted FHE ciphertext count does not match the participant count.
    #[error("FHE feasibility ciphertext count does not match participant count")]
    FheCiphertextCountMismatch,

    /// A FHE ciphertext belongs to a different job.
    #[error("FHE feasibility ciphertext job does not match")]
    FheCiphertextJobMismatch,

    /// A FHE ciphertext belongs to a different parameter set.
    #[error("FHE feasibility ciphertext parameter set does not match")]
    FheCiphertextParameterMismatch,

    /// A FHE ciphertext digest does not match the deterministic fixture.
    #[error("FHE feasibility ciphertext digest does not match")]
    FheCiphertextDigestMismatch,

    /// FHE sum evaluation would overflow or wrap the plaintext modulus.
    #[error("FHE feasibility sum evaluation overflowed")]
    FheEvaluationOverflow,

    /// Releasing an FHE result requires a completed evaluation.
    #[error("FHE feasibility release requires completed evaluation")]
    FheReleaseRequiresEvaluation,

    /// Releasing an FHE result requires policy approval.
    #[error("FHE feasibility release requires policy approval")]
    FheReleaseRequiresPolicyApproval,

    /// Releasing an FHE result requires governed release treatment.
    #[error("FHE feasibility release requires governed release treatment")]
    FheReleaseRequiresGovernedDataProduct,

    /// The plaintext/ciphertext boundary was violated before FHE release.
    #[error("FHE feasibility plaintext boundary was violated")]
    FhePlaintextBoundaryViolation,

    /// The FHE runtime binding is missing mandatory digest metadata.
    #[error("FHE runtime binding metadata is missing")]
    FheRuntimeBindingMissing,

    /// The FHE runtime has not completed external cryptographic review.
    #[error("FHE runtime external cryptographic review is required")]
    FheRuntimeExternalReviewRequired,

    /// The FHE runtime parameter set is outside the admitted profile.
    #[error("FHE runtime parameter set is rejected")]
    FheRuntimeParameterSetRejected,

    /// FHE runtime key custody metadata is missing.
    #[error("FHE runtime key custody metadata is missing")]
    FheRuntimeKeyCustodyMissing,

    /// The submitted FHE runtime ciphertext count does not match the participant count.
    #[error("FHE runtime ciphertext count does not match participant count")]
    FheRuntimeCiphertextCountMismatch,

    /// An FHE runtime ciphertext participant is inactive or removed.
    #[error("FHE runtime ciphertext participant is inactive")]
    FheRuntimeCiphertextParticipantInactive,

    /// An FHE runtime ciphertext participant aborted before submission.
    #[error("FHE runtime ciphertext participant aborted")]
    FheRuntimeCiphertextParticipantAborted,

    /// An FHE runtime participant ciphertext is bound to the wrong parameter set.
    #[error("FHE runtime ciphertext parameter set does not match")]
    FheRuntimeCiphertextParameterMismatch,

    /// An FHE runtime ciphertext transcript digest does not match.
    #[error("FHE runtime ciphertext transcript digest does not match")]
    FheRuntimeCiphertextDigestMismatch,

    /// An FHE runtime ciphertext transcript was replayed.
    #[error("FHE runtime ciphertext transcript replay detected")]
    FheRuntimeCiphertextReplayDetected,

    /// FHE runtime evaluation requires an accepted ciphertext transcript.
    #[error("FHE runtime evaluation requires accepted ciphertexts")]
    FheRuntimeEvaluationRequiresCiphertexts,

    /// An FHE runtime evaluation digest does not match.
    #[error("FHE runtime evaluation digest does not match")]
    FheRuntimeEvaluationDigestMismatch,

    /// An FHE runtime evaluation was replayed.
    #[error("FHE runtime evaluation replay detected")]
    FheRuntimeEvaluationReplayDetected,

    /// FHE runtime result release requires an accepted ciphertext transcript.
    #[error("FHE runtime release requires accepted ciphertext transcript")]
    FheRuntimeReleaseRequiresCiphertextTranscript,

    /// FHE runtime result release requires a completed evaluation.
    #[error("FHE runtime release requires completed evaluation")]
    FheRuntimeReleaseRequiresEvaluation,

    /// FHE runtime result release requires policy approval.
    #[error("FHE runtime release requires policy approval")]
    FheRuntimeReleaseRequiresPolicyApproval,

    /// FHE runtime result release requires governed release treatment.
    #[error("FHE runtime release requires governed release treatment")]
    FheRuntimeReleaseRequiresGovernedDataProduct,

    /// FHE runtime result release requires completed external review.
    #[error("FHE runtime release requires external review")]
    FheRuntimeReleaseRequiresExternalReview,

    /// FHE runtime result release requires an active key epoch.
    #[error("FHE runtime release requires an active key epoch")]
    FheRuntimeReleaseRequiresActiveKeyEpoch,

    /// A secret/confidential-compute provider profile is not supported.
    #[error("secret compute provider profile is unsupported")]
    SecretComputeProviderUnsupportedProfile,

    /// The reference primitive provider must use the reference provider kind.
    #[error("reference primitive provider kind does not match its profile")]
    ReferencePrimitiveProviderKindMismatch,

    /// Reference primitive providers must not be used for production claims.
    #[error("reference primitive provider cannot support a production claim")]
    ReferencePrimitiveProviderProductionClaimRejected,

    /// External providers require review evidence before production claims.
    #[error("external secret compute provider review evidence is required")]
    ExternalProviderReviewRequired,

    /// Runtime external providers require parameter-set evidence.
    #[error("external secret compute provider parameter-set evidence is required")]
    SecretComputeProviderParameterSetRequired,

    /// Provider capability does not match the selected mechanism family.
    #[error("secret compute provider capability does not match the mechanism")]
    SecretComputeProviderCapabilityMismatch,

    /// Shamir threshold reconstruction requires a threshold of at least two.
    #[error("Shamir threshold reconstruction requires a threshold of at least two")]
    ShamirThresholdTooSmall,

    /// Shamir participant indexes must fit in the configured finite field.
    #[error("Shamir participant count is outside the configured finite field")]
    ShamirParticipantCountOutsideField,

    /// The transcript does not contain enough Shamir shares for reconstruction.
    #[error("Shamir reconstruction requires at least threshold unique shares")]
    ShamirShareCountBelowThreshold,

    /// A field element needed as a denominator has no multiplicative inverse.
    #[error("secure aggregate field element has no multiplicative inverse")]
    FieldElementHasNoInverse,

    /// The transcript does not contain exactly one share per participant.
    #[error("secure aggregate share count does not match participant count")]
    ShareCountMismatch,

    /// A share names a participant index outside the configured participant set.
    #[error("secure aggregate participant index is outside the participant set")]
    ParticipantIndexOutOfRange,

    /// A participant contributed more than one share to the transcript.
    #[error("secure aggregate transcript contains a duplicate participant index")]
    DuplicateParticipantIndex,

    /// The transcript identifier is the reserved all-zero value.
    #[error("secure aggregate transcript identifier must not be empty")]
    EmptyTranscriptIdentifier,

    /// The share commitment is the reserved all-zero value.
    #[error("secure aggregate share commitment must not be empty")]
    EmptyShareCommitment,

    /// The Shamir share-opening nonce is the reserved all-zero value.
    #[error("Shamir share-opening nonce must not be empty")]
    EmptyShamirShareOpeningNonce,

    /// A Shamir share opening does not match the published commitment.
    #[error("Shamir share opening does not match its commitment")]
    ShamirShareCommitmentMismatch,

    /// The FROST threshold runtime binding is missing.
    #[error("FROST threshold runtime binding is missing")]
    ThresholdFrostRuntimeBindingMissing,

    /// External cryptographic review is required before FROST production release.
    #[error("FROST threshold external cryptographic review is required")]
    ThresholdFrostExternalReviewRequired,

    /// A FROST key ceremony does not contain exactly one transcript per participant.
    #[error("FROST threshold key ceremony participant count does not match")]
    ThresholdFrostCeremonyParticipantCountMismatch,

    /// A FROST key ceremony participant was removed or inactive.
    #[error("FROST threshold key ceremony participant is inactive")]
    ThresholdFrostParticipantInactive,

    /// A FROST participant aborted before a complete transcript existed.
    #[error("FROST threshold participant aborted")]
    ThresholdFrostParticipantAborted,

    /// A FROST complaint remains unresolved.
    #[error("FROST threshold complaint is unresolved")]
    ThresholdFrostComplaintUnresolved,

    /// FROST equivocation evidence was detected.
    #[error("FROST threshold equivocation was detected")]
    ThresholdFrostEquivocationDetected,

    /// FROST share custody evidence is missing.
    #[error("FROST threshold share custody evidence is missing")]
    ThresholdFrostShareCustodyMissing,

    /// A FROST key ceremony or signing transcript was replayed.
    #[error("FROST threshold transcript replay was detected")]
    ThresholdFrostReplayDetected,

    /// A FROST signing transcript has fewer than threshold signature shares.
    #[error("FROST threshold signing requires threshold signature shares")]
    ThresholdFrostSignatureShareCountBelowThreshold,

    /// A FROST signature digest does not match the transcript.
    #[error("FROST threshold signature digest does not match")]
    ThresholdFrostSignatureDigestMismatch,

    /// Releasing a FROST threshold signature requires a completed key ceremony.
    #[error("FROST threshold release requires a completed key ceremony")]
    ThresholdFrostReleaseRequiresKeyCeremony,

    /// Releasing a FROST threshold signature requires a computed signature.
    #[error("FROST threshold release requires a computed signature")]
    ThresholdFrostReleaseRequiresSignature,

    /// Releasing a FROST threshold signature requires policy approval.
    #[error("FROST threshold release requires policy approval")]
    ThresholdFrostReleaseRequiresPolicyApproval,

    /// Releasing a FROST threshold signature requires an active key epoch.
    #[error("FROST threshold release requires an active key epoch")]
    ThresholdFrostReleaseRequiresActiveKeyEpoch,

    /// Releasing a FROST threshold signature requires governed key use.
    #[error("FROST threshold release requires governed key use")]
    ThresholdFrostReleaseRequiresGovernedKeyUse,

    /// The share signing key identifier is the reserved all-zero value.
    #[error("secure aggregate signing key identifier must not be empty")]
    EmptySigningKeyIdentifier,

    /// The transport proof tag is the reserved all-zero value.
    #[error("secure aggregate transport proof must not be empty")]
    EmptyTransportProof,

    /// The transcript does not contain exactly one commitment slot per participant.
    #[error("secure aggregate commitment slot count does not match participant count")]
    CommitmentSlotCountMismatch,

    /// A commitment slot belongs to a different transcript.
    #[error("secure aggregate commitment slot transcript does not match")]
    CommitmentSlotTranscriptMismatch,

    /// A commitment slot duplicates a participant index.
    #[error("secure aggregate commitment slots contain a duplicate participant index")]
    DuplicateCommitmentSlotParticipantIndex,

    /// The transcript does not contain exactly one authenticated envelope per participant.
    #[error("secure aggregate authenticated envelope count does not match participant count")]
    EnvelopeCountMismatch,

    /// An authenticated envelope belongs to a different transcript.
    #[error("secure aggregate authenticated envelope transcript does not match")]
    EnvelopeTranscriptMismatch,

    /// An authenticated envelope does not match the expected participant slot.
    #[error("secure aggregate authenticated envelope participant does not match")]
    EnvelopeShareParticipantMismatch,

    /// An authenticated envelope does not match the expected commitment.
    #[error("secure aggregate authenticated envelope commitment does not match")]
    EnvelopeCommitmentMismatch,

    /// An authenticated envelope does not match the expected signing key.
    #[error("secure aggregate authenticated envelope signing key does not match")]
    EnvelopeSigningKeyMismatch,

    /// The signing public key is the reserved all-zero value.
    #[error("secure aggregate signing public key must not be empty")]
    EmptySigningPublicKey,

    /// The signing public key cannot be decoded as an Ed25519 verification key.
    #[error("secure aggregate signing public key is not a valid Ed25519 key")]
    InvalidSigningPublicKey,

    /// The signature is the reserved all-zero value.
    #[error("secure aggregate signature must not be empty")]
    EmptySignature,

    /// The signature does not have the required Ed25519 byte length.
    #[error("secure aggregate signature must be exactly 64 bytes")]
    SignatureLengthMismatch,

    /// The envelope signature does not verify against the configured key.
    #[error("secure aggregate authenticated envelope signature does not verify")]
    EnvelopeSignatureMismatch,

    /// The transcript does not contain exactly one verification key per participant.
    #[error("secure aggregate verification key count does not match participant count")]
    VerificationKeyCountMismatch,

    /// The verification key set contains a duplicate signing-key identifier.
    #[error("secure aggregate verification key set contains a duplicate signing-key identifier")]
    DuplicateVerificationKeyIdentifier,

    /// A commitment slot has no matching verification key.
    #[error("secure aggregate commitment slot has no matching verification key")]
    VerificationKeyMissing,

    /// The envelope sequence is the reserved zero value.
    #[error("secure aggregate envelope sequence must not be zero")]
    EnvelopeSequenceMustNotBeZero,

    /// The replay window lower bound exceeds the upper bound.
    #[error("secure aggregate replay window lower bound exceeds upper bound")]
    ReplayWindowLowerBoundExceedsUpperBound,

    /// The envelope sequence is outside the configured replay window.
    #[error("secure aggregate envelope sequence is outside the replay window")]
    EnvelopeSequenceOutsideReplayWindow,

    /// The envelope matches an already accepted replay record.
    #[error("secure aggregate envelope was already accepted")]
    EnvelopeReplayDetected,
}

/// Backward-compatible error name retained for callers of the original
/// additive secure-aggregate API.
pub type SecureAggregateError = SecureAggregateCompatibilityError;
