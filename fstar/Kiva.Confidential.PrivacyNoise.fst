module Kiva.Confidential.PrivacyNoise

type noise_evidence = {
  aggregate: int;
  noise: int;
  released: int;
  applied: bool
}

let valid (evidence:noise_evidence) : bool =
  evidence.applied && evidence.released = evidence.aggregate + evidence.noise

let valid_noise_requires_applied (evidence:noise_evidence)
  : Lemma (valid evidence == true ==> evidence.applied == true)
= ()

let valid_noise_binds_released_value (evidence:noise_evidence)
  : Lemma (
      valid evidence == true ==>
      evidence.released == evidence.aggregate + evidence.noise
    )
= ()

let missing_noise_is_invalid (aggregate:int) (released:int)
  : Lemma (
      valid {
        aggregate = aggregate;
        noise = 0;
        released = released;
        applied = false
      } == false
    )
= ()
