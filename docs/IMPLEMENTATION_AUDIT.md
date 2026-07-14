# Idle Cat Forest implementation audit and fix tracker

Last updated: 2026-07-14

This is the working ledger for the post-cutover design audit and exhaustive playtest pass.
It records only the current Rust/Bevy game's promises and gaps. Documents explicitly marked
historical or superseded are reference material, not an implementation backlog.

Status key: `open`, `in progress`, `verified`, `deferred`.

## Active fixes

| Area | Finding | Status | Verification required |
| --- | --- | --- | --- |
| Shrine economy | Population-relative surplus gates make food/refined tithes and physically carried material offerings reachable without draining survival or construction reserves. Both blessing faucets work manually and under Loremaster automation after establishment. | verified | Five-seed 200-game-hour unattended economy campaign, deterministic twin, manual signed-action campaign, reserve boundaries, carried-offering completion, and event assertions |
| Storehouse reservoir | The shrine remains the all-resource fallback reservoir; the P12 seeded general storehouse/local reservoir model is not live. | open | Founding storehouse inventory, spatial capacity/fallback rules, physical hauling, persistence, and inspector campaign |
| Tools | The Forester-owned raw chain deterministically produces tools, protects construction/offering reserves, and tool stock increases maintained labor productivity through a bounded multiplier. | verified | Live-second funded chain, real quarry-return campaign, repeated two-tool bootstrap, reserve/pre-emption boundaries, field commissioning, and determinism twins |
| Legacy research | A player may spend cat research points on any affordable original node, while blessings remain a distinct god-purchase path. A staffed Loremaster may complete at most one affordable original node per rolling real-life day. | verified | Signed manual purchase, research accrual, exact 24-hour boundary, persistence of the last-unlock timestamp, crisis non-staffing, and deterministic pacing tests |
| Generated research | The 500-node catalog and ledger are real, but the 476 non-legacy generated nodes remain deliberately non-actionable and their typed payloads have no runtime effects. | open | Purchase every affordable generated node with research points; persist ownership; apply building/recipe/resource/movement/labor/storage/defense effects; daily Loremaster choice across the complete catalog |
| Survival | The old five-cat population guardrail is superseded by the verified 15-adult founding contract. Five maintained seeds survive 300 game-hours without artificial resource grants or unintended resets; ordinary and leader/healer old-age pacing is 240/288 game-hours. | verified | Five 300h campaigns with byte-identical twins, population/housing/resource bounds, no unintended resets, focused birth/migration/aging/reset tests |
| Emergency water | Recovery is a real source→travel→carry→deposit job performed by a living cat, with deficit-scaled unique fetchers and runway-aware work pre-emption. No crisis phase grants free water. | verified | Threshold, uniqueness, pre-emption, no-free-mutation, physical cargo/deposit, live-cadence, five-seed survival, and determinism campaigns |
| Server responsiveness | Simulation, snapshot construction, and synchronous SQLite work run on Tokio's blocking pool. New sockets clone a startup-initialized last-completed snapshot; save ticks release the authoritative world lock before disk I/O; missed intervals skip rather than burst. | verified | One-worker injected 250 ms tick keeps health and initial snapshot under 50 ms; 28 server tests; live health/WS probes and tick/save timings; strict Clippy |
| Server security | Socket-bound identity, exhaustive mutation authentication, selected-colony routing, release-disabled test controls. | verified | Authentication matrix, signed manual/officer and two-player village campaigns, selected-colony isolation, release test-control denial, and strict Clippy |
| Web build | Release bundle uses same-origin WS; local serve retains explicit port 8787. | verified | Optimized Trunk build and combined-host live probes |
| Client reliability | Failed actions are visible and closed/error sockets reconnect with capped backoff. | verified | Focused action/reconnect tests and connected own-framebuffer smoke |
| Infinite map | Terrain/fog stream as a bounded camera-centered chunk cache. | verified | Normal and 80-tile-distant framebuffers plus loaded-chunk assertion |
| Officers/manual play | Steward, Accountant, Forester, Farmer, Captain, Loremaster, and Cloth Leader each own one automation category. A vacancy is manual-only; appointment requires the matching researched unlock and completed role station. Ownership and replacement persist, and the client exposes signed appointment/vacate and maintained manual-order paths. | verified | Seven-role vacancy matrix, staged 7→0 manual-frequency handoff, independent building/unlock denials, dead-holder succession, signed server isolation/authentication, client action tests, and deterministic guided campaigns |
| Manual controls | Farm/gather/road painting, staffing, raw-resource orders, expansion, building-type planning, military defense/training, ritual/tithe/offering, hauling, and research purchase have usable signed client paths. Farm and gather painters currently hardcode Grain and Materials; `PlanBuilding` has no coordinate and auto-selects a site; election/vote/vote-kick, clear-farm/remove-gather, exact per-cat labor assignment, and station production queues still lack complete client tools. | open | Add the missing exact/variant tools, script every signed action through the server, and exercise every visible control at all target resolutions |
| Manual raid defense | Each `DefendRaid` action applies exactly one `DEFEND_CLICK_DAMAGE`; the following tick performs terminal cleanup without replaying click telemetry or producing duplicate events. | verified | Per-click health assertions, killing-click cleanup on the next tick, exactly one repelled event, and guided multi-seed manual campaign |
| Spatial placement | Player and leader placement validates and commits atomically across terrain, claims, buildings, roads, stockpiles, gather spots, queued footprints, rendered 2×3 tree canopies, and 1×1 rocks. Buildings and trees are exact soft obstacles; water, mountains, and walls remain hard obstacles. | verified | Atomic placement/reservation campaigns, climate-decoration source parity, multi-cell footprint collision tests, soft-obstacle path tests, and clear-interior framebuffer |
| Staged wall growth | Expansion still replaces the derived perimeter immediately rather than constructing a closed outer wall before removing the old inner wall. | open | Interrupted and resumed construction, every intermediate state closed, exactly one south gate, persistence, and before/during/after framebuffers |
| Agricultural territory | Farms stay outside the permanent founding core, but expansion still lacks an explicit distinction between walled settlement parcels and claimed agricultural land. | open | Separate persisted territory classes, farm/gather placement rules, wall derivation, path access, and before/after guided framebuffers |
| Multi-village model | One canonical global village remains viewable by anonymous sockets and controllable by every authenticated player. Each stable signed identity can found exactly one deterministic distant personal village; ownership, selected routing, and foreign denial survive reconnect/restart. Explicit scout-delivery provenance creates mutual summary-only contact, after which owners can configure, propose, accept, or cancel bounded atomic resource barter without exposing or mutating a foreign private simulation. | verified | Deterministic site/collision properties, signed two-player found/join/denial/discovery/trade campaign, transactional SQLite rollback/restart/offer round trip, exact client controls, strict four-crate gates, and selected-village framebuffer |
| Shared-world depth | The communal village currently uses the same 15-cat/three-Den founding blueprint as a personal village. Colonies share a seed and absolute coordinates but own duplicated mutable terrain; meeting is a delivered summary, and barter atomically swaps a restricted scalar-resource set without physical cats, item stacks, or routes. | open | Distinct large-global blueprint, authoritative shared spatial mutation rules, physical encounter/contact, item/caravan trade, persistence, isolation, and multi-player guided campaign |
| Scout search behavior | Knowledge delivery is physically correct, but mission routing generates hidden terrain and selects the nearest useful unrevealed target. It does not perform the deterministic random-walk search described by P15. | open | Decide search-vs-oracle design; if retaining P15, test wander/change-direction/give-up/resource detection/return across seeds without reading hidden targets |
| Physical workshop logistics | Staffing currently stores a worker id but does not route the cat to the station; workshop inputs/outputs still use colony-global resources rather than local workshop/stockpile inventories. | open | Every chain must visibly move worker and item stacks workshop↔stockpile↔workshop, persist local queues/storage, and expose them in the inspector |
| Workshop inspector | The P15 inspector cannot show a real job queue or station-local input/output storage because neither is modeled in the snapshot. | open | Hover/click/cycle through every station; verify queue, worker, local inputs/outputs, blocked reasons, and persistence against real logistics state |
| Skills breadth | Role XP is exposed for only four legacy labors; Mill, Farm, Research, and other maintained production roles have no complete gain/effect path. | open | Per-labor XP gain, speed/yield effect, persistence/protocol/UI, and determinism campaigns for every labor |
| Farm labor truth | Designated plots require assigned field labor and Farmer automation can staff it, but no cat physically plants, tends, harvests, paths to each plot, or gains farming skill. | open | Manual and officer-driven field campaigns with visible cats, crop inputs/outputs, travel, skill gain, and vacancy stalls |
| Production breadth | Deterministic exterior farm plots, logging, grain→flour→food, logs→lumber, fibre/hide→cloth/leather, ore→metal, maintained staffing, tool productivity, lumber-first construction, persistence, and type-local escalating scaffold costs are verified. Broader recipes and material/item variants remain. | in progress | Build/staff/produce every remaining building and recipe from player actions; retain pinned plan-time cost and escalation boundaries |
| Client visibility | Crop/timber resources, farm stages, carried logs, Mill, and Sawmill now round-trip and render in the HUD/world. Ore/metal and generalized skills still need a complete mature-colony visibility audit. | in progress | Snapshot round trips and mature-colony UI inspection for every remaining resource/skill |
| Transport and fishing | Fine-biome movement factors are unused; rail is a distance-triggered global multiplier without tracks/trains; shipping makes water slow-walkable without vessels; maintained fishing gather/food paths are absent. | open | Exact 26-biome travel factors, built connected rail/ship routes and vehicles, fishing gather/haul/recipe paths, persistence, and distant-biome guided campaigns |
| Item and recipe breadth | The item taxonomy is broad, but live crafting covers a small wood/stone/cloth/leather subset; bone/gem/clay/metal variants and finished tool/weapon/armor item chains are incomplete. | open | Source and craft every maintained material/category combination; quality, inventory, trader, protocol, and UI campaigns |
| Adventure UI skin | Tracked Adventure art now drives sliced parchment, dark, and ornate panels; default, hovered, pressed, active, and disabled buttons; framed resource pills; need bars; the minimap ring; and pointer, interact, pressed, target, and disabled custom cursors across the HUD and research ledger. Exact native framebuffers are verified at 1024×768, 1280×800, and 1920×1080, and the release WASM bundle builds. | in progress | WASM visual/interaction capture and regression coverage as the remaining menus and controls become reachable |
| Visual truth | Persistent map plaques are removed. Residential rooms retain roof silhouettes; every maintained building, including the snapshot-reachable Accounting Tent, has a tested typed open/roofed composition, and the fixed core hides procedural nature props. The maintained Adventure skin is native-framebuffer verified. | in progress | Capture the integrated Accounting Tent in a real native village and complete the WASM framebuffer/interaction campaign |
| CI/hosting | Forgejo quality workflow and combined non-root server/WASM image are committed. | in progress | First pushed CI run remains; hosting live probes and deployment docs are verified |

