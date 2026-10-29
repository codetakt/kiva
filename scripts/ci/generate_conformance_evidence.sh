#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/scripts/ci/conformance_lib.sh"

OUT_DIR="${KIVA_CONFORMANCE_EVIDENCE_DIR:-target/conformance-evidence/latest}"
RUN_MEMORY=1
RUN_POSTGRES=1
RUN_POSTGRES_VERIFICATION=1
RUN_ADL=0
ISSUED_EXECUTION_ROOT=""
REQUIRE_CLEAN_GIT=0
REQUIRE_IEEE_REFERENCE="${KIVA_REQUIRE_IEEE_REFERENCE_ROOT:-1}"
IEEE_REFERENCE_ROOT="${KIVA_IEEE_XAPI_REFERENCE_ROOT:-/home/claude/Codex/opensource.ieee.org/xapi/xapi-base-standard-documentation}"
ADL_ROOT="${KIVA_ADL_COMPATIBILITY_ROOT:-/home/claude/Codex/github.com/adlnet/lrs-conformance-test-suite}"

usage() {
  cat <<'EOF'
usage: scripts/ci/generate_conformance_evidence.sh [--out DIR] [--issued-execution-root DIR] [--skip-live] [--skip-memory] [--skip-postgres] [--skip-postgres-verification] [--with-adl-run] [--require-clean-git] [--no-require-ieee-reference]

Generates a fixed conformance evidence bundle under DIR.
The default DIR is target/conformance-evidence/latest.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --issued-execution-root)
      [[ $# -ge 2 ]] || { echo "--issued-execution-root requires a directory" >&2; exit 2; }
      ISSUED_EXECUTION_ROOT="$2"
      shift 2
      ;;
    --out)
      if [[ $# -lt 2 ]]; then
        echo "--out requires a directory" >&2
        exit 2
      fi
      OUT_DIR="$2"
      shift 2
      ;;
    --skip-live)
      RUN_MEMORY=0
      RUN_POSTGRES=0
      RUN_POSTGRES_VERIFICATION=0
      shift
      ;;
    --skip-memory)
      RUN_MEMORY=0
      shift
      ;;
    --skip-postgres)
      RUN_POSTGRES=0
      RUN_POSTGRES_VERIFICATION=0
      shift
      ;;
    --skip-postgres-verification)
      RUN_POSTGRES_VERIFICATION=0
      shift
      ;;
    --with-adl-run)
      RUN_ADL=1
      shift
      ;;
    --require-clean-git)
      REQUIRE_CLEAN_GIT=1
      shift
      ;;
    --no-require-ieee-reference)
      REQUIRE_IEEE_REFERENCE=0
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

cd "$ROOT_DIR"

export PYTHONDONTWRITEBYTECODE=1

if [[ -z "$OUT_DIR" || "$OUT_DIR" == "/" ]]; then
  echo "refusing unsafe evidence output directory: ${OUT_DIR:-<empty>}" >&2
  exit 2
fi
if [[ -e "$OUT_DIR" && ! -d "$OUT_DIR" ]]; then
  echo "evidence output path exists and is not a directory: $OUT_DIR" >&2
  exit 2
fi

kiva_require_command git

normalize_dirty_flag() {
  case "$1" in
    0 | false | clean)
      printf 'false\n'
      ;;
    1 | true | dirty)
      printf 'true\n'
      ;;
    *)
      echo "invalid KIVA_CONFORMANCE_GIT_DIRTY=$1; expected 0/1/true/false/clean/dirty" >&2
      exit 2
      ;;
  esac
}

detect_git_commit() {
  if [[ -n "${KIVA_CONFORMANCE_GIT_COMMIT:-}" ]]; then
    printf '%s\n' "$KIVA_CONFORMANCE_GIT_COMMIT"
  elif git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git rev-parse HEAD
  else
    printf 'unknown\n'
  fi
}

