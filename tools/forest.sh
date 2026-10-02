#!/usr/bin/env bash
# Idle Cat Forest developer commands. See docs/unity/DEVELOPMENT.md.
# Compatible with the macOS system bash (3.2): no associative arrays, no empty-array expansion under set -u.
set -euo pipefail

forest_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
forest_project="$forest_root/unity"
forest_unity_version="6000.6.0f1"

# The main checkout owns artifacts, Unity slots and the Library that new worktrees clone.
forest_main="$forest_root"
if forest_common="$(git -C "$forest_root" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)"; then
  forest_main="$(cd "$forest_common/.." && pwd -P)"
fi
forest_worktrees="${FOREST_WORKTREES:-$(dirname "$forest_main")/$(basename "$forest_main")-worktrees}"
forest_worktree_name="$(basename "$forest_root")"
forest_slot_dir="$forest_main/artifacts/unity-slots"

: "${FOREST_ARTIFACTS:=$forest_main/artifacts}"
: "${FOREST_RUN_ID:=$(date +%Y%m%d-%H%M%S)-$$}"
: "${FOREST_UNITY_ROOT:=/Applications/Unity/Hub/Editor/$forest_unity_version}"
export FOREST_ARTIFACTS FOREST_RUN_ID FOREST_UNITY_ROOT
forest_unity_exe="$FOREST_UNITY_ROOT/Unity.app/Contents/MacOS/Unity"

forest_command="${1:-help}"
if [ "$#" -gt 0 ]; then shift; fi

die() { echo "forest.sh: $*" >&2; exit 1; }

valid_id() {
  case "$1" in
    ''|*[!A-Za-z0-9._-]*|.*) die "invalid package id '$1' (letters, digits, '.', '_' and '-' only)" ;;
  esac
}

# Runs a command with a wall-clock limit; the child is stopped on timeout or interrupt.
run_with_timeout() {
  local seconds="$1"; shift
  "$@" &
  local child=$!
  trap 'kill -TERM "$child" 2>/dev/null; wait "$child" 2>/dev/null; exit 130' INT TERM
  local deadline=$(( $(date +%s) + seconds ))
  while kill -0 "$child" 2>/dev/null; do
    if [ "$(date +%s)" -ge "$deadline" ]; then
      echo "forest.sh: timed out after ${seconds}s; stopping process $child" >&2
      kill -TERM "$child" 2>/dev/null || true
      sleep 10
      kill -KILL "$child" 2>/dev/null || true
      wait "$child" 2>/dev/null || true
      trap - INT TERM
      return 124
    fi
    sleep 1
  done
  local status=0
  wait "$child" || status=$?
  trap - INT TERM
  return "$status"
}

# ---------------------------------------------------------------------------------------------
# Unity slots: at most three batch Unity instances on this machine, one mkdir lock per slot.

slot_field() { sed -n "s/^$2=//p" "$1" 2>/dev/null | head -n 1; }

held_slot() { # [id] -> prints the slot held by this worktree (and id, when given)
  local n owner
  for n in 1 2 3; do
    owner="$forest_slot_dir/slot-$n/owner"
    [ -f "$owner" ] || continue
    [ "$(slot_field "$owner" worktree)" = "$forest_root" ] || continue
    if [ "$#" -gt 0 ] && [ "$(slot_field "$owner" id)" != "$1" ]; then continue; fi
    echo "slot-$n"
    return 0
  done
  return 1
}

slot_status() {
  local n dir owner now epoch age pid state
  now=$(date +%s)
  for n in 1 2 3; do
    dir="$forest_slot_dir/slot-$n"; owner="$dir/owner"
    if [ ! -d "$dir" ]; then echo "slot-$n free"; continue; fi
    if [ ! -f "$owner" ]; then echo "slot-$n locked without owner file (being acquired, or abandoned: inspect $dir)"; continue; fi
    epoch="$(slot_field "$owner" epoch)"; age=$(( (now - ${epoch:-$now}) / 60 ))
    pid="$(slot_field "$owner" pid)"
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then state="running"; else state="exited"; fi
    echo "slot-$n held by $(slot_field "$owner" id) since $(slot_field "$owner" since) (${age} min) worktree=$(slot_field "$owner" worktree) pid=${pid:-?} ($state)"
    if [ "$age" -ge 180 ]; then echo "  STALE: held for more than 3 h; confirm with its owner, then: bash tools/forest.sh unity-slot release $(slot_field "$owner" id)"; fi
    if [ "$state" = exited ]; then echo "  note: the acquiring shell has exited; the slot stays held until it is released"; fi
  done
}