The existing Paws & Whiskers cat and raider sheets are accepted project assets.

## Player playtest feedback — 2026-07-13

These are current product requirements discovered by playing the native client. They remain
open until the real player path and, where applicable, Bevy framebuffers prove them.

| Feedback | Required behavior | Status | Verification required |
| --- | --- | --- | --- |
| Fog of war and scouting | A new village reveals its exact 13×13 claim plus a two-tile halo. Only purposeful scouts lift provisional fog; wood/food/water/stone and general missions commit knowledge only after a living scout physically returns to the shrine. The first wood mission is a bounded fast round trip. In-flight notebooks persist across SQLite restarts. | verified | 32-seed founding-wood bound, four determinism twins, death/cancel/restart campaigns, five signed client controls, and fresh/provisional/committed own-framebuffer captures at 1280×800 and 1920×1080 plus a responsive 1024×768 toolbar capture |
| Open workshops | Houses retain roofs, while all current workshops and non-residential stations use explicit top-down floors and function-readable props. The shrine uses an altar, reliquary, candelabra, and brazier; fields use soil/crops rather than a market facade. Mill and Sawmill are distinct, and Accounting Tent has a tested open composition. | verified | Normal+dense own-framebuffer captures for the prior 24 variants at exact 1024×768, 1280×800, and 1920×1080; exhaustive 25-building visual grammar tests; integrated Accounting Tent capture remains under Visual truth |
| Building labels | Persistent map-name plaques are gone; hover/click inspector names remain available. | verified | Label-free normal and dense client framebuffers at 1024×768, 1280×800, and 1920×1080 |
| Clear village interior | Founding clears every claimed tile in authoritative simulation state: meadow ground, zero deposits/forage/ore/water/danger/wear, and no tree/rock overlay. Water is guaranteed outside the south wall, and ordinary expansion atomically clears each newly claimed cell. Farms and legacy Fields are kept beyond the fixed founding core, but the longer-term distinction between expanded walled settlement and agricultural territory still needs an explicit boundary model. | in progress | Multi-seed founding and expansion clearing plus renderer evidence are verified; add agricultural territory that can remain claimed and worked without becoming an interior wall parcel |
| Global and personal villages | The secure foundation is live: one communal village; one owner-only personal village per stable signed identity at a deterministic viable site at least 48 tiles away; restart-persistent selection; returned-scout contact; and signed bounded direct barter. The global site is not mechanically larger, mutable terrain is duplicated per colony, and meeting/trade are not physical. | in progress | Existing ownership/privacy/discovery/barter campaign is verified; add a distinct large-global blueprint, shared spatial-state rules, physical meeting and item/caravan trade |
| Sprite review tool | [`docs/sprite-review.html`](sprite-review.html) compares current art with three persisted/exportable proposals for all 22 current buildings plus Accounting Tent, Mill, Sawmill, and a global hall/market concept. | verified | Desktop/mobile browser runs: 26 rows, filters, favorites, reload persistence, path copying, JSON export, zero page/image errors |
| Research tree scale and UX | A pure 500-node catalog provides 167 building, 167 recipe/resource, and 166 upgrade nodes across named families, with stable IDs, typed payloads, AND prerequisites, and layout coordinates. The full-page client ledger renders the graph. Research-point purchase and once-per-real-day Loremaster choice are verified for the original 24 nodes; the other 476 remain read-only and effectless. | in progress | Legacy purchase/daily cadence and exact-size UI campaigns are verified; integrate generated-node purchasing/effects/persistence and repeat the live interaction campaign |
| Founding population and housing | Founding creates exactly 15 adult cats and three complete five-bed Dens. Slow pregnancy starts after establishment and reserves a permanent bed through 18 game-hours of gestation. Prosperity migration begins after 30 game-hours, checks every 12 hours, and gives a real unhoused arrival 36 game-hours to obtain a bed before departure. Once breeding is established, permanent migration leaves the last real vacancy for a family unless a pregnancy already owns it. Extinction atomically restores the founding state with run-scoped identities. | verified | Five-seed 300h twins, signed guided Den campaign, family-vacancy/migration tests, persistence/restart, exact 15/15 and selected-probation framebuffers, independent review, and four-crate gates |
| Roads | Authored stone roads and traffic-formed dirt roads are disjoint snapshot surfaces with cool-stone/warm-earth rendering. Stone roads move at 175%, dirt paths at 105%; mountain, cave, water, and authored stone ground cannot auto-form dirt. The founding cross connects the shrine to its single south gate and exterior approach. | verified | Wear-threshold and forbidden-surface boundaries, protocol compatibility, connectivity tests, and inspected own-framebuffer proof |
| Scout search semantics | Scouts correctly keep knowledge provisional until shrine return, but current resource missions use generated hidden terrain to choose the nearest useful target rather than searching by deterministic random walk. | open | Resolve the design mismatch, then verify target acquisition/give-up/return behavior without oracle access if P15 search remains authoritative |

