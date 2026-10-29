#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/scripts/ci/conformance_lib.sh"

PGDATA_DIR="${KIVA_POSTGRES_VERIFICATION_PGDATA:-$(mktemp -d "${TMPDIR:-/tmp}/kiva-postgres-verification.XXXXXX")}"
PGHOST_ADDR="${KIVA_POSTGRES_VERIFICATION_HOST:-127.0.0.1}"
PGDATABASE_NAME="${KIVA_POSTGRES_VERIFICATION_DB:-kiva}"
PGUSER_NAME="${KIVA_POSTGRES_VERIFICATION_USER:-$(id -un)}"
REPORT_FILE="${KIVA_POSTGRES_VERIFICATION_JSON_REPORT:-}"

STARTED_POSTGRES=0

cleanup() {
  if [[ "$STARTED_POSTGRES" == 1 ]]; then
    pg_ctl -D "$PGDATA_DIR" -m fast stop >/dev/null 2>&1 || true
  fi
  if [[ -z "${KIVA_POSTGRES_VERIFICATION_PGDATA:-}" ]]; then
    rm -rf "$PGDATA_DIR"
  fi
}
trap cleanup EXIT

kiva_require_command initdb
kiva_require_command pg_ctl
kiva_require_command createdb
kiva_require_command psql
kiva_require_command cargo
kiva_require_command python3

PGPORT_NUM="${KIVA_POSTGRES_VERIFICATION_PORT:-$(kiva_free_tcp_port "$PGHOST_ADDR")}"

cd "$ROOT_DIR"

initdb -D "$PGDATA_DIR" -A trust --no-locale --encoding=UTF8 --username "$PGUSER_NAME" >/dev/null
pg_ctl -D "$PGDATA_DIR" -o "-k /tmp -p $PGPORT_NUM -h $PGHOST_ADDR" -l "$PGDATA_DIR/postgres.log" start >/dev/null
STARTED_POSTGRES=1

createdb -h "$PGHOST_ADDR" -p "$PGPORT_NUM" -U "$PGUSER_NAME" "$PGDATABASE_NAME"
XAPI_MIGRATION_CONTROL_RAN=0
ATTACHMENT_MIGRATION_CONTROL_RAN=0
for migration in "$ROOT_DIR"/db/migrations/[0-9]*.sql; do
  if [[ "$migration" == */20261027000000_xapi_document_unicode.sql ]]; then
    # One connection retains the temporary expectations across the upgrade.
    # Include old JSON containing NUL outside the IFI and literal escape names.
    psql -h "$PGHOST_ADDR" -p "$PGPORT_NUM" -U "$PGUSER_NAME" -d "$PGDATABASE_NAME" -v ON_ERROR_STOP=1 \
      -f "$ROOT_DIR/tests/postgres/xapi_unicode_migration_before.sql" \
      -f "$migration" \
      -f "$ROOT_DIR/tests/postgres/xapi_unicode_migration_after.sql" >/dev/null
    XAPI_MIGRATION_CONTROL_RAN=1
  elif [[ "$migration" == */20261029000000_xapi_attachment_media_bytes.sql ]]; then
    psql -h "$PGHOST_ADDR" -p "$PGPORT_NUM" -U "$PGUSER_NAME" -d "$PGDATABASE_NAME" -v ON_ERROR_STOP=1 \
      -f "$ROOT_DIR/tests/postgres/xapi_attachment_media_migration_before.sql" \
      -f "$migration" \
      -f "$ROOT_DIR/tests/postgres/xapi_attachment_media_migration_after.sql" >/dev/null
    ATTACHMENT_MIGRATION_CONTROL_RAN=1
  else
    psql -h "$PGHOST_ADDR" -p "$PGPORT_NUM" -U "$PGUSER_NAME" -d "$PGDATABASE_NAME" -v ON_ERROR_STOP=1 \
      -f "$migration" >/dev/null
  fi
done
[[ "$XAPI_MIGRATION_CONTROL_RAN" == 1 ]] || { echo "missing xAPI migration control" >&2; exit 1; }
[[ "$ATTACHMENT_MIGRATION_CONTROL_RAN" == 1 ]] || { echo "missing attachment migration control" >&2; exit 1; }

export KIVA_POSTGRES_TEST_DATABASE_URL="postgres://${PGUSER_NAME}@${PGHOST_ADDR}:${PGPORT_NUM}/${PGDATABASE_NAME}"

# Discover and run the same complete ignored PostgreSQL test selection. Keep
# the receipt inventory synchronized when a new storage regression is added.
cargo test -q -p kiva-server --lib postgres_ -- --ignored --list > "$PGDATA_DIR/test-inventory.txt"
cargo test -q -p kiva-server --lib postgres_ -- --ignored --test-threads=1

if [[ -n "$REPORT_FILE" ]]; then
  mkdir -p "$(dirname "$REPORT_FILE")"
  python3 - "$REPORT_FILE" "$PGDATA_DIR/test-inventory.txt" <<'PY'
from __future__ import annotations

import json
import sys
from datetime import UTC, datetime
from pathlib import Path

report_path = Path(sys.argv[1])
checks = [line.removesuffix(": test") for line in Path(sys.argv[2]).read_text().splitlines()
          if line.endswith(": test")]
if not checks or len(set(checks)) != len(checks):
    raise SystemExit("missing or duplicate PostgreSQL test inventory")
checks.append("postgres_xapi_unicode_existing_row_migration")
checks.append("postgres_xapi_attachment_media_existing_row_migration")
report = {
    "schema_version": 1,
    "standard": "Kiva Architecture",
    "kind": "postgres-verification",
    "status": "passed",
    "backend": "postgres",
    "generated_at_utc": datetime.now(UTC).replace(microsecond=0).isoformat(),
    "temporary_cluster": True,
    "migrations_applied": True,
    "checks": [{"id": name, "crate": "kiva-server", "status": "passed"} for name in checks],
}
report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
fi
