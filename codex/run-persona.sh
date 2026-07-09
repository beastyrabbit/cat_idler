#!/usr/bin/env bash
# In-project codex persona runner.
#
#   codex/run-persona.sh <persona> "<card text>" [extra codex args...]
#
# Personas: scrum-master researcher test-engineer developer qa integrator
# The repo AGENTS.md is auto-read by codex; this injects the persona prompt +
# the card, and picks a sandbox/effort suited to the role. Runs gpt-5.5 headless.
set -euo pipefail

persona="${1:?usage: run-persona.sh <persona> \"<card>\"}"; shift
card="${1:?missing card text}"; shift || true

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pfile="$root/codex/personas/${persona}.md"
[ -f "$pfile" ] || { echo "unknown persona: $persona (see codex/personas/)" >&2; exit 2; }

# Reasoning effort per role. NOTE: xhigh reliably TIMES OUT on big modules/specs,
# so QA and researcher run at high (still catches real bugs; finishes in budget).
case "$persona" in
  scrum-master)               effort=high  ;;
  researcher|qa)              effort=high  ;;
  developer|test-engineer)    effort=high  ;;
  integrator)                 effort=medium ;;
  *)                          effort=high  ;;
esac

prompt="$(cat "$pfile")

## Your card
${card}"

# workspace-write keeps edits inside the repo; network on so cargo can fetch crates.
# stdin from /dev/null: codex exec otherwise blocks forever waiting for stdin EOF
# when launched in the background (the prompt is passed as an arg, not via stdin).
exec codex exec \
  --sandbox workspace-write \
  -c sandbox_workspace_write.network_access=true \
  -c model_reasoning_effort="${effort}" \
  "$@" \
  "${prompt}" < /dev/null
