# Handoff — continue the Idle Cat Forest build

You are picking up an in-progress rebuild of a cat-colony idle game. Read this whole
file, then `docs/GAME_VISION.md` (what we're building) and `docs/migration/BOARD.md`
(status + task board). Work on git branch **`migration/bevy-rust`** (repo root
`/mnt/storage/workspace/projects/cat_idler`). The old TypeScript/Next.js game is frozen
on branch `archive/web-game` (+ tag `web-final`) — reference only, never the target.

## The game (see docs/GAME_VISION.md for the full vision)
**"Idle Cat Forest" — an idle Dwarf Fortress, played by cats, in a forest.** TOP-DOWN,
single level. You direct a cat colony manually at first; as you unlock **leadership
roles/officers** (via the upgrade tree, escalating costs, in role-buildings) each one
automates a category of work → it becomes idle. More jobs than cats; cats get better at
labors over time; visible workshops, stockpiles, and farm plots; production chains
(catnip/grain → mill; fibre → clothier → cloth → armour).

## Architecture (a Cargo workspace under `crates/`)
- **cat-sim** — pure, deterministic simulation (no I/O). ~40 modules ported from the TS
  `lib/game/*` + the 37-phase `world_tick` master loop, multi-colony. `found_colony` is the
  village-founding primitive. `actions.rs` = pure `apply_action` + `build_snapshot`. DONE, ~360 tests.
- **cat-protocol** — serde wire DTOs: `WorldSnapshot`/`ColonySnapshot` + `ClientAction` (camelCase).
- **cat-server** — authoritative: tokio 1s tick loop runs `world_tick`, axum `/health` + `/ws`
  WebSocket broadcasts snapshots + receives actions, SQLite (rusqlite) persistence, HMAC
  identity, rate-limit. DONE, live-verified (found village over WS → survives restart).
- **cat-client** (lib, `run()`) + **cat-desktop** (bin) — the Bevy 0.19 game window. Connects
  over WebSocket (`ewebsock`), deserializes `WorldSnapshot` each frame, renders. IN PROGRESS
  (P9): currently renders cats + HUD; being extended to the full top-down world.
- **cat-web** — future WASM target (P10, not started).

## Status
- **P0–P8 DONE**: full simulation + authoritative server + persistence + multi-village
  founding, all committed and live-verified. `docs/migration/specs/world_tick.md` maps the tick.
- **P9 IN PROGRESS**: the top-down Bevy client. Design just pivoted from isometric → **top-down**
  (single level) per GAME_VISION.md.
- **P10** WASM, **P11** cutover (merge migration/bevy-rust → main), **P12** sim expansion
  (skills, role/officer system, spatial stockpiles, more workshops + chains, farm plots),
  **P13** client designation/role UI — all pending. See GAME_VISION.md "Build order".

## HOW TO WORK — hard-won lessons (do not relearn these)
- **codex stalls.** The `codex exec` workspace-write sandbox intermittently hangs (empty
  output, no writes), and Bevy's slow compiles make its dev-loop impractical. PREFER doing
  work yourself, or spawn **Claude subagents** (Agent tool, `general-purpose`, opus) — reliable.
  When adding a crate, run `cargo add` YOURSELF (codex sandbox has no network).
- **"It compiles" ≠ "it renders".** The client had a black screen for a long time because
  nobody looked at the actual output. ALWAYS verify Bevy visuals by capturing the client's
  OWN framebuffer (the window may be on a monitor you can't `grim`):
  add a temp system → after ~4s `commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk("/tmp/cc.png"))`,
  run, then READ `/tmp/cc.png` (view the image). Remove the temp system before committing.
- **Bevy 0.19 gotchas**: a default `Camera2d` sits at Z=0 and CLIPS sprites at Z>0 (renders
  them behind it → black screen) — keep the camera at **Z~1000**, sprites at z<1000. Also:
  `Sprite::from_color(Color, Vec2)`; `Text` is a tuple (`text.0 = ...`); `single_mut()` →
  `Result`; `World::insert_non_send` (not `_resource`); `WindowResolution::new(u32,u32)`;
  log macros need the `bevy_log` feature (enabled); `Anchor` is its own component.
- **Cat positions** in the snapshot are `i32` tile coords. Top-down projection:
  `Vec2::new(x as f32 * TILE, -(y as f32) * TILE)`, TILE ≈ 28. Center camera on the village
  (`cat_sim` VILLAGE_ANCHOR ~ (6,6)).
- **Commits**: `LEFTHOOK_EXCLUDE=typecheck,lint git commit ...` (the JS hooks aren't relevant;
  `tsc` isn't installed). Rust hooks: fmt on commit, clippy+nextest on push. End commit
  bodies with `Powered by human calories and mass GPU cycles.`
- **Quality gate before every commit**: `cargo nextest run -p <crate>`, `cargo clippy -p
  <crate> --all-targets -- -D warnings`, `cargo fmt --all -- --check`. If you hit a spurious
  linker error `undefined hidden symbol ... drop_in_place`, run `cargo clean -p cat-sim` and retest.
- Keep `docs/migration/BOARD.md` updated as you complete cards.

## RUN / VERIFY the whole thing
```bash
export PATH="$HOME/.cargo/bin:$PATH"
# 1) server (the world)
PORT=8787 GAME_DB_PATH=/tmp/db WORKER_TICK_MS=500 ./target/debug/cat-server   # or: cargo run -p cat-server
curl http://127.0.0.1:8787/health          # -> ok
# 2) client (the window) — needs a graphical session; assets load from BEVY_ASSET_ROOT
BEVY_ASSET_ROOT=$PWD CAT_SERVER_URL=ws://127.0.0.1:8787/ws cargo run -p cat-desktop
```
Art lives in `public/images/iso/tiles`, `public/images/iso/buildings`, `public/images/cats`.
The verified 1653-line render spike `crates/cat-client/reference/spike-bevy-0.19.rs` is the
source to lift camera/sprite/HUD/atlas patterns from (it renders correctly).

## NEXT STEPS
1. Finish **P9 top-down client** (BOARD P9): terrain grid from `world_seed` (via `cat_sim`
   world/terrain gen), cats + carried-item glyphs, labelled workshops/buildings, visible
   storage/stockpiles, avoid/gather zones, camera pan/zoom, dashboard + manual action buttons
   that send `ClientAction` over the WS (Supply food/water, Plan hunt, Found village). VERIFY each on the framebuffer.
2. Then **P12 sim expansion** (cat-sim): skills → role/officer system → spatial stockpiles →
   more workshops + production chains → visible farm plots (per GAME_VISION.md).
3. Then **P13 client** designation/role UI, **P10** WASM, **P11** cutover.
