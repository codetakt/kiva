#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/scripts/ci/conformance_lib.sh"

OUT_DIR="${KIVA_PDS_EVIDENCE_DIR:-target/pds-evidence/latest}"
RUN_POSTGRES_VERIFICATION=1
RUN_KANI=1
RUN_TAMARIN=1
KANI_SUITE="${KIVA_KANI_SUITE:-regression}"
REQUIRE_CLEAN_GIT=0

usage() {
  cat <<'EOF'
usage: scripts/ci/generate_pds_evidence.sh [--out DIR] [--skip-postgres-verification] [--skip-kani] [--skip-tamarin] [--kani-suite smoke|regression] [--require-clean-git]

Generates a fixed Kiva PDS publication evidence bundle under DIR.
The default DIR is target/pds-evidence/latest.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out)
      if [[ $# -lt 2 ]]; then
        echo "--out requires a directory" >&2
        exit 2
      fi
      OUT_DIR="$2"
      shift 2
      ;;
    --skip-postgres-verification)
      RUN_POSTGRES_VERIFICATION=0
      shift
      ;;
    --skip-kani)
      RUN_KANI=0
      shift
      ;;
    --skip-tamarin)
      RUN_TAMARIN=0
      shift
      ;;
    --kani-suite)
      if [[ $# -lt 2 ]]; then
        echo "--kani-suite requires smoke or regression" >&2
        exit 2
      fi
      KANI_SUITE="$2"
      shift 2
      ;;
    --require-clean-git)
      REQUIRE_CLEAN_GIT=1
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

case "$KANI_SUITE" in
  smoke | regression) ;;
  *)
    echo "unknown Kani suite: $KANI_SUITE; expected smoke or regression" >&2
    exit 2
    ;;
esac

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
kiva_require_command cargo
kiva_require_command python3
kiva_require_command sha256sum
if [[ "$RUN_POSTGRES_VERIFICATION" == 1 ]]; then
  kiva_require_command initdb
  kiva_require_command pg_ctl
  kiva_require_command createdb
  kiva_require_command psql
fi

normalize_dirty_flag() {
  case "$1" in
    0 | false | clean)
      printf 'false\n'
      ;;
    1 | true | dirty)
      printf 'true\n'
      ;;
    *)
      echo "invalid KIVA_PDS_GIT_DIRTY=$1; expected 0/1/true/false/clean/dirty" >&2
      exit 2
      ;;
  esac
}

detect_git_commit() {
  if [[ -n "${KIVA_PDS_GIT_COMMIT:-}" ]]; then
    printf '%s\n' "$KIVA_PDS_GIT_COMMIT"
  elif git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git rev-parse HEAD
  else
    printf 'unknown\n'
  fi
}

