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

## State of the game (verified 2026-07-15)

The **migration and P11 cutover are complete**; the maintained game is still pre-release and
several P12–P19 product promises remain partial. Do not turn a phase label or compiling crate
into a completion claim. `docs/IMPLEMENTATION_AUDIT.md` is the detailed evidence ledger.

Verified foundations include:

- the deterministic sim and authoritative SQLite/WebSocket server, with simulation and disk work
  on Tokio's blocking pool, startup-cached snapshots, responsive health/initial connections, and
  selected-colony action/snapshot routing;
- the native and browser Bevy clients, bounded world streaming, reconnect/action feedback, and a
  same-origin non-root production image with `/health`, `/ready`, compression, and Origin checks;
- atomic placement plus conserved physical scaffold inputs: exact type-local costs are pinned from
  visible piles, a living builder carries Lumber/Planks and Blocks through persisted transit/input
  ledgers, and progress cannot start before full delivery. Death, source loss, reassignment,
  removal, and restart conserve the bill; old funded scaffolds remain compatible. Its label-free
  inspector truth is accepted in a live 2048×1152 own-framebuffer, and the four touched crates pass
  their complete test and strict-Clippy gates. Label-free roofed homes and explicit open
  compositions for all 25 current protocol building variants, including an integrated legal
  Accounting Tent that retains all three founding Dens; exterior crop/logging production with
  distinct roofless processing stations, persisted outside-wall agriculture, and an accepted
  before/during/after staged-wall cutover. Mill, Sawmill, Workshop, and Smelter now have complete
  physical finite-store→station-local input→on-site work→station-local output→finite-store routes,
  editable queues, conserved transit, and truthful inspectors. Steward-managed local reserves feed
  those processors through durable physical balancing trips. The selected Mill's accepted
  1920×1080 client framebuffer shows Grain 4.0 locally, Flour 2.0 locally, Flour 1.5 outbound,
  the repeating recipe, half progress, and its worker hauling output;
- exact founding fog plus signed resource/general scouting whose provisional knowledge commits
  only on physical shrine return, including restart-safe in-flight notebooks and responsive
  controls at 1024×768 through 1920×1080. Optimized browser and signed fresh-native campaigns both
  prove permanent growth from the exact 289-tile founding baseline;
- a truthful 487-node (“about 500”) research runtime and full-page
  searchable/filterable/pannable client ledger. Thirteen generated capacity studies with no
  physical container are deliberately absent, and old ownership receives an exact one-time refund.
  Supported studies can be purchased with research points and persist; unsupported generated
  recipe/resource promises remain visible as disabled `FUTURE` cards. The living Leader may complete at
  most one affordable full-catalog node per rolling real-life day while research labor/building
  automation and rituals remain Loremaster-owned. Thirteen maintained runtime recipe IDs now have one
  data-owned station descriptor, canonical input/output resource domains, deterministic default
  queues, and exact catalog ownership metadata: ten are research-gated and three founding-bench
  baselines are immediately available. Four preparation studies gate the physical
  Mill/Sawmill/Workshop/Smelter queues; Textiles gates physical Tannery and Clothier,
  while Toolmaking, Weaponsmithing, and Armorsmithing independently gate the Smithy's selected
  tool, weapon, and armor recipes. All thirteen recipes execute through physical station-local
  queues. Mill grinding and baking are separate selected operations. Smithy consumes
  two Metal for one whole selected Tool, Weapon, or Armor after 900 game-seconds, and aggregate credit
  waits for outbound delivery; its old aggregate forge timers remain bit-frozen save data.
  Carpentry, Stonecraft, and Toolmaking studies gate later recipes rather than those three founding
  baselines; rules-v0 metadata
  remains grandfathered. The other 91 generated recipe IDs and
  all 64 generated resource IDs are visibly future-only and non-purchasable until a physical
  consumer exists; see `docs/RECIPE_RESOURCE_MATRIX.md`. Sawmill→Gather Logs is the
  sole catalog job entitlement; founding water/scouting, manual research, and Barracks training
  remain available without false job payloads. Research Hut is explicitly founding-placeable;
  Wood Cutter, Stone Prep, and Woodworking are data-declared placement-available without Basic
  Tools; `milling` is the sole Mill placement unlock; and generated Mill Foundations is durability
  only. The daily Leader choice shares one restart/reset/election-safe
  colony clock and never limits the player's direct research purchases;
