module Kiva.Confidential.PrivacyGate

type privacy_gate = {
  computed: bool;
  population_ok: bool;
  epsilon_ok: bool;
  noise_applied: bool
}

let can_release (state:privacy_gate) : bool =
  state.computed
  && state.population_ok
  && state.epsilon_ok
  && state.noise_applied

let release_requires_computed (state:privacy_gate)
  : Lemma (can_release state == true ==> state.computed == true)
= ()

let release_requires_population_ok (state:privacy_gate)
  : Lemma (can_release state == true ==> state.population_ok == true)
= ()

let release_requires_epsilon_ok (state:privacy_gate)
  : Lemma (can_release state == true ==> state.epsilon_ok == true)
= ()

let release_requires_noise (state:privacy_gate)
  : Lemma (can_release state == true ==> state.noise_applied == true)
= ()

let no_release_without_noise
  (computed:bool)
  (population_ok:bool)
  (epsilon_ok:bool)
  : Lemma (
      can_release {
        computed = computed;
        population_ok = population_ok;
        epsilon_ok = epsilon_ok;
        noise_applied = false
      } == false
    )
= ()
