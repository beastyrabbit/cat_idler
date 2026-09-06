#!/usr/bin/env bash
set -euo pipefail

forest_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
forest_project="$forest_root/unity"
forest_command="${1:-help}"
if [ "$#" -gt 0 ]; then shift; fi

case "$forest_command" in
  open) unity open "$forest_project" --editor-version 6000.6.0f1 "$@" ;;
  setup) unity command forest_setup --project-path "$forest_project" --format json "$@" ;;
  play) unity command editor_play --project-path "$forest_project" --format json "$@" ;;
  stop) unity command editor_stop --project-path "$forest_project" --format json "$@" ;;
  inspect) unity command forest_inspect --project-path "$forest_project" --format json "$@" ;;
  performance) unity command forest_performance --project-path "$forest_project" --format json "$@" ;;
  build)
    unity build "$forest_project" --target StandaloneOSX \
      --execute-method IdleCatForest.Editor.ForestEditor.Build \
      --output-path "$forest_root/artifacts/macos/Idle Cat Forest.app" \
      --allow-dirty-build --no-tail --timeout 1200 --format json "$@"
    ;;
  server) dotnet run --project "$forest_root/server/Forest.Server/Forest.Server.csproj" -- "$@" ;;
  server-test) dotnet run --project "$forest_root/server/Forest.Tests/Forest.Tests.csproj" -- "$@" ;;
  edit-test)
    mkdir -p "$forest_root/artifacts/tests"
    unity test "$forest_project" --mode EditMode --output "$forest_root/artifacts/tests/editmode.xml" --timeout 1200 --format json \
      --filter 'SimulationTests|regression\.|catalog\.|capability\.|chain\.|legacy_upgrade\.|legacy_effect\.|building_effect\.|service_effect\.|resource_effect\.|recipe\.' "$@"
    ;;
  play-test)
    mkdir -p "$forest_root/artifacts/tests"
    unity test "$forest_project" --mode PlayMode --output "$forest_root/artifacts/tests/playmode.xml" --timeout 600 --format json "$@"
    ;;
  *)
    echo 'Usage: bash tools/forest.sh open|setup|play|stop|inspect|performance|build|server|server-test|edit-test|play-test'
    echo 'Live commands require this project open in Unity. Batch build/test requires its Editor closed.'
    ;;
esac
