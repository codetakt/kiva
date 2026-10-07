#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(
  git rev-parse --show-toplevel 2>/dev/null || {
    cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd
  }
)"

if ! command -v cargo-kani >/dev/null 2>&1; then
  cat >&2 <<'MSG'
cargo-kani is not available on PATH.

Run Kani through the pinned verification shell:

  nix develop .#verification -c scripts/ci/run_kani_verification.sh

MSG
  exit 127
fi

KIVA_KANI_SOLVER="${KIVA_KANI_SOLVER:-cadical}"
KIVA_KANI_UNWIND="${KIVA_KANI_UNWIND:-16}"
KIVA_KANI_TIMEOUT_SECS="${KIVA_KANI_TIMEOUT_SECS:-300}"
KIVA_KANI_SUITE="${KIVA_KANI_SUITE:-regression}"

SMOKE_HARNESSES=(
  "crates/kiva-http-core|kani_harnesses::xapi_version_support_is_exact_for_known_inputs"
  "crates/kiva-policy|kani_harnesses::allow_set_is_exactly_the_baseline_policy"
  "crates/kiva-confidential|kani_harnesses::release_policy_validation_matches_structural_rules"
)

REGRESSION_HARNESSES=(
  "${SMOKE_HARNESSES[@]}"
  "crates/kiva-http-core|kani_harnesses::bearer_token_classification_is_exact_for_known_inputs"
  "crates/kiva-http-core|kani_harnesses::etag_matching_honors_star_exact_values_and_comma_lists"
  "crates/kiva-http-core|kani_harnesses::document_preconditions_follow_if_match_and_if_none_match_rules"
  "crates/kiva-http-core|kani_harnesses::document_put_decision_fails_closed_on_precondition_failure"
  "crates/kiva-http-core|kani_harnesses::document_put_decision_conflicts_only_without_preconditions_on_changed_existing_content"
  "crates/kiva-http-core|kani_harnesses::document_delete_decision_fails_closed_on_precondition_failure"
  "crates/kiva-http-core|kani_harnesses::document_delete_decision_deletes_only_existing_content_after_preconditions_pass"
  "crates/kiva-http-core|kani_harnesses::document_json_merge_requires_object_patch"
  "crates/kiva-http-core|kani_harnesses::document_json_merge_allows_only_empty_or_object_existing_content"
  "crates/kiva-http-core|kani_harnesses::statement_put_decision_matches_immutability_contract"
  "crates/kiva-http-core|kani_harnesses::statement_get_query_rejects_conflicting_statement_ids"
  "crates/kiva-http-core|kani_harnesses::statement_get_query_single_statement_rejects_search_parameters"
  "crates/kiva-http-core|kani_harnesses::statement_get_query_rejects_invalid_scalar_parameters"
  "crates/kiva-http-core|kani_harnesses::statement_get_query_allows_collection_filters_when_scalars_are_valid"
  "crates/kiva-policy|kani_harnesses::raw_learning_record_reads_are_custodian_only"
  "crates/kiva-policy|kani_harnesses::confidential_result_release_requires_policy_authority"
  "crates/kiva-policy|kani_harnesses::pds_delegated_reads_require_active_consent"
  "crates/kiva-policy|kani_harnesses::pds_delegated_reads_require_matching_scope"
  "crates/kiva-policy|kani_harnesses::pds_delegation_never_grants_analyst_raw_record_reads"
  "crates/kiva-policy|kani_harnesses::pds_data_product_release_gate_matches_source_kind_rules"
  "crates/kiva-policy|kani_harnesses::pds_data_product_access_gate_matches_scope_rules"
  "crates/kiva-policy|kani_harnesses::pds_public_sector_governance_profile_gate_matches_strict_metadata_rules"
  "crates/kiva-policy|kani_harnesses::oauth_scope_gate_matches_exact_resource_server_operations"
  "crates/kiva-policy|kani_harnesses::lifecycle_case_transition_gate_matches_strict_state_machine"
  "crates/kiva-confidential|kani_harnesses::secure_aggregate_field_addition_stays_inside_field"
  "crates/kiva-confidential|kani_harnesses::threshold_release_authority_set_rejects_missing_or_ambiguous_authorities"
  "crates/kiva-confidential|kani_harnesses::threshold_release_authority_classification_is_unambiguous"
  "crates/kiva-confidential|kani_harnesses::threshold_release_duplicate_approval_is_idempotent"
  "crates/kiva-confidential|kani_harnesses::threshold_release_requires_both_authority_approvals"
  "crates/kiva-confidential|kani_harnesses::threshold_release_request_requires_matching_computed_aggregate"
  "crates/kiva-confidential|kani_harnesses::threshold_release_completion_requires_actor_and_full_threshold"
  "crates/kiva-confidential|kani_harnesses::signing_key_rotation_requires_owner_participant_and_valid_states"
  "crates/kiva-confidential|kani_harnesses::signing_key_rotation_revokes_old_and_activates_new_key"
  "crates/kiva-confidential|kani_harnesses::signing_key_revocation_is_idempotent_and_fail_closed"
  "crates/kiva-confidential|kani_harnesses::differential_privacy_policy_rejects_single_subject_minimum"
  "crates/kiva-confidential|kani_harnesses::differential_privacy_budget_ledger_rejects_over_spent_state"
  "crates/kiva-confidential|kani_harnesses::differential_privacy_budget_spend_fails_closed_and_is_monotonic"
  "crates/kiva-confidential|kani_harnesses::differential_privacy_composition_summary_fails_closed"
  "crates/kiva-confidential|kani_harnesses::differential_privacy_advanced_composition_delta_fails_closed"
  "crates/kiva-confidential|kani_harnesses::differential_privacy_budget_policy_lifecycle_fails_closed"
  "crates/kiva-confidential|kani_harnesses::differential_privacy_noise_application_matches_checked_arithmetic"
  "crates/kiva-confidential|kani_harnesses::differential_privacy_release_gate_fails_closed"
  "crates/kiva-confidential|kani_harnesses::confidential_mechanism_plan_enforces_additive_and_shamir_threshold_rules"
  "crates/kiva-confidential|kani_harnesses::confidential_job_lifecycle_matches_strict_transition_table"
  "crates/kiva-confidential|kani_harnesses::confidential_job_admission_gate_fails_closed"
  "crates/kiva-confidential|kani_harnesses::confidential_participant_lifecycle_matches_strict_transition_table"
  "crates/kiva-confidential|kani_harnesses::confidential_participant_admission_is_active_only"
  "crates/kiva-confidential|kani_harnesses::reference_provider_rejects_production_claims"
  "crates/kiva-confidential|kani_harnesses::provider_capability_mismatch_fails_closed"
  "crates/kiva-confidential|kani_harnesses::fhe_feasibility_parameter_profile_is_strict"
  "crates/kiva-confidential|kani_harnesses::fhe_feasibility_release_gate_fails_closed"
  "crates/kiva-confidential|kani_harnesses::fhe_runtime_ciphertext_transcript_gate_fails_closed"
  "crates/kiva-confidential|kani_harnesses::fhe_runtime_release_gate_fails_closed"
  "crates/kiva-confidential|kani_harnesses::psi_cohort_intersection_exposes_only_common_digests"
  "crates/kiva-confidential|kani_harnesses::psi_oprf_intersection_exposes_only_common_elements"
  "crates/kiva-confidential|kani_harnesses::psi_oprf_release_gate_fails_closed"
  "crates/kiva-confidential|kani_harnesses::threshold_frost_key_ceremony_gate_fails_closed"
  "crates/kiva-confidential|kani_harnesses::threshold_frost_release_gate_fails_closed"
  "crates/kiva-confidential|kani_harnesses::tee_attestation_gate_fails_closed_for_policy_freshness_and_replay"
  "crates/kiva-confidential|kani_harnesses::tee_secret_and_result_release_gates_fail_closed"
  "crates/kiva-confidential|kani_harnesses::tee_aws_nitro_attestation_gate_fails_closed"
  "crates/kiva-confidential|kani_harnesses::tee_aws_nitro_key_and_result_release_gates_fail_closed"
  "crates/kiva-confidential|kani_harnesses::shamir_reconstruction_rejects_insufficient_or_duplicate_shares"
  "crates/kiva-confidential|kani_harnesses::shamir_share_complaint_resolution_fails_closed"
  "crates/kiva-confidential|kani_harnesses::additive_secure_aggregate_requires_all_participants"
  "crates/kiva-confidential|kani_harnesses::authenticated_share_acceptance_matches_commitment_slot"
  "crates/kiva-confidential|kani_harnesses::verification_key_validation_requires_one_registered_key_per_slot"
  "crates/kiva-confidential|kani_harnesses::replay_windowed_acceptance_requires_sequence_inside_window"
)

