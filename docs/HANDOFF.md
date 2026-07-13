# Handoff — continue the Idle Cat Forest build

You are picking up **Idle Cat Forest** — "an idle Dwarf Fortress, played by cats, in a
forest": a top-down god-sim where a cat colony lives, works, breeds, researches, and fights
on its own, ticked once a second by an authoritative server. Read this file, then
`docs/GAME_VISION.md` (what we're building) and `docs/migration/BOARD.md` (what shipped).

**The web→Rust/Bevy migration is COMPLETE.** Work on **`main`** (repo root
`/mnt/storage/workspace/projects/cat_idler`). The P11 cutover landed 2026-07-11: the old
TypeScript/Next.js game was removed from this tree and is preserved — runnable — on branch
`archive/web-game` (tag `web-final`, `8d3bc5a`). Reference only, never the target. The Rust
module doc-comments' "ported from `lib/game/*.ts`" citations point into that branch.

## State of the game (verified 2026-07-13)

- **P0–P19 all shipped** (see `docs/migration/BOARD.md` for the rollup): pure deterministic
  sim core (`cat-sim`, ~680 tests), authoritative server (`cat-server`: 1s tick, WS
  snapshots, SQLite), Bevy 0.19 top-down client with the "cozy ledger" UI kit, spatial
  stockpiles + gather spots, officer roles, workshop/crafting chains, climate biomes +
  ore/metal mining, traders + coin economy, fog of war, multi-village founding.
- **WASM/web build works end-to-end**: `scripts/build-web.sh` → release bundle verified in
  Chromium (WebGL2, live WS, 0 console errors). Native ships as
  `cargo build --release -p cat-desktop`.
- **Dev tooling** (`be7cdee`): `CAT_BRP=1` starts the Bevy Remote Protocol server (port
  15702) in the native client so the bevy MCP can `world.query` the live game; and a
  headless playtest harness prints hourly colony vitals + anomaly flags:
  `SEED=... HOURS=48 CADENCE_MS=1000 cargo run --release -p cat-sim --example playtest`.

Architecture, module map, persistence, and testing contract live in `CLAUDE.md` and
`docs/ARCHITECTURE.md` — both refreshed at the cutover and accurate. Don't re-derive them.

## NEXT STEPS (the real open work, in rough priority order)

1. **Playtest-driven balance + polish.** The previous session built the playtest harness and
   BRP introspection specifically to *play the game* and fix what a player would notice.
   Run the harness on several seeds (true 1s cadence), watch for anomalies (extinction,
   resets, starvation windows, idle stalls, unfought raids), and fix in `cat-sim` with a
   guardrail test + determinism twin per the pattern in `CLAUDE.md` "Testing Contract".
   **Findings so far (2026-07-13 sweep, first two fixed in `4f0cde8`):** research was
   dormant in all real play (capacity-ratio comfort bar vs the per-capita breeding
   homeostat — fixed with a per-capita bar + establishment window + non-sticky scholars);
   extinction left a permanently dead, reset-storming world (TS respawned starter cats —
   fixed). **Still open, same disease:** the shrine **tithe** (`TITHE_FOOD_RATIO` 0.6 of
   food capacity) and **offering** (`OFFERING_MATERIALS_RATIO` 0.6 of materials capacity)
   faucets never fire in unattended play — the breeding homeostat pins food at ~0.1–0.3
   of capacity, and the quarry deficit curve pins materials at ~0.3 — so on-map
   `blessings` stay 0 for whole 48h runs (god-currency has a separate reachable path via
   shrine devotion). Consider per-capita/per-flow bars like `is_research_comfortable`.
   Also: tool crafting is near-zero at live cadence (0–1 tools per 48h), and at the
   harsher 5-min proxy cadence seed 7 now takes one `UnattendedCollapse` at gh~182
   (pre-fix it took none over 200gh; the trough guardrail's 150gh window stays green).
2. **Finish the officer/role split** (the one tracked gameplay gap — `docs/ARCHITECTURE.md`
   "Known gaps", `docs/GAME_VISION.md` pillar 2). Officer roles exist as an *additive*
   assignable layer (`officers.rs`, `AssignOfficer`/`UnassignOfficer`), but the single
   leader director (`leader_director.rs`) still runs most labor allocation. Target: each
   filled role automates its category (Steward hauling/stockpiles, Forester wood, Farmer
   food, Captain defense, Loremaster research); unfilled roles stay manual. Spec:
   `docs/migration/specs/p12-idle-cat-forest.md` + `docs/migration/specs/leader_director.md`.
3. **Cat-art licensing (pre-1.0 release blocker, not a dev blocker).** The in-use cat walk
   sheet `public/images/cats/cat-sheet.png` is the Paws & Whiskers pack — **non-commercial
   license**, and it's tracked in git. Before any public release: replace with a CC0/CC-BY
   32×64-cell 8-dir×4-frame walk sheet (or commission/confirm a license with the owner).
   The rest of the art is Kenney (CC0) — `docs/assets/SELECTION.md` is the source of truth.
4. **Deploy-time follow-ups** (from the P10 close-out, `docs/migration/WASM.md`): hosting
   for the web bundle + transfer-weight optimization. Not blockers.

## HOW TO WORK — hard-won lessons (do not relearn these)

- **codex stalls.** The `codex exec` workspace-write sandbox intermittently hangs (empty
  output, no writes), and Bevy's slow compiles make its dev-loop impractical. Prefer doing
  work yourself, or spawn Claude subagents (Agent tool) — reliable. When adding a crate,
  run `cargo add` YOURSELF (the codex sandbox has no network).
- **"It compiles" ≠ "it renders".** The client once shipped a long-lived black screen
  because nobody looked at the output. ALWAYS verify Bevy visuals by capturing the client's
  OWN framebuffer (the window may be on a monitor you can't `grim`): add a temp system →
  after ~4s `commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
  .observe(bevy::render::view::screenshot::save_to_disk("/tmp/cc.png"))`, run against a
  booted cat-server, then READ `/tmp/cc.png` (view the image). Remove the temp system
  before committing. With `CAT_BRP=1`, the bevy MCP's screenshot/query tools work too.
- **Bevy 0.19 gotchas**: a default `Camera2d` sits at Z=0 and CLIPS sprites at Z>0 (silent
  black screen) — keep the camera at **Z~1000**, sprites below it. Also:
  `Sprite::from_color(Color, Vec2)`; `Text` is a tuple (`text.0 = ...`); `single_mut()` →
  `Result`; `WindowResolution::new(u32,u32)`; `Anchor` is its own component.
- **Determinism discipline** (`cat-sim`): all randomness through the seeded LCG in `rng.rs`;
  subsystems fork the seed by fixed offsets (movement `+1_000_003`, life sim `+2_000_003`,
  raids `+3_000_003`). No `rand`, no `std::time`, `#![forbid(unsafe_code)]`. Pair any
  long-horizon guardrail test with a determinism twin (same seed twice → byte-identical).
- **Commits**: hooks are Rust-only since the cutover (`lefthook.yml`) — no
  `LEFTHOOK_EXCLUDE` needed. fmt+gitleaks on commit; clippy+nextest on push. End commit
  bodies with `Powered by human calories and mass GPU cycles.`
- **Quality gate before every commit**: `cargo nextest run -p <crate>`,
  `cargo clippy -p <crate> --all-targets -- -D warnings`, `cargo fmt --all -- --check`.
  Spurious linker error `undefined hidden symbol ... drop_in_place` → `cargo clean -p
  cat-sim` and retest.
- Keep `docs/migration/BOARD.md` (and this file's NEXT STEPS) updated as you finish things.

## RUN / VERIFY

```bash
cargo dev                    # builds + runs cat-server + cat-desktop together.
                             # Refuses to start if the port is taken — that's a stale
                             # server; `pkill -f target/debug/cat-server` first.

# Or the two halves yourself:
cargo run -p cat-server                                                    # the world
BEVY_ASSET_ROOT=$PWD CAT_SERVER_URL=ws://127.0.0.1:8787/ws cargo run -p cat-desktop  # the window
curl http://127.0.0.1:8787/health    # -> ok

rm data/cat.db               # reset to a fresh founding (recreated on next boot)

# Headless colony health read (no graphics needed):
SEED=20240712 HOURS=48 CADENCE_MS=1000 cargo run --release -p cat-sim --example playtest

# Tests / lint
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Art root: `AssetPlugin { file_path: "." }` → load paths are `public/images/game/...`
(curated Kenney Roguelike 16px family; manifest in `docs/assets/`). Crisp pixels via
`ImagePlugin::default_nearest()`.