detect_git_dirty() {
  if [[ -n "${KIVA_CONFORMANCE_GIT_DIRTY:-}" ]]; then
    normalize_dirty_flag "$KIVA_CONFORMANCE_GIT_DIRTY"
  elif git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    if [[ -n "$(git status --porcelain)" ]]; then
      printf 'true\n'
    else
      printf 'false\n'
    fi
  else
    printf 'true\n'
  fi
}

GIT_COMMIT="$(detect_git_commit)"
GIT_DIRTY="$(detect_git_dirty)"

if [[ "$REQUIRE_CLEAN_GIT" == 1 && "$GIT_DIRTY" == "true" ]]; then
  echo "refusing release evidence generation from a dirty git tree" >&2
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git status --short >&2
  else
    echo "no Git work tree is available; set KIVA_CONFORMANCE_GIT_DIRTY=0 only for fixed source inputs" >&2
  fi
  exit 2
fi

kiva_require_command cargo
kiva_require_command curl
kiva_require_command sha256sum
kiva_require_command python3
if [[ "$RUN_POSTGRES" == 1 || "$RUN_POSTGRES_VERIFICATION" == 1 ]]; then
  kiva_require_command initdb
  kiva_require_command pg_ctl
  kiva_require_command createdb
  kiva_require_command psql
fi
if [[ "$RUN_ADL" == 1 && -z "${KIVA_ADL_SUITE_ROOT:-}" ]]; then
  echo "--with-adl-run requires KIVA_ADL_SUITE_ROOT" >&2
  exit 2
fi

REPORT_DIR="$OUT_DIR/reports"
JUNIT_DIR="$OUT_DIR/junit"
LOG_DIR="$OUT_DIR/logs"
METADATA_DIR="$OUT_DIR/metadata"
rm -rf "$REPORT_DIR" "$JUNIT_DIR" "$LOG_DIR" "$METADATA_DIR" "$OUT_DIR/README.md"
mkdir -p "$REPORT_DIR" "$JUNIT_DIR" "$LOG_DIR" "$METADATA_DIR"
if [[ -n "$ISSUED_EXECUTION_ROOT" ]]; then
  [[ ! -e "$OUT_DIR/issued" ]] || { echo "issued evidence already exists" >&2; exit 2; }
  [[ -f "$ISSUED_EXECUTION_ROOT/memory/execution.json" && -f "$ISSUED_EXECUTION_ROOT/postgres/execution.json" ]] || {
    echo "issued execution root requires memory and postgres captures" >&2; exit 2;
  }
  cp -a "$ISSUED_EXECUTION_ROOT" "$OUT_DIR/issued"
fi

run_logged() {
  local label="$1"
  shift
  "$@" >"$LOG_DIR/${label}.log" 2>&1
}

run_to_file() {
  local label="$1"
  local output="$2"
  shift 2
  "$@" >"$output" 2>"$LOG_DIR/${label}.log"
}

run_logged tool-versions bash -c '
  git --version
  cargo --version
  rustc --version
  python3 --version
  if command -v psql >/dev/null 2>&1; then psql --version; fi
  if command -v initdb >/dev/null 2>&1; then initdb --version; fi
'

reference_args=(--reference-root "$IEEE_REFERENCE_ROOT")
if [[ "$REQUIRE_IEEE_REFERENCE" == 1 ]]; then
  reference_args+=(--require-reference-root)
fi

run_logged openapi-validation \
  python3 scripts/ci/validate_openapi.py spec/openapi/kiva-xapi.yaml
run_logged lrs-artifact-generation \
  python3 -B scripts/ci/lrs_generated_artifacts.py generate
run_logged conformance-plan-validation \
  python3 scripts/ci/validate_conformance.py spec/conformance/xapi-lrs-discharge.yaml
run_logged conformance-fixture-validation \
  python3 scripts/ci/validate_conformance_fixtures.py --check-schemas

projection_args=(--source "$IEEE_REFERENCE_ROOT/9274.1.1 xAPI Base Standard for LRSs.md")
if [[ "$REQUIRE_IEEE_REFERENCE" != 1 && ! -f "$IEEE_REFERENCE_ROOT/9274.1.1 xAPI Base Standard for LRSs.md" ]]; then
  projection_args=(--skip-source)
