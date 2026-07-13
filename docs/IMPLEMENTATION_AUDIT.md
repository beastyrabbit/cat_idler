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
| Survival | The production integration restored a zero-reset five-seed × 200h proxy campaign after exterior-field expansion initially caused two distinct collapse paths. Further balance changes still require live-cadence comparison. | in progress | Repeat five seeds × 200h and live-cadence campaigns after officer/economy/housing changes; determinism twin |
| Server responsiveness | A normal freshly booted server can hold its world work long enough that `/health` times out and the client receives no initial snapshot for over a minute; the art-only tick bypass answers in about 5 ms. | in progress | Isolated before/after world-tick timing, health latency while a tick is slow, prompt initial WS snapshot, production tick cadence, restart equality |
| Server security | Socket-bound identity, exhaustive mutation authentication, selected-colony routing, release-disabled test controls. | verified | 15 server, 692 sim, and 61 client tests; strict Clippy |
| Web build | Release bundle uses same-origin WS; local serve retains explicit port 8787. | verified | Optimized Trunk build and combined-host live probes |
| Client reliability | Failed actions are visible and closed/error sockets reconnect with capped backoff. | verified | 62 client tests and connected framebuffer smoke |
| Infinite map | Terrain/fog stream as a bounded camera-centered chunk cache. | verified | Normal and 80-tile-distant framebuffers plus loaded-chunk assertion |
| Officers/manual play | Officers add capacity but vacant roles do not make categories manual. | open | Role-by-role automated/manual scenarios and framebuffer playtest |
| Manual controls | Most protocol actions, buildings, workers, research, military, and hauling lack usable client paths. | open | Script every action through server and exercise all visible controls |
| Manual raid defense | `DefendRaid` applies damage immediately and the next tick replays the banked telemetry counter, so each player click currently damages twice. | open | One action across the following tick deals exactly one `DEFEND_CLICK_DAMAGE`; guided raid twin |
| Multi-village | Socket-selected routing and a persistent client village selector survive reorder/reconnect/founding and missing-village fallback. | verified | 72 client tests, authenticated server routing test, and opposite-target 1024×768/1920×1080 framebuffers |
| Spatial invariants | Player and leader placement now validates and commits atomically across terrain, claims, buildings, roads, stockpiles, gather spots, and queued future footprints. Expansion-backed sites persist their exact donor link, duplicate reservations are excluded, and paid scaffolds resume monotonically after every builder-death path. | verified | 708 sim and 24 server tests, including player campaigns, persistence/legacy round trips, real old-age/survival/raid death paths, and independent correctness review; strict Clippy |
| Production breadth | Deterministic exterior farm plots (catnip/grain/herbs), logging, grain→flour→food, logs→lumber, Mill/Sawmill staffing, lumber-first construction, persistence, and a real guided action campaign are verified. Broader recipes and upgrade-gated escalating per-building costs remain. | in progress | Build/staff/produce every remaining building and recipe from player actions; pinned plan-time cost and escalation boundaries |
| Client visibility | Crop/timber resources, farm stages, carried logs, Mill, and Sawmill now round-trip and render in the HUD/world. Ore/metal and generalized skills still need a complete mature-colony visibility audit. | in progress | Snapshot round trips and mature-colony UI inspection for every remaining resource/skill |
| Visual truth | Persistent map plaques are removed. Residential rooms retain roof silhouettes; every current workshop, civic/supply station, field, and shrine is a typed open floor-and-prop composition. The fixed settlement core no longer renders procedural nature props. | verified | Label/core-clear and open-station normal+dense framebuffers inspected at exact 1024×768, 1280×800, and 1920×1080; 84 client tests and strict Clippy |
| CI/hosting | Forgejo quality workflow and combined non-root server/WASM image are committed. | in progress | First pushed CI run remains; hosting live probes and deployment docs are verified |

The existing Paws & Whiskers cat and raider sheets are accepted project assets.

## Player playtest feedback — 2026-07-13

These are current product requirements discovered by playing the native client. They remain
open until the real player path and, where applicable, Bevy framebuffers prove them.

