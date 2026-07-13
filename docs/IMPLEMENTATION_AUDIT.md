# Idle Cat Forest implementation audit and fix tracker

Last updated: 2026-07-13

This is the working ledger for the post-cutover design audit and exhaustive playtest pass.
It records only the current Rust/Bevy game's promises and gaps. Documents explicitly marked
historical or superseded are reference material, not an implementation backlog.

Status key: `open`, `in progress`, `verified`, `deferred`.

## Active fixes

| Area | Finding | Status | Verification required |
| --- | --- | --- | --- |
| Shrine economy | Organic tithe and offering gates use unreachable capacity ratios. | in progress | Multi-seed 96h faucet events, resource reserves, determinism twin |
| Tools | Woodworking is always staffed after both input benches and tools have no protected construction reserve. | in progress | Tool throughput, field commissioning, long-step reserve test |
| Research | First autonomous node arrives around game-hour 120 at live cadence. | in progress | Healthy multi-seed first-node window and crisis non-staffing |
| Survival | Harsh 5-minute proxy runs expose cadence-sensitive collapses on several seeds. | in progress | Five seeds × 200h plus live-cadence comparison and determinism |
| Server security | Several mutations and test-time controls bypass WebSocket identity checks. | in progress | Every action classified; invalid/unsigned mutations rejected; test controls gated |
| Web build | The build script bakes localhost despite documenting same-origin WebSocket use. | in progress | Release build configuration test and browser smoke |
| Client reliability | Failed actions are silent and a closed WebSocket does not reconnect. | in progress | Unit tests plus disconnect/reconnect live smoke |
| Infinite map | Main terrain/fog are spawned once around the founding anchor. | in progress | Pan across chunk boundaries and framebuffer capture |
| Officers/manual play | Officers add capacity but vacant roles do not make categories manual. | open | Role-by-role automated/manual scenarios and framebuffer playtest |
| Manual controls | Most protocol actions, buildings, workers, research, military, and hauling lack usable client paths. | open | Script every action through server and exercise all visible controls |
| Multi-village | New colonies can be founded but actions/rendering remain pinned to colony zero. | open | Found, select, join, mutate, persist, reconnect, and render two villages |
| Spatial invariants | Stockpiles can overlap land/buildings/other piles; roads are eventually repaired rather than guaranteed at placement. | open | Per-tick spatial property campaign across seeds |
| Production breadth | Farming choices/chains and several material/item recipes promised by current specs are missing or unreachable. | open | Build/staff/produce every building and recipe from player actions |
| Client visibility | Ore/metal and generalized production/skills are not fully represented in protocol/HUD/inspectors. | open | Snapshot round trips and mature-colony UI inspection |
| Visual truth | Several dedicated building assets are loaded but the runtime collapses stations to generic props. | open | Native captures at 1024×768, 1280×800, and 1920×1080 |
| CI/hosting | No automated Forgejo build/test workflow or production hosting definition exists. | open | Clean CI run; deploy configuration documented without embedded secrets |

The existing Paws & Whiskers cat and raider sheets are accepted project assets.

## Current design-document traceability

| Document | Implementation status | Follow-up |
| --- | --- | --- |
| `docs/GAME_VISION.md` | partial | Finish manual-to-officer automation, usable multi-village, complete visible production controls |
| `docs/ARCHITECTURE.md` | structurally useful but stale | Rewrite post-fix; remove cutover-era contradictions |
| `docs/HANDOFF.md` | current | Replace NEXT STEPS with verified outcomes from this tracker |
| `docs/migration/BOARD.md` | core migration complete, expansion rows overclaim | Reopen partial P12–P19 slices and close them only after feature campaigns |
| `p12-idle-cat-forest.md` | partial | Officers, manual work, farming/production exposure, shrine reachability |
| `p14-spatial-placement.md` | partial | Stockpile collision/claim rules and hard connectivity invariants |
| `p15-playtest-feedback.md` | partial | Dynamic infinite map, multi-village UX, richer inspectors/actions |
| `p16-village-blueprint.md` | partial | Player-facing gather controls and reachable production chain |
| `p17-biome-generator.md` | simulation-heavy, product partial | Expose resources/logistics and render arbitrary chunks |
| `p18-visual-polish.md` | partial/diverged | Verify actual runtime visuals and either use or remove misleading asset claims |
| `p19-items-materials-trade.md` | partial | Complete material/recipe breadth and expose all chains to play |
| `docs/migration/WASM.md` | development path works | Production URL, reconnect, hosting, transfer/performance campaign |

The following are explicitly historical/superseded and do not create open features:
`ENGINE_FRONTEND.md`, `ENGINE_PLATFORM.md`, `LEADER_AI_DESIGN.md`, `ROADMAP.md`, `TASKS.md`,
`TERRAIN_DESIGN.md`, `TESTING.md`, `UI_CONCEPTS.md`, `plan.md`, and the old browser campaign
documents. `ENGINE_MCP_EVALUATION.md` should receive the same historical banner.

## Full playtest matrix

- Survival, needs, breeding, aging, death, extinction recovery, and determinism.
- Every leader/manual job and every officer both vacant and filled.
- Every constructible building: plan, build, staff, produce, inspect, and persist.
- Research through both cat research points and shrine blessings, including prerequisites.
- Tithe, offerings, rituals, raids, defense, training, elections, votes, and vote-kick.
- Stockpiles, gather spots, hauling, collision rules, roads, walls, and gate access.
- Every biome/deposit, extraction chain, item recipe, quality, trader buy/sell/restock.
- Found/join/select multiple villages; route actions and persistence to the chosen colony.
- Native UI states at multiple resolutions plus distant-camera and dense-village captures.
- WASM boot, connect, action feedback, reconnect, resize, caching, and transfer budget.
- Invalid authentication, malformed actions, rate limiting, restart equality, and test-control denial.

## Completion rule

An item moves to `verified` only when the behavior is reachable through the real player path,
its deterministic simulation tests pass, relevant server/client integration tests pass, and any
visual claim has been checked from the Bevy client's own framebuffer. Compiling alone is not
verification.
