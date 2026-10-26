#!/usr/bin/env bash

kiva_require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 127
  fi
}

kiva_free_tcp_port() {
  python3 - "$1" <<'PY'
import socket
import sys

host = sys.argv[1]
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind((host, 0))
    print(sock.getsockname()[1])
PY
}

kiva_export_conformance_fixture_env() {
  export KIVA_MAX_STATEMENT_BODY_BYTES=4096
  export KIVA_RATE_LIMIT_PATH=/__kiva/conformance/rate-limit
  export KIVA_RATE_LIMIT_REQUEST_COUNT=3
  export KIVA_RATE_LIMIT_WINDOW_SECONDS=1
  export KIVA_CONFORMANCE_WRITER_A_TOKEN="${KIVA_CONFORMANCE_WRITER_A_TOKEN:-statement-writer-a}"
  export KIVA_CONFORMANCE_READER_TOKEN="${KIVA_CONFORMANCE_READER_TOKEN:-statement-reader}"
  export KIVA_CONFORMANCE_INVALID_TOKEN="${KIVA_CONFORMANCE_INVALID_TOKEN:-invalid-credential}"
}

kiva_build_conformance_binaries() {
  python3 -B scripts/ci/lrs_generated_artifacts.py generate
  cargo build -q -p kiva-server -p xapi-lrs-conformance
}

kiva_apply_postgres_migrations() {
  local root_dir="$1"
  local host="$2"
  local port="$3"
  local user="$4"
  local database="$5"

  for migration in "$root_dir"/db/migrations/[0-9]*.sql; do
    psql -h "$host" -p "$port" -U "$user" -d "$database" -v ON_ERROR_STOP=1 \
      -f "$migration" >/dev/null
  done
}

kiva_wait_for_server() {
  local addr="$1"
  local attempts="${KIVA_CONFORMANCE_READY_ATTEMPTS:-300}"

  for ((_attempt = 0; _attempt < attempts; _attempt++)); do
    if curl -fsS "http://${addr}/about" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.1
  done

  echo "kiva-server did not become ready on http://${addr}" >&2
  return 1
}

kiva_run_xapi_conformance() {
  local addr="$1"
  local suite_status=0
  local args=(
    run
    -q
    -p
    xapi-lrs-conformance
    --
    --endpoint
    "http://${addr}"
    --allow-destructive
    --endpoint-profile
    spec/conformance/endpoint-profile.example.yaml
    --auth-profile-file
    spec/conformance/auth-profile.example.yaml
  )

  if [[ -n "${KIVA_CONFORMANCE_JUNIT_REPORT:-}" ]]; then
    mkdir -p "$(dirname "$KIVA_CONFORMANCE_JUNIT_REPORT")"
    args+=(--junit "$KIVA_CONFORMANCE_JUNIT_REPORT")
  fi

  if [[ -n "${KIVA_CONFORMANCE_JSON_REPORT:-}" ]]; then
    mkdir -p "$(dirname "$KIVA_CONFORMANCE_JSON_REPORT")"
    if cargo "${args[@]}" --json >"$KIVA_CONFORMANCE_JSON_REPORT"; then
      suite_status=0
    else
      suite_status=$?
      # Failed Nix builds do not install the report; keep diagnostics in the log.
      echo "xAPI conformance exited with status $suite_status; report: $KIVA_CONFORMANCE_JSON_REPORT" >&2
      python3 - "$KIVA_CONFORMANCE_JSON_REPORT" <<'PY' >&2 || echo "unable to summarize conformance report" >&2
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    report = json.load(handle)

print(f"status: {report['status']}; totals: {json.dumps(report['totals'], sort_keys=True)}")
for suite in report["suites"]:
    for case in suite["cases"]:
        if case["outcome"] not in {"failed", "not-run"}:
            continue
        print(f"[{case['outcome']}] {suite['id']}/{case['id']}")
        for failure in case.get("failures", []):
            print(f"  - {failure}")
        for reason in case.get("not_run_reasons", []):
            print(f"  - {reason}")
PY
    fi
  else
    if cargo "${args[@]}"; then
      suite_status=0
    else
      suite_status=$?
    fi
  fi
  if [[ -n "${KIVA_CONFORMANCE_PROCEDURE_REPORT_ROOT:-}" ]]; then
    local procedure_status=0
    python3 scripts/ci/run_lrs_procedures.py --endpoint "http://${addr}" \
      --runner target/debug/xapi-lrs-conformance \
      --fixture "${KIVA_CONFORMANCE_PROCEDURE_FIXTURE:?procedure fixture must be declared}" \
      --out "$KIVA_CONFORMANCE_PROCEDURE_REPORT_ROOT" || procedure_status=$?
    if [[ "$suite_status" == 0 ]]; then
      suite_status="$procedure_status"
    fi
  fi
  return "$suite_status"
}
