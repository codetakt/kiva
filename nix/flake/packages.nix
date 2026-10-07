{
  pkgs,
  lib,
  craneLib,
  src,
  cargoArtifacts,
  kani',
  sourceRevision ? "unknown",
  sourceDirty ? true,
  ...
}:

{
  kani = kani';

  default = craneLib.buildPackage {
    pname = "kiva-workspace";
    version = "0.0.0";
    inherit src cargoArtifacts;
    cargoExtraArgs = "--workspace --locked";
    doCheck = false;
    meta = {
      mainProgram = "kiva-server";
    };
  };

  conformance-evidence-static =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
        pythonPackages.pyyaml
      ]);
    in
    craneLib.mkCargoDerivation {
      pname = "kiva-conformance-evidence-static";
      version = "0.0.0";
      inherit src cargoArtifacts;
      doCheck = true;
      doInstallCargoArtifacts = false;
      nativeBuildInputs = [
        pkgs.coreutils
        pkgs.curl
        pkgs.findutils
        pkgs.git
        pkgs.openssl
        python
      ];
      buildPhaseCargoCommand = "";
      checkPhaseCargoCommand = ''
        export HOME="$PWD/.home"
        export CARGO_TARGET_DIR="$PWD/target-conformance-evidence"
        export KIVA_CONFORMANCE_GIT_COMMIT=${lib.escapeShellArg sourceRevision}
        export KIVA_CONFORMANCE_GIT_DIRTY=${if sourceDirty then "1" else "0"}
        mkdir -p "$HOME"
        bash scripts/ci/generate_conformance_evidence.sh \
          --out "$PWD/conformance-evidence" \
          --skip-live \
          --no-require-ieee-reference
      '';
      installPhaseCommand = ''
        mkdir -p "$out"
        cp -r conformance-evidence/. "$out"/
      '';
    };

  pds-evidence-static =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
        pythonPackages.pyyaml
      ]);
    in
    craneLib.mkCargoDerivation {
      pname = "kiva-pds-evidence-static";
      version = "0.0.0";
      inherit src cargoArtifacts;
      doCheck = true;
      doInstallCargoArtifacts = false;
      nativeBuildInputs = [
        pkgs.coreutils
        pkgs.findutils
        pkgs.git
        python
      ];
      buildPhaseCargoCommand = "";
      checkPhaseCargoCommand = ''
        export HOME="$PWD/.home"
        export CARGO_TARGET_DIR="$PWD/target-pds-evidence"
        export KIVA_PDS_GIT_COMMIT=${lib.escapeShellArg sourceRevision}
        export KIVA_PDS_GIT_DIRTY=${if sourceDirty then "1" else "0"}
        mkdir -p "$HOME"
        bash scripts/ci/generate_pds_evidence.sh \
          --out "$PWD/pds-evidence" \
          --skip-postgres-verification \
          --skip-kani \
          --skip-tamarin
      '';
      installPhaseCommand = ''
        mkdir -p "$out"
        cp -r pds-evidence/. "$out"/
      '';
    };

  formal-verification-evidence-smoke =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
      ]);
    in
    craneLib.mkCargoDerivation {
      pname = "kiva-formal-verification-evidence-smoke";
      version = "0.0.0";
      inherit src cargoArtifacts;
      doCheck = true;
      doInstallCargoArtifacts = false;
      nativeBuildInputs = [
        pkgs.coreutils
        pkgs.findutils
        pkgs.fstar
        pkgs.git
        pkgs.tamarin-prover
        kani'
        python
      ];
      buildPhaseCargoCommand = "";
      checkPhaseCargoCommand = ''
        export HOME="$PWD/.home"
        export CARGO_TARGET_DIR="$PWD/target-formal-evidence-kani"
        export LANG=C.UTF-8
        export LC_ALL=C.UTF-8
        export KIVA_KANI_TIMEOUT_SECS=300
        export KIVA_FORMAL_VERIFICATION_GIT_COMMIT=${lib.escapeShellArg sourceRevision}
        export KIVA_FORMAL_VERIFICATION_GIT_DIRTY=${if sourceDirty then "1" else "0"}
        mkdir -p "$HOME"
        bash scripts/ci/generate_formal_verification_evidence.sh \
          --out "$PWD/formal-verification-evidence" \
          --kani-suite smoke
      '';
      installPhaseCommand = ''
        mkdir -p "$out"
        cp -r formal-verification-evidence/. "$out"/
      '';
    };
}
