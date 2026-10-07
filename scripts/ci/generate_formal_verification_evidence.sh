#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/scripts/ci/conformance_lib.sh"

OUT_DIR="${KIVA_FORMAL_VERIFICATION_EVIDENCE_DIR:-target/formal-verification-evidence/latest}"
RUN_FSTAR=1
RUN_KANI=1
RUN_TAMARIN=1
KANI_SUITE="${KIVA_KANI_SUITE:-regression}"
REQUIRE_CLEAN_GIT=0

usage() {
  cat <<'EOF'
usage: scripts/ci/generate_formal_verification_evidence.sh [--out DIR] [--skip-fstar] [--skip-kani] [--skip-tamarin] [--kani-suite smoke|regression] [--require-clean-git]

Generates a fixed formal verification evidence bundle under DIR.
The default DIR is target/formal-verification-evidence/latest.
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
    --skip-fstar)
      RUN_FSTAR=0
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
kiva_require_command python3
kiva_require_command sha256sum

normalize_dirty_flag() {
  case "$1" in
    0 | false | clean)
      printf 'false\n'
      ;;
    1 | true | dirty)
      printf 'true\n'
      ;;
    *)
      echo "invalid KIVA_FORMAL_VERIFICATION_GIT_DIRTY=$1; expected 0/1/true/false/clean/dirty" >&2
      exit 2
      ;;
  esac
}

detect_git_commit() {
  if [[ -n "${KIVA_FORMAL_VERIFICATION_GIT_COMMIT:-}" ]]; then
    printf '%s\n' "$KIVA_FORMAL_VERIFICATION_GIT_COMMIT"
  elif git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git rev-parse HEAD
  else
    printf 'unknown\n'
  fi
}