## Current design-document traceability

| Document | Implementation status | Follow-up |
| --- | --- | --- |
| `docs/GAME_VISION.md` | current intent, partially implemented | Seven-role ownership is verified; finish physical workshop/farm labor, exact placement/election tools, generated research, large-global/shared-space depth, and complete production controls |
| `docs/ARCHITECTURE.md` | reconciled | Keep the maintained founding/life-pacing contract and known-gap section synchronized with real runtime state |
| `docs/HANDOFF.md` | reconciled | Keep NEXT STEPS and verified evidence synchronized here |
| `docs/migration/BOARD.md` | core migration complete, expansion rollup reconciled | Close partial P12–P19 slices only after their feature campaigns |
| `p12-idle-cat-forest.md` | partial | Seven strict officer roles, role-building gates, signed manual work, shrine reachability, useful tools, costs, and Accounting Tent are verified; finish all-labor skills, physical workshop/farm logistics, local inventories, and exact tools |
| `p14-spatial-placement.md` | partial | Atomic placement/reservations/scaffold recovery, full 2×3 tree/1×1 rock occupancy, soft obstacles, and exact road surfaces are verified; staged outer-wall growth remains |
| `p15-playtest-feedback.md` | partial | Roads, exact footprints/depth, shrine-return fog, and secure village foundations are fixed; oracle scout targeting conflicts with the random-walk requirement, while exact placement/election and physical shared-world depth remain |
| `p16-village-blueprint.md` | partial | The 15-adult/three-five-bed-Den lifecycle, founding clearing, exterior water, exact roads/gate, and basic gather controls are verified; fishing, gather variants/removal, agricultural territory, and reachable physical production chains remain |
| `p17-biome-generator.md` | simulation-heavy, product partial | Apply fine-biome movement, expose resources/logistics, and replace placeholder rail/shipping with real transport |
| `p18-visual-polish.md` | partial | Open stations, Accounting Tent reachability/composition, the label-free map, and the Adventure panel/button/progress/cursor skin are verified in code/native campaigns; integrated Accounting Tent and WASM visual interaction captures remain |
| `p19-items-materials-trade.md` | partial | Complete per-material items/recipes, physical inventories, fishing, transport, and expose every chain to play |
| `docs/migration/WASM.md` | development and production packaging work | Optional transfer/performance campaign remains |
| `README.md`, `CLAUDE.md`, and `docs/assets/*.md` | reconciled | Keep cutover/CI/labels/WASM claims and current top-down Mill/Sawmill/cat-sheet selections synchronized with verified behavior |

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
| Needs, autonomous survival, aging, genetics, breeding, skills, specialization, leaders, elections, raids, death, and extinction recovery | Carried at the world-tick level into deterministic `cat-sim`; shared colony food/water currently restore cats abstractly, rather than cats physically seeking food/drink/sleep. The current design deliberately replaces the prototype's 48/57.6-hour old-age thresholds with 240/288 hours and replaces its five-cat recovery roster with the 15-adult/three-Den invariant. Balance and role-aware guided campaigns remain part of this audit. |
| Dynamic colony grid plus a fixed 16×16 world map | Superseded by one flat, streamed, effectively infinite world containing multiple villages |
| Fog, expeditions, path wear, terrain travel cost, walls, and gates | Carried forward; shrine-return resource/general scouting and the current dirt/stone road model are verified above, while staged closed-perimeter expansion remains open |
| Click-to-feed/heal/assign/fight and a browser task queue | Superseded by signed typed management actions and the verified seven-role manual-to-officer loop; coordinate placement, election UI, variant/removal tools, and production queues remain open above |
| Blessings, buildings, and the original ~18-node research tree | Carried forward: active shrine faucets, research-point purchases, daily legacy Loremaster cadence, useful tools, and escalating costs are verified. The expanded 500-node catalog still has 476 inert nodes. |
| Multiple colonies and inter-colony trade | Carried forward and extended: secure global/personal ownership, shrine-knowledge contact, and direct consensual resource barter are live alongside visiting traders. A larger global site, shared mutable terrain, and physical meeting/trade remain open. |
| External sprite-render service, DOM/Pixi rendering, isometric/elevation experiments, and newspaper UI | Superseded or explicitly dropped by the Rust/Bevy top-down direction |
| Seasonal events, achievements, accessories, sound/music, and a mobile app | Listed only as non-MVP future ideas in the original document; not current commitments unless promoted into `GAME_VISION.md` |
| Historical roadmap stretch items such as traveler interception and elevation-aware zones | Not current commitments. Fishing and real rail/ship transport were later promoted by P16/P17/P19 and are open above. |

