#![allow(clippy::wildcard_imports)]

use super::*;

#[test]
fn validates_bundled_shamir_threshold_vectors() {
    let parsed: Result<ShamirVectorFile, serde_yaml::Error> =
        serde_yaml::from_str(SHAMIR_THRESHOLD_VECTORS);
    assert!(parsed.is_ok());
    let Ok(parsed) = parsed else {
        return;
    };

    for vector in parsed.vectors {
        assert_eq!(vector.field_modulus, SECURE_AGGREGATE_FIELD_MODULUS);
        assert_eq!(vector.verification_profile, "kiva-shamir-hash-vss-v1");
        let plan = shamir_plan(vector.participant_count, vector.threshold);
        let shares = vector
            .shares
            .iter()
            .map(|share| shamir_share(share.participant_index, share.value))
            .collect::<Vec<_>>();

        assert_eq!(
            reconstruct_shamir_secret(plan, &shares),
            Ok(field(vector.secret))
        );
        let transcript_id = transcript(vector.transcript_seed);
        let share_openings = vector
            .share_openings
            .iter()
            .map(|opening| {
                verifiable_shamir_share(
                    transcript_id,
                    opening.participant_index,
                    opening.value,
                    opening.opening_nonce_seed,
                )
            })
            .collect::<Vec<_>>();
        for (opening, expected) in share_openings.iter().zip(vector.share_openings.iter()) {
            assert_eq!(
                opening.commitment().bytes(),
                hex_32(&expected.commitment_hex)
            );
        }
        assert_eq!(
            reconstruct_verified_shamir_secret(plan, transcript_id, &share_openings),
            Ok(field(vector.secret))
        );
    }
}

#[test]
fn validates_bundled_psi_cohort_vectors() {
    let parsed: Result<PsiVectorFile, serde_yaml::Error> = serde_yaml::from_str(PSI_COHORT_VECTORS);
    assert!(parsed.is_ok());
    let Ok(parsed) = parsed else {
        return;
    };

    for vector in parsed.vectors {
        assert_eq!(vector.profile, "kiva-psi-cohort-hash-v1");
        assert_eq!(vector.domain.as_bytes(), PSI_COHORT_IDENTIFIER_DOMAIN);
        let job_id = transcript(vector.job_seed);
        let datasets = vector
            .participants
            .iter()
            .map(|participant| {
                assert_eq!(participant.identifiers.len(), participant.digest_hex.len());
                let digests = participant
                    .identifiers
                    .iter()
                    .zip(participant.digest_hex.iter())
                    .map(|(identifier, expected)| {
                        let digest = psi_digest(job_id, &vector.namespace, identifier.as_str());
                        assert_eq!(digest.bytes(), hex_32(expected));
                        digest
                    })
                    .collect::<Vec<_>>();
                psi_dataset(participant.participant_index, digests)
            })
            .collect::<Vec<_>>();

        let result = compute_psi_cohort_intersection(
            nonzero(vector.participant_count),
            job_id,
            &[],
            &datasets,
        );
        assert!(result.is_ok());
        let Ok(result) = result else {
            continue;
        };
        let expected = vector
            .expected_intersection_hex
            .iter()
            .map(|hex| {
                let digest = PsiIdentifierDigest::new(hex_32(hex));
                assert!(digest.is_ok());
                let Ok(digest) = digest else {
                    return PsiIdentifierDigest { bytes: artifact(1) };
                };
                digest
            })
            .collect::<Vec<_>>();
        assert_eq!(result.intersection_digests(), expected.as_slice());
    }
}

