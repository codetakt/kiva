{
  pkgs,
  lib,
  cargoArtifacts,
  rustToolchain,
  kani',
  pre-commit-check,
  ...
}:

let
  python = pkgs.python3.withPackages (pythonPackages: [
    pythonPackages.jsonschema
    pythonPackages.pyyaml
  ]);

  commonPackages = with pkgs; [
    atlas
    cargo-audit
    cargo-deny
    cargo-nextest
    git
    nixfmt
    postgresql_18
    python
    rust-analyzer
    rustToolchain
    sqlx-cli
    taplo
  ];

  strictShellHook = ''
    export RUST_BACKTRACE=1
    export RUSTDOCFLAGS="-D warnings"
    echo "Loaded Kiva strict development shell"
    if [ -f scripts/ci/generate_pkix_unicode32.py ]; then
      python3 -B scripts/ci/generate_pkix_unicode32.py --if-needed
    fi
  '';
in
{
  conformance-definitions = pkgs.mkShell {
    packages = [ python ];
  };

  default = pkgs.mkShell {
    inputsFrom = [ cargoArtifacts ];
    packages = lib.unique (commonPackages ++ pre-commit-check.enabledPackages);
    shellHook = pre-commit-check.shellHook + strictShellHook;
  };

  verification = pkgs.mkShell {
    inputsFrom = [ cargoArtifacts ];
    packages = lib.unique (
      commonPackages
      ++ (with pkgs; [
        fstar
        graphviz
        tamarin-prover
        z3
      ])
      ++ [
        kani'
      ]
    );
    shellHook = strictShellHook + ''
      export PATH=${kani'}/bin:$PATH
      echo "Loaded Kiva verification shell"
    '';
  };
}