slot_acquire() {
  local id="$1" n held now start reported=0
  local poll="${FOREST_SLOT_POLL:-15}" timeout="${FOREST_SLOT_TIMEOUT:-7200}"
  valid_id "$id"
  mkdir -p "$forest_slot_dir"
  if held="$(held_slot "$id")"; then echo "unity-slot: $id already holds $held"; return 0; fi
  start=$(date +%s)
  while :; do
    for n in 1 2 3; do
      if mkdir "$forest_slot_dir/slot-$n" 2>/dev/null; then
        printf 'id=%s\nworktree=%s\npid=%s\nsince=%s\nepoch=%s\n' "$id" "$forest_root" "$PPID" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(date +%s)" > "$forest_slot_dir/slot-$n/owner.tmp"
        mv "$forest_slot_dir/slot-$n/owner.tmp" "$forest_slot_dir/slot-$n/owner"
        echo "unity-slot: acquired slot-$n for $id"
        return 0
      fi
    done
    now=$(date +%s)
    if [ $(( now - start )) -ge "$timeout" ]; then
      slot_status >&2
      die "unity-slot: no slot became free within ${timeout}s"
    fi
    if [ "$reported" -eq 0 ]; then
      echo "unity-slot: all 3 slots are busy; waiting for one (poll ${poll}s, timeout ${timeout}s)" >&2
      slot_status >&2
      reported=1
    fi
    sleep "$poll"
  done
}

slot_release() {
  local id="$1" n owner
  valid_id "$id"
  for n in 1 2 3; do
    owner="$forest_slot_dir/slot-$n/owner"
    [ -f "$owner" ] || continue
    [ "$(slot_field "$owner" id)" = "$id" ] || continue
    if [ "$(slot_field "$owner" worktree)" != "$forest_root" ]; then
      die "unity-slot: slot-$n for $id belongs to $(slot_field "$owner" worktree); release it from there"
    fi
    rm -f "$owner"
    rmdir "$forest_slot_dir/slot-$n"
    echo "unity-slot: released slot-$n for $id"
    return 0
  done
  die "unity-slot: $id holds no slot"
}

# Every Unity batch command goes through this guard.
unity_guard() {
  if [ "$forest_root" = "$forest_main" ] && [ "${FOREST_INTEGRATOR:-}" != 1 ]; then
    die "the main checkout's Unity project belongs to the wave integrator; work in a package worktree or set FOREST_INTEGRATOR=1"
  fi
  held_slot >/dev/null || die "no Unity slot held for $forest_root; run: bash tools/forest.sh unity-slot acquire <package-id>"
}

# ---------------------------------------------------------------------------------------------

check() {
  local log status=0
  log="$(mktemp "${TMPDIR:-/tmp}/forest-check.XXXXXX")"
  DOTNET_CLI_UI_LANGUAGE=en dotnet build "$forest_root/tools/unity-check/UnityCheck.slnx" -nologo -v:q -clp:NoSummary "$@" > "$log" 2>&1 || status=$?
  if [ "$status" -eq 0 ]; then
    echo "check: ok"
  else
    if grep -E ': error( [A-Z]+[0-9]+)? *:' "$log" | sed -E 's/ \[[^]]*\]$//' | sort -u | grep .; then :; else tail -n 40 "$log"; fi
    echo "check: failed"
  fi
  rm -f "$log"
  return "$status"
}

worktree_init() {
  [ "$#" -ge 1 ] && [ "$#" -le 2 ] || die "usage: worktree-init <package-id> [base=demo-polish]"
  local id="$1" base="${2:-demo-polish}" target
  valid_id "$id"
  target="$forest_worktrees/$id"
  if [ -d "$target" ]; then
    echo "worktree-init: reusing $target"
  elif git -C "$forest_main" show-ref --verify --quiet "refs/heads/demo/$id"; then
    git -C "$forest_main" worktree add "$target" "demo/$id"
  else
    git -C "$forest_main" worktree add -b "demo/$id" "$target" "$base"
  fi
  if [ -d "$target/unity/Library" ]; then
    echo "worktree-init: $target/unity/Library already exists; kept"
  elif [ -d "$forest_main/unity/Library" ]; then
    cp -c -R "$forest_main/unity/Library" "$target/unity/Library"
    # Process ids of the main checkout's Editor helpers must not leak into the clone.
    rm -f "$target/unity/Library/ilpp.pid" "$target/unity/Library/burst.pid"
    echo "worktree-init: cloned unity/Library from $forest_main"
  else
    echo "worktree-init: $forest_main/unity/Library does not exist; the first Unity run will import from scratch" >&2
  fi
  echo "worktree-init: ready at $target on branch demo/$id"
}