The old result templates and browser test campaigns document measurements of the retired web
client. They are evidence archives, not Bevy acceptance criteria; the current matrix below
replaces them.

## Full playtest matrix

This section is required completion coverage, not a claim that every listed campaign has passed.

- Multi-seed unattended runs plus longitudinal player-guided runs: fully manual survival,
  staged officer handoff, productive expansion, and deliberately poor decisions with real
  consequences. Guided runs dispatch real `ClientAction`s from observed colony state rather
  than mutating fixtures behind the action layer.
- Survival, needs, slow reserved-bed breeding, 15-adult/three-Den founding, prosperity migration,
  36-hour probation/retention/departure, 240/288-hour aging, atomic extinction recovery, and
  determinism. Emergency water must be fetched, carried, and deposited by a living cat.
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

### Shrine-return fog and resource scouting — 2026-07-13

Founding knowledge is exactly the 13×13 claimed village plus a two-tile Chebyshev halo
(17×17, 289 permanent tiles). Ordinary cats still wear paths but never reveal the wilderness.
Signed player actions and the leader can dispatch general, wood, food, water, or stone scouts;
their per-cat notebooks remain provisional until the living cat physically reaches the shrine,
and are discarded on death/cancellation. The notebooks, mission, destination, and permanent map
round-trip through SQLite, with a legacy empty default.