fi
run_logged lrs-source-projection-validation \
  python3 scripts/ci/validate_lrs_projection.py --verify-artifacts "${projection_args[@]}"
run_to_file lrs-source-projection-json \
  "$REPORT_DIR/lrs-source-projection.json" \
  python3 scripts/ci/audit_lrs_source_projection.py "${projection_args[@]}" --json
run_to_file lrs-interpretation-dependencies-json \
  "$REPORT_DIR/lrs-interpretation-dependencies.json" \
  python3 scripts/ci/report_lrs_interpretations.py "${projection_args[@]}" --json

run_to_file requirement-catalog-audit-json \
  "$REPORT_DIR/requirement-catalog-audit.json" \
  python3 scripts/ci/audit_requirement_catalog.py "${reference_args[@]}" --json
run_to_file requirement-catalog-audit-md \
  "$REPORT_DIR/requirement-catalog-audit.md" \
  python3 scripts/ci/audit_requirement_catalog.py "${reference_args[@]}"

run_to_file xapi-validate-only-json \
  "$REPORT_DIR/xapi-validate-only.json" \
  cargo run --release -q -p xapi-lrs-conformance -- --validate-only --json \
    --junit "$JUNIT_DIR/xapi-validate-only.xml"

run_to_file conformance-case-matrix-json \
  "$REPORT_DIR/conformance-case-matrix.json" \
  python3 scripts/ci/report_conformance_case_matrix.py --json
run_to_file conformance-case-matrix-md \
  "$REPORT_DIR/conformance-case-matrix.md" \
  python3 scripts/ci/report_conformance_case_matrix.py
run_to_file conformance-coverage-json \
  "$REPORT_DIR/conformance-coverage.json" \
  python3 scripts/ci/report_conformance_coverage.py \
    spec/conformance/xapi-lrs-discharge.yaml --json
run_to_file conformance-coverage-md \
  "$REPORT_DIR/conformance-coverage.md" \
  python3 scripts/ci/report_conformance_coverage.py \
    spec/conformance/xapi-lrs-discharge.yaml

adl_args=(spec/conformance/adl-compatibility-map.yaml)
if [[ -d "$ADL_ROOT" ]]; then
  adl_args+=(--adl-root "$ADL_ROOT")
fi
run_to_file adl-compatibility-json \
  "$REPORT_DIR/adl-compatibility.json" \
  python3 scripts/ci/report_adl_compatibility.py "${adl_args[@]}" --json
run_to_file adl-compatibility-md \
  "$REPORT_DIR/adl-compatibility.md" \
  python3 scripts/ci/report_adl_compatibility.py "${adl_args[@]}"

run_logged static-report-schema-validation \
  python3 scripts/ci/validate_conformance_reports.py \
    --check-schemas \
    --run-report "$REPORT_DIR/xapi-validate-only.json" \
    --case-matrix-report "$REPORT_DIR/conformance-case-matrix.json" \
    --coverage-report "$REPORT_DIR/conformance-coverage.json" \
    --adl-report "$REPORT_DIR/adl-compatibility.json" \
    --requirement-audit-report "$REPORT_DIR/requirement-catalog-audit.json"

if [[ "$RUN_MEMORY" == 1 ]]; then
  KIVA_CONFORMANCE_JSON_REPORT="$REPORT_DIR/xapi-memory-http.json" \
    KIVA_CONFORMANCE_JUNIT_REPORT="$JUNIT_DIR/xapi-memory-http.xml" \
    bash scripts/ci/run_memory_conformance.sh >"$LOG_DIR/xapi-memory-http.log" 2>&1
  run_logged xapi-memory-http-report-validation \
    python3 scripts/ci/validate_conformance_reports.py \
      --run-report "$REPORT_DIR/xapi-memory-http.json"
fi

if [[ "$RUN_POSTGRES" == 1 ]]; then
  KIVA_CONFORMANCE_JSON_REPORT="$REPORT_DIR/xapi-postgres-http.json" \
    KIVA_CONFORMANCE_JUNIT_REPORT="$JUNIT_DIR/xapi-postgres-http.xml" \
    bash scripts/ci/run_postgres_conformance.sh >"$LOG_DIR/xapi-postgres-http.log" 2>&1
  run_logged xapi-postgres-http-report-validation \
    python3 scripts/ci/validate_conformance_reports.py \
      --run-report "$REPORT_DIR/xapi-postgres-http.json"