- specialist manual-to-officer ownership across Steward, Accountant, Forester, Farmer, Captain,
  Loremaster, and Cloth Leader: beyond the founding Leader's hunt/water/scout safety floor, a
  vacant office leaves its category manual. Appointment requires the matching researched and
  completed role station, and signed client controls cover the basic manual work paths. Manual
  raid clicks deal exactly one hit. Reachable tithes and physical two-stage material offerings
  (stockpile pickup and shrine delivery, then ritual) produce blessings without early credit;
  tools improve construction, crafting, quarrying, and hauling;
  repeated buildings have escalating type-local costs, and the Accounting Tent is live in the
  snapshot and client;
- automatic elections expose an authoritative between-term schedule in snapshots and the
  governance panel: term start, next election boundary, term length, and a server-derived
  countdown remain visible even while no election is open;
- finite item units have stable identities, physical weight, durability, work-driven wear, and
  persistent broken state. A player can repair a damaged unit at its appropriate completed,
  staffed workshop by spending one visible matching material; the live durability research
  multiplier affects the repair result. Traders respect a 20 kg item-load limit, and the Goods
  panel exposes weight, condition, damaged/broken counts, and repair controls. This closes the
  condition loop, not the still-open material and recipe breadth;
- one canonical communal village plus one owner-only personal village per stable signed identity,
  with deterministic distant viable sites, secure selected-village routing, restart-safe
  ownership and selection, explicit returned-scout discovery, summary-only foreign contact,
  configurable capped atomic direct barter, transactional world saves, and collision-safe
  colony-local SQLite child identities; native bearer/selection rewrites are atomic;
- the complete founding lifecycle: exactly 15 adult cats in three five-bed Dens, slow
  reserved-bed breeding, prosperity migration with a 36-game-hour unhoused probation,
  240/288-game-hour old-age thresholds, atomic extinction recovery, and physical emergency
  water fetching. Prosperity migrants now materialize at one deterministic dry exterior tile,
  visibly follow the authoritative south-gate path before joining the census or simulation, and
  reuse that persisted origin when an unhoused probation expires. A blocked gate pauses the
  journey; housing begins only after entry, and departure/removal completes only after exit;
- physical exterior farming: a living assigned cat walks through the retained village gate,
  plants, tends, harvests bounded baskets into a local handoff, and returns crop cargo to finite
  storage before aggregate credit. Farmer automation and signed manual assignment share this
  route; vacancy, blocked storage, death, restart, skill gain, and moved-gate reachability are
  covered without free crop mutation.
- physical accounting rounds: a tent worker visits reachable stockpiles in deterministic order,
  counts each pile for five game-seconds, and returns to the tent. Per-pile and aggregate reports
  remain stale until that contact, blocked piles stay stale for a later round, and in-progress
  routes survive SQLite restart. The HUD, stockpile inspector, and Accounting Tent inspector expose
  stale estimates and current round progress. Initial, tick-broadcast, post-action, and reconnect
  WebSocket projections expose only those reports, zero uncounted pile contents, and no equality
  oracle; the trusted server cache remains exact. Blessings stay exact because they are divine
  currency rather than stockpiled goods. No tent, a vacant Accountant office, or an unassigned
  completed tent receives a periodic authoritative recount.

The founding/housing integration replaced the archived five-cat loop and passed its full
simulation, protocol, server, client, persistence, guided-action, determinism, and framebuffer
gate. The exact evidence is recorded in `docs/IMPLEMENTATION_AUDIT.md`.

