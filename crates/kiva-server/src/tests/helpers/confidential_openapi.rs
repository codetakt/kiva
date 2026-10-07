#![allow(clippy::wildcard_imports)]

use super::*;
use kiva_confidential_contract as confidential_contract;

pub(in crate::tests) fn confidential_additive_aggregate_body(
    request: &super::ConfidentialAdditiveAggregateRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_additive_aggregate_openapi_request(request),
        "confidential aggregate request",
    )
}

pub(in crate::tests) fn confidential_share_submission_body(
    request: &super::ConfidentialShareSubmissionRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::ShareSubmission {
            signed_envelope: signed_envelope_openapi(request.signed_envelope),
        },
        "confidential share submission",
    )
}

pub(in crate::tests) fn confidential_signing_key_body(
    request: &super::ConfidentialShareSigningKeyRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::ShareSigningKeyRegistration {
            signing_key_id: bytes(request.signing_key_id.bytes()),
            participant_index: request.participant_index,
            public_key: bytes(request.public_key.bytes()),
        },
        "confidential signing key",
    )
}

pub(in crate::tests) fn confidential_signing_key_rotation_body(
    request: &super::ConfidentialShareSigningKeyRotationRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::ShareSigningKeyRotation {
            old_signing_key_id: bytes(request.old_signing_key_id.bytes()),
            new_signing_key_id: bytes(request.new_signing_key_id.bytes()),
            participant_index: request.participant_index,
            public_key: bytes(request.public_key.bytes()),
        },
        "confidential signing key rotation",
    )
}

pub(in crate::tests) fn confidential_signing_key_revocation_body(
    request: &super::ConfidentialShareSigningKeyRevocationRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::ShareSigningKeyRevocation {
            signing_key_id: bytes(request.signing_key_id.bytes()),
        },
        "confidential signing key revocation",
    )
}

pub(in crate::tests) fn confidential_threshold_release_body(
    request: &super::ConfidentialThresholdReleaseRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::ThresholdReleaseRequest {
            release_id: bytes(request.release_id.bytes()),
            transcript_id: bytes(request.transcript_id.bytes()),
            aggregate: request.aggregate.value(),
            left_authority_subject: request.left_authority_subject.clone(),
            right_authority_subject: request.right_authority_subject.clone(),
        },
        "threshold request",
    )
}

pub(in crate::tests) fn confidential_threshold_release_approval_body(
    request: &super::ConfidentialThresholdReleaseApprovalRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::ThresholdReleaseApproval {
            release_id: bytes(request.release_id.bytes()),
        },
        "threshold approval",
    )
}

pub(in crate::tests) fn confidential_shamir_reconstruction_body(
    request: &super::ConfidentialShamirThresholdReconstructionRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::ShamirThresholdReconstruction {
            reconstruction_id: bytes(request.reconstruction_id.bytes()),
            transcript_id: bytes(request.transcript_id.bytes()),
            participant_count: request.participant_count,
            threshold: request.threshold,
            share_openings: request
                .share_openings
                .iter()
                .copied()
                .map(verifiable_shamir_share_openapi)
                .collect(),
        },
        "Shamir reconstruction",
    )
}

pub(in crate::tests) fn confidential_differential_privacy_budget_body(
    request: &super::ConfidentialDifferentialPrivacyBudgetRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::DifferentialPrivacyBudget {
            budget_id: bytes(request.budget_id.bytes()),
            total_epsilon_microunits: request.total_epsilon_microunits,
        },
        "differential privacy budget",
    )
}

pub(in crate::tests) fn confidential_differential_privacy_budget_spend_body(
    request: &super::ConfidentialDifferentialPrivacyBudgetSpendRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::DifferentialPrivacyBudgetSpend {
            budget_id: bytes(request.budget_id.bytes()),
            epsilon_microunits: request.epsilon_microunits,
        },
        "differential privacy budget spend",
    )
}

pub(in crate::tests) fn confidential_differential_privacy_release_body(
    request: &super::ConfidentialDifferentialPrivacyReleaseRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::DifferentialPrivacyRelease {
            release_id: bytes(request.release_id.bytes()),
            budget_id: bytes(request.budget_id.bytes()),
            transcript_id: bytes(request.transcript_id.bytes()),
            aggregate: request.aggregate.value(),
            data_subject_count: request.data_subject_count,
            minimum_population: request.minimum_population,
            epsilon_microunits: request.epsilon_microunits,
            l1_sensitivity: request.l1_sensitivity,
            noise_value: request.noise_value,
        },
        "differential privacy release",
    )
}

pub(in crate::tests) fn confidential_differential_privacy_sampled_release_body(
    request: &super::ConfidentialDifferentialPrivacySampledReleaseRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::DifferentialPrivacySampledRelease {
            release_id: bytes(request.release_id.bytes()),
            budget_id: bytes(request.budget_id.bytes()),
            transcript_id: bytes(request.transcript_id.bytes()),
            data_subject_count: request.data_subject_count,
            minimum_population: request.minimum_population,
            epsilon_microunits: request.epsilon_microunits,
            l1_sensitivity: request.l1_sensitivity,
        },
        "differential privacy sampled release",
    )
}