detect_git_dirty() {
  if [[ -n "${KIVA_FORMAL_VERIFICATION_GIT_DIRTY:-}" ]]; then
    normalize_dirty_flag "$KIVA_FORMAL_VERIFICATION_GIT_DIRTY"
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
  echo "refusing formal verification evidence generation from a dirty git tree" >&2
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git status --short >&2
  else
    echo "no Git work tree is available; set KIVA_FORMAL_VERIFICATION_GIT_DIRTY=0 only for fixed source inputs" >&2
  fi
  exit 2
fi

REPORT_DIR="$OUT_DIR/reports"
LOG_DIR="$OUT_DIR/logs"
METADATA_DIR="$OUT_DIR/metadata"
rm -rf "$REPORT_DIR" "$LOG_DIR" "$METADATA_DIR" "$OUT_DIR/README.md"
mkdir -p "$REPORT_DIR" "$LOG_DIR" "$METADATA_DIR"

run_logged() {
  local label="$1"
  shift
  "$@" >"$LOG_DIR/${label}.log" 2>&1
}

run_logged tool-versions bash -c '
  git --version
  python3 --version
  if command -v cargo >/dev/null 2>&1; then cargo --version; fi
  if command -v rustc >/dev/null 2>&1; then rustc --version; fi
  if command -v cargo-kani >/dev/null 2>&1; then cargo kani --version; fi
  if command -v tamarin-prover >/dev/null 2>&1; then tamarin-prover --version; fi
  if command -v fstar.exe >/dev/null 2>&1; then fstar.exe --version; fi
'

if [[ "$RUN_FSTAR" == 1 ]]; then
  KIVA_FSTAR_JSON_REPORT="$REPORT_DIR/fstar.json" \
    bash scripts/ci/run_fstar_verification.sh >"$LOG_DIR/fstar.log" 2>&1
  run_logged fstar-report-validation \
    python3 scripts/ci/validate_formal_verification_reports.py "$REPORT_DIR/fstar.json"
fi

if [[ "$RUN_KANI" == 1 ]]; then
  KIVA_KANI_SUITE="$KANI_SUITE" \
    KIVA_KANI_JSON_REPORT="$REPORT_DIR/kani.json" \
    bash scripts/ci/run_kani_verification.sh >"$LOG_DIR/kani.log" 2>&1
  run_logged kani-report-validation \
    python3 scripts/ci/validate_formal_verification_reports.py "$REPORT_DIR/kani.json"
fi

if [[ "$RUN_TAMARIN" == 1 ]]; then
  LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    KIVA_TAMARIN_JSON_REPORT="$REPORT_DIR/tamarin.json" \
    bash scripts/ci/run_tamarin_verification.sh >"$LOG_DIR/tamarin.log" 2>&1
  run_logged tamarin-report-validation \
    python3 scripts/ci/validate_formal_verification_reports.py "$REPORT_DIR/tamarin.json"
fi

python3 - \
  "$OUT_DIR" \
  "$RUN_FSTAR" \
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
run_fstar = sys.argv[2] == "1"
run_kani = sys.argv[3] == "1"
run_tamarin = sys.argv[4] == "1"
require_clean_git = sys.argv[5] == "1"
git_commit = sys.argv[6]
git_dirty = sys.argv[7] == "true"


def load_json(relative: str) -> dict[str, Any] | None:
    path = out_dir / relative
    if not path.is_file():
        return None
    with path.open("r", encoding="utf-8") as handle:
        data = json.load(handle)
    if not isinstance(data, dict):
        raise TypeError(f"{relative} root must be a JSON object")
    return data


def summarize(enabled: bool, report_relative: str, log_relative: str) -> dict[str, Any]:
    if not enabled:
        return {
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

    report = load_json(report_relative)
    if report is None:
        raise FileNotFoundError(report_relative)
    checks = report["checks"]
    passed = sum(1 for check in checks if check["status"] == "passed")
    failed = sum(1 for check in checks if check["status"] == "failed")
    timeout = sum(1 for check in checks if check["status"] == "timeout")
    status = report["status"]
    if failed or timeout:
        status = "failed"
    return {
        "enabled": True,
        "status": status,
        "report_path": report_relative,
        "log_path": log_relative,
        "tool_version": report["tool_version"],
        "suite": report.get("suite"),
        "checks": len(checks),
        "passed": passed,
        "failed": failed,
        "timeout": timeout,
    }


verification = {
    "fstar": summarize(run_fstar, "reports/fstar.json", "logs/fstar.log"),
    "kani": summarize(run_kani, "reports/kani.json", "logs/kani.log"),
    "tamarin": summarize(run_tamarin, "reports/tamarin.json", "logs/tamarin.log"),
}
totals = {
    "enabled_tools": sum(1 for item in verification.values() if item["enabled"]),
    "skipped_tools": sum(1 for item in verification.values() if not item["enabled"]),
    "checks": sum(item["checks"] for item in verification.values()),
    "passed": sum(item["passed"] for item in verification.values()),
    "failed": sum(item["failed"] for item in verification.values()),
    "timeout": sum(item["timeout"] for item in verification.values()),
}
reports = sorted(str(path.relative_to(out_dir)) for path in (out_dir / "reports").glob("*"))
logs = sorted(
    {
        *(str(path.relative_to(out_dir)) for path in (out_dir / "logs").glob("*")),
        "logs/formal-evidence-manifest-validation.log",
    }
)
metadata = sorted(
    {
        *(str(path.relative_to(out_dir)) for path in (out_dir / "metadata").glob("*")),
        "metadata/checksums.sha256",
        "metadata/manifest.json",
    }
)

manifest = {
    "schema_version": 1,
    "standard": "Kiva Architecture",
    "kind": "formal-verification-evidence",
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
    "verification": verification,
    "totals": totals,
    "artifacts": {
        "reports": reports,
        "logs": logs,
        "metadata": metadata,
    },
}
print(json.dumps(manifest, indent=2, sort_keys=True))
PY

run_logged formal-evidence-manifest-validation \
  python3 scripts/ci/validate_formal_verification_reports.py \
    --manifest "$METADATA_DIR/manifest.json"

python3 - "$OUT_DIR" <<'PY' >"$OUT_DIR/README.md"
from __future__ import annotations

import json
import sys
from pathlib import Path

out_dir = Path(sys.argv[1])

with (out_dir / "metadata/manifest.json").open("r", encoding="utf-8") as handle:
    manifest = json.load(handle)

print("# Kiva Formal Verification Evidence")
print()
print(f"- Standard: `{manifest['standard']}`")
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
for name in ["fstar", "kani", "tamarin"]:
    tool = manifest["verification"][name]
    print(
        f"- {name}: `{tool['status']}`, "
        f"{tool['passed']}/{tool['checks']} passed"
    )
print("- Checksums: `metadata/checksums.sha256`")
print()
print(
    "This bundle is internal assurance evidence for selected Kani, Tamarin, "
    "and F* boundary properties. It is not an IEEE conformance certificate, "
    "a production confidential-computation security proof, or a complete "
    "formal verification record for Kiva."
)
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

python3 scripts/ci/validate_formal_verification_evidence_bundle.py "$OUT_DIR"

printf 'generated formal verification evidence bundle: %s\n' "$OUT_DIR"
