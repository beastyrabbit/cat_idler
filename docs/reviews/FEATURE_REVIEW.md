# Feature Review — docs vs implementation

> Post-review status: findings have been dispositioned and implemented where required. See
> [`RESOLUTION.md`](RESOLUTION.md); this file preserves the pre-fix evidence.

Date: 2026-07-16
Scope: every concrete feature claim in `README.md`, `CLAUDE.md`, `docs/ARCHITECTURE.md`,
`docs/GAME_VISION.md`, `docs/IMPLEMENTATION_AUDIT.md`, `docs/FIX_LOG.md`, `docs/HANDOFF.md`,
`docs/migration/BOARD.md`, `docs/migration/specs/*`, `docs/assets/SELECTION.md`,
`docs/migration/WASM.md`, verified against the code in `crates/`. Claims were traced to code
directly; `docs/IMPLEMENTATION_AUDIT.md`'s own verification matrix was spot-checked, not trusted.

## Verdict

**High parity — no claimed system is missing.** Every load-bearing mechanism and numeric count
the docs assert was found implemented and wired. The one clear documentation defect is a **stale
recipe count** (`CLAUDE.md`/`README.md` say 104 physical recipes; the code and all other docs say
108, and the code test asserts 108). Two soft undercounts ("~40 phases", "26 fine biomes" wording)
and a handful of undocumented-but-real modules round out the findings. Fix list at the bottom.

## Verified claims (docs → code)

| Claim | Doc source | Verdict | Code evidence |
| --- | --- | --- | --- |
| ~40-phase `world_tick`, single entry point | CLAUDE.md, README, ARCHITECTURE | VERIFIED (undercount) | `crates/cat-sim/src/world_tick.rs` has **53** `fn phase_*` functions |
| Multi-colony `WorldState { colonies: Vec<..> }` + `found_colony` | CLAUDE.md, ARCHITECTURE | VERIFIED | `world_tick.rs:162`; `found_colony` `world_tick.rs:3532`, `found_colony_at` `:3573` |
| Seeded LCG + per-subsystem offsets (movement +1_000_003, life +2_000_003, raids +3_000_003) | CLAUDE.md | VERIFIED | `rng.rs:4-5` (1_664_525 / 1_013_904_223); offsets at `rng.rs:33/37/41` |
| `#![forbid(unsafe_code)]`, no `rand` in cat-sim | CLAUDE.md | VERIFIED | `cat-sim/src/lib.rs:7`; no `rand` in `cat-sim/Cargo.toml` |
| Determinism-twin tests | CLAUDE.md, ARCHITECTURE | VERIFIED | e.g. `found_colony_is_byte_identical...` `world_tick.rs:53386` |
| 7 officer roles with building gates, vacancy → manual | CLAUDE.md, IMPL_AUDIT, p12 spec | VERIFIED | `officers.rs:17` (`Steward, Accountant, Forester, Farmer, Captain, Loremaster, ClothLeader`); per-role `OfficerPrerequisite { building: .. }` `officers.rs:51-76` |
| Leader founding safety floor 6 hunt / 2 water / 1 scout at 15 cats, scaled | IMPL_AUDIT | VERIFIED | `leader_director.rs:89-91` baseline slot constants; `proportional_cap` applied `:654-656` |
| 19 typed skills | CLAUDE.md, IMPL_AUDIT | VERIFIED | `skills.rs` `enum SkillKind` — 19 variants |
| Old-age 240 h ordinary / 288 h leader-healer | CLAUDE.md testing contract | VERIFIED | `age.rs:7-8` |
| 15-adult / three 5-bed-Den founding, pregnancy bed reservations, 36 h probation | CLAUDE.md, p16 spec | VERIFIED (system level) | constants + named tests across `housing.rs` / `breeding.rs` / `life_sim.rs` |
| 487 research nodes (165 Building / 167 Recipe / 155 Upgrade), zero `FUTURE` | ARCHITECTURE, README, p18 spec | VERIFIED | `research_catalog.rs:19` `RESEARCH_NODE_COUNT = 487`; category split asserted `:1231-1234`; zero-FUTURE guards `:1487/1724` |
| 12 Crews concurrent-station studies + 13 completed-building services | CLAUDE.md, IMPL_AUDIT | VERIFIED | `research_catalog.rs:1717` truthful-services test; 13 service IDs listed `:1741-1766` |
| **108 physical recipes** | ARCHITECTURE, GAME_VISION, BOARD, specs, IMPL_AUDIT | VERIFIED (code = 108) | `station_recipes.rs:1072` `assert_eq!(ids.len(), 108)` — **CLAUDE.md/README say 104, stale** |
| 10 processor types | CLAUDE.md, IMPL_AUDIT | VERIFIED | station recipe registry groups into 10 processors; per-station recipe tests |
| 25 building compositions | CLAUDE.md, IMPL_AUDIT | VERIFIED | `cost_constants.rs:49` `BUILDING_COSTS: [(BuildingType, u32); 25]` |
| 52 `ClientAction` variants, all handled | IMPL_AUDIT | VERIFIED | protocol enum = 52 variants; 52 unique `ClientAction::` arms in `cat-sim/src/actions.rs` |
| HMAC sessions; refuse production boot without secret | CLAUDE.md | VERIFIED | `cat-server/src/identity.rs:6,9,23-31` |
| Rate limit 30 actions / 10 s | CLAUDE.md | VERIFIED | `rate_limit.rs` sliding window; `rate_limit_blocks_the_31st_action` test |
| `/health`, `/ready`, `/ws` routes | CLAUDE.md, ARCHITECTURE | VERIFIED | `cat-server/src/main.rs:159-163` |
| Fixed 1 s tick; save every 5 ticks + shutdown save | CLAUDE.md, ARCHITECTURE | VERIFIED | `main.rs:363` / `:62` / `:417` / `:452-482` |
| Blocking-pool sim/save, clone-and-release lock, Skip missed intervals | CLAUDE.md, IMPL_AUDIT | VERIFIED | `main.rs` tick loop |
| Test-acceleration controls disabled in release | CLAUDE.md, IMPL_AUDIT | VERIFIED | `main.rs:336-347` `#[cfg(not(debug_assertions))] .. false` |
| Persistence tables (world, colonies, cats, jobs, buildings, world_tiles, events, zones, elections, votes, raiders) | CLAUDE.md | VERIFIED | `persistence.rs:55-277`; resources as JSON blob; plus undocumented `shared_world_tiles` |
| Spoilage, genetics, zones, elections, combat, threat, warriors, trader wired into tick | CLAUDE.md module map | VERIFIED | modules exist and are called from `world_tick.rs` |
| 32 scalar resource kinds, protocol/icon bijection | IMPL_AUDIT | VERIFIED | `actions.rs:9960` asserts `ResourceKind::ALL.len() == 32` |
| 26 fine biomes affect travel | IMPL_AUDIT, p17 spec | VERIFIED (wording caveat) | `climate.rs:255` `BIOME_CLIMATE: [BiomeClimate; 26]`; coarse `BiomeType` enum has 11 variants (`biomes.rs:207`) |
| Client HUD (resources/census/events/trade/officers/village selection/inspectors), fog, minimap, crop stages, zone overlays | ARCHITECTURE, IMPL_AUDIT | VERIFIED | `cat-client/src/lib.rs` — all surfaces present |
| Client-side terrain from `world_seed` via `generate_terrain_chunk` | CLAUDE.md, ARCHITECTURE | VERIFIED | `cat-client/src/lib.rs:45` |
| Full-page 487-study ledger: filter/search/pan/zoom + signed purchase | CLAUDE.md, README, p18 spec | VERIFIED | `research_ui.rs`; dispatches `ResearchNode`/`UnlockNode` `:1364/1371` |
| Transport rendering (track/rail/dock/wagon/vessel, label-free) | IMPL_AUDIT | VERIFIED | `cat-client/src/lib.rs:1247/10823/13114` |
| Adventure UI skin + custom cursors | IMPL_AUDIT, p18 spec | VERIFIED | cursor/skin assets loaded `lib.rs:1897/1913` |
| Reconnect with capped backoff | IMPL_AUDIT | VERIFIED | `lib.rs:78/80/485` |
| `cargo dev` refuses busy port | CLAUDE.md | VERIFIED (not deep-read) | `crates/cat-dev` port-in-use guard |