The founding-wood guardrail covers 31 contiguous seeds plus the production seed that originally
took more than eight real minutes; every scout returns within 180 live seconds, with four
byte-identical determinism twins. The focused gate passed 26 scout tests and 119 server/client
tests with strict four-crate Clippy and formatting. The full 922-test gate reached every touched
test; its sole failure was the independently reproduced, pre-existing seed-2024 population
guardrail tracked above.

The Bevy client captured its own fresh/provisional/committed frames at exact 1280×800 and
1920×1080. Both sequences measured 289/0 → 289/5 → 331/0 permanent/provisional tiles; the
committed event records the scout touching the shrine with 42 newly mapped tiles. Raw images are
opaque sRGB and were inspected in lossless regions because the image viewer intermittently hid
valid PNG tiles. A separate exact 1024×768 framebuffer verified the responsive wrapped toolbar:
all five scout commands and all existing controls remain visible. All temporary capture code and
processes were removed.

### Founding/housing lifecycle — 2026-07-14

The old five-cat loop is fully replaced by exact 15-adult/three-five-bed-Den founding, permanent
pregnancy bed reservations, 18-game-hour gestation, a 30-game-hour migration establishment
window, 12-hour migration checks, and a 36-game-hour probation for physically present unhoused
arrivals. Ordinary and leader/healer old-age thresholds are intentionally 240 and 288 game-hours.
Extinction recovery recreates the complete state atomically with run-scoped identities.