case "$KIVA_KANI_SUITE" in
  smoke)
    HARNESSES=("${SMOKE_HARNESSES[@]}")
    ;;
  regression)
    HARNESSES=("${REGRESSION_HARNESSES[@]}")
    ;;
  *)
    echo "unknown KIVA_KANI_SUITE=$KIVA_KANI_SUITE; expected smoke or regression" >&2
    exit 2
    ;;
esac

LOG_DIR="$REPO_ROOT/artifacts/kani"
mkdir -p "$LOG_DIR"
RUN_ID="$(date +%Y%m%dT%H%M%S)"
SUMMARY_FILE="${KIVA_KANI_JSON_REPORT:-$LOG_DIR/report.json}"
HISTORY_FILE="$LOG_DIR/report.log"
mkdir -p "$(dirname "$SUMMARY_FILE")"

KANI_VERSION="$(cargo kani --version | head -n1)"
echo "[KANI] $KANI_VERSION"
echo "[KANI] suite=$KIVA_KANI_SUITE solver=$KIVA_KANI_SOLVER unwind=$KIVA_KANI_UNWIND timeout=${KIVA_KANI_TIMEOUT_SECS}s"

declare -A RESULTS=()
STATUS="passed"

for spec in "${HARNESSES[@]}"; do
  IFS="|" read -r crate_dir harness <<<"$spec"
  echo "[KANI] running $crate_dir :: $harness"

  if (
    cd "$REPO_ROOT/$crate_dir"
    timeout "$KIVA_KANI_TIMEOUT_SECS" cargo kani \
      --solver "$KIVA_KANI_SOLVER" \
      --unwind "$KIVA_KANI_UNWIND" \
      --harness "$harness" \
      --quiet
  ); then
    RESULTS["$spec"]="passed"
    echo "[KANI] passed $crate_dir :: $harness"
  else
    code=$?
    STATUS="failed"
    if [ "$code" -eq 124 ]; then
      RESULTS["$spec"]="timeout"
      echo "[KANI] timeout $crate_dir :: $harness" >&2
    else
      RESULTS["$spec"]="failed"
      echo "[KANI] failed $crate_dir :: $harness" >&2
    fi
  fi
