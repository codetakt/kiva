module Kiva.Confidential.TranscriptGate

type transcript_gate = {
  commitment_slots_complete: bool;
  verification_keys_complete: bool;
  replay_records_persisted: bool;
  all_shares_authenticated: bool
}

let can_compute (state:transcript_gate) : bool =
  state.commitment_slots_complete
  && state.verification_keys_complete
  && state.replay_records_persisted
  && state.all_shares_authenticated

let compute_requires_commitment_slots (state:transcript_gate)
  : Lemma (
      can_compute state == true ==> state.commitment_slots_complete == true
    )
= ()

let compute_requires_verification_keys (state:transcript_gate)
  : Lemma (
      can_compute state == true ==> state.verification_keys_complete == true
    )
= ()

let compute_requires_replay_records (state:transcript_gate)
  : Lemma (
      can_compute state == true ==> state.replay_records_persisted == true
    )
= ()

let compute_requires_authenticated_shares (state:transcript_gate)
  : Lemma (
      can_compute state == true ==> state.all_shares_authenticated == true
    )
= ()

let no_compute_without_persisted_replay
  (commitments:bool)
  (verification_keys:bool)
  (authenticated_shares:bool)
  : Lemma (
      can_compute {
        commitment_slots_complete = commitments;
        verification_keys_complete = verification_keys;
        replay_records_persisted = false;
        all_shares_authenticated = authenticated_shares
      } == false
    )
= ()
