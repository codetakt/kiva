#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/scripts/ci/conformance_lib.sh"

KIVA_HOST_ADDR="${KIVA_MEMORY_CONFORMANCE_KIVA_HOST:-127.0.0.1}"

SERVER_PID=""

cleanup() {
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

kiva_require_command curl
kiva_require_command cargo
kiva_require_command python3

KIVA_PORT_NUM="${KIVA_MEMORY_CONFORMANCE_KIVA_PORT:-$(kiva_free_tcp_port "$KIVA_HOST_ADDR")}"
KIVA_ADDR="${KIVA_MEMORY_CONFORMANCE_ADDR:-${KIVA_HOST_ADDR}:${KIVA_PORT_NUM}}"

cd "$ROOT_DIR"

unset DATABASE_URL
unset KIVA_POSTGRES_TEST_DATABASE_URL
export KIVA_NATIVE_BACKEND=memory
export KIVA_LISTEN_ADDR="$KIVA_ADDR"
kiva_export_conformance_fixture_env
kiva_build_conformance_binaries

"${CARGO_TARGET_DIR:-target}/release/kiva-server" &
SERVER_PID="$!"

kiva_wait_for_server "$KIVA_ADDR"
kiva_run_xapi_conformance "$KIVA_ADDR"
