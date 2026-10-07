module Kiva.Confidential.ReleaseGate

type approval_side =
  | Left
  | Right

type release_gate = {
  computed: bool;
  left_approved: bool;
  right_approved: bool
}

let empty (computed:bool) : release_gate =
  {
    computed = computed;
    left_approved = false;
    right_approved = false
  }

let approve (state:release_gate) (side:approval_side) : release_gate =
  match side with
  | Left -> { state with left_approved = true }
  | Right -> { state with right_approved = true }

let can_release (state:release_gate) : bool =
  state.computed && state.left_approved && state.right_approved

let release_requires_computed (state:release_gate)
  : Lemma (can_release state == true ==> state.computed == true)
= ()

let release_requires_left_approval (state:release_gate)
  : Lemma (can_release state == true ==> state.left_approved == true)
= ()

let release_requires_right_approval (state:release_gate)
  : Lemma (can_release state == true ==> state.right_approved == true)
= ()

let duplicate_left_approval_does_not_release_without_right (computed:bool)
  : Lemma (
      can_release (approve (approve (empty computed) Left) Left) == false
    )
= ()

let no_release_without_computed_aggregate (left:bool) (right:bool)
  : Lemma (
      can_release {
        computed = false;
        left_approved = left;
        right_approved = right
      } == false
    )
= ()
