> Historical Rust/Bevy record. Current development follows [AGENTS.md](../AGENTS.md),
> [Unity architecture](ARCHITECTURE.md), and the [acceptance ledger](unity/ACCEPTANCE.md).
> Old commands, entry points and backlog statuses below are retained as history.

# Handoff — continue the Idle Cat Forest build

**Permanent project status:** Idle Cat Forest is a non-commercial game project. This is settled
project context and not an open implementation or product decision.

You are picking up **Idle Cat Forest** — "an idle Dwarf Fortress, played by cats, in a
forest": a top-down god-sim where a cat colony lives, works, breeds, researches, and fights
on its own, ticked once a second by an authoritative server. Read this file, then
`docs/GAME_VISION.md` (the maintained design) and `docs/IMPLEMENTATION_AUDIT.md` (the evidence
ledger). `docs/migration/BOARD.md` preserves the phase history and its current rollup.

**The web→Rust/Bevy migration is complete.** Work on **`main`** (repo root
`/mnt/storage/workspace/projects/cat_idler`). The P11 cutover landed 2026-07-11: the old
TypeScript/Next.js game was removed from this tree and is preserved — runnable — on branch
`archive/web-game` (tag `web-final`, `8d3bc5a`). It is reference material, never the target.

## State of the game (verified 2026-07-16)

The maintained P12–P19 design is implemented and evidence-backed. Do not infer a new backlog
from historical phase prose or dated intermediate counts; use `docs/IMPLEMENTATION_AUDIT.md`
and `docs/FIX_LOG.md`.

The 2026-07-16 integrated correction set is closed. Its combined exact-cadence passive,
observed-state signed player, persistence, and 1024×768/1920×1080 client-framebuffer gate passes;
the reproducible evidence is maintained in `docs/FIX_LOG.md`.

Key shipped contracts:

- The deterministic `cat-sim` and authoritative SQLite/WebSocket `cat-server` tick every colony
  once per second. Simulation and disk work use Tokio's blocking pool; selected-village routing,
  restart persistence, authentication, rate limits, health/readiness, and production hosting are
  verified.
- The native and WASM Bevy client is a single-level top-down view with a camera near Z=1000,
  label-free roofed homes, open/cutaway workshops, visible physical stockpiles and cargo, fog,
  roads, farms, staged walls, inspectors, and an Adventure-styled HUD.
- One ownerless 30-cat/six-Den Grand Commons and one private 15-cat/three-Den village per stable
  signed player share one world. Returned scouts establish contact; scalar goods and exact finite
  equipment travel in persisted, visible, obstacle-aware caravans.
- A personal village starts with exactly 15 adult cats, three five-bed Dens, a resource-free
  13×13 interior, an exterior water guarantee, one south gate, and a roughly two-tile knowledge
  halo. Breeding reserves a bed; prosperous migration has a physical arrival/departure journey and
  a 36-game-hour housing probation; extinction atomically restores the founding contract.
- The founding Leader owns only a bounded hunt/water/scout safety floor. Steward, Accountant,
  Forester, Farmer, Captain, Loremaster, and Cloth Leader own their specialist automation; vacant
  roles leave those systems manual. Appointment requires the matching researched, completed role
  station.
- Fog knowledge becomes permanent only when a living scout returns to and touches the shrine.
  Deficit-driven wood/resource/general scouting belongs to the founding Leader, including the fast
  first wood search; research labor/building automation and rituals belong to the Loremaster.
- The research ledger contains exactly **487 live studies**: 165 Building, 167 Recipe/Resource,
  and 155 Upgrade. All 487 are dependency-order purchasable and persistent; there are no disabled
  `FUTURE` studies. The Leader may complete at most one affordable study per rolling real-life day,
  while the player may buy any affordable studies directly.
- The physical production graph contains **108 recipes**. Finite input is reserved and carried to
  station-local storage, staffed work creates a finite output, and a living carrier delivers it
  before aggregate credit. All generated recipe and resource payloads have authoritative runtime
  consumers. The 32-resource wire/storage/carrying vocabulary is exhaustive; the complete inventory
  is in Stores while the world HUD intentionally pins only four critical kinds.
- `Crews` research is truthful: 12 studies expand real concurrent worker slots at physical
  multi-worker stations; 13 studies enable bounded services scoped to their completed building.
  Those service studies do not invent worker slots.