Dev tooling (`be7cdee`) includes `CAT_BRP=1` for Bevy Remote Protocol inspection and a headless
playtest harness:
`SEED=... HOURS=48 CADENCE_MS=1000 cargo run --release -p cat-sim --example playtest`.
Set `COMMUNAL=1` for the larger Grand Commons. Fresh passive defaults assert only
officer-independent behavior; use `EXPECT_FEATURES=...` with the signed guided campaigns when
testing a player-established economy. The harness reports fog growth/shrine deliveries separately
so a correct scout route cannot conceal a missing founding dispatch.

## NEXT STEPS (maintained backlog, in rough priority order)

1. **Finish the canonical physical economy.** P19 now owns the resource taxonomy and production
   table; P12 owns manual/officer logistics and P16 owns the fixed founding benches. Preserve the
   stable `materials`/`refined` wire and save IDs as Supplies/Crafted Supplies, every existing
   `BuildingType`, and the verified open-top station direction. The P19.C1 source boundary is now
   verified: defaulted raw Stone never aliases legacy Supplies, Stone Prep consumes Stone, quarry
   Stone plus renewable rubble/Supplies and mountain Ore are finite carried loads, while hunts carry
   three Food loads followed by distinct Hide and Bone loads. Bone is defaulted independently across
   save, wire, storage, trade, HUD, and private Accountant projections. Persistence, interruption/full-
   storage conservation, passive play, signed guidance, and the exact 1024×768 client-owned
   `/tmp/raw-stone-bone-final.png` framebuffer cover that slice. It visibly projects truthful
   counted Stone `~12/100` and Bone `~3/100` without clipping.
   Bone item variants and downstream recipes remain future breadth, not the raw source itself.
   Final gates pass 1,169 simulation, 43 protocol, 82 server, and 134 client tests plus strict
   Clippy for all four touched crates.
   All six now own stable recipe descriptors, canonical resource sets, deterministic default queues,
   selected-recipe research/block metadata, and the generic persisted queue surface. Wood Cutter is
   the first C2 conversion: its Logs→Planks queue owns finite local input/output, physical transit,
   selected ordered/repeatable/pausable work, and final-delivery credit. Stone Prep is the second:
   five Stone move through local input, one 600-second selected batch creates one local Block, and
   one outbound haul credits finite storage. Woodworking is the third: two Planks and two Blocks
   arrive sequentially, one Craft worker consumes both atomically in a selected 600-second batch,
   and one whole scalar Tool reaches finite storage only through an outbound haul. Tannery is the
   fourth conversion: five physically hunted Hide move through inbound and local storage, one
   Textile worker advances the selected 600-second batch, and one Leather moves through local
   output and an outbound carrier before aggregate credit. Its legacy parallel leather/clothing
   timer is frozen. Clothier is the fifth conversion: physical forage leaves Fibre in the
   gatherer's paws, one Textile worker carries five Fibre through the selected 600-second
   `fibre_to_cloth` batch, and one Cloth reaches finite storage before aggregate credit. Its local
   store remains fixed at 10 and its legacy parallel clothing timer is frozen. The hidden
   `wood_craft_progress` timer is frozen and rules-v3 persistence preserves player-authored queue
   intent without seeding; rules-v4 does the same for Tannery, version-only rules-v5 for Clothier,
   and version-only rules-v6 for Smithy. Smithy is the sixth conversion: one Metalwork worker
   carries two Metal into one selected ordered/repeatable/pausable 900-second weapon or armor
   batch, then carries the one whole selected output to storage before aggregate credit. Its old
   aggregate forge timers are frozen. P19.C3 is now complete: Woodworking and Smithy create one
   stable finite Tool/Weapon/Armor identity in local output, the same ID travels through carrier,
   storage, equipment, work/combat wear, repair, and exact trader sale, and the stable scalar fields
   are compatibility projections rather than a second inventory. Signed equip/unequip, physical
   Captain issue, capacity-safe death/departure/reset spill recovery, rules-v1/SQLite migration,
   and Accountant-confidential wire/UI projection are verified by passive and guided campaigns.
   Preserve the now-verified rule that the three P16 founding benches need no placement study and
   that future studies gate recipes instead. Preserve the verified construction contract: player and autonomous
   scaffolds reserve finite Lumber/Planks plus Blocks at exact visible sources, builders carry those
   goods through persisted transit/input ledgers, and timed work begins only after on-site delivery.
   Keep paid-scaffold recovery and exact type-local escalation intact.

   The finite founding storehouse and complete physical
   logs→Wood Cutter→planks, Stone→Stone Prep→Blocks, logs→Sawmill→lumber, grain→Mill→flour+food,
   Materials→Workshop→Refined, Ore→Smelter→Metal, Hide→Tannery→Leather,
   Fibre→Clothier→Cloth, and Metal→Smithy→selected Weapon/Armor routes are verified, including station-local
   ledgers, transit reservation, delivery-before-credit, death conservation, restart persistence,
   real editable queues, and live inspectors.
   Exterior farming now follows the same physical truth through plot work, bounded baskets, local
   handoff, and storage delivery. Steward-managed exact-resource piles now feed all ten physical
   processors through conserved balancing trips without consuming the player's designation budget.
   Fresh player and Leader scaffolds use the same conserved source→transit→scaffold contract and
   do not begin timed work until the pinned bill arrives.
   Apply that contract to the remaining sources and workshops, then complete recipe/material
   breadth. Physical Accountant rounds now keep reports truthful one visited pile at a time,
   vacancy never performs a hidden recount, and the socket projection cannot bypass those books.
   Keep future offer/block metadata from copying hidden exact totals or recreating an equality
   oracle. All 19
   maintained labor skills have truthful gain sources, bounded effects, persistence,
   and inspector visibility. Signed per-cat preferences and all ten physical processors' real
   editable queues are live; extend the generic queue control as additional physical recipes land.
   Preserve the verified tool productivity, type-local escalating costs, shrine faucets, and
   research pacing in balance campaigns. The physical offering decision threshold is 20 Supplies
   (ten carried, ten retained), and essential Field demand counts completed physical Fields rather
   than an unpaid queued promise. The signed farm→Mill campaign explicitly paves a second Workshop
   reservation before appointing the Steward. Its construction-road planning uses a deterministic
   topology-signature cache; do not cache mutable claims/future sites or remove the exact route bar.

   The NPC visiting-trader correction is implemented and focused-verified: each deterministic
   visit owns a reachable exterior, ordinary A* route through the retained gate to shrine contact,
   finite resource manifest, finite purse and wagon capacity, exact purchased item-unit cargo,
   physical return route, and restart-persistent phase/deadline/stock. Signed guided actions prove
   exact depletion and sold-out denial, and a seed-41 live-cadence 60-hour passive twin observed one
   arrival, shrine trading window, and departure with identical replay and no deaths or resets. Exact
   physical transition times are invariant across one-second, minute, hourly, and coarse tick
   partitions; a route that was unavailable at the scheduled boundary never grants backdated travel.
   The trade panel opens only on shrine contact, pages every finite craft offer six at a time within
   1024×768, and derives storage-full guidance only from the Accountant's reported books.
   The accepted client-owned 1024×768 logical framebuffer `/tmp/trader-physical-1024.png` shows
   the merchant at the shrine, page 2/2, finite quantities, Food sold out, bounded controls, and
   report-derived storage guidance. A broad-gate-found blocked-reopen timestamp regression is also
   fixed with persisted route-block observation state; 1,153 simulation and 80 server tests plus
   strict touched-crate Clippy pass. The physical finite visiting-trader slice is closed.