Deep numeric literals inside large, tested subsystems (trader 100 kg wagon / 20 kg sale cap,
18 h gestation, 30 h/12 h migration cadence) were confirmed at the system level — the systems
exist and have named tests — without re-deriving every constant.

## Missing or partial features

**None missing.** Caveats:

1. **Phase count understated.** Docs say "~40 ordered phases"; the code has 53 `fn phase_*`
   functions. Cosmetic, but the headline number is wrong. Same for `README.md:200` "~40 modules"
   — `cat-sim/src` has ~60.
2. **"26 fine biomes" is a climate classification, not an enum.** The `BiomeType` enum has 11
   coarse variants; the 26 comes from the `BIOME_CLIMATE` table (`climate.rs:255`). The claim
   holds behaviorally, but the wording can mislead someone grepping for a 26-variant enum.

## Undocumented features (code present, docs silent or understated)

- `shared_world_tiles` persistence table (`cat-server/src/persistence.rs:62`) — the shared-world
  tile authority's table, absent from CLAUDE.md's listed table set.
- cat-sim modules missing from CLAUDE.md's module map: `labor_pressure`, `productivity`,
  `village_sites`, `village_trade_routes`, `transport`, `station_recipes`, `research_catalog`,
  `migration`, `farming`, `processing`. These are real subsystems (labor telemetry, transport
  routing, village trade), not helpers.
- The 487-node catalog is procedurally expanded from `research_catalog_legacy.json` +
  `research_catalog_tracks.json` (families × stages), not a flat list — the generation mechanism
  is not described anywhere in docs.

## Stale / contradictory documentation (fix list)

1. **Recipe count 104 → 108** (the real defect; code asserts 108 at `station_recipes.rs:1072`):
   - `CLAUDE.md:146` "ten processor types with 104 physical recipes"
   - `CLAUDE.md:190` "all 104 recipes have physical descriptors"
   - `README.md:114` "104 physical recipes"
   - `README.md:246` "all 104 physical recipes"
2. **"~40 phases" / "~40 modules"** in README/CLAUDE.md → actual 53 phases, ~60 modules.
3. **Module map + persistence table list in CLAUDE.md** → add the modules and
   `shared_world_tiles` listed above.
4. Superseded docs (`plan.md`, `ROADMAP.md`, `LEADER_AI_DESIGN.md`, `TERRAIN_DESIGN.md`,
   `ENGINE_PLATFORM.md`, `ENGINE_FRONTEND.md`, `TASKS.md`, `TESTING.md`, `UI_CONCEPTS.md`,
   `ENGINE_MCP_EVALUATION.md`) all carry correct historical banners — **no action needed**.