Emergency water is an actual source→travel→carry→deposit job. Deficit-scaled unique fetchers can
pre-empt eligible leader work while player/construction jobs remain protected. The same slice
fixed two playtest-discovered lifecycle failures: non-sticky raw-bench release could strand cats
in orphaned `Working/Build` states, and a distant exploration timer could expire before the scout
reached its observation tile, causing an empty return loop. Both repairs are deterministic and
covered by compatibility/long-distance tests.

Five maintained seeds and their byte-identical twins completed 300 game-hours without unintended
resets. A signed server campaign guided construction of a Den through the authenticated action
handler and proved housing retention/departure; persistence/restart and repeated five-minute
guided farming actions exercise the physical world rather than fabricated snapshots. The exact
gate passed 835 `cat-sim`, 27 `cat-protocol`, 35 `cat-server`, and 96 `cat-client` tests, strict
four-crate Clippy, and formatting.

The Bevy client captured and visually proved two own-framebuffer PNGs at 1920×1080: founding at
`POP 15`, `BEDS 15/15`, with all 15 cats and three homes visible; and an organic 16-cat state with
`Awaiting homes 1`, `Unhoused 1`, plus the selected migrant's 35h30m housing countdown. All
temporary capture systems, accelerated server cadence, and processes were removed; the committed
tree was byte-clean afterward.

### Physical village terrain and road surfaces — 2026-07-14

The founding claim is now cleared in authoritative simulation state rather than hidden only by
the renderer. Every claimed cell becomes inert meadow with zero forage, herbs, water, deposit
capacity, danger, wear, and natural overlay; the deterministic emergency water source is outside
the south wall beside the gate approach. Ordinary village expansion performs the same atomic
clearing before it accepts a cell and rolls the original terrain back if road connectivity cannot
be preserved. The renderer and placement validator share the climate-driven decoration source:
trees occupy their complete 2×3 canopy, rocks occupy 1×1, and neither can be silently built over.

Authored road overlays and traffic wear are separate wire surfaces. Built stone remains 175%
speed and renders cool grey; eligible worn earth becomes a warm-brown 105% dirt path. Authored
stone, mountains, caves, and water never masquerade as traffic dirt. Focused tests cover the wear
threshold, forbidden surfaces, snapshot disjointness, climate-decoration parity, multi-cell
collisions, founding/expansion clearing, and the shrine→single-south-gate→exterior route.

The booted client captured its own 1920×1080 primary-window framebuffer. It visibly shows the
resource-free interior, three roofed homes, open stations without map labels, cool paved shrine
cross, warm exterior footpath, single south gate, and nature outside the enclosure. The proof
path-wear tile and screenshot system were temporary capture scaffolding and were removed before
the committed quality gate. The final gate passed 842 simulation, 27 protocol, 96 client, and 35
server tests plus strict Clippy and formatting. It includes the organic seed-7 research horizon,
the finite-civic-before-open-field expansion regression, and a cold-reload assertion for the
derived decoration cache. The remaining world-layout item is staged outer-wall construction; the
longer-term farm-territory boundary is tracked separately above.