| Feedback | Required behavior | Status | Verification required |
| --- | --- | --- | --- |
| Fog of war and scouting | A new village reveals only its viable core plus roughly two tiles. Scouts choose a resource target or general exploration; their discoveries remain provisional until they return and touch the shrine. Nearby founding wood should be a very short scout trip. | in progress | Fresh-village and resource/general scout campaigns; outbound/return/shrine boundary tests; before/after framebuffers |
| Open workshops | Houses retain roofs, while all current workshops and non-residential stations use explicit top-down floors and function-readable props. The shrine uses an altar, reliquary, candelabra, and brazier; fields use soil/crops rather than a market facade. Mill and Sawmill are distinct open stations. | verified | Normal+dense own-framebuffer captures at exact 1024×768, 1280×800, and 1920×1080; exhaustive 24-building visual grammar and distinct-workshop tests |
| Building labels | Persistent map-name plaques are gone; hover/click inspector names remain available. | verified | Label-free normal and dense client framebuffers at 1024×768, 1280×800, and 1920×1080 |
| Clear village interior | Procedural tree/rock props are hidden in the radius-six core; designated farms and legacy Field construction are forced outside it, and logging cannot target hidden interior trees. Natural sim resources/deposits still need to be cleared from the founding core itself. | in progress | Rendering and farm/logging boundaries are verified; founding/expansion sim resource-clearing properties across biomes remain |
| Global and personal villages | Player-keyed personal-site allocation is now pure, deterministic, order-stable, grass/lowland buildable, overflow-safe, and separated by at least 48 tiles. The canonical global village, durable ownership/access, discovery, and inter-village trade are not wired yet. | in progress | Site properties are verified; ownership/access/routing/persistence, distant discovery, and two-player join/found/trade campaign remain |
| Sprite review tool | [`docs/sprite-review.html`](sprite-review.html) compares current art with three persisted/exportable proposals for all 22 current buildings plus Accounting Tent, Mill, Sawmill, and a global hall/market concept. | verified | Desktop/mobile browser runs: 26 rows, filters, favorites, reload persistence, path copying, JSON export, zero page/image errors |
| Research tree scale and UX | A pure 500-node catalog provides 167 building, 167 recipe/resource, and 166 upgrade nodes across named families, with stable IDs, typed payloads, AND prerequisites, and layout coordinates. The live Mill study is reconciled without changing those totals. The full-page client ledger renders the complete graph; generated nodes honestly remain read-only until runtime integration. | in progress | Catalog validation and exact-size UI campaign are verified; integrate purchasing/effects/persistence and the once-per-real-day leader choice, then repeat the live interaction campaign |
| Founding population and housing | A pure migration policy now enforces 30-hour establishment, per-cat prosperity, domain-separated deterministic cohorts, over-capacity probation, deterministic vacancy allocation, and 36-hour unhoused departure. Live founding still starts five cats and old housing/breeding rules remain. | in progress | Migration policy/full-sim gate is verified; exact 15-cat/three-house snapshot, five-per-house housing, slow breeding, tick/persistence integration and live campaigns remain |
| Roads | Authored stone-road connectivity and movement multipliers exist, but the snapshot exposes only paved tiles, so traffic-formed dirt paths are invisible to the player. Enforce the complete P16 model: authored stone roads, traffic-formed dirt roads, exact surface restrictions, connected shrine/gate/exterior routes, and the single south founding gate. | in progress | Real traffic crosses the wear threshold; stone ground never auto-forms dirt; both surfaces persist and reach the protocol; exact movement boundaries, connectivity properties, and before/after framebuffers |

## Current design-document traceability

| Document | Implementation status | Follow-up |
| --- | --- | --- |
| `docs/GAME_VISION.md` | partial | Finish manual-to-officer automation, usable multi-village, complete visible production controls |
| `docs/ARCHITECTURE.md` | corrected for the post-cutover workspace | Keep current as remaining gaps close |
| `docs/HANDOFF.md` | current | Replace NEXT STEPS with verified outcomes from this tracker |
| `docs/migration/BOARD.md` | core migration complete, expansion rows overclaim | Reopen partial P12–P19 slices and close them only after feature campaigns |
| `p12-idle-cat-forest.md` | partial | Officers, manual work, farming/production exposure, shrine reachability |
| `p14-spatial-placement.md` | verified | Atomic occupancy, future reservations, road connectivity, and scaffold recovery are covered by the spatial campaign recorded below |
| `p15-playtest-feedback.md` | partial | Dynamic infinite map is fixed; multi-village UX and richer actions remain |
| `p16-village-blueprint.md` | partial | Player-facing gather controls and reachable production chain |
| `p17-biome-generator.md` | simulation-heavy, product partial | Expose resources/logistics and render arbitrary chunks |
| `p18-visual-polish.md` | partial | The open craft-station and label-free map slices are verified; continue the broader framed-panel/icon UI pass only where current screens remain visually unfinished |
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