#[test]
fn validates_bundled_psi_oprf_vectors() {
    let parsed: Result<PsiOprfVectorFile, serde_yaml::Error> =
        serde_yaml::from_str(PSI_OPRF_VECTORS);
    assert!(parsed.is_ok());
    let Ok(parsed) = parsed else {
        return;
    };

    for vector in parsed.vectors {
        assert_eq!(vector.profile, PSI_OPRF_DH_VOPRF_PROFILE);
        assert_eq!(
            vector.element_domain.as_bytes(),
            PSI_OPRF_DH_VOPRF_ELEMENT_DOMAIN
        );
        assert_eq!(
            vector.transcript_domain.as_bytes(),
            PSI_OPRF_DH_VOPRF_TRANSCRIPT_DOMAIN
        );
        assert_eq!(vector.suite, PsiOprfSuite::Ristretto255Sha512Voprf.label());
        let job_id = transcript(vector.job_seed);
        let binding = PsiOprfRuntimeBinding::new(
            PsiOprfSuite::Ristretto255Sha512Voprf,
            commitment(vector.runtime_artifact_seed),
            commitment(vector.review_evidence_seed),
            vector.external_review_completed,
        );
        assert!(binding.is_ok());
        let Ok(binding) = binding else {
            continue;
        };
        let plan = PsiOprfPlan::new(
            policy(vector.participant_count, vector.participant_count),
            binding,
        );
        assert!(plan.is_ok());
        let Ok(plan) = plan else {
            continue;
        };
        let transcripts = vector
            .participants
            .iter()
            .map(|participant| {
                assert_eq!(participant.identifiers.len(), participant.element_hex.len());
                let elements = participant
                    .identifiers
                    .iter()
                    .zip(participant.element_hex.iter())
                    .map(|(identifier, expected)| {
                        let element = psi_oprf_element(job_id, &vector.namespace, identifier);
                        assert_eq!(element.bytes(), hex_32(expected));
                        element
                    })
                    .collect::<Vec<_>>();
                psi_oprf_transcript(participant.participant_index, elements)
            })
            .collect::<Vec<_>>();

        let result = compute_psi_oprf_intersection(plan, job_id, &[], &transcripts);
        assert!(result.is_ok());
        let Ok(result) = result else {
            continue;
        };
        let expected = vector
            .expected_intersection_element_hex
            .iter()
            .map(|hex| {
                let element = PsiOprfElement::new(hex_32(hex));
                assert!(element.is_ok());
                let Ok(element) = element else {
                    return PsiOprfElement { bytes: artifact(1) };
                };
                element
            })
            .collect::<Vec<_>>();
        assert_eq!(result.intersection_elements(), expected.as_slice());
        assert_eq!(
            result.record().transcript_digest().bytes(),
            hex_32(&vector.expected_transcript_digest_hex)
        );
    }
}

#[test]
fn validates_bundled_threshold_frost_vectors() {
    let parsed: Result<ThresholdFrostVectorFile, serde_yaml::Error> =
        serde_yaml::from_str(THRESHOLD_FROST_VECTORS);
    assert!(parsed.is_ok());
    let Ok(parsed) = parsed else {
        return;
    };

    for vector in parsed.vectors {
        assert_eq!(vector.profile, THRESHOLD_FROST_PROFILE);
        assert_eq!(
            vector.key_ceremony_domain.as_bytes(),
            THRESHOLD_FROST_KEY_CEREMONY_DOMAIN
        );
        assert_eq!(
            vector.signature_transcript_domain.as_bytes(),
            THRESHOLD_FROST_SIGNATURE_TRANSCRIPT_DOMAIN
        );
        assert_eq!(
            vector.suite,
            ThresholdFrostSuite::Ed25519Sha512Frost.label()
        );
        let job_id = transcript(vector.job_seed);
        let binding = ThresholdFrostRuntimeBinding::new(
            ThresholdFrostSuite::Ed25519Sha512Frost,
            commitment(vector.runtime_artifact_seed),
            commitment(vector.review_evidence_seed),
            vector.external_review_completed,
        );
        assert!(binding.is_ok());
        let Ok(binding) = binding else {
            continue;
        };
        let plan = ThresholdFrostKeyCeremonyPlan::new(
            policy(vector.participant_count, vector.threshold),
            binding,
            sequence(vector.key_epoch),
        );
        assert!(plan.is_ok());
        let Ok(plan) = plan else {
            continue;
        };
        let participants = vector
            .participants
            .iter()
            .map(|participant| {
                threshold_frost_participant(
                    participant.participant_index,
                    participant.public_commitment_seed,
                    participant.share_custody_seed,
                )
            })
            .collect::<Vec<_>>();
        let accepted = accept_threshold_frost_key_ceremony(plan, job_id, &[], &participants);
        assert!(accepted.is_ok());
        let Ok(accepted) = accepted else {
            continue;
        };
        assert_eq!(
            accepted.record().ceremony_digest().bytes(),
            hex_32(&vector.expected_key_ceremony_digest_hex)
        );

        let shares = vector
            .signature_shares
            .iter()
            .map(|share| {
                threshold_frost_signature_share(share.participant_index, share.signature_share_seed)
            })
            .collect::<Vec<_>>();
        let message_digest = commitment(vector.message_digest_seed);
        let signature_digest = commitment(vector.signature_digest_seed);
        let transcript_digest =
            commitment_from_hex(&vector.expected_signature_transcript_digest_hex);
        let signed = accept_threshold_frost_signature_transcript(
            accepted,
            message_digest,
            signature_digest,
            transcript_digest,
            &[],
            &shares,
        );
        assert!(signed.is_ok());
        let Ok(signed) = signed else {
            continue;
        };
        assert_eq!(
            signed.record().transcript_digest().bytes(),
            hex_32(&vector.expected_signature_transcript_digest_hex)
        );
    }
}

