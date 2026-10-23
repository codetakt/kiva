{
  pkgs,
  craneLib,
  src,
  cargoArtifacts,
  ...
}:

let
  python = pkgs.python3.withPackages (p: [
    p.jsonschema
    p.pyyaml
  ]);
  backend =
    name:
    let
      evidence = craneLib.mkCargoDerivation {
        pname = "kiva-${name}-conformance-evidence";
        version = "0.0.0";
        inherit src cargoArtifacts;
        doCheck = true;
        doInstallCargoArtifacts = false;
        nativeBuildInputs = [
          pkgs.curl
          python
        ]
        ++ pkgs.lib.optionals (name == "postgres") [ pkgs.postgresql ];
        buildPhaseCargoCommand = "";
        checkPhaseCargoCommand = ''
          python -B scripts/ci/check_conformance_regressions.py capture \
            --backend ${name} --output-dir "$PWD/reports"
        '';
        installPhaseCommand = ''
          mkdir -p "$out"
          cp -r reports "$out/"
        '';
      };
      gate =
        pkgs.runCommand "kiva-${name}-conformance"
          {
            nativeBuildInputs = [ python ];
          }
          ''
            cp -r ${src} source
            chmod -R u+w source
            cd source
            python -B scripts/ci/lrs_generated_artifacts.py generate
            python -B scripts/ci/check_conformance_regressions.py gate \
              --backend ${name} --capture-dir ${evidence}/reports --output-dir "$PWD/decision"
            mkdir -p "$out"
            cp -r decision "$out/"
            ln -s ${evidence}/reports "$out/reports"
            touch "$out/success"
          '';
    in
    {
      inherit evidence gate;
    };
in
{
  memory = backend "memory";
  postgres = backend "postgres";
}
