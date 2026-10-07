module Kiva.Confidential.PrivacyBudget

type budget = {
  budget_total: nat;
  spent: nat
}

let valid (state:budget) : bool =
  state.spent <= state.budget_total

let can_spend (state:budget) (request:nat) : bool =
  valid state && request > 0 && state.spent + request <= state.budget_total

let spend (state:budget) (request:nat) : budget =
  {
    budget_total = state.budget_total;
    spent = state.spent + request
  }

let spend_requires_valid_ledger (state:budget) (request:nat)
  : Lemma (can_spend state request == true ==> valid state == true)
= ()

let spend_requires_nonzero_request (state:budget) (request:nat)
  : Lemma (can_spend state request == true ==> request > 0)
= ()

let spend_preserves_total (state:budget) (request:nat)
  : Lemma (
      can_spend state request == true ==>
      (spend state request).budget_total == state.budget_total
    )
= ()

let spend_is_monotonic (state:budget) (request:nat)
  : Lemma (can_spend state request == true ==> state.spent <= (spend state request).spent)
= ()

let spend_stays_within_total (state:budget) (request:nat)
  : Lemma (can_spend state request == true ==> valid (spend state request) == true)
= ()

let zero_request_cannot_spend (state:budget)
  : Lemma (can_spend state 0 == false)
= ()
