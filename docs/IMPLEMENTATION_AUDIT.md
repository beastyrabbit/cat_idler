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
| Server security | Socket-bound identity, exhaustive mutation authentication, selected-colony routing, release-disabled test controls. | verified | 15 server, 692 sim, and 61 client tests; strict Clippy |
| Web build | Release bundle uses same-origin WS; local serve retains explicit port 8787. | verified | Optimized Trunk build and combined-host live probes |
| Client reliability | Failed actions are visible and closed/error sockets reconnect with capped backoff. | verified | 62 client tests and connected framebuffer smoke |
| Infinite map | Terrain/fog stream as a bounded camera-centered chunk cache. | verified | Normal and 80-tile-distant framebuffers plus loaded-chunk assertion |
| Officers/manual play | Officers add capacity but vacant roles do not make categories manual. | open | Role-by-role automated/manual scenarios and framebuffer playtest |
| Manual controls | Most protocol actions, buildings, workers, research, military, and hauling lack usable client paths. | open | Script every action through server and exercise all visible controls |
| Multi-village | Socket-selected routing and a persistent client village selector survive reorder/reconnect/founding and missing-village fallback. | verified | 72 client tests, authenticated server routing test, and opposite-target 1024×768/1920×1080 framebuffers |
| Spatial invariants | Stockpiles can overlap land/buildings/other piles; roads are eventually repaired rather than guaranteed at placement. | open | Per-tick spatial property campaign across seeds |
| Production breadth | Farming choices/chains and several material/item recipes promised by current specs are missing or unreachable. | open | Build/staff/produce every building and recipe from player actions |
| Client visibility | Ore/metal and generalized production/skills are not fully represented in protocol/HUD/inspectors. | open | Snapshot round trips and mature-colony UI inspection |
| Visual truth | Current buildings are distinct and inspected, but player feedback supersedes the roofed workshop facades and persistent map plaques. Workshops must become open-top/cutaway and self-explanatory without labels. | in progress | Sprite-candidate approval page, then label-free normal/dense native framebuffers at 1024×768, 1280×800, and 1920×1080 |
| CI/hosting | Forgejo quality workflow and combined non-root server/WASM image are committed. | in progress | First pushed CI run remains; hosting live probes and deployment docs are verified |

The existing Paws & Whiskers cat and raider sheets are accepted project assets.

## Player playtest feedback — 2026-07-13

These are current product requirements discovered by playing the native client. They remain
open until the real player path and, where applicable, Bevy framebuffers prove them.

| Feedback | Required behavior | Status | Verification required |
| --- | --- | --- | --- |
| Fog of war and scouting | A new village reveals only its viable core plus roughly two tiles. Scouts choose a resource target or general exploration; their discoveries remain provisional until they return and touch the shrine. Nearby founding wood should be a very short scout trip. | open | Fresh-village and resource/general scout campaigns; outbound/return/shrine boundary tests; before/after framebuffers |
| Open workshops | Houses may retain roofs, but workshops use readable open-top/cutaway sprites in the top-down DF-Steam style. | open | Candidate approval page followed by dense-village native framebuffers |
| Building labels | Remove persistent map-name plaques; a building's purpose must read from its sprite and inspector/hover affordance. | open | Label-free 1024×768, 1280×800, and 1920×1080 captures |
| Clear village interior | Claimed village interiors contain no natural trees, stone/deposits, or farm fields. Player-designated farms and resource work sites belong outside the settlement interior. | open | Founding/expansion spatial property tests across biomes and native captures |
| Global and personal villages | One large global village is available to everyone. Each player may also found a personal village at a different deterministic world location; all remain on the same world map and can eventually meet/trade. | open | Ownership/access/routing/persistence tests, distant placement, two-player join/found/trade campaign |
| Sprite review tool | Provide a responsive standalone HTML asset-review page with proposed alternatives from tracked public sprites for the altar and every building, including source path and selection controls. | in progress | Browser visual inspection at desktop/mobile widths; user approval drives final mappings |
| Research tree scale and UX | Research opens as a full-page Cities-Skylines-style dependency tree. Target about 500 data-driven nodes: at least one third building unlocks and one third recipes/resources, with the remainder covering movement, labor, capacity, defense, and other upgrades. Players may buy any affordable nodes; the leader may autonomously select at most one node per real-life day. | open | Schema/content validation, dependency/topology tests, daily leader boundary, full-page Bevy/WASM framebuffer and interaction campaign |
| Founding population and housing | Start with three early houses and 15 cats; each house holds five. Breeding is slow and only happens with free housing. Prosperity attracts migrants, potentially above capacity, but unhoused arrivals leave if housing is not built in time. | open | Exact founding snapshot, housing boundary, breeding cadence, prosperity migration and departure campaigns across seeds |
| Roads | Enforce the written P16 road model: authored stone roads, traffic-formed dirt roads, exact movement speeds/surface restrictions, connected shrine/gate/exterior routes, and the single south founding gate. | in progress | Exact speed boundaries, traffic threshold, no stone auto-dirt, connectivity properties, and framebuffer inspection |

## Current design-document traceability

| Document | Implementation status | Follow-up |
| --- | --- | --- |
| `docs/GAME_VISION.md` | partial | Finish manual-to-officer automation, usable multi-village, complete visible production controls |
| `docs/ARCHITECTURE.md` | corrected for the post-cutover workspace | Keep current as remaining gaps close |
| `docs/HANDOFF.md` | current | Replace NEXT STEPS with verified outcomes from this tracker |
| `docs/migration/BOARD.md` | core migration complete, expansion rows overclaim | Reopen partial P12–P19 slices and close them only after feature campaigns |
| `p12-idle-cat-forest.md` | partial | Officers, manual work, farming/production exposure, shrine reachability |
| `p14-spatial-placement.md` | partial | Stockpile collision/claim rules and hard connectivity invariants |
| `p15-playtest-feedback.md` | partial | Dynamic infinite map is fixed; multi-village UX and richer actions remain |
| `p16-village-blueprint.md` | partial | Player-facing gather controls and reachable production chain |
| `p17-biome-generator.md` | simulation-heavy, product partial | Expose resources/logistics and render arbitrary chunks |
| `p18-visual-polish.md` | partial; earlier facade pass is superseded by player feedback | Replace roofed workshops and labels with approved open-top visual stations; retain the verified inspectors and mature-village fit |
| `p19-items-materials-trade.md` | partial | Complete material/recipe breadth and expose all chains to play |
| `docs/migration/WASM.md` | development and production packaging work | Optional transfer/performance campaign remains |

The following are explicitly historical/superseded and do not create open features:
`ENGINE_FRONTEND.md`, `ENGINE_PLATFORM.md`, `LEADER_AI_DESIGN.md`, `ROADMAP.md`, `TASKS.md`,
`TERRAIN_DESIGN.md`, `TESTING.md`, `UI_CONCEPTS.md`, `plan.md`, and the old browser campaign
documents. `ENGINE_MCP_EVALUATION.md` carries the same historical banner.

## Full playtest matrix

- Multi-seed unattended runs plus longitudinal player-guided runs: fully manual survival,
  staged officer handoff, productive expansion, and deliberately poor decisions with real
  consequences. Guided runs dispatch real `ClientAction`s from observed colony state rather
  than mutating fixtures behind the action layer.
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