pub(in crate::tests) fn confidential_job_body(
    request: &super::ConfidentialJobRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::ConfidentialJobRequest {
            job_id: bytes(request.job_id.bytes()),
            mechanism: confidential_job_mechanism(&request.mechanism),
            mechanism_profile: request.mechanism_profile.clone(),
            workload_id: request.workload_id.clone(),
            participant_subjects: request.participant_subjects.clone(),
            policy_binding_id: request.policy_binding_id.clone(),
            deadline_at: request.deadline_at.clone(),
            admission_quota_key: request.admission_quota_key.clone(),
            max_retry_count: request.max_retry_count,
            evidence_refs: request.evidence_refs.clone(),
        },
        "confidential job",
    )
}

pub(in crate::tests) fn confidential_job_transition_body(
    request: &super::ConfidentialJobTransitionRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::ConfidentialJobTransitionRequest {
            job_id: bytes(request.job_id.bytes()),
            target_status: confidential_job_status_openapi(request.target_status),
            reason: request.reason.clone(),
            evidence_refs: request.evidence_refs.clone(),
        },
        "confidential job transition",
    )
}

pub(in crate::tests) fn confidential_participant_body(
    request: &super::ConfidentialParticipantRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::ConfidentialParticipantRequest {
            participant_subject: request.participant_subject.clone(),
            participant_index: request.participant_index,
            mechanism_roles: request
                .mechanism_roles
                .iter()
                .map(|role| confidential_participant_role(role))
                .collect(),
            authorization_scope: request.authorization_scope.clone(),
            key_ceremony_ref: request.key_ceremony_ref.clone(),
            recovery_contact_ref: request.recovery_contact_ref.clone(),
            evidence_refs: request.evidence_refs.clone(),
        },
        "confidential participant",
    )
}

pub(in crate::tests) fn confidential_participant_transition_body(
    request: &super::ConfidentialParticipantTransitionRequest,
) -> Result<Bytes, String> {
    json_body(
        &confidential_contract::ConfidentialParticipantTransitionRequest {
            participant_subject: request.participant_subject.clone(),
            participant_index: request.participant_index,
            target_status: confidential_participant_status_openapi(request.target_status),
            reason: request.reason.clone(),
            evidence_refs: request.evidence_refs.clone(),
        },
        "confidential participant transition",
    )
}

pub(in crate::tests) fn body_from_request() -> Bytes {
    match confidential_additive_aggregate_body(&confidential_aggregate_request()) {
        Ok(body) => body,
        Err(error) => panic!("confidential aggregate request should serialize: {error:?}"),
    }
}

fn confidential_additive_aggregate_openapi_request(
    request: &super::ConfidentialAdditiveAggregateRequest,
) -> confidential_contract::AdditiveAggregateRequest {
    confidential_contract::AdditiveAggregateRequest {
        participant_count: request.participant_count,
        transcript_id: bytes(request.transcript_id.bytes()),
        slots: request
            .slots
            .iter()
            .copied()
            .map(additive_share_commitment_slot_openapi)
            .collect(),
        replay_window: confidential_contract::AdditiveShareReplayWindow {
            first_sequence: request.replay_window.first_sequence().value(),
            last_sequence: request.replay_window.last_sequence().value(),
        },
        signed_envelopes: request
            .signed_envelopes
            .iter()
            .copied()
            .map(signed_envelope_openapi)
            .collect(),
    }
}

fn additive_share_commitment_slot_openapi(
    slot: AdditiveShareCommitmentSlot,
) -> confidential_contract::AdditiveShareCommitmentSlot {
    confidential_contract::AdditiveShareCommitmentSlot {
        transcript_id: bytes(slot.transcript_id().bytes()),
        participant_index: slot.participant_index(),
        commitment: bytes(slot.commitment().bytes()),
        signing_key_id: bytes(slot.signing_key_id().bytes()),
    }
}

fn signed_envelope_openapi(
    signed: SignedAuthenticatedAdditiveShareEnvelope,
) -> confidential_contract::SignedAuthenticatedAdditiveShareEnvelope {
    confidential_contract::SignedAuthenticatedAdditiveShareEnvelope {
        envelope: authenticated_envelope_openapi(signed.envelope()),
        signature: bytes(signed.signature().bytes()),
    }
}

fn authenticated_envelope_openapi(
    envelope: AuthenticatedAdditiveShareEnvelope,
) -> confidential_contract::AuthenticatedAdditiveShareEnvelope {
    confidential_contract::AuthenticatedAdditiveShareEnvelope {
        transcript_id: bytes(envelope.transcript_id().bytes()),
        share: secure_aggregate_share_openapi(envelope.share()),
        commitment: bytes(envelope.commitment().bytes()),
        signing_key_id: bytes(envelope.signing_key_id().bytes()),
        sequence: envelope.sequence().value(),
        proof: bytes(envelope.transport_proof().bytes()),
    }
}