### Secure shared-world villages and direct barter — 2026-07-14

The shared world now enforces one canonical global village and at most one personal village for
each stable HMAC-backed player identity. Personal site choice is keyed by world seed and player
identity, scans deterministic viable grass/lowland candidates, remains order-stable and
overflow-safe, and keeps every village at least 48 tiles apart. Replaying Found is idempotent.
Anonymous sockets can inspect the global village but cannot mutate it; authenticated players can
control the global village, owners additionally receive their personal simulation, and every
foreign private simulation is removed server-side rather than merely hidden by the client.
Reconnects and SQLite restarts restore the same bearer, ownership, village, and selection.
Native bearer updates use a private same-directory temporary file, `sync_all`, and atomic rename;
a forced temporary-write failure proves the previous bearer and selected village survive intact.

Village contact uses explicit shrine-delivery provenance: neither a provisional notebook nor a
generic permanent reveal from expansion, recovery, or a legacy save can count. Only tiles carried
home by a living scout during that tick can reach another shrine and create mutual contact.
Contact exposes a name/kind/anchor summary, never the other owner's cats,
resources, jobs, owner id, or mutation capability. Once contact exists, a source owner can make a
signed resource-for-resource proposal; only the target village's controller can accept it.
Acceptance rechecks both inventories and receiving storage, swaps both sides atomically,
reconciles physical stockpile totals, and removes the offer. The source may cancel; invalid,
unknown, foreign, self, non-finite, same-resource, underfunded, and over-capacity trades fail
without mutation. Each source village is capped at 32 open offers, preventing unbounded persisted
state without pretending offered stock is reserved; acceptance always performs the authoritative
stock recheck.

The server campaign uses two independent signed players to found separated villages, create
mutual contact, verify summary-only projection, propose a barter, and accept it through the real
action handler. Separate intruder, reconnect, database-restart, collision-exhaustion, and
generic-reveal-versus-shrine-delivery discovery tests cover the boundaries. Full world saves use
one SQLite transaction, with forced-insert rollback proving the prior complete save survives.
Colony-local cat/job/building/event/election/vote/raider ids are namespaced only in SQLite storage,
so two villages queuing the same timestamp-derived runtime id cannot collide. A simultaneous
two-village tick/save/reload test guards the exact failure found during the live framebuffer run,
and a shipped-schema regression proves old global-primary-key building tables also remain usable.
The compact selector persists its selected village across process restart and exposes
known-village coordinates plus configurable give/ask resource and amount controls alongside
offer/accept/cancel. The exact gate lists 845
`cat-sim`, 29 `cat-protocol`, 44 `cat-server`, and 105 `cat-client` tests (1,023 total) with strict four-crate
Clippy and formatting. A booted-server 1920×1080 own-framebuffer capture visibly proves the
communal and owned-personal selector rows; the known/contact actions are covered by the signed
campaign and exact client-action tests. All temporary capture code and processes were removed.

### Strict officer ownership and active economy — 2026-07-14

All seven maintained offices now have exclusive automation ownership: Steward, Accountant,
Forester, Farmer, Captain, Loremaster, and Cloth Leader. With every office vacant, three seeds
survived 30 production-cadence real minutes plus a bounded accelerated continuation under repeated
signed manual guidance; the same longitudinal states completed research-point purchases, staffed a
Research Hut, paid both kinds of tithe, carried a material offering to the shrine, planned an
Accounting Tent, trained a warrior, and resolved a raid at exactly six damage per click. A separate
staged handoff proved manual-order frequency falls exactly `7, 6, 5, 4, 3, 2, 1, 0` as role stations
and their researched prerequisites are satisfied and offices are appointed one by one. Missing
prerequisites fail independently, dead holders receive deterministic living successors, and poor
guidance has bounded, visible, byte-identical consequences rather than being silently corrected by
vacant automation.

A fresh integrated server database and native client produced its own
`Screenshot::primary_window` framebuffer after eight seconds at exact 1920×1080. The inspected PNG
visibly shows `15/15` cats, three roofed Dens, label-free open workshop and shrine stations, the
resource-free interior, stone and traffic-dirt roads, the black fog boundary, exterior water and
trees, all seven vacant officer rows, the village selector, and the complete current
manual/scout/farm/gather/road control surface. The temporary screenshot hook was removed and the
source tree was clean afterward. Accounting Tent reachability/composition is code-tested but was
not present in this fresh-founding frame.