#[test]
fn validates_bundled_tee_attestation_vectors() {
    let parsed: Result<TeeVectorFile, serde_yaml::Error> =
        serde_yaml::from_str(TEE_ATTESTATION_VECTORS);
    assert!(parsed.is_ok());
    let Ok(parsed) = parsed else {
        return;
    };

    for vector in parsed.vectors {
        assert_eq!(vector.profile, TEE_PORTABLE_ATTESTATION_PROFILE);
        assert_eq!(vector.domain.as_bytes(), TEE_ATTESTATION_EVIDENCE_DOMAIN);
        assert_eq!(vector.family, "deployment-bound-portable");
        assert_eq!(vector.format, "portable-evidence-v1");
        let claims = tee_claims(
            vector.job_seed,
            vector.measurement_seed,
            vector.image_digest_seed,
            vector.policy_binding_seed,
            vector.not_before_epoch_seconds,
            vector.not_after_epoch_seconds,
            vector.evidence_sequence,
        );
        assert_eq!(
            derive_tee_attestation_evidence_digest(claims),
            Ok(commitment_from_hex(&vector.expected_evidence_digest_hex))
        );
        let evidence = TeeAttestationEvidence::new(claims);
        assert!(evidence.is_ok());
        let Ok(evidence) = evidence else {
            continue;
        };
        assert_eq!(
            evidence.evidence_digest().bytes(),
            hex_32(&vector.expected_evidence_digest_hex)
        );
    }
}

