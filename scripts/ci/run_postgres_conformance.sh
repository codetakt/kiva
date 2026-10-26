#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/scripts/ci/conformance_lib.sh"

PGDATA_DIR="${KIVA_POSTGRES_CONFORMANCE_PGDATA:-$(mktemp -d "${TMPDIR:-/tmp}/kiva-postgres-conformance.XXXXXX")}"
PGHOST_ADDR="${KIVA_POSTGRES_CONFORMANCE_HOST:-127.0.0.1}"
PGDATABASE_NAME="${KIVA_POSTGRES_CONFORMANCE_DB:-kiva}"
PGUSER_NAME="${KIVA_POSTGRES_CONFORMANCE_USER:-$(id -un)}"
KIVA_HOST_ADDR="${KIVA_POSTGRES_CONFORMANCE_KIVA_HOST:-127.0.0.1}"
KIVA_TENANT="${KIVA_POSTGRES_CONFORMANCE_TENANT:-pg-conformance}"

SERVER_PID=""
STARTED_POSTGRES=0

cleanup() {
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  if [[ "$STARTED_POSTGRES" == 1 ]]; then
    pg_ctl -D "$PGDATA_DIR" -m fast stop >/dev/null 2>&1 || true
  fi
  if [[ -z "${KIVA_POSTGRES_CONFORMANCE_PGDATA:-}" ]]; then
    rm -rf "$PGDATA_DIR"
  fi
}
trap cleanup EXIT

kiva_require_command initdb
kiva_require_command pg_ctl
kiva_require_command createdb
kiva_require_command psql
kiva_require_command curl
kiva_require_command cargo
kiva_require_command python3

PGPORT_NUM="${KIVA_POSTGRES_CONFORMANCE_PORT:-$(kiva_free_tcp_port "$PGHOST_ADDR")}"
KIVA_PORT_NUM="${KIVA_POSTGRES_CONFORMANCE_KIVA_PORT:-$(kiva_free_tcp_port "$KIVA_HOST_ADDR")}"
KIVA_ADDR="${KIVA_POSTGRES_CONFORMANCE_ADDR:-${KIVA_HOST_ADDR}:${KIVA_PORT_NUM}}"

cd "$ROOT_DIR"

initdb -D "$PGDATA_DIR" -A trust --no-locale --encoding=UTF8 --username "$PGUSER_NAME" >/dev/null
pg_ctl -D "$PGDATA_DIR" -o "-k /tmp -p $PGPORT_NUM -h $PGHOST_ADDR" -l "$PGDATA_DIR/postgres.log" start >/dev/null
STARTED_POSTGRES=1

createdb -h "$PGHOST_ADDR" -p "$PGPORT_NUM" -U "$PGUSER_NAME" "$PGDATABASE_NAME"
kiva_apply_postgres_migrations "$ROOT_DIR" "$PGHOST_ADDR" "$PGPORT_NUM" "$PGUSER_NAME" "$PGDATABASE_NAME"

export DATABASE_URL="postgres://${PGUSER_NAME}@${PGHOST_ADDR}:${PGPORT_NUM}/${PGDATABASE_NAME}"
export KIVA_TENANT_ID="$KIVA_TENANT"
export KIVA_LISTEN_ADDR="$KIVA_ADDR"
kiva_export_conformance_fixture_env
kiva_build_conformance_binaries

cargo run --release -q -p kiva-server &
SERVER_PID="$!"

kiva_wait_for_server "$KIVA_ADDR"
kiva_run_xapi_conformance "$KIVA_ADDR"