done

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  printf "%s" "$value"
}

{
  printf '{\n'
  printf '  "schema_version": 1,\n'
  printf '  "standard": "Kiva Architecture",\n'
  printf '  "kind": "kani-verification",\n'
  printf '  "status": "%s",\n' "$(json_escape "$STATUS")"
  printf '  "generated_at_utc": "%s",\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf '  "tool_version": "%s",\n' "$(json_escape "$KANI_VERSION")"
  printf '  "suite": "%s",\n' "$(json_escape "$KIVA_KANI_SUITE")"
  printf '  "run_id": "%s",\n' "$(json_escape "$RUN_ID")"
  printf '  "solver": "%s",\n' "$(json_escape "$KIVA_KANI_SOLVER")"
  printf '  "unwind": %s,\n' "$KIVA_KANI_UNWIND"
  printf '  "timeout_seconds": %s,\n' "$KIVA_KANI_TIMEOUT_SECS"
  printf '  "checks": ['
  first=1
  for spec in $(printf '%s\n' "${!RESULTS[@]}" | sort); do
    IFS="|" read -r crate_dir harness <<<"$spec"
    if [ "$first" -eq 1 ]; then
      first=0
    else
      printf ','
    fi
    printf '\n    {'
    printf '"id": "%s", ' "$(json_escape "$harness")"
    printf '"target": "%s", ' "$(json_escape "$crate_dir")"
    printf '"status": "%s"' "$(json_escape "${RESULTS[$spec]}")"
    printf '}'
  done
  if [ "$first" -eq 0 ]; then
    printf '\n'
  fi
  printf '  ],\n'
  printf '  "harnesses": {'
  first=1
  for spec in $(printf '%s\n' "${!RESULTS[@]}" | sort); do
    if [ "$first" -eq 1 ]; then
      first=0
    else
      printf ','
    fi
    printf '\n    "%s": "%s"' "$(json_escape "$spec")" "$(json_escape "${RESULTS[$spec]}")"
  done
  if [ "$first" -eq 0 ]; then
    printf '\n'
  fi
  printf '  }\n'
  printf '}\n'
} >"$SUMMARY_FILE"

{
  printf '%s status=%s suite=%s\n' "$(date -Iseconds)" "$STATUS" "$KIVA_KANI_SUITE"
  for spec in $(printf '%s\n' "${!RESULTS[@]}" | sort); do
    printf '  - %s: %s\n' "$spec" "${RESULTS[$spec]}"
  done
  printf '\n'
} >>"$HISTORY_FILE"

if [ "$STATUS" != "passed" ]; then
  echo "[KANI] verification completed with failures; see $SUMMARY_FILE" >&2
  exit 1
fi

echo "[KANI] verification completed successfully; summary: $SUMMARY_FILE"