fi

if [[ "$RUN_POSTGRES_VERIFICATION" == 1 ]]; then
  KIVA_POSTGRES_VERIFICATION_JSON_REPORT="$REPORT_DIR/postgres-verification.json" \
    bash scripts/ci/run_postgres_verification.sh >"$LOG_DIR/postgres-verification.log" 2>&1
  run_logged postgres-verification-report-validation \
    python3 scripts/ci/validate_conformance_reports.py \
      --postgres-verification-report "$REPORT_DIR/postgres-verification.json"
fi

if [[ "$RUN_ADL" == 1 ]]; then
  KIVA_ADL_RUN_REPORT="$REPORT_DIR/adl-run.json" \
    KIVA_ADL_RUN_RAW_LOG="$LOG_DIR/adl-run-raw.json" \
    KIVA_ADL_RUN_CONSOLE_LOG="$LOG_DIR/adl-run-console.log" \
    KIVA_ADL_RUN_SERVER_LOG="$LOG_DIR/adl-run-server.log" \
    bash scripts/ci/run_adl_conformance.sh >"$LOG_DIR/adl-run-runner.log" 2>&1
  run_logged adl-run-report-validation \
    python3 scripts/ci/validate_conformance_reports.py \
      --adl-run-report "$REPORT_DIR/adl-run.json"
fi

python3 - \
  "$OUT_DIR" \
  "$RUN_MEMORY" \
  "$RUN_POSTGRES" \
  "$RUN_POSTGRES_VERIFICATION" \
  "$REQUIRE_CLEAN_GIT" \
  "$IEEE_REFERENCE_ROOT" \
  "$ADL_ROOT" \
  "$GIT_COMMIT" \
  "$GIT_DIRTY" <<'PY' \
  >"$METADATA_DIR/manifest.json"
from __future__ import annotations

import json
import sys
from datetime import UTC, datetime
from pathlib import Path

out_dir = Path(sys.argv[1])
run_memory = sys.argv[2] == "1"
run_postgres = sys.argv[3] == "1"
run_postgres_verification = sys.argv[4] == "1"
require_clean_git = sys.argv[5] == "1"
ieee_reference_root = Path(sys.argv[6])
adl_root = Path(sys.argv[7])
git_commit = sys.argv[8]
git_dirty = sys.argv[9] == "true"


reports = sorted(str(path.relative_to(out_dir)) for path in (out_dir / "reports").glob("*"))
junit = sorted(str(path.relative_to(out_dir)) for path in (out_dir / "junit").glob("*"))
logs = sorted(str(path.relative_to(out_dir)) for path in (out_dir / "logs").glob("*"))
metadata = sorted(
    {
        *(str(path.relative_to(out_dir)) for path in (out_dir / "metadata").glob("*")),
        "metadata/checksums.sha256",
        "metadata/manifest.json",
    }
)

manifest = {
    "schema_version": 1,
    "standard": "IEEE/ISO/IEC 39274-1-1-2025",
    "kind": "conformance-evidence",
    "generated_at_utc": datetime.now(UTC).replace(microsecond=0).isoformat(),
    "git": {
        "commit": git_commit,
        "dirty": git_dirty,
    },
    "release_requirements": {
        "require_clean_git": require_clean_git,
        "met_clean_git": not git_dirty,
    },
    "reference_inputs": {
        "ieee_reference_root": str(ieee_reference_root),
        "ieee_reference_root_exists": ieee_reference_root.is_dir(),
        "adl_root": str(adl_root),
        "adl_root_exists": adl_root.is_dir(),
    },
    "live_http": {
        "memory": run_memory,
        "postgres": run_postgres,
    },
    "verification": {
        "postgres": run_postgres_verification,
    },
    "artifacts": {
        "reports": reports,
        "junit": junit,
        "logs": logs,
        "metadata": metadata,
    },
}
print(json.dumps(manifest, indent=2, sort_keys=True))
PY