#[test]
fn validates_bundled_tee_aws_nitro_attestation_vectors() {
    let parsed: Result<TeeAwsNitroVectorFile, serde_yaml::Error> =
        serde_yaml::from_str(TEE_AWS_NITRO_ATTESTATION_VECTORS);
    assert!(parsed.is_ok());
    let Ok(parsed) = parsed else {
        return;
    };

    for vector in parsed.vectors {
        assert_eq!(vector.profile, TEE_AWS_NITRO_ATTESTATION_PROFILE);
        assert_eq!(
            vector.document_domain.as_bytes(),
            TEE_AWS_NITRO_ATTESTATION_DOCUMENT_DOMAIN,
        );
        assert_eq!(vector.family, "aws-nitro-enclaves");
        assert_eq!(vector.format, "verified-nitro-attestation-document-v1");
        let claims = nitro_claims_with_seeds(
            vector.job_seed,
            vector.pcr0_seed,
            vector.pcr1_seed,
            vector.pcr2_seed,
            vector.pcr3_seed,
            vector.pcr8_seed,
            vector.image_digest_seed,
            vector.policy_binding_seed,
            vector.nonce_digest_seed,
            vector.certificate_chain_digest_seed,
            vector.cose_signing_key_digest_seed,
            vector.not_before_epoch_seconds,
            vector.not_after_epoch_seconds,
            vector.evidence_sequence,
            vector.debug_mode,
        );
        assert_eq!(
            derive_tee_aws_nitro_attestation_document_digest(claims),
            Ok(commitment_from_hex(&vector.expected_document_digest_hex))
        );
        let evidence = TeeAwsNitroAttestationEvidence::new(
            claims,
            nitro_verifier_binding(true, 2),
            true,
            true,
        );
        assert!(evidence.is_ok());
        let Ok(evidence) = evidence else {
            continue;
        };
        assert_eq!(
            evidence.document_digest(),
            commitment_from_hex(&vector.expected_document_digest_hex),
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn validates_bundled_fhe_feasibility_vectors() {
    let parsed: Result<FheVectorFile, serde_yaml::Error> =
        serde_yaml::from_str(FHE_FEASIBILITY_VECTORS);
    assert!(parsed.is_ok());
    let Ok(parsed) = parsed else {
        return;
    };

    for vector in parsed.vectors {
        assert_eq!(vector.profile, FHE_FEASIBILITY_PROFILE);
        assert_eq!(
            vector.ciphertext_domain.as_bytes(),
            FHE_FEASIBILITY_CIPHERTEXT_DOMAIN
        );
        assert_eq!(
            vector.evaluation_domain.as_bytes(),
            FHE_FEASIBILITY_EVALUATION_DOMAIN
        );
        assert_eq!(
            vector.scheme_family,
            FheFeasibilitySchemeFamily::BfvStyleIntegerSum.label()
        );
        assert_eq!(
            vector.operation,
            super::FheFeasibilityOperation::Sum.label()
        );

        let parameter_set = FheFeasibilityParameterSet::new(
            FheFeasibilitySchemeFamily::BfvStyleIntegerSum,
            super::FheFeasibilityOperation::Sum,
            vector.polynomial_modulus_degree,
            vector.plaintext_modulus,
            vector.ciphertext_modulus_bits,
            vector.security_level_bits,
            nonzero(vector.max_participant_count),
            vector.max_plaintext_value,
        );
        assert!(parameter_set.is_ok());
        let Ok(parameter_set) = parameter_set else {
            continue;
        };
        let plan = FheFeasibilityPlan::new(
            policy(vector.participant_count, vector.participant_count),
            parameter_set,
        );
        assert!(plan.is_ok());
        let Ok(plan) = plan else {
            continue;
        };
        let job_id = transcript(vector.job_seed);
        let ciphertexts = vector
            .participants
            .iter()
            .map(|participant| {
                let plaintext = fhe_plaintext(
                    parameter_set,
                    participant.participant_index,
                    participant.plaintext_value,
                );
                let nonce = fhe_nonce(participant.nonce_seed);
                let digest = derive_fhe_feasibility_ciphertext_digest(
                    job_id,
                    parameter_set,
                    plaintext,
                    nonce,
                );
                assert!(digest.is_ok());
                let Ok(digest) = digest else {
                    return FheFeasibilityCiphertextFixture::from_fixture_parts(
                        job_id,
                        parameter_set,
                        nonzero(participant.participant_index),
                        commitment(1),
                        NonZeroUsize::MIN,
                        0,
                        0,
                    );
                };
                assert_eq!(
                    digest.bytes(),
                    hex_32(&participant.expected_ciphertext_digest_hex)
                );
                fhe_ciphertext(
                    plan,
                    job_id,
                    participant.participant_index,
                    participant.plaintext_value,
                    participant.nonce_seed,
                )
            })
            .collect::<Vec<_>>();

        let evaluation = evaluate_fhe_feasibility_sum(plan, job_id, &ciphertexts);
        assert!(evaluation.is_ok());
        let Ok(evaluation) = evaluation else {
            continue;
        };
        assert_eq!(
            release_fhe_feasibility_sum_fixture(
                evaluation,
                FheFeasibilityReleaseContext::new(FheFeasibilityReleaseFacts::from_bits(
                    FheFeasibilityReleaseFacts::EVALUATION_COMPLETED
                        | FheFeasibilityReleaseFacts::POLICY_APPROVED
                        | FheFeasibilityReleaseFacts::GOVERNED_DATA_PRODUCT
                        | FheFeasibilityReleaseFacts::PLAINTEXT_BOUNDARY_CLEAR,
                )),
            )
            .map(super::FheFeasibilityReleasedAggregate::aggregate_value),
            Ok(vector.expected_sum)
        );
        assert_eq!(
            evaluation.evaluated_ciphertext_digest().bytes(),
            hex_32(&vector.expected_evaluated_ciphertext_digest_hex)
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn validates_bundled_fhe_runtime_vectors() {
    let parsed: Result<FheRuntimeVectorFile, serde_yaml::Error> =
        serde_yaml::from_str(FHE_RUNTIME_VECTORS);
    assert!(parsed.is_ok());
    let Ok(parsed) = parsed else {
        return;
    };

    for vector in parsed.vectors {
        assert_eq!(vector.profile, FHE_RUNTIME_PROFILE);
        assert_eq!(
            vector.ciphertext_transcript_domain.as_bytes(),
            FHE_RUNTIME_CIPHERTEXT_TRANSCRIPT_DOMAIN
        );
        assert_eq!(
            vector.evaluation_domain.as_bytes(),
            FHE_RUNTIME_EVALUATION_DOMAIN
        );
        assert_eq!(
            vector.scheme_family,
            FheRuntimeSchemeFamily::BfvIntegerSum.label()
        );
        assert_eq!(vector.operation, FheRuntimeOperation::Sum.label());

        let binding = FheRuntimeBinding::new(
            FheRuntimeSchemeFamily::BfvIntegerSum,
            FheRuntimeOperation::Sum,
            commitment(vector.runtime_artifact_seed),
            commitment(vector.parameter_set_seed),
            commitment(vector.review_evidence_seed),
            vector.external_review_completed,
        );
        assert!(binding.is_ok());
        let Ok(binding) = binding else {
            continue;
        };
        let plan = FheRuntimePlan::new(
            policy(vector.participant_count, vector.participant_count),
            binding,
            sequence(vector.key_epoch),
        );
        assert!(plan.is_ok());
        let Ok(plan) = plan else {
            continue;
        };
        let job_id = transcript(vector.job_seed);
        let ciphertexts = vector
            .participants
            .iter()
            .map(|participant| {
                fhe_runtime_ciphertext(
                    participant.participant_index,
                    participant.ciphertext_seed,
                    participant.parameter_set_seed,
                    Some(participant.key_custody_seed),
                )
            })
            .collect::<Vec<_>>();
        let transcript_digest =
            derive_fhe_runtime_ciphertext_transcript_digest(plan, job_id, &ciphertexts);
        assert!(transcript_digest.is_ok());
        let Ok(transcript_digest) = transcript_digest else {
            continue;
        };
        assert_eq!(
            transcript_digest.bytes(),
            hex_32(&vector.expected_ciphertext_transcript_digest_hex)
        );
        let accepted = accept_fhe_runtime_ciphertext_transcript(
            plan,
            job_id,
            transcript_digest,
            &[],
            &ciphertexts,
        );
        assert!(accepted.is_ok());
        let Ok(accepted) = accepted else {
            continue;
        };
        let evaluated_ciphertext_digest = commitment(vector.evaluated_ciphertext_seed);
        let evaluation_proof_digest = commitment(vector.evaluation_proof_seed);
        let evaluation_digest = derive_fhe_runtime_evaluation_digest(
            accepted,
            evaluated_ciphertext_digest,
            evaluation_proof_digest,
        );
        assert!(evaluation_digest.is_ok());
        let Ok(evaluation_digest) = evaluation_digest else {
            continue;
        };
        assert_eq!(
            evaluation_digest.bytes(),
            hex_32(&vector.expected_evaluation_digest_hex)
        );
        assert!(
            accept_fhe_runtime_evaluation(
                accepted,
                evaluated_ciphertext_digest,
                evaluation_proof_digest,
                evaluation_digest,
                &[],
            )
            .is_ok()
        );
    }
}
