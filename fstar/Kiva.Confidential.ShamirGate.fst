module Kiva.Confidential.ShamirGate

type shamir_gate = {
  participant_count_admitted: bool;
  threshold_at_least_two: bool;
  threshold_within_participants: bool;
  share_count_at_least_threshold: bool;
  participant_indexes_unique: bool;
  share_openings_verified: bool
}

let can_reconstruct (state:shamir_gate) : bool =
  state.participant_count_admitted
  && state.threshold_at_least_two
  && state.threshold_within_participants
  && state.share_count_at_least_threshold
  && state.participant_indexes_unique
  && state.share_openings_verified

let reconstruction_requires_participant_admission (state:shamir_gate)
  : Lemma (
      can_reconstruct state == true ==> state.participant_count_admitted == true
    )
= ()

let reconstruction_requires_threshold_at_least_two (state:shamir_gate)
  : Lemma (
      can_reconstruct state == true ==> state.threshold_at_least_two == true
    )
= ()

let reconstruction_requires_threshold_within_participants (state:shamir_gate)
  : Lemma (
      can_reconstruct state == true ==> state.threshold_within_participants == true
    )
= ()

let reconstruction_requires_enough_shares (state:shamir_gate)
  : Lemma (
      can_reconstruct state == true ==> state.share_count_at_least_threshold == true
    )
= ()

let reconstruction_requires_unique_participant_indexes (state:shamir_gate)
  : Lemma (
      can_reconstruct state == true ==> state.participant_indexes_unique == true
    )
= ()

let reconstruction_requires_verified_share_openings (state:shamir_gate)
  : Lemma (
      can_reconstruct state == true ==> state.share_openings_verified == true
    )
= ()

let no_reconstruction_with_duplicate_participant_indexes
  (participant_count_admitted:bool)
  (threshold_at_least_two:bool)
  (threshold_within_participants:bool)
  (share_count_at_least_threshold:bool)
  : Lemma (
      can_reconstruct {
        participant_count_admitted = participant_count_admitted;
        threshold_at_least_two = threshold_at_least_two;
        threshold_within_participants = threshold_within_participants;
        share_count_at_least_threshold = share_count_at_least_threshold;
        participant_indexes_unique = false;
        share_openings_verified = true
      } == false
    )
= ()

let no_reconstruction_without_threshold_shares
  (participant_count_admitted:bool)
  (threshold_at_least_two:bool)
  (threshold_within_participants:bool)
  (participant_indexes_unique:bool)
  : Lemma (
      can_reconstruct {
        participant_count_admitted = participant_count_admitted;
        threshold_at_least_two = threshold_at_least_two;
        threshold_within_participants = threshold_within_participants;
        share_count_at_least_threshold = false;
        participant_indexes_unique = participant_indexes_unique;
        share_openings_verified = true
      } == false
    )
= ()

let no_reconstruction_without_verified_share_openings
  (participant_count_admitted:bool)
  (threshold_at_least_two:bool)
  (threshold_within_participants:bool)
  (share_count_at_least_threshold:bool)
  (participant_indexes_unique:bool)
  : Lemma (
      can_reconstruct {
        participant_count_admitted = participant_count_admitted;
        threshold_at_least_two = threshold_at_least_two;
        threshold_within_participants = threshold_within_participants;
        share_count_at_least_threshold = share_count_at_least_threshold;
        participant_indexes_unique = participant_indexes_unique;
        share_openings_verified = false
      } == false
    )
= ()
