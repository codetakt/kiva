#!/usr/bin/env bash
set -euo pipefail

if ! command -v tamarin-prover >/dev/null 2>&1; then
  cat >&2 <<'MSG'
tamarin-prover is not available on PATH.

Run this script from the verification shell:

  nix develop .#verification -c scripts/ci/run_tamarin_verification.sh
MSG
  exit 127
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is not available on PATH" >&2
  exit 127
fi

REPORT_FILE="${KIVA_TAMARIN_JSON_REPORT:-}"

mapfile -t models < <(find proofs/tamarin -type f -name '*.spthy' | sort)

if [[ "${#models[@]}" -eq 0 ]]; then
  echo "No Tamarin models found under proofs/tamarin"
  exit 0
fi

TAMARIN_VERSION="$(tamarin-prover --version | head -n1)"
echo "$TAMARIN_VERSION"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/kiva-tamarin.XXXXXX")"
RESULTS_FILE="$TMP_DIR/results.tsv"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
touch "$RESULTS_FILE"

write_report() {
  local status="$1"
  local tool_version="$2"
  if [[ -z "$REPORT_FILE" ]]; then
    return 0
  fi
  mkdir -p "$(dirname "$REPORT_FILE")"
  python3 - "$REPORT_FILE" "$status" "$RESULTS_FILE" "$tool_version" <<'PY'
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
        model, lemma, lemma_status = row
        checks.append(
            {
                "id": lemma,
                "target": model,
                "status": lemma_status,
            }
        )

report = {
    "schema_version": 1,
    "standard": "Kiva Architecture",
    "kind": "tamarin-verification",
    "status": status,
    "generated_at_utc": datetime.now(UTC).replace(microsecond=0).isoformat(),
    "tool_version": tool_version,
    "checks": checks,
}
report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
}

for model in "${models[@]}"; do
  mapfile -t lemmas < <(sed -n 's/^lemma \([^:]*\):.*/\1/p' "$model")
  if [[ "${#lemmas[@]}" -eq 0 ]]; then
    tamarin-prover --prove "$model"
    continue
  fi
  for lemma in "${lemmas[@]}"; do
    log_file="$TMP_DIR/$(basename "$model").$lemma.log"
    if ! tamarin-prover --prove="$lemma" "$model" >"$log_file" 2>&1; then
      printf '%s\t%s\tfailed\n' "$model" "$lemma" >>"$RESULTS_FILE"
      write_report failed "$TAMARIN_VERSION"
      cat "$log_file" >&2
      exit 1
    fi
    if ! grep -Eq "^  ${lemma} \\([^)]*\\): verified " "$log_file"; then
      printf '%s\t%s\tfailed\n' "$model" "$lemma" >>"$RESULTS_FILE"
      write_report failed "$TAMARIN_VERSION"
      cat "$log_file" >&2
      exit 1
    fi
    printf '%s\t%s\tpassed\n' "$model" "$lemma" >>"$RESULTS_FILE"
    echo "[TAMARIN] verified ${model} :: ${lemma}"
  done
done

write_report passed "$TAMARIN_VERSION"