detect_git_dirty() {
  if [[ -n "${KIVA_PDS_GIT_DIRTY:-}" ]]; then
    normalize_dirty_flag "$KIVA_PDS_GIT_DIRTY"
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
  echo "refusing PDS evidence generation from a dirty git tree" >&2
  git status --short >&2 || true
  exit 2
fi

REPORT_DIR="$OUT_DIR/reports"
LOG_DIR="$OUT_DIR/logs"
EXAMPLES_DIR="$OUT_DIR/examples"
SCHEMAS_DIR="$OUT_DIR/schemas"
DOCS_DIR="$OUT_DIR/docs"
METADATA_DIR="$OUT_DIR/metadata"
rm -rf "$REPORT_DIR" "$LOG_DIR" "$EXAMPLES_DIR" "$SCHEMAS_DIR" "$DOCS_DIR" "$METADATA_DIR" "$OUT_DIR/README.md"
mkdir -p "$REPORT_DIR" "$LOG_DIR" "$EXAMPLES_DIR" "$SCHEMAS_DIR" "$DOCS_DIR" "$METADATA_DIR"

run_logged() {
  local label="$1"
  shift
  "$@" >"$LOG_DIR/${label}.log" 2>&1
}

write_simple_report() {
  local path="$1"
  local kind="$2"
  local check_id="$3"
  python3 - "$path" "$kind" "$check_id" <<'PY'
from __future__ import annotations

import json
import sys
from datetime import UTC, datetime
from pathlib import Path

path = Path(sys.argv[1])
kind = sys.argv[2]
check_id = sys.argv[3]
path.write_text(
    json.dumps(
        {
            "schema_version": 1,
            "standard": "Kiva Architecture",
            "kind": kind,
            "status": "passed",
            "generated_at_utc": datetime.now(UTC).replace(microsecond=0).isoformat(),
            "checks": [
                {
                    "id": check_id,
                    "status": "passed",
                }
            ],
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
PY
}

run_logged tool-versions bash -c '
  git --version
  cargo --version
  rustc --version
  python3 --version
  if command -v cargo-kani >/dev/null 2>&1; then cargo kani --version; fi
  if command -v tamarin-prover >/dev/null 2>&1; then tamarin-prover --version; fi
  if command -v psql >/dev/null 2>&1; then psql --version; fi
'

run_logged pds-openapi-validation \
  python3 scripts/ci/validate_pds_openapi.py spec/openapi/kiva-pds.yaml
write_simple_report "$REPORT_DIR/pds-openapi-validation.json" \
  pds-openapi-validation validate_pds_openapi

run_logged pds-publication-validation \
  python3 scripts/ci/validate_pds_publication.py \
    --json-report "$REPORT_DIR/pds-publication-validation.json"

run_logged pds-deployment-template-validation \
  python3 scripts/ci/validate_pds_deployment_templates.py \
    --json-report "$REPORT_DIR/pds-deployment-template-validation.json"

python3 scripts/ci/export_pds_openapi_schemas.py --out "$SCHEMAS_DIR" \
  >"$LOG_DIR/pds-schema-export.log" 2>&1
cp "$SCHEMAS_DIR/index.json" "$REPORT_DIR/pds-schema-export.json"

cp examples/pds/* "$EXAMPLES_DIR"/
cp docs/architecture/pds-extension-compatibility.md "$DOCS_DIR"/
cp docs/architecture/pds-milestones.md "$DOCS_DIR"/
cp docs/architecture/pds-public-sector-deployment-templates.md "$DOCS_DIR"/
cp spec/pds/publication-milestones.yaml "$METADATA_DIR"/
cp spec/pds/deployment-templates.yaml "$METADATA_DIR"/
cp spec/pds/schemas/pds-deployment-template-catalog.schema.json "$METADATA_DIR"/

cargo test -q -p kiva-policy pds_ >"$LOG_DIR/pds-policy-tests.log" 2>&1
write_simple_report "$REPORT_DIR/pds-policy-tests.json" pds-policy-tests "cargo-test-kiva-policy-pds"

cargo test -q -p kiva-server pds_ >"$LOG_DIR/pds-server-tests.log" 2>&1
write_simple_report "$REPORT_DIR/pds-server-tests.json" pds-server-tests "cargo-test-kiva-server-pds"

if [[ "$RUN_POSTGRES_VERIFICATION" == 1 ]]; then
  KIVA_POSTGRES_VERIFICATION_JSON_REPORT="$REPORT_DIR/postgres-verification.json" \
    bash scripts/ci/run_postgres_verification.sh >"$LOG_DIR/postgres-verification.log" 2>&1
else
  printf 'skipped\n' >"$LOG_DIR/postgres-verification.log"
fi

if [[ "$RUN_KANI" == 1 ]]; then
  KIVA_KANI_SUITE="$KANI_SUITE" \
    KIVA_KANI_JSON_REPORT="$REPORT_DIR/kani.json" \
    bash scripts/ci/run_kani_verification.sh >"$LOG_DIR/kani.log" 2>&1
else
  printf 'skipped\n' >"$LOG_DIR/kani.log"
fi

if [[ "$RUN_TAMARIN" == 1 ]]; then
  LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    KIVA_TAMARIN_JSON_REPORT="$REPORT_DIR/tamarin.json" \
    bash scripts/ci/run_tamarin_verification.sh >"$LOG_DIR/tamarin.log" 2>&1
else
  printf 'skipped\n' >"$LOG_DIR/tamarin.log"
fi

python3 - \
  "$OUT_DIR" \
  "$RUN_POSTGRES_VERIFICATION" \
  "$RUN_KANI" \
  "$RUN_TAMARIN" \
  "$REQUIRE_CLEAN_GIT" \
  "$GIT_COMMIT" \
  "$GIT_DIRTY" <<'PY' \
  >"$METADATA_DIR/manifest.json"
from __future__ import annotations

import json
import sys
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

out_dir = Path(sys.argv[1])
run_postgres = sys.argv[2] == "1"
run_kani = sys.argv[3] == "1"
run_tamarin = sys.argv[4] == "1"
require_clean_git = sys.argv[5] == "1"
git_commit = sys.argv[6]
git_dirty = sys.argv[7] == "true"


def load_json(relative: str) -> dict[str, Any]:
    with (out_dir / relative).open("r", encoding="utf-8") as handle:
        data = json.load(handle)
    if not isinstance(data, dict):
        raise TypeError(f"{relative} root must be a JSON object")
    return data


def skipped() -> dict[str, Any]:
    return {
        "enabled": False,
        "status": "skipped",
        "report_path": None,
        "log_path": None,
        "checks": 0,
        "passed": 0,
        "failed": 0,
        "timeout": 0,
    }


def summarize(enabled: bool, report_relative: str, log_relative: str) -> dict[str, Any]:
    if not enabled:
        return skipped()
    report = load_json(report_relative)
    checks = report.get("checks", [])
    if not isinstance(checks, list):
        raise TypeError(f"{report_relative}.checks must be an array")
    passed = sum(1 for check in checks if check.get("status") == "passed")
    failed = sum(1 for check in checks if check.get("status") == "failed")
    timeout = sum(1 for check in checks if check.get("status") == "timeout")
    return {
        "enabled": True,
        "status": "failed" if failed or timeout else report.get("status", "passed"),
        "report_path": report_relative,
        "log_path": log_relative,
        "checks": len(checks),
        "passed": passed,
        "failed": failed,
        "timeout": timeout,
    }


tests = {
    "policy": summarize(True, "reports/pds-policy-tests.json", "logs/pds-policy-tests.log"),
    "server": summarize(True, "reports/pds-server-tests.json", "logs/pds-server-tests.log"),
    "postgres": summarize(
        run_postgres,
        "reports/postgres-verification.json",
        "logs/postgres-verification.log",
    ),
}
verification = {
    "kani": summarize(run_kani, "reports/kani.json", "logs/kani.log"),
    "tamarin": summarize(run_tamarin, "reports/tamarin.json", "logs/tamarin.log"),
}
all_checks = [*tests.values(), *verification.values()]
totals = {
    "enabled_checks": sum(1 for check in all_checks if check["enabled"]),
    "skipped_checks": sum(1 for check in all_checks if not check["enabled"]),
    "checks": sum(check["checks"] for check in all_checks),
    "passed": sum(check["passed"] for check in all_checks),
    "failed": sum(check["failed"] for check in all_checks),
    "timeout": sum(check["timeout"] for check in all_checks),
}


def artifact_list(group: str) -> list[str]:
    return sorted(
        path.relative_to(out_dir).as_posix()
        for path in (out_dir / group).glob("*")
        if path.is_file()
    )


manifest = {
    "schema_version": 1,
    "standard": "Kiva Architecture",
    "kind": "pds-publication-evidence",
    "status": "failed" if totals["failed"] or totals["timeout"] else "passed",
    "generated_at_utc": datetime.now(UTC).replace(microsecond=0).isoformat(),
    "git": {
        "commit": git_commit,
        "dirty": git_dirty,
    },
    "release_requirements": {
        "require_clean_git": require_clean_git,
        "met_clean_git": not git_dirty,
    },
    "contract": {
        "openapi_validated": True,
        "examples_validated": True,
        "schemas_exported": True,
        "deployment_templates_validated": True,
        "compatibility_note": "docs/pds-extension-compatibility.md",
    },
    "tests": tests,
    "verification": verification,
    "totals": totals,
    "artifacts": {
        "reports": artifact_list("reports"),
        "logs": sorted({*artifact_list("logs"), "logs/pds-evidence-manifest-validation.log"}),
        "examples": artifact_list("examples"),
        "schemas": artifact_list("schemas"),
        "docs": artifact_list("docs"),
        "metadata": sorted(
            {
                *artifact_list("metadata"),
                "metadata/checksums.sha256",
                "metadata/manifest.json",
            }
        ),
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
with (out_dir / "metadata/manifest.json").open("r", encoding="utf-8") as handle:
    manifest = json.load(handle)

print("# Kiva PDS Publication Evidence")
print()
print(f"- Standard: `{manifest['standard']}`")
print("- Boundary: Kiva PDS platform extension, not IEEE xAPI LRS conformance.")
print(f"- Generated at UTC: `{manifest['generated_at_utc']}`")
print(f"- Git commit: `{manifest['git']['commit']}`")
print(f"- Git dirty: `{str(manifest['git']['dirty']).lower()}`")
print(
    "- Totals: "
    f"`{manifest['status']}`, "
    f"{manifest['totals']['passed']}/{manifest['totals']['checks']} passed, "
    f"{manifest['totals']['failed']} failed, "
    f"{manifest['totals']['timeout']} timeout"
)
for group_name in ["tests", "verification"]:
    for name, evidence in manifest[group_name].items():
        print(
            f"- {group_name}.{name}: `{evidence['status']}`, "
            f"{evidence['passed']}/{evidence['checks']} passed"
        )
print("- Compatibility note: `docs/pds-extension-compatibility.md`")
print("- Deployment templates: `metadata/deployment-templates.yaml`")
print("- Checksums: `metadata/checksums.sha256`")
print()
print(
    "This bundle supports Kiva PDS extension publication readiness. It does "
    "not expand the IEEE/ISO/IEC 39274-1-1:2025 xAPI LRS conformance claim."
)
PY

run_logged pds-evidence-manifest-validation \
  python3 scripts/ci/validate_pds_evidence_bundle.py "$OUT_DIR" --skip-checksums

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

python3 scripts/ci/validate_pds_evidence_bundle.py "$OUT_DIR"

printf 'generated PDS evidence bundle: %s\n' "$OUT_DIR"