- Exact Tool/Weapon/Armor units preserve identity, material, quality, weight, durability, wear,
  repair, equipment state, trade escrow, and persistence. Roads, rail, shipping, visiting traders,
  shrine offerings, farming, fishing, forestry, extraction, processing, and all 19 maintained labor
  skills follow the same physical-state discipline.
- Passive deterministic campaigns, observed-state guided campaigns, all public action variants,
  persistence/restart campaigns, and native/WASM framebuffers are recorded in the audit. The
  compact world HUD keeps only critical survival stores visible, the complete inventory is in
  Stores [G], and one command category expands at a time. The research ledger remains 487/487 with
  no `FUTURE` entries.

Dev tooling includes `CAT_BRP=1` for Bevy Remote Protocol inspection and a headless playtest
harness:

```bash
SEED=20240712 HOURS=48 CADENCE_MS=1000 cargo run --release -p cat-sim --example playtest
COMMUNAL=1 SEED=20240712 HOURS=48 CADENCE_MS=1000 cargo run --release -p cat-sim --example playtest
```

Fresh passive defaults assert only officer-independent behavior. Use the signed guided campaigns
for a player-established economy; they must dispatch real `ClientAction`s from observed state.

## NEXT STEPS

1. **Keep the tiered test workflow healthy.** Run focused regressions and the two-thread smoke
   profile locally; every push must leave the single capped Forgejo full-suite job green. See
   `docs/TESTING.md`.
2. **Optionally tune WASM transfer weight.** Browser boot, reconnect, action feedback, responsive
   layout, caching, and the production image are verified. Further bundle-size/thread work is an
   optimization campaign, not a gameplay blocker; see `docs/migration/WASM.md`.

Any newly found gameplay or visual defect belongs in `docs/FIX_LOG.md` with a reproduction before
implementation. Do not resurrect completed work from the historical migration board.

## HOW TO WORK — hard-won lessons (do not relearn these)

- **Use subagents for bounded parallel slices.** Isolate overlapping implementations in worktrees
  and integrate only after their own focused gates pass.
- **"It compiles" ≠ "it renders".** Always verify Bevy visuals by capturing the client's own
  framebuffer: add a temporary system that spawns
  `bevy::render::view::screenshot::Screenshot::primary_window()` and observes
  `bevy::render::view::screenshot::save_to_disk("/tmp/cc.png")`, run against a booted server,
  inspect `/tmp/cc.png`, then remove the temporary system before committing.
- **Bevy 0.19:** a default `Camera2d` sits at Z=0 and clips sprites at Z>0. Keep the camera near
  **Z=1000** and sprites below it. `Sprite::from_color(Color, Vec2)`; `Text` is a tuple;
  `single_mut()` returns `Result`; `WindowResolution::new` takes `u32`; `Anchor` is a component.
- **Determinism:** all `cat-sim` randomness goes through `rng.rs`; movement, life, and raids use
  their fixed fork offsets. No `rand`, no wall clock, no I/O. Pair long-horizon guardrails with a
  byte-identical determinism twin.
- **Dependencies:** use `cargo add`/`cargo remove`; never edit dependency versions by hand.
- **Commits:** hooks are Rust-only. End commit bodies with
  `Powered by human calories and mass GPU cycles.`
- **Quality gate:** run the focused regression, the local smoke profile, strict touched-crate
  Clippy, and `cargo fmt --all -- --check`. The complete workspace suite runs only after push on
  the resource-capped `cat-idler-heavy` runner. Documentation-only changes require targeted
  link/content scans and whitespace validation. See `docs/TESTING.md`.

## RUN / VERIFY

```bash
cargo dev

# Or run the two halves yourself:
cargo run -p cat-server
BEVY_ASSET_ROOT=$PWD CAT_SERVER_URL=ws://127.0.0.1:8787/ws cargo run -p cat-desktop
curl http://127.0.0.1:8787/health    # -> ok

rm data/cat.db                       # reset to a fresh founding

cargo nextest run --workspace --profile smoke
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Art root: `AssetPlugin { file_path: "." }`; runtime load paths are
`public/images/game/...`. Keep crisp pixel filtering through `ImagePlugin::default_nearest()`.
