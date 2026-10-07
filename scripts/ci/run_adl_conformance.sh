#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/scripts/ci/conformance_lib.sh"

if [[ -z "${KIVA_ADL_SUITE_ROOT:-}" ]]; then
  echo "KIVA_ADL_SUITE_ROOT must name a local ADL suite checkout" >&2
  exit 2
fi

ADL_ROOT="$(cd "$KIVA_ADL_SUITE_ROOT" && pwd)"
if [[ ! -f "$ADL_ROOT/bin/console_runner.js" || ! -f "$ADL_ROOT/package.json" ]]; then
  echo "invalid ADL suite checkout: $ADL_ROOT" >&2
  exit 2
fi
if [[ ! -d "$ADL_ROOT/node_modules" ]]; then
  echo "ADL suite dependencies are absent; run npm install in $ADL_ROOT" >&2
  exit 2
fi

kiva_require_command cargo
kiva_require_command curl
kiva_require_command git
kiva_require_command node
kiva_require_command python3

HOST="${KIVA_ADL_RUN_KIVA_HOST:-127.0.0.1}"
PORT="${KIVA_ADL_RUN_KIVA_PORT:-$(kiva_free_tcp_port "$HOST")}"
ADDR="${KIVA_ADL_RUN_KIVA_ADDR:-${HOST}:${PORT}}"
REPORT="${KIVA_ADL_RUN_REPORT:-$ROOT_DIR/target/adl-run/adl-run.json}"
RAW_LOG="${KIVA_ADL_RUN_RAW_LOG:-$ROOT_DIR/target/adl-run/adl-raw.json}"
CONSOLE_LOG="${KIVA_ADL_RUN_CONSOLE_LOG:-$ROOT_DIR/target/adl-run/adl-console.log}"
SERVER_LOG="${KIVA_ADL_RUN_SERVER_LOG:-$ROOT_DIR/target/adl-run/kiva-server.log}"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/kiva-adl-run.XXXXXX")"
MARKER="$TMP_DIR/run-started"
SERVER_PID=""

cleanup() {
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

mkdir -p "$(dirname "$REPORT")" "$(dirname "$RAW_LOG")" \
  "$(dirname "$CONSOLE_LOG")" "$(dirname "$SERVER_LOG")"
touch "$MARKER"

cd "$ROOT_DIR"
unset DATABASE_URL
unset KIVA_REQUIRE_AUTHORIZATION
export KIVA_LISTEN_ADDR="$ADDR"
cargo build -q -p kiva-server
cargo run -q -p kiva-server >"$SERVER_LOG" 2>&1 &
SERVER_PID="$!"
kiva_wait_for_server "$ADDR"

set +e
(
  cd "$ADL_ROOT"
  node bin/console_runner.js --endpoint "http://${ADDR}" --xapiVersion 2.0.0
) >"$CONSOLE_LOG" 2>&1
ADL_STATUS="$?"
set -e

mapfile -t NEW_LOGS < <(find "$ADL_ROOT/logs" -maxdepth 1 -type f -newer "$MARKER" -print | sort)
if [[ "${#NEW_LOGS[@]}" -ne 1 ]]; then
  echo "expected exactly one new ADL JSON log, found ${#NEW_LOGS[@]}" >&2
  printf '%s\n' "${NEW_LOGS[@]}" >&2
  exit 1
fi
cp "${NEW_LOGS[0]}" "$RAW_LOG"

python3 scripts/ci/report_adl_run.py \
  --raw-report "$RAW_LOG" \
  --output "$REPORT" \
  --adl-root "$ADL_ROOT" \
  --kiva-root "$ROOT_DIR"

printf 'ADL runner exit status (evaluated through expectations): %s\n' "$ADL_STATUS"
printf 'normalized ADL report: %s\n' "$REPORT"
printf 'ADL raw log: %s\n' "$RAW_LOG"
printf 'ADL console log: %s\n' "$CONSOLE_LOG"
