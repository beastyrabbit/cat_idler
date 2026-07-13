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
| Sprite review tool | [`docs/sprite-review.html`](sprite-review.html) compares current art with three persisted/exportable proposals for all 22 current buildings plus Accounting Tent, Mill, Sawmill, and a global hall/market concept. | verified | Desktop/mobile browser runs: 26 rows, filters, favorites, reload persistence, path copying, JSON export, zero page/image errors |
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

### Original TypeScript-design reconciliation

The frozen implementation and original documents remain on `archive/web-game` at tag
`web-final`; maintained copies of the design rationale remain under `docs/`. They were
checked by requirement group so “historical” does not conceal a dropped current promise:

| Original design group | Current disposition |
| --- | --- |
| Needs, autonomous survival, aging, genetics, breeding, skills, specialization, leaders, elections, raids, death, and extinction recovery | Carried into deterministic `cat-sim`; balance and role-aware guided campaigns remain part of this audit |
| Dynamic colony grid plus a fixed 16×16 world map | Superseded by one flat, streamed, effectively infinite world containing multiple villages |
| Fog, expeditions, path wear, terrain travel cost, walls, and gates | Carried forward; shrine-return scouting and the exact current road model are open above |
| Click-to-feed/heal/assign/fight and a browser task queue | Superseded by typed management actions and the manual-to-officer loop; missing usable Bevy controls remain open above |
| Blessings, buildings, and the original ~18-node research tree | Carried forward; faucet reachability is in progress and the current direction expands the tree to about 500 nodes |
| Multiple colonies and inter-colony trade | Partially carried forward; authoritative multi-colony state and traders exist, while global/personal ownership, meeting, and direct trade remain open |
| External sprite-render service, DOM/Pixi rendering, isometric/elevation experiments, and newspaper UI | Superseded or explicitly dropped by the Rust/Bevy top-down direction |
| Seasonal events, achievements, accessories, sound/music, and a mobile app | Listed only as non-MVP future ideas in the original document; not current commitments unless promoted into `GAME_VISION.md` |
| Historical roadmap stretch items such as fishing, traveler interception, and elevation-aware zones | Not current commitments; bridges/transport that were later promoted are represented in the Rust-era specs |

The old result templates and browser test campaigns document measurements of the retired web
client. They are evidence archives, not Bevy acceptance criteria; the current matrix below
replaces them.

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

## Playtest evidence

### Unattended live cadence — 2026-07-13

Three seeded colonies ran for 48 game-hours at the production one-second cadence, followed
by an identical silent repeat. Every repeat matched exactly and every seed completed with
zero resets. Feature reachability was not uniform, so the run remains a failing pacing
baseline rather than a blanket pass:

| Seed | Population | Reached | Unreached by hour 48 |
| --- | --- | --- | --- |
| 7 | 5–12, 9 births, 3 deaths | fields, raids, elections, 4 tools, 6 offerings | research, item recipes, tithe |
| 555 | 5–8, 5 births, 5 deaths | fields, raids, elections, tithe, 2 offerings | research, item recipes, tools |
| 99 | 3–6, 2 births, 4 deaths | fields, raids, elections, tithe, research staffing/0.46 points | offerings, item recipes, tools |

Raw logs are local test artifacts at `/tmp/cat-playtest-live-{7,555,99}.log`. The officer,
production, and research fixes must rerun this campaign, while the manual-role design also
requires the guided ClientAction campaigns above; unattended behavior alone is not the
target player experience.

## Completion rule

An item moves to `verified` only when the behavior is reachable through the real player path,
its deterministic simulation tests pass, relevant server/client integration tests pass, and any
visual claim has been checked from the Bevy client's own framebuffer. Compiling alone is not
verification.
