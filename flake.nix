{
  description = "Kiva, an IEEE/ISO/IEC 39274-1-1-2025 aligned xAPI platform";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay.url = "github:oxalica/rust-overlay";
    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      self,
      nixpkgs,
      flake-utils,
      crane,
      rust-overlay,
      git-hooks,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config = {
            allowBroken = true;
          };
        };
        inherit (pkgs) lib;

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        kani' = pkgs.callPackage ./nix/kani/package.nix { };

        src = lib.cleanSourceWith {
          src = ./.;
          filter =
            path: type:
            let
              rel = lib.removePrefix "${toString ./.}/" (toString path);
              isTargetPath = rel == "target" || lib.hasPrefix "target/" rel;
              keepPrefixes = [
                ".cargo"
                "artifacts"
                "ci"
                "crates"
                "db"
                "docs"
                "examples"
                "fstar"
                "generated"
                "infra"
                "nix"
                "proofs"
                "scripts"
                "spec"
                "tests"
                "tools"
              ];
              keepFiles = [
                "Cargo.lock"
                "Cargo.toml"
                "README.md"
                "clippy.toml"
                "deny.toml"
                "rust-toolchain.toml"
                "rustfmt.toml"
              ];
            in
            (!isTargetPath)
            && (
              lib.cleanSourceFilter path type
              || builtins.any (prefix: rel == prefix || lib.hasPrefix "${prefix}/" rel) keepPrefixes
              || builtins.elem rel keepFiles
            );
        };

        cargoArtifacts = craneLib.buildDepsOnly {
          pname = "kiva-cargo-artifacts";
          version = "0.0.0";
          inherit src;
          cargoToml = ./Cargo.toml;
          cargoLock = ./Cargo.lock;
        };

        strictClippyArgs =
          "-- -D warnings -W clippy::pedantic -W clippy::nursery -W clippy::cargo "
          + "-A clippy::multiple_crate_versions "
          + "-A clippy::missing_errors_doc -A clippy::missing_panics_doc "
          + "-A clippy::module_name_repetitions "
          + "-D clippy::unwrap_used -D clippy::expect_used -D clippy::panic "
          + "-D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro";

        pre-commit-check = git-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            check-added-large-files = {
              enable = true;
              # Source-bound conformance artifacts are checked for reproducibility and drift.
              excludes = [
                "^tests/conformance/http/generated/(behaviors|tables)\\.yaml$"
                "^spec/conformance/lrs-(assertion-manifest|duty-review|table-bindings)\\.yaml$"
              ];
            };
            end-of-file-fixer.enable = true;
            trim-trailing-whitespace.enable = true;
            rustfmt.enable = true;
            nixfmt = {
              enable = true;
              name = "nixfmt";
              entry = "${pkgs.nixfmt}/bin/nixfmt";
              files = "\\.nix$";
              language = "system";
            };
          };
        };

        sharedArgs = {
          inherit
            pkgs
            lib
            craneLib
            src
            cargoArtifacts
            rustToolchain
            kani'
            pre-commit-check
            strictClippyArgs
            ;
          sourceRevision = self.rev or self.dirtyRev or "unknown";
          sourceDirty = !(self ? rev);
        };
      in
      {
        packages = import ./nix/flake/packages.nix sharedArgs;
        checks = import ./nix/flake/checks.nix sharedArgs;
        devShells = import ./nix/flake/dev-shells.nix sharedArgs;
        formatter = pkgs.writeShellApplication {
          name = "kiva-format";
          runtimeInputs = [
            pkgs.findutils
            pkgs.nixfmt
            rustToolchain
          ];
          text = ''
            find . \
              -path ./.git -prune -o \
              -path ./target -prune -o \
              -name '*.nix' -print0 \
              | xargs -0 --no-run-if-empty nixfmt
            cargo fmt --all
          '';
        };
        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/kiva-server";
          meta = {
            description = "Run the Kiva xAPI platform server";
          };
        };
      }
    );
}