fn secure_aggregate_share_openapi(
    share: SecureAggregateShare,
) -> confidential_contract::SecureAggregateShare {
    confidential_contract::SecureAggregateShare {
        participant_index: share.participant_index(),
        value: share.value().value(),
    }
}

fn verifiable_shamir_share_openapi(
    share: VerifiableShamirShare,
) -> confidential_contract::VerifiableShamirShare {
    confidential_contract::VerifiableShamirShare {
        share: confidential_contract::ShamirShare {
            participant_index: share.share().participant_index(),
            value: share.share().value().value(),
        },
        commitment: bytes(share.commitment().bytes()),
        opening_nonce: bytes(share.opening_nonce().bytes()),
    }
}

fn confidential_job_mechanism(label: &str) -> confidential_contract::ConfidentialJobMechanism {
    match label {
        "additive-secure-aggregation" => {
            confidential_contract::ConfidentialJobMechanism::AdditiveSecureAggregation
        }
        "differential-privacy-release" => {
            confidential_contract::ConfidentialJobMechanism::DifferentialPrivacyRelease
        }
        "shamir-vss-admission" => {
            confidential_contract::ConfidentialJobMechanism::ShamirVssAdmission
        }
        "threshold-frost-signing" => {
            confidential_contract::ConfidentialJobMechanism::ThresholdFrostSigning
        }
        "psi-cohort-hash-token" => {
            confidential_contract::ConfidentialJobMechanism::PsiCohortHashToken
        }
        "psi-oprf-dh-voprf" => confidential_contract::ConfidentialJobMechanism::PsiOprfDhVoprf,
        "tee-portable-attestation" => {
            confidential_contract::ConfidentialJobMechanism::TeePortableAttestation
        }
        "tee-aws-nitro-attestation" => {
            confidential_contract::ConfidentialJobMechanism::TeeAwsNitroAttestation
        }
        "fhe-sum-feasibility" => confidential_contract::ConfidentialJobMechanism::FheSumFeasibility,
        "fhe-sum-runtime" => confidential_contract::ConfidentialJobMechanism::FheSumRuntime,
        other => panic!("unknown confidential job mechanism label: {other}"),
    }
}

fn confidential_job_status_openapi(
    status: ConfidentialJobStatus,
) -> confidential_contract::ConfidentialJobStatus {
    match status {
        ConfidentialJobStatus::Draft => confidential_contract::ConfidentialJobStatus::Draft,
        ConfidentialJobStatus::Admitted => confidential_contract::ConfidentialJobStatus::Admitted,
        ConfidentialJobStatus::Running => confidential_contract::ConfidentialJobStatus::Running,
        ConfidentialJobStatus::AwaitingRelease => {
            confidential_contract::ConfidentialJobStatus::AwaitingRelease
        }
        ConfidentialJobStatus::Released => confidential_contract::ConfidentialJobStatus::Released,
        ConfidentialJobStatus::Failed => confidential_contract::ConfidentialJobStatus::Failed,
        ConfidentialJobStatus::Cancelled => confidential_contract::ConfidentialJobStatus::Cancelled,
        ConfidentialJobStatus::Expired => confidential_contract::ConfidentialJobStatus::Expired,
        ConfidentialJobStatus::Retired => confidential_contract::ConfidentialJobStatus::Retired,
    }
}

fn confidential_participant_role(role: &str) -> confidential_contract::ConfidentialParticipantRole {
    match role {
        "additive-share-contributor" => {
            confidential_contract::ConfidentialParticipantRole::AdditiveShareContributor
        }
        "data-custodian" => confidential_contract::ConfidentialParticipantRole::DataCustodian,
        "policy-authority" => confidential_contract::ConfidentialParticipantRole::PolicyAuthority,
        "psi-participant" => confidential_contract::ConfidentialParticipantRole::PsiParticipant,
        "tee-operator" => confidential_contract::ConfidentialParticipantRole::TeeOperator,
        "fhe-key-authority" => confidential_contract::ConfidentialParticipantRole::FheKeyAuthority,
        "auditor" => confidential_contract::ConfidentialParticipantRole::Auditor,
        other => panic!("unknown confidential participant role label: {other}"),
    }
}

fn confidential_participant_status_openapi(
    status: super::ConfidentialParticipantStatus,
) -> confidential_contract::ConfidentialParticipantStatus {
    match status {
        super::ConfidentialParticipantStatus::Invited => {
            confidential_contract::ConfidentialParticipantStatus::Invited
        }
        super::ConfidentialParticipantStatus::Active => {
            confidential_contract::ConfidentialParticipantStatus::Active
        }
        super::ConfidentialParticipantStatus::Suspended => {
            confidential_contract::ConfidentialParticipantStatus::Suspended
        }
        super::ConfidentialParticipantStatus::Frozen => {
            confidential_contract::ConfidentialParticipantStatus::Frozen
        }
        super::ConfidentialParticipantStatus::Removed => {
            confidential_contract::ConfidentialParticipantStatus::Removed
        }
    }
}

fn bytes<const N: usize>(value: [u8; N]) -> Vec<u8> {
    value.to_vec()
}
