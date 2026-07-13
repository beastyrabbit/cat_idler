# Handoff — continue the Idle Cat Forest build

You are picking up **Idle Cat Forest** — "an idle Dwarf Fortress, played by cats, in a
forest": a top-down god-sim where a cat colony lives, works, breeds, researches, and fights
on its own, ticked once a second by an authoritative server. Read this file, then
`docs/GAME_VISION.md` (what we're building) and `docs/migration/BOARD.md` (what landed and what
remains partial).

**The web→Rust/Bevy migration is COMPLETE.** Work on **`main`** (repo root
`/mnt/storage/workspace/projects/cat_idler`). The P11 cutover landed 2026-07-11: the old
TypeScript/Next.js game was removed from this tree and is preserved — runnable — on branch
`archive/web-game` (tag `web-final`, `8d3bc5a`). Reference only, never the target. The Rust
module doc-comments' "ported from `lib/game/*.ts`" citations point into that branch.

## State of the game (verified 2026-07-13)

The **migration and P11 cutover are complete**; the maintained game is still pre-release and
several P12–P19 product promises remain partial. Do not turn a phase label or compiling crate
into a completion claim. `docs/IMPLEMENTATION_AUDIT.md` is the detailed evidence ledger.

Verified foundations include:

- the deterministic sim and authoritative SQLite/WebSocket server, with simulation and disk work
  on Tokio's blocking pool, startup-cached snapshots, responsive health/initial connections, and
  selected-colony action/snapshot routing;
- the native and browser Bevy clients, bounded world streaming, reconnect/action feedback, and a
  same-origin non-root production image with `/health`, `/ready`, compression, and Origin checks;
- atomic placement/reservations/scaffold recovery; label-free roofed homes and explicit open
  compositions for the 24 current protocol building variants; exterior crop/logging production
  with distinct Mill and Sawmill stations;
- exact founding fog plus signed resource/general scouting whose provisional knowledge commits
  only on physical shrine return, including restart-safe in-flight notebooks and responsive
  controls at 1024×768 through 1920×1080;
- a pure 500-node research catalog and full-page searchable/filterable/pannable client ledger.
  Generated studies honestly remain read-only until their live effects are implemented;
- a persistent village selector and secure selected-village routing. This is not yet the
  canonical-global/personal-ownership/discovery/trade model.

Dev tooling (`be7cdee`) includes `CAT_BRP=1` for Bevy Remote Protocol inspection and a headless
playtest harness:
`SEED=... HOURS=48 CADENCE_MS=1000 cargo run --release -p cat-sim --example playtest`.

## NEXT STEPS (maintained backlog, in rough priority order)

1. **Make the intended player loop real.** Finish vacant-role manual ownership, role-building
   gates, exact placement/farm/gather/road/staffing/military/ritual/election controls, and the
   manual-raid single-damage fix. Exercise every path with signed `ClientAction`s, not fixture
   mutation.
2. **Finish the physical economy.** Route workers and carried inputs/outputs through local
   stations and stockpiles, seed the intended storehouse reservoir, extend skills to maintained
   labors, make tools deliberately useful or explicitly input-only, complete recipes/materials,
   and add upgrade-gated escalating building costs. Keep tithe/offering reachability and
   research pacing in the same balance campaigns.
3. **Integrate the larger product models.** Wire purchases/effects/persistence and the daily
   leader choice for the 500-study catalog; implement 15-cat/three-house founding with slow
   breeding plus prosperity migration/unhoused departure; complete global/personal village
   ownership, discovery, and direct trade.
4. **Close world and presentation gaps.** Finish full tree/rock occupancy and staged wall growth,
   clear actual interior deposits/resources, expose traffic dirt roads, replace rail/shipping
   multipliers with built routes/vehicles, restore fishing, and implement the maintained
   9-patch/cursor UI skin plus Accounting Tent reachability.
5. **Prove the whole game.** Rerun unattended true-live and long proxy campaigns after every
   balance slice, then run the guided matrix in `docs/IMPLEMENTATION_AUDIT.md` across native,
   WASM, persistence/restart, multiple villages, and all target resolutions. The Forgejo quality
   workflow is committed; its first pushed run is still unverified. Transfer-weight optimization
   is optional and hosting itself is no longer open.

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