### Label-free, clear-core native framebuffers — 2026-07-13

The client rendered both a normal founding village and a dense 16-structure village through
its own primary-window screenshot path at exact 1024×768, 1280×800, and 1920×1080 physical
resolutions. All six images were inspected: no building-name text or plaque entities remain,
inspector labels are retained, the permanent radius-six core contains no procedural tree/rock
props, and outside vegetation remains visible. The temporary screenshot/window-size systems
were removed before the 73-test client gate, strict Clippy, and formatting checks.

### Full-page research catalog native framebuffers — 2026-07-13

The client rendered the 500-study ledger through its own primary-window screenshot path at
exact 1024×768, 1280×800, and 1920×1080 physical resolutions. The opening legacy graph,
filtered/panned generated building branch, and filtered/panned generated upgrade branch were
all inspected. Search/category controls, responsive header and inspector, dependency edges,
and the explicit generated-node `Runtime integration pending` state remained readable at the
required sizes. A first capture with an empty canvas was rejected; its centre-origin scaling
bug was corrected and covered by a transform test before the three accepted captures. All
temporary capture/window systems and compositor processes were removed before the 80-test
client gate, strict Clippy, and formatting checks.

### Atomic spatial placement and recovery — 2026-07-13

All authored placement paths now reject hard conflicts before mutation and preserve connected
shrine/gate/exterior access. Future construction footprints are exclusive reservations; forced
expansion claims only the footprint plus its required orthogonal fence margin, records the exact
source build, and charges/times construction only at real breakground. A paid scaffold survives
and resumes from remaining work when its builder dies through ordinary survival, old age, or a
raid. Duplicate-reservation, phase-order, and diagonal-overclaim failures found by the long
research campaigns were fixed rather than hidden by weaker fixtures. The final branch gate was
708 simulation and 24 server tests with strict Clippy; an independent review found no remaining
correctness blocker.

### Label-free open-station native framebuffers — 2026-07-13

Normal and injected dense settlements were rendered through the client's own primary-window
screenshot path at exact 1024×768, 1280×800, and 1920×1080 physical resolutions. The frames show
roofed residential rooms alongside distinct open timber, masonry, metal, textile, research,
school, barracks, supply, farm, and shrine compositions with no persistent map names. An initial
shrine gold-pile choice was rejected because it resembled a resource deposit; the accepted
recaptures use the reviewed reliquary. During this art-only capture, the server tick task was
temporarily bypassed so the real booted server could serve its real starter snapshot despite
concurrent debug-simulation contention; that bypass and all screenshot/window hooks were removed
before the 84-test client gate, strict Clippy, and commit. Production snapshot cadence still gets
an isolated benchmark after the spatial/production merge.

### Farming, timber, and exterior production campaign — 2026-07-13

The production integration added persistent exterior catnip/grain/herb plots with five visible
growth stages, deterministic logging, a staffed Mill (`grain → flour → food`), a staffed
Sawmill (`logs → lumber`), and lumber-first construction with legacy-plank fallback. A
three-seed deterministic guided campaign uses real `AssignOfficer`, `AssignWorker`,
`DesignateFarm`, `RequestJob(GatherLogs)`, `PlanBuilding`, `ClearFarm`, and `AdvanceTime`
actions; it does not delete jobs or assignments behind the action layer. Farm/Field placement
and logging reject the fixed settlement interior. Linked exterior Field claims remain visible
one tile at a time but use a one-game-minute job cadence so the five-cat founding roster is not
consumed by a day-long boundary job.

The final four-crate gate passed all 905 tests (one intentionally skipped), including five seeds
over 200 game-hours with zero resets, strict Clippy, and formatting. Exact 1280×800 and
1920×1080 primary-window captures were inspected: Mill and Sawmill are distinct roofless
stations, three crop/stage plots sit beyond the wall, the HUD remains usable, and persistent map
labels remain absent. Normal server ticking starved the art client and `/health`, so the already
documented temporary tick bypass was used only for these frames and removed; the runtime defect
is the active server-responsiveness item above, not accepted visual behavior.

## Completion rule

An item moves to `verified` only when the behavior is reachable through the real player path,
its deterministic simulation tests pass, relevant server/client integration tests pass, and any
visual claim has been checked from the Bevy client's own framebuffer. Compiling alone is not
verification.