edit_test() {
  unity_guard
  local out="$FOREST_ARTIFACTS/tests/$FOREST_RUN_ID" filter='!\.campaign\.' status=0
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --filter) [ "$#" -ge 2 ] || die "--filter needs a regular expression"; filter="$2;$filter"; shift 2 ;;
      --filter=*) filter="${1#--filter=};$filter"; shift ;;
      *) die "edit-test: unknown argument '$1' (supported: --filter <regex>)" ;;
    esac
  done
  [ -x "$forest_unity_exe" ] || die "Unity editor not found at $forest_unity_exe (set FOREST_UNITY_ROOT)"
  mkdir -p "$out"
  echo "edit-test: report $out/editmode.xml, log $out/editmode.log"
  # The '!\.campaign\.' name guard keeps the multi-hour campaign twins out until they carry the Campaign category.
  run_with_timeout 2400 "$forest_unity_exe" -batchmode -nographics -projectPath "$forest_project" \
    -runTests -testPlatform EditMode \
    -assemblyNames 'IdleCatForest.Acceptance.Tests;IdleCatForest.Simulation.Tests' \
    -testCategory '!Campaign' -testFilter "$filter" \
    -testResults "$out/editmode.xml" -logFile "$out/editmode.log" || status=$?
  if [ -f "$out/editmode.xml" ]; then
    grep -m 1 -o '<test-run [^>]*' "$out/editmode.xml" | grep -oE ' (total|passed|failed|skipped|result)="[^"]*"' | tr -d '\n'; echo
  else
    echo "edit-test: no report was written; see $out/editmode.log" >&2
  fi
  return "$status"
}

build() {
  unity_guard
  local output="$FOREST_ARTIFACTS/macos/$forest_worktree_name/$FOREST_RUN_ID/Idle Cat Forest.app"
  unity build "$forest_project" --target StandaloneOSX \
    --execute-method IdleCatForest.Editor.ForestEditor.Build \
    --output-path "$output" \
    --allow-dirty-build --no-tail --timeout 1200 --format json "$@"
  echo "build: $output"
}