python3 - "$OUT_DIR" <<'PY' >"$OUT_DIR/README.md"
from __future__ import annotations

import json
import sys
from pathlib import Path

out_dir = Path(sys.argv[1])


def load_json(relative: str) -> dict:
    with (out_dir / relative).open("r", encoding="utf-8") as handle:
        return json.load(handle)


manifest = load_json("metadata/manifest.json")
audit = load_json("reports/requirement-catalog-audit.json")
coverage = load_json("reports/conformance-coverage.json")
matrix = load_json("reports/conformance-case-matrix.json")
adl = load_json("reports/adl-compatibility.json")
validate_only = load_json("reports/xapi-validate-only.json")
memory = load_json("reports/xapi-memory-http.json") if (out_dir / "reports/xapi-memory-http.json").is_file() else None
postgres = load_json("reports/xapi-postgres-http.json") if (out_dir / "reports/xapi-postgres-http.json").is_file() else None
postgres_verification = load_json("reports/postgres-verification.json") if (out_dir / "reports/postgres-verification.json").is_file() else None
adl_run = load_json("reports/adl-run.json") if (out_dir / "reports/adl-run.json").is_file() else None

print("# Kiva xAPI LRS Conformance Evidence")
print()
print(f"- Standard: `{manifest['standard']}`")
print(f"- Generated at UTC: `{manifest['generated_at_utc']}`")
print(f"- Git commit: `{manifest['git']['commit']}`")
print(f"- Git dirty: `{manifest['git']['dirty']}`")
print(f"- Requirement audit: `{audit['status']}`, {audit['totals']['requirements']} requirements")
print(
    "- Discharge coverage: "
    f"{coverage['totals']['active']}/{coverage['totals']['requirements']} active, "
    f"{coverage['totals']['planned_only']} planned-only"
)
print(
    "- Executable matrix: "
    f"{matrix['totals']['requirements_with_cases']}/{matrix['totals']['requirements']} "
    "requirements with executable cases, "
    f"{matrix['totals']['cases']} cases"
)
print(
    "- ADL compatibility: "
    f"{adl['totals']['equivalent']} equivalent, "
    f"{adl['totals']['partial']} partial, "
    f"{adl['totals']['absent']} absent"
)
print(
    "- Validate-only runner: "
    f"`{validate_only['status']}`, {validate_only['totals']['total']} cases"
)
if memory is not None:
    print(
        "- Memory HTTP runner: "
        f"`{memory['status']}`, "
        f"{memory['totals']['passed']}/{memory['totals']['total']} passed"
    )
if postgres is not None:
    print(
        "- PostgreSQL HTTP runner: "
        f"`{postgres['status']}`, "
        f"{postgres['totals']['passed']}/{postgres['totals']['total']} passed"
    )
if postgres_verification is not None:
    passed = sum(1 for check in postgres_verification["checks"] if check["status"] == "passed")
    total = len(postgres_verification["checks"])
    print(
        "- PostgreSQL execution verification: "
        f"`{postgres_verification['status']}`, {passed}/{total} passed"
    )
if adl_run is not None:
    print(
        "- ADL execution compatibility: "
        f"`{adl_run['status']}`, "
        f"{adl_run['totals']['matched']}/{adl_run['totals']['expected']} expectations matched, "
        f"{adl_run['totals']['expected_open_defect']} open defects"
    )
print("- Checksums: `metadata/checksums.sha256`")
print()
print("This bundle is conformance evidence for Kiva. It is not an IEEE certification record.")
PY

(
  cd "$OUT_DIR"
  find . \
    -type f \
    ! -path './metadata/checksums.sha256' \
    -print0 \
    | sort -z \
    | xargs -0 --no-run-if-empty sha256sum \
    > metadata/checksums.sha256
)

python3 scripts/ci/validate_conformance_evidence_bundle.py "$OUT_DIR"

printf 'generated conformance evidence bundle: %s\n' "$OUT_DIR"
