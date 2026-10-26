{
  pkgs,
  craneLib,
  src,
  cargoArtifacts,
  kani',
  pre-commit-check,
  strictClippyArgs,
  ...
}:

let
  rustUseScan = craneLib.buildPackage {
    pname = "kiva-rust-use-scan";
    version = "0.0.0";
    inherit src cargoArtifacts;
    cargoExtraArgs = "--locked -p kiva-rust-use-scan";
    doCheck = false;
  };
in
{
  inherit pre-commit-check;

  fmt = craneLib.cargoFmt {
    pname = "kiva-workspace-fmt";
    version = "0.0.0";
    inherit src cargoArtifacts;
    cargoExtraArgs = "--all";
  };

  clippy = craneLib.cargoClippy {
    pname = "kiva-workspace-clippy";
    version = "0.0.0";
    inherit src cargoArtifacts;
    cargoExtraArgs = "--workspace --all-targets --all-features --locked";
    cargoClippyExtraArgs = strictClippyArgs;
  };

  tests = craneLib.cargoTest {
    pname = "kiva-workspace-tests";
    version = "0.0.0";
    inherit src cargoArtifacts;
    cargoExtraArgs = "--workspace --all-features --locked";
    nativeBuildInputs = [
      (pkgs.python3.withPackages (pythonPackages: [ pythonPackages.pyyaml ]))
    ];
    preCheck = ''
      python -B scripts/ci/lrs_generated_artifacts.py generate
    '';
  };

  postgres-verification = craneLib.mkCargoDerivation {
    pname = "kiva-postgres-verification";
    version = "0.0.0";
    inherit src cargoArtifacts;
    doCheck = true;
    doInstallCargoArtifacts = false;
    nativeBuildInputs = [
      pkgs.postgresql
      pkgs.python3
    ];
    buildPhaseCargoCommand = "";
    checkPhaseCargoCommand = "bash scripts/ci/run_postgres_verification.sh";
    installPhaseCommand = ''
      mkdir -p "$out"
      touch "$out/success"
    '';
  };

  docs = craneLib.cargoDoc {
    pname = "kiva-workspace-docs";
    version = "0.0.0";
    inherit src cargoArtifacts;
    cargoExtraArgs = "--workspace --all-features --locked";
    RUSTDOCFLAGS = "-D warnings";
  };

  cargo-deny = craneLib.cargoDeny {
    pname = "kiva-workspace";
    version = "0.0.0";
    inherit src;
    cargoDenyChecks = "bans licenses sources";
    cargoExtraArgs = "--locked";
  };

  openapi-contract =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
        pythonPackages.pyyaml
      ]);
    in
    pkgs.runCommand "kiva-openapi-contract"
      {
        nativeBuildInputs = [
          python
          rustUseScan
        ];
        KIVA_RUST_USE_SCAN = "${rustUseScan}/bin/kiva-rust-use-scan";
      }
      ''
        cp -r ${src} source
        chmod -R u+w source
        cd source
        mkdir -p reports
        python scripts/ci/run_evidence_checks.py \
          --group openapi-contract \
          --reports-dir reports \
          --json-report reports/openapi-contract-run.json
        mkdir -p "$out"
        cp -r reports "$out"/
        touch "$out/success"
      '';

  confidential-computation-assurance =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
        pythonPackages.pyyaml
      ]);
    in
    pkgs.runCommand "kiva-confidential-computation-assurance"
      {
        nativeBuildInputs = [ python ];
      }
      ''
        cp -r ${src} source
        chmod -R u+w source
        cd source
        mkdir -p reports
        python scripts/ci/run_evidence_checks.py \
          --group confidential-computation-assurance \
          --reports-dir reports \
          --json-report reports/confidential-computation-assurance-run.json
        mkdir -p "$out"
        cp -r reports "$out"/
        touch "$out/success"
      '';

  design-review-suite =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
        pythonPackages.pyyaml
      ]);
    in
    pkgs.runCommand "kiva-design-review-suite"
      {
        nativeBuildInputs = [
          python
          rustUseScan
        ];
        KIVA_RUST_USE_SCAN = "${rustUseScan}/bin/kiva-rust-use-scan";
      }
      ''
        cp -r ${src} source
        chmod -R u+w source
        cd source
        mkdir -p reports
        python scripts/ci/run_design_review_suite.py \
          --suite all \
          --reports-dir reports/design-review \
          --json-report reports/design-review-suite-run.json
        mkdir -p "$out"
        cp -r reports "$out"/
        touch "$out/success"
      '';

  pds-publication-readiness =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
        pythonPackages.pyyaml
      ]);
    in
    craneLib.mkCargoDerivation {
      pname = "kiva-pds-publication-readiness";
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
        export CARGO_TARGET_DIR="$PWD/target-pds-publication"
        mkdir -p "$HOME" reports
        python scripts/ci/export_pds_openapi_schemas.py --check
        python scripts/ci/validate_pds_publication.py \
          --json-report "$PWD/reports/pds-publication-validation.json"
        bash scripts/ci/generate_pds_evidence.sh \
          --out "$PWD/pds-evidence" \
          --skip-postgres-verification \
          --skip-kani \
          --skip-tamarin
      '';
      installPhaseCommand = ''
        mkdir -p "$out"
        cp -r reports pds-evidence "$out"/
        touch "$out/success"
      '';
    };

  memory-conformance =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
        pythonPackages.pyyaml
      ]);
    in
    craneLib.mkCargoDerivation {
      pname = "kiva-memory-conformance";
      version = "0.0.0";
      inherit src cargoArtifacts;
      doCheck = true;
      doInstallCargoArtifacts = false;
      nativeBuildInputs = [
        pkgs.curl
        python
      ];
      buildPhaseCargoCommand = "";
      checkPhaseCargoCommand = ''
        mkdir -p reports junit
        KIVA_CONFORMANCE_JSON_REPORT="$PWD/reports/xapi-memory-http.json" \
          KIVA_CONFORMANCE_JUNIT_REPORT="$PWD/junit/xapi-memory-http.xml" \
          bash scripts/ci/run_memory_conformance.sh
        python scripts/ci/validate_conformance_reports.py \
          --run-report "$PWD/reports/xapi-memory-http.json"
      '';
      installPhaseCommand = ''
        mkdir -p "$out"
        cp -r reports junit "$out"/
        touch "$out/success"
      '';
    };

  postgres-conformance =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
        pythonPackages.pyyaml
      ]);
    in
    craneLib.mkCargoDerivation {
      pname = "kiva-postgres-conformance";
      version = "0.0.0";
      inherit src cargoArtifacts;
      doCheck = true;
      doInstallCargoArtifacts = false;
      nativeBuildInputs = [
        pkgs.curl
        pkgs.postgresql
        python
      ];
      buildPhaseCargoCommand = "";
      checkPhaseCargoCommand = ''
        mkdir -p reports junit
        KIVA_CONFORMANCE_JSON_REPORT="$PWD/reports/xapi-postgres-http.json" \
          KIVA_CONFORMANCE_JUNIT_REPORT="$PWD/junit/xapi-postgres-http.xml" \
          bash scripts/ci/run_postgres_conformance.sh
        python scripts/ci/validate_conformance_reports.py \
          --run-report "$PWD/reports/xapi-postgres-http.json"
      '';
      installPhaseCommand = ''
        mkdir -p "$out"
        cp -r reports junit "$out"/
        touch "$out/success"
      '';
    };

  database-migrations =
    pkgs.runCommand "kiva-database-migrations"
      {
        nativeBuildInputs = [ pkgs.atlas ];
      }
      ''
        cp -r ${src} source
        chmod -R u+w source
        cd source
        atlas migrate validate --dir file://db/migrations
        mkdir -p "$out"
        touch "$out/success"
      '';

  lrs-generated-artifacts =
    let
      python = pkgs.python3.withPackages (pythonPackages: [ pythonPackages.pyyaml ]);
    in
    pkgs.runCommand "kiva-lrs-generated-artifacts" { nativeBuildInputs = [ python ]; } ''
      cp -r ${src} source
      chmod -R u+w source
      cd source
      python -B scripts/ci/lrs_generated_artifacts.py generate
      python -B scripts/ci/lrs_generated_artifacts.py check
      mkdir -p "$out"
      python -B scripts/ci/lrs_generated_artifacts.py bundle \
        --output "$out/lrs-static.tar.gz" > "$out/bundle.json"
      cp spec/conformance/lrs-generated-artifacts.lock.json "$out/"
    '';

  lrs-oracle-controls =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
        pythonPackages.pyyaml
      ]);
    in
    pkgs.runCommand "kiva-lrs-oracle-controls" { nativeBuildInputs = [ python ]; } ''
      cp -r ${src} source
      chmod -R u+w source
      cd source
      python -B scripts/ci/lrs_generated_artifacts.py generate
      python -B -m unittest discover -s scripts/ci -p 'test_*.py'
      mkdir -p "$out"
      touch "$out/success"
    '';

  conformance-plan =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
        pythonPackages.pyyaml
      ]);
    in
    pkgs.runCommand "kiva-conformance-plan"
      {
        nativeBuildInputs = [
          python
          pkgs.openssl
        ];
      }
      ''
        cp -r ${src} source
        chmod -R u+w source
        cd source
        python scripts/ci/validate_lrs_projection.py --definitions-only --skip-source
        python scripts/ci/validate_conformance.py spec/conformance/xapi-lrs-discharge.yaml
        python scripts/ci/validate_conformance_fixtures.py --check-schemas
        python scripts/ci/report_lrs_interpretations.py --skip-source --json > lrs-interpretations.json
        python scripts/ci/audit_requirement_catalog.py --no-reference-root > requirement-audit.md
        python scripts/ci/audit_requirement_catalog.py --no-reference-root --json > requirement-audit.json
        python scripts/ci/report_conformance_case_matrix.py > case-matrix.md
        python scripts/ci/report_conformance_case_matrix.py --json > case-matrix.json
        python scripts/ci/report_conformance_coverage.py spec/conformance/xapi-lrs-discharge.yaml > coverage.md
        python scripts/ci/report_conformance_coverage.py spec/conformance/xapi-lrs-discharge.yaml --json > coverage.json
        python scripts/ci/report_adl_compatibility.py spec/conformance/adl-compatibility-map.yaml > adl-compatibility.md
        python scripts/ci/report_adl_compatibility.py spec/conformance/adl-compatibility-map.yaml --json > adl-compatibility.json
        python scripts/ci/validate_conformance_reports.py \
          --check-schemas \
          --case-matrix-report case-matrix.json \
          --coverage-report coverage.json \
          --adl-report adl-compatibility.json \
          --requirement-audit-report requirement-audit.json
        python - <<'PY'
        from __future__ import annotations

        import hashlib
        import json
        from datetime import UTC, datetime
        from pathlib import Path

        root = Path("conformance-evidence-fixture")
        (root / "reports").mkdir(parents=True)
        (root / "junit").mkdir()
        (root / "logs").mkdir()
        (root / "metadata").mkdir()

        def write_json(relative: str, data: object) -> None:
            (root / relative).write_text(
                json.dumps(data, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )

        standard = "IEEE/ISO/IEC 39274-1-1-2025"
        write_json(
            "reports/requirement-catalog-audit.json",
            {
                "schema_version": 1,
                "standard": standard,
                "status": "passed",
                "requirement_catalog": "fixture",
                "source": {
                    "repository": "fixture",
                    "document": "fixture",
                    "reference_root": None,
                    "reference_path": None,
                    "reference_checked": False,
                    "source_document_exists": False,
                    "git_commit": None,
                    "git_dirty": None,
                },
                "totals": {
                    "requirements": 0,
                    "clauses": 0,
                    "source_clauses": 0,
                    "source_clauses_with_requirements": 0,
                    "source_clauses_triaged_without_requirements": 0,
                },
                "levels": {},
                "projections": {},
                "source_triage": {
                    "path": None,
                    "checked": False,
                    "triaged_clauses": 0,
                },
                "clauses": [],
                "issues": [],
            },
        )
        write_json(
            "reports/xapi-validate-only.json",
            {
                "schema_version": 1,
                "standard": standard,
                "mode": "validate-only",
                "status": "validated",
                "totals": {
                    "total": 0,
                    "passed": 0,
                    "failed": 0,
                    "validated": 0,
                },
                "suites": [],
            },
        )
        write_json(
            "reports/conformance-case-matrix.json",
            {
                "schema_version": 1,
                "standard": standard,
                "requirement_catalog": "fixture",
                "executable_gap_map": "fixture",
                "case_roots": ["tests/conformance/http"],
                "totals": {
                    "requirements": 0,
                    "suites": 0,
                    "cases": 0,
                    "requirements_with_cases": 0,
                    "requirements_without_cases": 0,
                    "profile_sensitive_requirements": 0,
                    "unclassified_uncovered_requirements": 0,
                    "endpoint_profile_required_cases": 0,
                    "destructive_cases": 0,
                    "low_side_effect_cases": 0,
                },
                "suites": [],
                "cases": [],
                "requirements": [],
                "uncovered_requirements": [],
                "profile_sensitive_requirements": [],
                "unclassified_uncovered_requirements": [],
                "endpoint_profile_required_cases": [],
            },
        )
        write_json(
            "reports/conformance-coverage.json",
            {
                "schema_version": 1,
                "standard": standard,
                "requirement_catalog": "fixture",
                "discharge_plan": "fixture",
                "totals": {
                    "requirements": 0,
                    "active": 0,
                    "planned_only": 0,
                },
                "active_by_method": {},
                "planned_only": [],
                "requirements": [],
            },
        )
        write_json(
            "reports/adl-compatibility.json",
            {
                "schema_version": 1,
                "standard": standard,
                "compatibility_map": "fixture",
                "requirement_catalog": "fixture",
                "adl_suite": {
                    "id": "fixture",
                    "upstream": "fixture",
                    "test_root": "fixture",
                    "role": "fixture",
                    "license": "fixture",
                },
                "adl_source_verification": {
                    "checked": False,
                    "root": None,
                    "exists": False,
                    "git_commit": None,
                    "git_dirty": None,
                },
                "totals": {
                    "requirements": 0,
                    "mappings": 0,
                    "equivalent": 0,
                    "partial": 0,
                    "absent": 0,
                    "unclassified": 0,
                },
                "mappings_by_relationship": {
                    "absent": 0,
                    "equivalent": 0,
                    "obsolete": 0,
                    "partial": 0,
                },
                "requirements": {
                    "equivalent": [],
                    "partial": [],
                    "absent": [],
                    "unclassified": [],
                },
            },
        )

        for report_name in [
            "requirement-catalog-audit.md",
            "conformance-case-matrix.md",
            "conformance-coverage.md",
            "adl-compatibility.md",
        ]:
            (root / "reports" / report_name).write_text("fixture\n", encoding="utf-8")
        (root / "junit/xapi-validate-only.xml").write_text(
            '<testsuite tests="0" failures="0"></testsuite>\n',
            encoding="utf-8",
        )
        for log_name in [
            "tool-versions.log",
            "openapi-validation.log",
            "conformance-plan-validation.log",
            "conformance-fixture-validation.log",
            "requirement-catalog-audit-json.log",
            "requirement-catalog-audit-md.log",
            "xapi-validate-only-json.log",
            "conformance-case-matrix-json.log",
            "conformance-case-matrix-md.log",
            "conformance-coverage-json.log",
            "conformance-coverage-md.log",
            "adl-compatibility-json.log",
            "adl-compatibility-md.log",
            "static-report-schema-validation.log",
        ]:
            (root / "logs" / log_name).write_text("fixture\n", encoding="utf-8")

        manifest = {
            "schema_version": 1,
            "standard": standard,
            "kind": "conformance-evidence",
            "generated_at_utc": datetime.now(UTC).replace(microsecond=0).isoformat(),
            "git": {
                "commit": "fixture",
                "dirty": False,
            },
            "release_requirements": {
                "require_clean_git": False,
                "met_clean_git": True,
            },
            "reference_inputs": {
                "ieee_reference_root": "fixture/ieee",
                "ieee_reference_root_exists": False,
                "adl_root": "fixture/adl",
                "adl_root_exists": False,
            },
            "live_http": {
                "memory": False,
                "postgres": False,
            },
            "verification": {
                "postgres": False,
            },
            "artifacts": {
                "reports": sorted(
                    path.relative_to(root).as_posix()
                    for path in (root / "reports").glob("*")
                    if path.is_file()
                ),
                "junit": sorted(
                    path.relative_to(root).as_posix()
                    for path in (root / "junit").glob("*")
                    if path.is_file()
                ),
                "logs": sorted(
                    path.relative_to(root).as_posix()
                    for path in (root / "logs").glob("*")
                    if path.is_file()
                ),
                "metadata": [
                    "metadata/checksums.sha256",
                    "metadata/manifest.json",
                ],
            },
        }
        (root / "metadata/manifest.json").write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        (root / "README.md").write_text("# Fixture\n", encoding="utf-8")

        checksums = []
        for path in sorted(root.rglob("*")):
            if not path.is_file() or path.name == "checksums.sha256":
                continue
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            checksums.append(f"{digest}  ./{path.relative_to(root).as_posix()}\n")
        (root / "metadata/checksums.sha256").write_text("".join(checksums), encoding="utf-8")
        PY
        python scripts/ci/validate_conformance_evidence_bundle.py conformance-evidence-fixture
        mkdir -p "$out"
        cp requirement-audit.md "$out/requirement-catalog-audit.md"
        cp requirement-audit.json "$out/requirement-catalog-audit.json"
        cp case-matrix.md "$out/conformance-case-matrix.md"
        cp case-matrix.json "$out/conformance-case-matrix.json"
        cp coverage.md "$out/conformance-coverage.md"
        cp coverage.json "$out/conformance-coverage.json"
        cp adl-compatibility.md "$out/adl-compatibility.md"
        cp adl-compatibility.json "$out/adl-compatibility.json"
        touch "$out/success"
      '';

  fstar-boundary =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
      ]);
    in
    pkgs.runCommand "kiva-fstar-boundary"
      {
        nativeBuildInputs = [
          pkgs.fstar
          python
        ];
      }
      ''
        cp -r ${src} source
        chmod -R u+w source
        cd source
        export KIVA_FSTAR_JSON_REPORT="$PWD/fstar-report.json"
        bash scripts/ci/run_fstar_verification.sh
        python scripts/ci/validate_formal_verification_reports.py "$KIVA_FSTAR_JSON_REPORT"
        mkdir -p "$out"
        cp "$KIVA_FSTAR_JSON_REPORT" "$out"/
        touch "$out/success"
      '';

  formal-verification-contract =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
      ]);
    in
    pkgs.runCommand "kiva-formal-verification-contract"
      {
        nativeBuildInputs = [ python ];
      }
      ''
        cp -r ${src} source
        chmod -R u+w source
        cd source
        python scripts/ci/validate_formal_verification_reports.py --check-schemas
        python - <<'PY'
        from __future__ import annotations

        import hashlib
        import json
        from datetime import UTC, datetime
        from pathlib import Path

        root = Path("formal-evidence-fixture")
        (root / "reports").mkdir(parents=True)
        (root / "logs").mkdir()
        (root / "metadata").mkdir()

        report = {
            "schema_version": 1,
            "standard": "Kiva Architecture",
            "kind": "fstar-verification",
            "status": "passed",
            "generated_at_utc": datetime.now(UTC).replace(microsecond=0).isoformat(),
            "tool_version": "fixture-fstar",
            "checks": [
                {
                    "id": "Fixture",
                    "target": "fstar/Fixture.fst",
                    "status": "passed",
                }
            ],
        }
        (root / "reports/fstar.json").write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        for log_name in [
            "formal-evidence-manifest-validation.log",
            "fstar-report-validation.log",
            "fstar.log",
            "tool-versions.log",
        ]:
            (root / "logs" / log_name).write_text("fixture\n", encoding="utf-8")

        skipped = {
            "enabled": False,
            "status": "skipped",
            "report_path": None,
            "log_path": None,
            "tool_version": None,
            "suite": None,
            "checks": 0,
            "passed": 0,
            "failed": 0,
            "timeout": 0,
        }
        manifest = {
            "schema_version": 1,
            "standard": "Kiva Architecture",
            "kind": "formal-verification-evidence",
            "status": "passed",
            "generated_at_utc": datetime.now(UTC).replace(microsecond=0).isoformat(),
            "git": {
                "commit": "fixture",
                "dirty": False,
            },
            "release_requirements": {
                "require_clean_git": False,
                "met_clean_git": True,
            },
            "verification": {
                "fstar": {
                    "enabled": True,
                    "status": "passed",
                    "report_path": "reports/fstar.json",
                    "log_path": "logs/fstar.log",
                    "tool_version": "fixture-fstar",
                    "suite": None,
                    "checks": 1,
                    "passed": 1,
                    "failed": 0,
                    "timeout": 0,
                },
                "kani": skipped,
                "tamarin": skipped,
            },
            "totals": {
                "enabled_tools": 1,
                "skipped_tools": 2,
                "checks": 1,
                "passed": 1,
                "failed": 0,
                "timeout": 0,
            },
            "artifacts": {
                "reports": ["reports/fstar.json"],
                "logs": [
                    "logs/formal-evidence-manifest-validation.log",
                    "logs/fstar-report-validation.log",
                    "logs/fstar.log",
                    "logs/tool-versions.log",
                ],
                "metadata": [
                    "metadata/checksums.sha256",
                    "metadata/manifest.json",
                ],
            },
        }
        (root / "metadata/manifest.json").write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        (root / "README.md").write_text("# Fixture\n", encoding="utf-8")

        checksums = []
        for path in sorted(root.rglob("*")):
            if not path.is_file() or path.name == "checksums.sha256":
                continue
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            checksums.append(f"{digest}  ./{path.relative_to(root).as_posix()}\n")
        (root / "metadata/checksums.sha256").write_text("".join(checksums), encoding="utf-8")
        PY
        python scripts/ci/validate_formal_verification_evidence_bundle.py formal-evidence-fixture
        mkdir -p "$out"
        touch "$out/success"
      '';

  release-evidence-contract =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
      ]);
    in
    pkgs.runCommand "kiva-release-evidence-contract"
      {
        nativeBuildInputs = [ python ];
      }
      ''
        cp -r ${src} source
        chmod -R u+w source
        cd source
        python - <<'PY'
        from __future__ import annotations

        import hashlib
        import json
        from datetime import UTC, datetime
        from pathlib import Path

        now = datetime.now(UTC).replace(microsecond=0).isoformat()
        commit = "fixture-release"
        standard = "IEEE/ISO/IEC 39274-1-1-2025"
        root = Path("release-evidence-fixture")
        conformance_root = root / "conformance"
        formal_root = root / "formal"

        def write_json(base: Path, relative: str, data: object) -> None:
            path = base / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")

        def write_text(base: Path, relative: str, text: str = "fixture\n") -> None:
            path = base / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")

        def sha256sum(base: Path) -> None:
            checksums = []
            for path in sorted(base.rglob("*")):
                if not path.is_file() or path.name == "checksums.sha256":
                    continue
                digest = hashlib.sha256(path.read_bytes()).hexdigest()
                checksums.append(f"{digest}  ./{path.relative_to(base).as_posix()}\n")
            (base / "metadata/checksums.sha256").write_text("".join(checksums), encoding="utf-8")

        def artifact_list(base: Path, group: str) -> list[str]:
            return sorted(
                path.relative_to(base).as_posix()
                for path in (base / group).glob("*")
                if path.is_file()
            )

        def conformance_run_report(mode: str) -> dict[str, object]:
            if mode == "validate-only":
                return {
                    "schema_version": 1,
                    "standard": standard,
                    "mode": "validate-only",
                    "status": "validated",
                    "totals": {
                        "total": 1,
                        "passed": 0,
                        "failed": 0,
                        "validated": 1,
                    },
                    "suites": [
                        {
                            "id": "fixture-suite",
                            "path": "tests/conformance/http/fixture.yaml",
                            "cases": [
                                {
                                    "id": "fixture-validate",
                                    "requirements": ["fixture-requirement"],
                                    "safety": {
                                        "destructive": False,
                                        "requires_isolated_lrs": False,
                                    },
                                    "outcome": "validated",
                                    "failures": [],
                                }
                            ],
                        }
                    ],
                }
            return {
                "schema_version": 1,
                "standard": standard,
                "mode": "execute-http",
                "status": "passed",
                "totals": {
                    "total": 1,
                    "passed": 1,
                    "failed": 0,
                    "validated": 0,
                },
                "suites": [
                    {
                        "id": "fixture-suite",
                        "path": "tests/conformance/http/fixture.yaml",
                        "cases": [
                            {
                                "id": f"fixture-{mode}",
                                "requirements": ["fixture-requirement"],
                                "safety": {
                                    "destructive": True,
                                    "requires_isolated_lrs": True,
                                },
                                "outcome": "passed",
                                "failures": [],
                            }
                        ],
                    }
                ],
            }

        def create_conformance_fixture() -> None:
            for directory in ["reports", "junit", "logs", "metadata"]:
                (conformance_root / directory).mkdir(parents=True, exist_ok=True)
            write_json(
                conformance_root,
                "reports/requirement-catalog-audit.json",
                {
                    "schema_version": 1,
                    "standard": standard,
                    "status": "passed",
                    "requirement_catalog": "fixture",
                    "source": {
                        "repository": "fixture",
                        "document": "fixture",
                        "reference_root": None,
                        "reference_path": None,
                        "reference_checked": False,
                        "source_document_exists": False,
                        "git_commit": None,
                        "git_dirty": None,
                    },
                    "totals": {
                        "requirements": 1,
                        "clauses": 1,
                        "source_clauses": 1,
                        "source_clauses_with_requirements": 1,
                        "source_clauses_triaged_without_requirements": 0,
                    },
                    "levels": {
                        "must": 1,
                    },
                    "projections": {
                        "http": 1,
                    },
                    "source_triage": {
                        "path": None,
                        "checked": False,
                        "triaged_clauses": 0,
                    },
                    "clauses": [],
                    "issues": [],
                },
            )
            write_json(conformance_root, "reports/xapi-validate-only.json", conformance_run_report("validate-only"))
            write_json(conformance_root, "reports/xapi-memory-http.json", conformance_run_report("memory-http"))
            write_json(conformance_root, "reports/xapi-postgres-http.json", conformance_run_report("postgres-http"))
            write_json(
                conformance_root,
                "reports/conformance-case-matrix.json",
                {
                    "schema_version": 1,
                    "standard": standard,
                    "requirement_catalog": "fixture",
                    "executable_gap_map": "fixture",
                    "case_roots": ["tests/conformance/http"],
                    "totals": {
                        "requirements": 1,
                        "suites": 1,
                        "cases": 1,
                        "requirements_with_cases": 1,
                        "requirements_without_cases": 0,
                        "profile_sensitive_requirements": 0,
                        "unclassified_uncovered_requirements": 0,
                        "endpoint_profile_required_cases": 0,
                        "destructive_cases": 1,
                        "low_side_effect_cases": 0,
                    },
                    "suites": [
                        {
                            "id": "fixture-suite",
                            "path": "tests/conformance/http/fixture.yaml",
                            "cases": ["fixture-case"],
                        }
                    ],
                    "cases": [
                        {
                            "id": "fixture-case",
                            "suite": "fixture-suite",
                            "path": "tests/conformance/http/fixture.yaml",
                            "requirements": ["fixture-requirement"],
                            "safety": {
                                "destructive": True,
                                "requires_isolated_lrs": True,
                            },
                            "requires_endpoint_profiles": [],
                        }
                    ],
                    "requirements": [],
                    "uncovered_requirements": [],
                    "profile_sensitive_requirements": [],
                    "unclassified_uncovered_requirements": [],
                    "endpoint_profile_required_cases": [],
                },
            )
            write_json(
                conformance_root,
                "reports/conformance-coverage.json",
                {
                    "schema_version": 1,
                    "standard": standard,
                    "requirement_catalog": "fixture",
                    "discharge_plan": "fixture",
                    "totals": {
                        "requirements": 1,
                        "active": 1,
                        "planned_only": 0,
                    },
                    "active_by_method": {
                        "http-test": 1,
                    },
                    "planned_only": [],
                    "requirements": [],
                },
            )
            write_json(
                conformance_root,
                "reports/adl-compatibility.json",
                {
                    "schema_version": 1,
                    "standard": standard,
                    "compatibility_map": "fixture",
                    "requirement_catalog": "fixture",
                    "adl_suite": {
                        "id": "fixture",
                        "upstream": "fixture",
                        "test_root": "fixture",
                        "role": "fixture",
                        "license": "fixture",
                    },
                    "adl_source_verification": {
                        "checked": False,
                        "root": None,
                        "exists": False,
                        "git_commit": None,
                        "git_dirty": None,
                    },
                    "totals": {
                        "requirements": 1,
                        "mappings": 0,
                        "equivalent": 0,
                        "partial": 0,
                        "absent": 1,
                        "unclassified": 0,
                    },
                    "mappings_by_relationship": {
                        "absent": 1,
                        "equivalent": 0,
                        "obsolete": 0,
                        "partial": 0,
                    },
                    "requirements": {
                        "equivalent": [],
                        "partial": [],
                        "absent": [],
                        "unclassified": [],
                    },
                },
            )
            write_json(
                conformance_root,
                "reports/postgres-verification.json",
                {
                    "schema_version": 1,
                    "standard": "Kiva Architecture",
                    "kind": "postgres-verification",
                    "status": "passed",
                    "backend": "postgres",
                    "generated_at_utc": now,
                    "temporary_cluster": True,
                    "migrations_applied": True,
                    "checks": [
                        {
                            "id": "fixture-postgres-check",
                            "crate": "kiva-server",
                            "status": "passed",
                        }
                    ],
                },
            )
            for report_name in [
                "requirement-catalog-audit.md",
                "conformance-case-matrix.md",
                "conformance-coverage.md",
                "adl-compatibility.md",
            ]:
                write_text(conformance_root, f"reports/{report_name}")
            for junit_name in [
                "xapi-validate-only.xml",
                "xapi-memory-http.xml",
                "xapi-postgres-http.xml",
            ]:
                write_text(conformance_root, f"junit/{junit_name}", '<testsuite tests="1" failures="0"></testsuite>\n')
            for log_name in [
                "tool-versions.log",
                "openapi-validation.log",
                "conformance-plan-validation.log",
                "conformance-fixture-validation.log",
                "requirement-catalog-audit-json.log",
                "requirement-catalog-audit-md.log",
                "xapi-validate-only-json.log",
                "conformance-case-matrix-json.log",
                "conformance-case-matrix-md.log",
                "conformance-coverage-json.log",
                "conformance-coverage-md.log",
                "adl-compatibility-json.log",
                "adl-compatibility-md.log",
                "static-report-schema-validation.log",
                "xapi-memory-http.log",
                "xapi-memory-http-report-validation.log",
                "xapi-postgres-http.log",
                "xapi-postgres-http-report-validation.log",
                "postgres-verification.log",
                "postgres-verification-report-validation.log",
            ]:
                write_text(conformance_root, f"logs/{log_name}")
            write_text(conformance_root, "README.md", "# Conformance Fixture\n")
            write_json(
                conformance_root,
                "metadata/manifest.json",
                {
                    "schema_version": 1,
                    "standard": standard,
                    "kind": "conformance-evidence",
                    "generated_at_utc": now,
                    "git": {
                        "commit": commit,
                        "dirty": False,
                    },
                    "release_requirements": {
                        "require_clean_git": True,
                        "met_clean_git": True,
                    },
                    "reference_inputs": {
                        "ieee_reference_root": "fixture/ieee",
                        "ieee_reference_root_exists": False,
                        "adl_root": "fixture/adl",
                        "adl_root_exists": False,
                    },
                    "live_http": {
                        "memory": True,
                        "postgres": True,
                    },
                    "verification": {
                        "postgres": True,
                    },
                    "artifacts": {
                        "reports": artifact_list(conformance_root, "reports"),
                        "junit": artifact_list(conformance_root, "junit"),
                        "logs": artifact_list(conformance_root, "logs"),
                        "metadata": [
                            "metadata/checksums.sha256",
                            "metadata/manifest.json",
                        ],
                    },
                },
            )
            sha256sum(conformance_root)

        def formal_report(kind: str, suite: str | None = None) -> dict[str, object]:
            report = {
                "schema_version": 1,
                "standard": "Kiva Architecture",
                "kind": kind,
                "status": "passed",
                "generated_at_utc": now,
                "tool_version": f"fixture-{kind}",
                "checks": [
                    {
                        "id": f"{kind}-fixture-check",
                        "target": "fixture",
                        "status": "passed",
                    }
                ],
            }
            if suite is not None:
                report.update(
                    {
                        "suite": suite,
                        "run_id": "fixture-run",
                        "solver": "fixture-solver",
                        "unwind": 1,
                        "timeout_seconds": 1,
                        "harnesses": {
                            "fixture": "passed",
                        },
                    }
                )
            return report

        def create_formal_fixture() -> None:
            for directory in ["reports", "logs", "metadata"]:
                (formal_root / directory).mkdir(parents=True, exist_ok=True)
            reports = {
                "fstar": formal_report("fstar-verification"),
                "kani": formal_report("kani-verification", "regression"),
                "tamarin": formal_report("tamarin-verification"),
            }
            for name, report in reports.items():
                write_json(formal_root, f"reports/{name}.json", report)
            for log_name in [
                "tool-versions.log",
                "formal-evidence-manifest-validation.log",
                "fstar.log",
                "fstar-report-validation.log",
                "kani.log",
                "kani-report-validation.log",
                "tamarin.log",
                "tamarin-report-validation.log",
            ]:
                write_text(formal_root, f"logs/{log_name}")
            verification = {}
            for name, report in reports.items():
                verification[name] = {
                    "enabled": True,
                    "status": "passed",
                    "report_path": f"reports/{name}.json",
                    "log_path": f"logs/{name}.log",
                    "tool_version": report["tool_version"],
                    "suite": report.get("suite"),
                    "checks": 1,
                    "passed": 1,
                    "failed": 0,
                    "timeout": 0,
                }
            write_text(formal_root, "README.md", "# Formal Fixture\n")
            write_json(
                formal_root,
                "metadata/manifest.json",
                {
                    "schema_version": 1,
                    "standard": "Kiva Architecture",
                    "kind": "formal-verification-evidence",
                    "status": "passed",
                    "generated_at_utc": now,
                    "git": {
                        "commit": commit,
                        "dirty": False,
                    },
                    "release_requirements": {
                        "require_clean_git": True,
                        "met_clean_git": True,
                    },
                    "verification": verification,
                    "totals": {
                        "enabled_tools": 3,
                        "skipped_tools": 0,
                        "checks": 3,
                        "passed": 3,
                        "failed": 0,
                        "timeout": 0,
                    },
                    "artifacts": {
                        "reports": artifact_list(formal_root, "reports"),
                        "logs": artifact_list(formal_root, "logs"),
                        "metadata": [
                            "metadata/checksums.sha256",
                            "metadata/manifest.json",
                        ],
                    },
                },
            )
            sha256sum(formal_root)

        create_conformance_fixture()
        create_formal_fixture()
        PY
        python scripts/ci/validate_release_evidence_set.py \
          --conformance release-evidence-fixture/conformance \
          --formal release-evidence-fixture/formal
        mkdir -p "$out"
        touch "$out/success"
      '';

  kani-boundary =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
      ]);
    in
    craneLib.mkCargoDerivation {
      pname = "kiva-kani-boundary";
      version = "0.0.0";
      inherit src cargoArtifacts;
      doCheck = true;
      doInstallCargoArtifacts = false;
      nativeBuildInputs = [
        kani'
        python
      ];
      buildPhaseCargoCommand = "";
      checkPhaseCargoCommand = ''
        export HOME="$PWD/.home"
        export CARGO_TARGET_DIR="$PWD/target-kani"
        export KIVA_KANI_TIMEOUT_SECS=300
        export KIVA_KANI_JSON_REPORT="$PWD/kani-report.json"
        mkdir -p "$HOME"
        bash scripts/ci/run_kani_verification.sh
        python scripts/ci/validate_formal_verification_reports.py "$KIVA_KANI_JSON_REPORT"
      '';
      installPhaseCommand = ''
        mkdir -p "$out"
        cp kani-report.json "$out"/
        touch "$out/success"
      '';
    };

  tamarin-boundary =
    let
      python = pkgs.python3.withPackages (pythonPackages: [
        pythonPackages.jsonschema
      ]);
    in
    pkgs.runCommand "kiva-tamarin-boundary"
      {
        nativeBuildInputs = [
          pkgs.tamarin-prover
          python
        ];
      }
      ''
        cp -r ${src} source
        chmod -R u+w source
        cd source
        export LANG=C.UTF-8
        export LC_ALL=C.UTF-8
        export KIVA_TAMARIN_JSON_REPORT="$PWD/tamarin-report.json"
        bash scripts/ci/run_tamarin_verification.sh
        python scripts/ci/validate_formal_verification_reports.py "$KIVA_TAMARIN_JSON_REPORT"
        mkdir -p "$out"
        cp "$KIVA_TAMARIN_JSON_REPORT" "$out"/
        touch "$out/success"
      '';
}
