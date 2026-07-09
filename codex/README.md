# codex — the in-project build team

The Rust/Bevy migration is built by a team of **codex personas** (gpt-5.5),
orchestrated by Claude. Everything the team needs lives in this repo so the setup
is portable and versioned.

- `personas/*.md` — the role prompts (scrum-master, researcher, test-engineer,
  developer, qa, integrator). See `docs/migration/BOARD.md` for how they pipeline.
- `../AGENTS.md` — shared context codex auto-reads (arch, parity + determinism
  rules, test conventions). Every persona run inherits it.
- `run-persona.sh` — injects a persona + card and runs codex headless with a
  role-appropriate sandbox/effort.

## Usage
```bash
codex/run-persona.sh researcher   "CARD P3.1: spec lib/game/pathfinding.ts → cat-sim/src/pathfinding.rs"
codex/run-persona.sh test-engineer "CARD P3.1: write failing tests for cat-sim pathfinding per specs/pathfinding.md"
codex/run-persona.sh developer     "CARD P3.1: implement cat-sim pathfinding to green"
codex/run-persona.sh qa            "CARD P3.1: verify cat-sim pathfinding parity vs lib/game/pathfinding.ts"
```
Independent cards run in parallel (background); QA runs concurrently once a card's
dev finishes. Effort: xhigh for scrum-master/researcher/qa, high for dev/test, medium
for integrator. Sandbox is `workspace-write` with network on (for `cargo add`).

## MCP for codex
The `bevy` MCP is registered with codex (`codex mcp list`) so QA/dev on **client**
cards can `world_query` / screenshot the running Bevy game to self-verify. Sim
cards don't need it.

## Model note
gpt-5.5 is the implementer (bulk work). High-value slices (cat-AI, world_tick,
protocol) also get an independent Claude review (fable-5 / opus) as a second pair
of eyes on top of the codex QA persona.