2. **Deepen the shared world.** The durable 30-cat/six-Den/19×19 communal hub is mechanically
   larger than exact 15-cat/three-Den personal villages. Mutable terrain, ecology, authored and
   traffic roads, wear, depletion/regrowth, and Fish populations now have one deterministic
   world-scoped authority; colony maps are compatibility/view caches, while committed/provisional
   fog and learned contact remain private. Legacy saves migrate conservatively and SQLite persists
   the whole shared ledger transactionally. Next, turn summary contact and atomic scalar barter
   into physical meeting, item trade, and routes. Deterministic
   knowledge-blind scout search is verified; preserve its physical-observation and shrine-return
   contracts as shared-world depth grows. The physical finite NPC trader is a local proof of the
   route, inventory, and restart rules, but player villages still need real inter-village caravans,
   meeting cats, finite item exchange, and shared-world routes instead of atomic scalar barter.
3. **Close remaining world-system gaps.** Baseline Leader hunt/water/scout survival, optimized
   browser and signed-native shrine-return fog, staged closed-perimeter growth, integrated
   Accounting Tent, and persisted outside-wall agriculture are verified. Fine-biome factors now
   drive A* and physical per-tile travel through one cached truth, with road/obstacle composition
   and tick-partition determinism. Finite fresh-Fish habitats deplete only on physical catches,
   replenish at 0.5 fish per game-hour up to 24, survive repaint/restart, and pass guided plus
   unattended twins. Rail and Shipping studies are now truthful blueprint entitlements:
   Shipping cannot make a walking cat enter water and Rail cannot accelerate a long walk merely
   from ownership. Preserve those guardrails while adding built tracks, vehicles, docks, boarding,
   vessels, and staffed routes, making fine biomes own their promised physical resource ecology, and
   extending physical logistics to the remaining workshops. Item durability now has a real
   wear/break/repair consumer. Food Storage, Water Bowl, and Smithy capacity studies are now
   target-correct across clamp, physical routing, snapshot, trade, and persistence. Keep the other
   22 generated `*_stores` studies and the fixed 10-unit Mill/Sawmill/Wood Cutter/Stone Prep/Woodworking/Workshop/Smelter local stores
   explicitly open until each has a real physical domain. Then turn the remaining registry-only
   research payloads (91 generated-only recipes and 64 resources) into observable behavior without
   advertising IDs that have no runtime object. Twelve Crews studies now add real independently
   owned worker stations to the existing physical/research labor domains; the other thirteen are
   visibly future and non-purchasable until those buildings gain a safe concurrent labor state
   machine. Preserve the
   catalog-derived Sawmill logging entitlement and the verified authority split: the Leader owns
   the daily strategic study choice, while
   research labor/building automation and rituals remain Loremaster-owned.
   The maintained Forester contract is complete: a manual or appointed-Forester physical route
   consumes a persisted stump/root stock into a visible sapling, and unobstructed growth restores
   the same deterministic tree after 24 game-hours. Preserve its mapped-terrain, occupancy,
   vacancy, persistence, and exact-site guardrails while extending the remaining ecology.
   Authored stone roads now use a durable physical job: one exact visible Material is reserved,
   carried, and worked by a living Build worker per ordered tile before paving and debit. Signed
   actions and Steward automation share that route; death/reassignment, source/spill recovery,
   map-cell reservation, tool wear, cadence, and SQLite restart are verified. Preserve the exact
   mapped-terrain, shrine-network attachment, disjoint stone/dirt surface, and movement rules.
4. **Keep the verified semantic HUD complete while proving the whole game.** All 25 maintained
   resource mappings now use unique, truthful tracked Board Game/Fish glyphs under the semantic
   icon directory; Stone has a block glyph and Bone a fish-skeleton glyph. The exact 1024×768
   client-owned frame `/tmp/semantic-hud-25-final.png` shows every row without clipping. Preserve
   the exhaustive 25-entry mapping/path tests as new resources land. All twenty-five physical
   `CarryingKind` values also reuse their exact tracked HUD identities above moving cats;
   the inspected exact 1024×768 frame `/tmp/semantic-cargo-icons-1024.png` shows ten distinct
   simultaneous loads and no colored-square fallback. Preserve that exhaustive cargo mapping and
   continue whole-game visual/playtesting passes across supported bounds.
   Rerun unattended true-live and long proxy campaigns after every
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