latest_build() { # newest complete player built from this worktree
  local run
  while IFS= read -r run; do
    if [ -f "$run/Idle Cat Forest.app/Contents/Info.plist" ]; then echo "$run/Idle Cat Forest.app"; return 0; fi
  done < <(ls -1dt "$FOREST_ARTIFACTS/macos/$forest_worktree_name"/*/ 2>/dev/null | sed 's:/$::')
  return 1
}

capture() {
  local script="" out="" fixture="fresh" seed="4242" size="2400x1500" rebuild=0
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --out) [ "$#" -ge 2 ] || die "--out needs a directory"; out="$2"; shift 2 ;;
      --out=*) out="${1#--out=}"; shift ;;
      --fixture) [ "$#" -ge 2 ] || die "--fixture needs a name"; fixture="$2"; shift 2 ;;
      --fixture=*) fixture="${1#--fixture=}"; shift ;;
      --seed) [ "$#" -ge 2 ] || die "--seed needs a number"; seed="$2"; shift 2 ;;
      --seed=*) seed="${1#--seed=}"; shift ;;
      --size) [ "$#" -ge 2 ] || die "--size needs WIDTHxHEIGHT"; size="$2"; shift 2 ;;
      --size=*) size="${1#--size=}"; shift ;;
      --rebuild) rebuild=1; shift ;;
      -*) die "capture: unknown option '$1'" ;;
      *) [ -z "$script" ] || die "capture: only one script may be given"; script="$1"; shift ;;
    esac
  done
  [ -n "$script" ] && [ -n "$out" ] || die "usage: capture <script.json> --out <dir> [--fixture fresh|founding30|expanded150|demo35] [--seed N] [--size 2400x1500] [--rebuild]"
  [ -f "$script" ] || die "capture: script '$script' not found"
  script="$(cd "$(dirname "$script")" && pwd -P)/$(basename "$script")"
  [[ "$size" =~ ^[0-9]+x[0-9]+$ ]] || die "capture: --size must look like 2400x1500"
  [[ "$seed" =~ ^-?[0-9]+$ ]] || die "capture: --seed must be an integer"
  case "$fixture" in
    fresh|founding30|expanded150) ;;
    demo35) die "capture: the demo35 fixture does not exist yet (W4-04 adds it); use fresh, founding30 or expanded150" ;;
    *) die "capture: unknown fixture '$fixture'" ;;
  esac
  unity_guard

  local app=""
  if [ "$rebuild" -eq 0 ]; then app="$(latest_build || true)"; fi
  if [ -z "$app" ]; then
    build
    app="$FOREST_ARTIFACTS/macos/$forest_worktree_name/$FOREST_RUN_ID/Idle Cat Forest.app"
  fi
  [ -f "$app/Contents/Info.plist" ] || die "capture: no player at $app"
  local exe
  exe="$app/Contents/MacOS/$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app/Contents/Info.plist")"

  # The fixture save and its identity live only in this temp directory and are removed on exit.
  capture_tmp="$(mktemp -d "${TMPDIR:-/tmp}/forest-capture.XXXXXX")"
  trap 'rm -rf "$capture_tmp"' EXIT
  local save="$capture_tmp/world.json"
  case "$fixture" in
    founding30) dotnet run --project "$forest_root/tools/presentation-fixture/Forest.PresentationFixture.csproj" -c Release -- "$save" 30 ;;
    expanded150) dotnet run --project "$forest_root/tools/presentation-fixture/Forest.PresentationFixture.csproj" -c Release -- "$save" 150 ;;
    fresh) ;;
  esac

  mkdir -p "$out"
  out="$(cd "$out" && pwd -P)"
  echo "capture: $app"
  echo "capture: script=$script fixture=$fixture size=$size out=$out"
  local status=0
  run_with_timeout 600 "$exe" -screen-width "${size%x*}" -screen-height "${size#*x}" -screen-fullscreen 0 \
    -logFile "$out/player.log" \
    --forest-save "$save" --forest-seed "$seed" \
    --forest-capture-script "$script" --forest-capture-out "$out" || status=$?
  local shots
  shots="$(ls -1 "$out"/*.png 2>/dev/null | wc -l | tr -d ' ')"
  echo "capture: exit=$status shots=$shots log=$out/capture-log.txt"
  [ -f "$out/capture-log.txt" ] || { echo "capture: the player wrote no capture-log.txt; see $out/player.log" >&2; [ "$status" -ne 0 ] || status=1; }
  return "$status"
}

usage() {
  cat <<'EOF'
Usage: bash tools/forest.sh <command> [args]

Compile and test without Unity
  check                       compile Presentation, Editor and PlayMode tests against Unity's reference assemblies
  server-test                 authority and server tests
  server                      run the shared-world host

Worktrees and Unity slots
  worktree-init <id> [base]   git worktree at <main>-worktrees/<id> on demo/<id> (base demo-polish), Library cloned
  unity-slot acquire <id>     wait for one of the 3 Unity batch slots (FOREST_SLOT_POLL, FOREST_SLOT_TIMEOUT)
  unity-slot release <id>     free the slot
  unity-slot status           list slot holders

Unity batch commands (need a held slot; the main checkout also needs FOREST_INTEGRATOR=1)
  edit-test [--filter RE]     EditMode acceptance and simulation assemblies, Campaign excluded
  play-test [unity test args] PlayMode tests
  build                       Apple Silicon player into $FOREST_ARTIFACTS/macos/<worktree>/<run id>/
  capture <script.json> --out <dir> [--fixture fresh|founding30|expanded150|demo35] [--seed N] [--size WxH] [--rebuild]
  setup | inspect | performance

Live Editor commands
  open | play | stop

Environment: FOREST_ARTIFACTS (default <main>/artifacts), FOREST_RUN_ID, FOREST_UNITY_ROOT, FOREST_UNITY_LIBRARY,
FOREST_INTEGRATOR, FOREST_WORKTREES.
EOF
}

case "$forest_command" in
  open) unity open "$forest_project" --editor-version "$forest_unity_version" "$@" ;;
  setup) unity_guard; unity command forest_setup --project-path "$forest_project" --format json "$@" ;;
  play) unity command editor_play --project-path "$forest_project" --format json "$@" ;;
  stop) unity command editor_stop --project-path "$forest_project" --format json "$@" ;;
  inspect) unity_guard; unity command forest_inspect --project-path "$forest_project" --format json "$@" ;;
  performance) unity_guard; unity command forest_performance --project-path "$forest_project" --format json "$@" ;;
  build) build "$@" ;;
  capture) capture "$@" ;;
  check) check "$@" ;;
  worktree-init) worktree_init "$@" ;;
  unity-slot)
    case "${1:-}" in
      acquire) [ "$#" -eq 2 ] || die "usage: unity-slot acquire <id>"; slot_acquire "$2" ;;
      release) [ "$#" -eq 2 ] || die "usage: unity-slot release <id>"; slot_release "$2" ;;
      status) slot_status ;;
      *) die "usage: unity-slot acquire|release|status [<id>]" ;;
    esac
    ;;
  server) dotnet run --project "$forest_root/server/Forest.Server/Forest.Server.csproj" -- "$@" ;;
  server-test) dotnet run --project "$forest_root/server/Forest.Tests/Forest.Tests.csproj" -- "$@" ;;
  edit-test) edit_test "$@" ;;
  play-test)
    unity_guard
    mkdir -p "$FOREST_ARTIFACTS/tests/$FOREST_RUN_ID"
    unity test "$forest_project" --mode PlayMode --output "$FOREST_ARTIFACTS/tests/$FOREST_RUN_ID/playmode.xml" --timeout 600 --format json "$@"
    ;;
  help|-h|--help) usage ;;
  *) usage >&2; exit 2 ;;
esac
