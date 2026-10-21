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
kiva_apply_postgres_migrations "$ROOT_DIR" "$PGHOST_ADDR" "$PGPORT_NUM" "$PGUSER_NAME" "$PGDATABASE_NAME"

export KIVA_POSTGRES_TEST_DATABASE_URL="postgres://${PGUSER_NAME}@${PGHOST_ADDR}:${PGPORT_NUM}/${PGDATABASE_NAME}"

cargo test -q -p kiva-server postgres_confidential_ \
  -- --ignored --test-threads=1
cargo test -q -p kiva-server postgres_pds_ \
  -- --ignored --test-threads=1
cargo test -q -p kiva-server postgres_analytics_ \
  -- --ignored --test-threads=1
cargo test -q -p kiva-server postgres_xapi_ \
  -- --ignored --test-threads=1

if [[ -n "$REPORT_FILE" ]]; then
  mkdir -p "$(dirname "$REPORT_FILE")"
  python3 - "$REPORT_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from datetime import UTC, datetime
from pathlib import Path

report_path = Path(sys.argv[1])
report = {
    "schema_version": 1,
    "standard": "Kiva Architecture",
    "kind": "postgres-verification",
    "status": "passed",
    "backend": "postgres",
    "generated_at_utc": datetime.now(UTC).replace(microsecond=0).isoformat(),
    "temporary_cluster": True,
    "migrations_applied": True,
    "checks": [
        {
            "id": "postgres_xapi_substatement_retries_preserve_stored_timestamps",
            "crate": "kiva-server",
            "status": "passed",
        },
        {
            "id": "postgres_xapi_continuations_survive_collection_changes",
            "crate": "kiva-server",
            "status": "passed",
        },
        {
            "id": "postgres_confidential_threshold_release_is_idempotent_under_retries",
            "crate": "kiva-server",
            "status": "passed",
        },
        {
            "id": "postgres_confidential_differential_privacy_budget_spend_is_serialized",
            "crate": "kiva-server",
            "status": "passed",
        },
        {
            "id": "postgres_confidential_differential_privacy_release_is_idempotent_under_retries",
            "crate": "kiva-server",
            "status": "passed",
        },
        {
            "id": "postgres_confidential_differential_privacy_sampled_release_is_idempotent_under_retries",
            "crate": "kiva-server",
            "status": "passed",
        },
        {
            "id": "postgres_confidential_differential_privacy_composition_summary_tracks_release_and_manual_spend",
            "crate": "kiva-server",
            "status": "passed",
        },
        {
            "id": "postgres_confidential_job_lifecycle_is_idempotent_and_audited",
            "crate": "kiva-server",
            "status": "passed",
        },
        {
            "id": "postgres_confidential_participant_lifecycle_is_idempotent_and_audited",
            "crate": "kiva-server",
            "status": "passed",
        },
        {
            "id": "postgres_pds_consent_revocation_and_access_audit_are_persistent",
            "crate": "kiva-server",
            "status": "passed",
        },
        {
            "id": "postgres_pds_data_products_require_available_release_and_owner_scope",
            "crate": "kiva-server",
            "status": "passed",
        },
        {
            "id": "postgres_pds_governance_profiles_are_strict_and_persistent",
            "crate": "kiva-server",
            "status": "passed",
        },
        {
            "id": "postgres_pds_learner_portability_export_is_subject_bound_paged_and_audited",
            "crate": "kiva-server",
            "status": "passed",
        }
    ],
}
report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
fi
