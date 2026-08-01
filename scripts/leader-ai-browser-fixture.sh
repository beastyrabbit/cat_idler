#!/usr/bin/env bash
set -euo pipefail

# Launch/check the real Rust server and Trunk client for LAI browser acceptance.
# No fixture is generated here: GAME_DB_PATH must point at an existing signed,
# deterministic SQLite fixture and the browser may only use shipped controls.

ROOT=$(cd "$(dirname "$0")/.." && pwd)
API_NAME=${LAI_PLAYTEST_API_NAME:-leader-ai-api}
BROWSER_NAME=${LAI_PLAYTEST_BROWSER_NAME:-leader-ai-browser}
TRUNK_PROFILE=${LAI_PLAYTEST_TRUNK_PROFILE:-release}
DEFAULT_FIXTURE="$ROOT/docs/leader-ai-overhaul/fixtures/lai33a/manifest.json"
FIXTURE=${LAI_PLAYTEST_FIXTURE:-$DEFAULT_FIXTURE}
SEED=${LAI_PLAYTEST_SEED:-}
DB=${GAME_DB_PATH:-}

usage() {
  printf '%s\n' "Usage: $0 --check | --run" \
    "  --check  validate deterministic fixture and print Playwright environment" \
    "  --run    launch the real server and Trunk through named Portless routes"
}

check_inputs() {
  [[ "$TRUNK_PROFILE" == release || "$TRUNK_PROFILE" == debug ]] ||
    { echo "BLOCKED: LAI_PLAYTEST_TRUNK_PROFILE must be release or debug" >&2; return 2; }
  test -f "$FIXTURE" || { echo "BLOCKED: fixture does not exist: $FIXTURE" >&2; return 2; }
  command -v jq >/dev/null || { echo "BLOCKED: jq is required to validate the fixture manifest" >&2; return 2; }
  command -v sha256sum >/dev/null || { echo "BLOCKED: sha256sum is required to validate the SQLite fixture" >&2; return 2; }
  local manifest_db expected_before expected_after actual
  manifest_db=$(jq -er '.fixture.sqlitePath' "$FIXTURE") ||
    { echo "BLOCKED: manifest has no fixture.sqlitePath" >&2; return 2; }
  if [[ "$manifest_db" != /* ]]; then
    manifest_db="$ROOT/$manifest_db"
  fi
  if [[ -z "$DB" ]]; then
    DB="$manifest_db"
  fi
  test -n "$DB" || { echo "BLOCKED: GAME_DB_PATH must point at the existing SQLite fixture" >&2; return 2; }
  test -f "$DB" || { echo "BLOCKED: SQLite fixture does not exist: $DB" >&2; return 2; }
  [[ "$(realpath "$DB")" == "$(realpath "$manifest_db")" ]] ||
    { echo "BLOCKED: GAME_DB_PATH does not match fixture.sqlitePath" >&2; return 2; }
  expected_before=$(jq -er '.fixture.sqliteSha256Before' "$FIXTURE") ||
    { echo "BLOCKED: manifest has no pre-run SQLite checksum" >&2; return 2; }
  expected_after=$(jq -er '.fixture.sqliteSha256After' "$FIXTURE") ||
    { echo "BLOCKED: manifest has no post-run SQLite checksum" >&2; return 2; }
  [[ "$expected_before" == "$expected_after" ]] ||
    { echo "BLOCKED: authoritative fixture checksums disagree before execution" >&2; return 2; }
  actual=$(sha256sum "$DB" | awk '{print $1}')
  [[ "$actual" == "$expected_before" ]] ||
    { echo "BLOCKED: SQLite checksum does not match authoritative manifest" >&2; return 2; }
  if [[ -z "$SEED" ]]; then
    SEED=$(jq -er '.freshSeed' "$FIXTURE") ||
      { echo "BLOCKED: manifest has no freshSeed" >&2; return 2; }
  fi
  command -v portless >/dev/null || { echo "BLOCKED: portless is required" >&2; return 2; }
  command -v trunk >/dev/null || { echo "BLOCKED: trunk is required" >&2; return 2; }
  printf 'LAI_PLAYTEST_SEED=%s\nLAI_PLAYTEST_FIXTURE=%s\nGAME_DB_PATH=%s\n' "$SEED" "$FIXTURE" "$DB"
  printf 'LAI_PLAYTEST_API_NAME=%s\nLAI_PLAYTEST_BROWSER_NAME=%s\n' "$API_NAME" "$BROWSER_NAME"
  printf 'LAI_PLAYTEST_TRUNK_PROFILE=%s\n' "$TRUNK_PROFILE"
  printf 'LAI_PLAYTEST_BROWSER_URL=https://%s.localhost:1355\n' "$BROWSER_NAME"
  printf 'LAI_PLAYTEST_API_URL=wss://%s.localhost:1355/ws\n' "$API_NAME"
}

case "${1:-}" in
  --check)
    check_inputs
    ;;
  --run)
    check_inputs
    runtime_dir=$(mktemp -d)
    runtime_db="$runtime_dir/authoritative-runtime.sqlite3"
    cp --reflink=auto "$DB" "$runtime_db"
    # The committed fixture is intentionally immutable. The disposable copy is
    # the authoritative runtime database and must accept action receipts/saves.
    chmod u+rw "$runtime_db"
    cleanup() {
      [[ -z "${browser_pid:-}" ]] || kill "$browser_pid" 2>/dev/null || true
      [[ -z "${api_pid:-}" ]] || kill "$api_pid" 2>/dev/null || true
      # The server performs one final authoritative save during graceful
      # shutdown. Do not delete its disposable database out from underneath it.
      [[ -z "${browser_pid:-}" ]] || wait "$browser_pid" 2>/dev/null || true
      [[ -z "${api_pid:-}" ]] || wait "$api_pid" 2>/dev/null || true
      # Portless can finish just before its forwarded child completes a signal
      # handler. Give that bounded final save time to close the SQLite handle.
      sleep 2
      [[ -z "${runtime_dir:-}" ]] || rm -rf -- "$runtime_dir"
    }
    trap cleanup INT TERM EXIT
    export GAME_DB_PATH="$runtime_db"
    export CARGO_BUILD_JOBS=1
    export CAT_SERVER_ALLOWED_ORIGINS="https://${BROWSER_NAME}.localhost:1355"
    # The debug server keeps this immutable fixture at its authored tick while
    # still authenticating, validating, persisting, and broadcasting real user
    # actions. Release servers ignore this switch.
    export CAT_SERVER_BROWSER_FIXTURE_FREEZE=1
    portless "$API_NAME" taskset -c 0-3 cargo run -p cat-server &
    api_pid=$!
    portless "$BROWSER_NAME" taskset -c 0-3 sh -c \
      'if [ "$3" = release ]; then
         exec env NO_COLOR=true CAT_SERVER_URL="$1" trunk serve --release --address 127.0.0.1 --port "$PORT" --config "$2"
       else
         exec env NO_COLOR=true CAT_SERVER_URL="$1" trunk serve --address 127.0.0.1 --port "$PORT" --config "$2"
       fi' \
      sh "wss://${API_NAME}.localhost:1355/ws" "$ROOT/crates/cat-web/Trunk.toml" "$TRUNK_PROFILE" &
    browser_pid=$!
    wait "$api_pid" "$browser_pid"
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
