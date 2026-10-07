#!/usr/bin/env bash
set -euo pipefail

if ! command -v fstar.exe >/dev/null 2>&1; then
  cat >&2 <<'MSG'
fstar.exe is not available on PATH.

Run this script from the verification shell:

  nix develop .#verification -c scripts/ci/run_fstar_verification.sh
MSG
  exit 127
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is not available on PATH" >&2
  exit 127
fi

REPORT_FILE="${KIVA_FSTAR_JSON_REPORT:-}"

mapfile -t models < <(find fstar -type f -name '*.fst' | sort)

if [[ "${#models[@]}" -eq 0 ]]; then
  echo "No F* models found under fstar"
  exit 0
fi

FSTAR_VERSION="$(fstar.exe --version | head -n1)"
echo "$FSTAR_VERSION"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/kiva-fstar.XXXXXX")"
RESULTS_FILE="$TMP_DIR/results.tsv"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
touch "$RESULTS_FILE"

write_report() {
  local status="$1"
  if [[ -z "$REPORT_FILE" ]]; then
    return 0
  fi
  mkdir -p "$(dirname "$REPORT_FILE")"
  python3 - "$REPORT_FILE" "$status" "$RESULTS_FILE" "$FSTAR_VERSION" <<'PY'
from __future__ import annotations

import csv
import json
import sys
from datetime import UTC, datetime
from pathlib import Path

report_path = Path(sys.argv[1])
status = sys.argv[2]
results_path = Path(sys.argv[3])
tool_version = sys.argv[4]

checks = []
with results_path.open("r", encoding="utf-8", newline="") as handle:
    for row in csv.reader(handle, delimiter="\t"):
        if len(row) != 3:
            continue
        model, module, module_status = row
        checks.append(
            {
                "id": module,
                "target": model,
                "status": module_status,
            }
        )

report = {
    "schema_version": 1,
    "standard": "Kiva Architecture",
    "kind": "fstar-verification",
    "status": status,
    "generated_at_utc": datetime.now(UTC).replace(microsecond=0).isoformat(),
    "tool_version": tool_version,
    "checks": checks,
}
report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
}

for model in "${models[@]}"; do
  module="$(basename "$model" .fst)"
  if fstar.exe "${model}"; then
    printf '%s\t%s\tpassed\n' "$model" "$module" >>"$RESULTS_FILE"
  else
    printf '%s\t%s\tfailed\n' "$model" "$module" >>"$RESULTS_FILE"
    write_report failed
    exit 1
  fi
done

write_report passed