The unattended counterpart established all offices and ran seeds 7, 555, 2024, 42, and 99 for 200
game-hours at the deliberately harsh five-minute proxy cadence. Every colony remained extant
without a reset and reached a food/refined tithe, a physically carried material offering, positive
blessings, tool production, and live research; an identical seed-7 twin matched byte-for-byte.
Focused live-second campaigns cover funded and real quarry→carry→refine→tool chains, repeated tool
bootstrap without crossing construction/offering reserves, the capped non-consumable tool
productivity effect for construction/crafting/quarrying/hauling, and per-type escalating scaffold
costs. Legacy research tests cover explicit research-point purchase and the rolling 24-hour
Loremaster boundary. The final full `cat-sim` gate passed **906/906** tests with strict Clippy and
formatting; the other crate gates and signed server/client action tests were also green. This is
not evidence for the 476 inert generated studies or physical station-local logistics, both still
tracked above.

### Unattended live cadence — 2026-07-13

This is a **pre-housing and pre-strict-officer baseline**, superseded by the campaign above and
retained only so pacing regressions can be compared;
its five-cat population ranges are not the current founding target. Three seeded colonies ran
for 48 game-hours at the production one-second cadence, followed
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
before the 84-test client gate, strict Clippy, and commit. The server responsiveness campaign
recorded below subsequently verified production snapshot cadence without that bypass.

### Farming, timber, and exterior production campaign — 2026-07-13

The production integration added persistent exterior catnip/grain/herb plots with five visible
growth stages, deterministic logging, a staffed Mill (`grain → flour → food`), a staffed
Sawmill (`logs → lumber`), and lumber-first construction with legacy-plank fallback. A
three-seed deterministic guided campaign uses real `AssignOfficer`, `AssignWorker`,
`DesignateFarm`, `RequestJob(GatherLogs)`, `PlanBuilding`, `ClearFarm`, and `AdvanceTime`
actions; it does not delete jobs or assignments behind the action layer. Farm/Field placement
and logging reject the fixed settlement interior. Linked exterior Field claims remain visible
one tile at a time but use a one-game-minute job cadence so the then-current five-cat founding
roster was not consumed by a day-long boundary job. This is dated evidence for the production
slice, not the current founding contract.

The final four-crate gate passed all 905 tests (one intentionally skipped), including five seeds
over 200 game-hours with zero resets, strict Clippy, and formatting. Exact 1280×800 and
1920×1080 primary-window captures were inspected: Mill and Sawmill are distinct roofless
stations, three crop/stage plots sit beyond the wall, the HUD remains usable, and persistent map
labels remain absent. Normal server ticking starved the art client and `/health` at the time, so
the documented temporary tick bypass was used only for these frames and removed. The responsive
authoritative-server slice recorded below subsequently fixed and verified that runtime defect.

### Responsive authoritative server — 2026-07-13

The tick loop now performs CPU-heavy simulation and synchronous SQLite work through Tokio's
blocking pool while retaining the authoritative world lock for mutation ordering. A canonical
last-completed snapshot is built at startup and updated only after a completed tick, so a new
WebSocket can receive real state without waiting behind an in-progress simulation. Periodic
saves clone the completed world and release the world lock before taking the database lock;
missed intervals use `Skip` to avoid burst amplification.

A current-thread Tokio regression injects a 250 ms tick while the world lock is held and proves
both `/health` and the cached initial snapshot finish within 50 ms. On a normal fresh founding,
20 independent root probes measured health at 0.10–0.41 ms; the implementation branch measured
five initial 21,557-byte WebSocket snapshots at 0.10–0.45 ms, ordinary simulation at 6–22 ms,
and a save tick at roughly 23 ms. The first minute-rollover ticks in the independent debug run
occasionally took about 0.4–0.5 s but did not block liveness. The same slice backfills a missing
legacy `upgradeLevels` SQLite column and verifies a real save/load round trip. The independent
gate passed 28 server tests, strict Clippy, and formatting.

## Completion rule

An item moves to `verified` only when the behavior is reachable through the real player path,
its deterministic simulation tests pass, relevant server/client integration tests pass, and any
visual claim has been checked from the Bevy client's own framebuffer. Compiling alone is not
verification.
