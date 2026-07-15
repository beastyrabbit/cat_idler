# Idle Cat Forest implementation audit and fix tracker

Last updated: 2026-07-15

This is the working ledger for the post-cutover design audit and exhaustive playtest pass.
It records only the current Rust/Bevy game's promises and gaps. Documents explicitly marked
historical or superseded are reference material, not an implementation backlog.

Status key: `open`, `in progress`, `verified`, `deferred`.

## Active fixes

| Area | Finding | Status | Verification required |
| --- | --- | --- | --- |
| Shrine economy | Population-relative surplus gates make food/refined tithes and physically carried material offerings reachable without draining survival or construction reserves. Both faucets credit one canonical spendable blessing bank: its current balance drives fertility, god-purchases debit it immediately, cat research points remain separate, extinction preserves the remainder, and both the resource HUD and research ledger project the same value without inventing stockpiled blessings. | verified | Five-seed 200-game-hour unattended economy campaign; signed tithe and physical material-offering delivery; exact conception chance before/after credit and spending; no-double-credit, cat-research separation, snapshot, deterministic extinction-reset, reserve, cooldown, and event assertions |
| Storehouse reservoir | Founding seeds a finite 2×2 general storehouse inside the clear core. Legacy shrine inventory migrates into finite spatial storage, aggregate capacity follows real designated containers, and persisted transit ledgers reserve cargo without acting as map obstacles. | verified | Capacity/headroom boundaries, legacy migration, real designation in the guided farming campaign, nearest routing, death conservation, restart persistence, and deterministic long-horizon economy tests |
| Tools | The Forester-owned raw chain deterministically produces tools, protects construction/offering reserves, and tool stock increases maintained labor productivity through a bounded multiplier. | verified | Live-second funded chain, real quarry-return campaign, repeated two-tool bootstrap, reserve/pre-emption boundaries, field commissioning, and determinism twins |
| Research runtime | A player may spend research points on any affordable study in the complete 500-node catalog, while blessings remain a distinct original-node path. The living Leader may complete at most one affordable full-catalog study per rolling real-life day; research labor/building automation and rituals remain Loremaster-owned. Ownership, the colony-wide last-choice clock, typed modeled effects, and future-content registries persist. | verified | Exhaustive dependency-order purchase and payload resolution, signed unlimited manual purchases, living/dead/replacement authority, exact 24-hour boundary/reset/no-backlog behavior, legacy-column SQLite restart, crisis non-staffing, deterministic pacing, and truthful research-ledger copy |
| Established survival | The old five-cat population guardrail is superseded by the verified 15-adult founding contract. Five maintained role-established seeds survive 300 game-hours without artificial resource grants or unintended resets; ordinary and leader/healer old-age pacing is 240/288 game-hours. | verified | Five 300h established-office campaigns with byte-identical twins, population/housing/resource bounds, no unintended resets, focused birth/migration/aging/reset tests |
| Fresh idle viability | The always-present founding Leader keeps a new village alive with deficit-scaled primitive work capped at six hunts, two water trips, and one scout at 15 cats, scaled proportionally thereafter, before specialist offices exist. Vacant roles still leave farms, workshops, research, rituals, defense, and expansion manual. Role-vacancy cleanup deterministically preserves no more than those caps so physical trips can finish. | verified | The deterministic failing matrix had 8 collapses/48h. Three fixed hunts still starved at hour 5 and uncapped hunts starved the second tool cycle; the proportional ceiling passes three personal 48h one-second campaigns plus byte-identical twins, the 30-cat communal 48h campaign, tool regression, surplus-vacancy cleanup, specialist-leakage checks, and deliberate-collapse guardrails |
| Emergency water | Recovery is a real source→travel→carry→deposit job performed by a living cat, with deficit-scaled unique fetchers and runway-aware work pre-emption. No crisis phase grants free water. | verified | Threshold, uniqueness, pre-emption, no-free-mutation, physical cargo/deposit, live-cadence, five-seed survival, and determinism campaigns |
| Server responsiveness | Simulation, snapshot construction, and synchronous SQLite work run on Tokio's blocking pool. New sockets clone a startup-initialized last-completed snapshot; save ticks release the authoritative world lock before disk I/O; missed intervals skip rather than burst. | verified | One-worker injected 250 ms tick keeps health and initial snapshot under 50 ms; 28 server tests; live health/WS probes and tick/save timings; strict Clippy |
| Server security | Socket-bound identity, exhaustive mutation authentication, selected-colony routing, release-disabled test controls. | verified | Authentication matrix, signed manual/officer and two-player village campaigns, selected-colony isolation, release test-control denial, and strict Clippy |
| Web build | Release bundle uses same-origin WS; local serve retains explicit port 8787. | verified | Optimized Trunk build and combined-host live probes |
| Client reliability | Failed actions are visible and closed/error sockets reconnect with capped backoff. | verified | Focused action/reconnect tests and connected own-framebuffer smoke |
| Infinite map | Terrain/fog stream as a bounded camera-centered chunk cache. | verified | Normal and 80-tile-distant framebuffers plus loaded-chunk assertion |
| Officers/manual play | Steward, Accountant, Forester, Farmer, Captain, Loremaster, and Cloth Leader each own one specialist automation category. Beyond the founding Leader's bounded hunt/water/scout safety floor, a vacancy is manual-only; appointment requires the matching researched unlock and completed role station. Ownership and replacement persist, and the client exposes signed appointment/vacate and maintained manual-order paths. | verified | Seven-role vacancy matrix, staged 7→0 manual-frequency handoff, independent building/unlock denials, dead-holder succession, signed server isolation/authentication, client action tests, and deterministic guided campaigns |
| Manual controls | Every currently modeled management path has a signed client control: farm/gather/road painting, exact construction, staffing, clear/remove, governance, raw resources, expansion, defense/training, ritual/shrine, hauling, research, durable per-cat typed labor preferences, and the four physical processors' add/remove/reorder/repeat/pause queues. Preferences bias eligible matching without bypassing survival, liveness, busy, station, unlock, or officer gates. | verified | Deterministic guided actions, foreign-village denial, HMAC/SQLite restart, death durability, queue conservation/no-early-credit, exact placement/governance, exhaustive action coverage, current four-crate tests, and strict four-crate Clippy |
| Election schedule visibility | Automatic term elections remain authoritative in `cat-sim`; between open elections, the snapshot exposes the resolved term start, next election boundary, scaled term length, and nonnegative server-derived remaining time. The governance panel shows that countdown rather than implying that no election means no schedule. | verified | Exact boundary/overdue timing, acceleration scaling, deterministic snapshot projection, legacy SQLite backfill, persistence round trip, open-election display precedence, protocol compatibility, client text/layout tests, and accepted full-frame native governance capture |
| Manual raid defense | Each `DefendRaid` action applies exactly one `DEFEND_CLICK_DAMAGE`; the following tick performs terminal cleanup without replaying click telemetry or producing duplicate events. | verified | Per-click health assertions, killing-click cleanup on the next tick, exactly one repelled event, and guided multi-seed manual campaign |
| Spatial placement | Player and leader placement validates and commits atomically across terrain, claims, buildings, roads, stockpiles, gather spots, queued footprints, rendered 2×3 tree canopies, and 1×1 rocks. Buildings and trees are exact soft obstacles; water, mountains, and walls remain hard obstacles. | verified | Atomic placement/reservation campaigns, climate-decoration source parity, multi-cell footprint collision tests, soft-obstacle path tests, and clear-interior framebuffer |
| Staged wall growth | Expansion constructs a durable outer perimeter while the complete old enclosure and exact south gate remain authoritative. Finished prospective segments block travel; death, officer interruption, and restart preserve progress; completing every segment atomically cuts over to one new south gate and retires the old shared edge. | verified | Interrupted/resumed construction, closure, pathfinding, one-gate cutover, persistence, Sawmill compatibility, and accepted native before/during/after framebuffers with exact snapshot geometry |
| Agricultural territory | Claimed agricultural tiles are a distinct persisted/snapshotted subset excluded from settlement-wall derivation, so exterior farms remain outside both the active and prospective enclosure during expansion. | verified | Territory class, wall derivation, persistence, path behavior, and a persisted mature 3×3 exterior grain plot retained across the accepted wall-cutover framebuffers |
| Multi-village model | One canonical global village remains viewable by anonymous sockets and controllable by every authenticated player. Each stable signed identity can found exactly one deterministic distant personal village; ownership, selected routing, and foreign denial survive reconnect/restart. Explicit scout-delivery provenance creates mutual summary-only contact, after which owners can configure, propose, accept, or cancel bounded atomic resource barter without exposing or mutating a foreign private simulation. | verified | Deterministic site/collision properties, signed two-player found/join/denial/discovery/trade campaign, transactional SQLite rollback/restart/offer round trip, exact client controls, strict four-crate gates, and selected-village framebuffer |
| Shared-world depth | A durable `Communal` scale gives the ownerless global hub 30 adults, six five-bed Dens, a 19×19 clear core, doubled raw workshops/runway, finite storage, and FoodStorage/ResearchHut/Barracks civic buildings; `Personal` villages remain exact 15/3/13×13 settlements, including extinction recovery. Colonies still own duplicated mutable terrain; contact is a delivered summary and barter swaps scalar resources without physical cats, items, or routes. | in progress | Communal/personal deterministic blueprint, extinction, SQLite isolation, signed multiplayer, 48-hour staffed viability, selector/census, and all current overlaps are verified; implement authoritative shared spatial mutation, physical encounter, and item/caravan trade |
| Scout search behavior | Outbound resource/general missions follow deterministic knowledge-blind wander legs with bounded alternate-heading retries. A scout recognizes resources only after physical observation, changes direction or gives up on survey/deadline/route exhaustion, and returns with provisional notes; no hidden target exists before a genuine hit. | verified | 32-seed fast-wood bound, wander/change-direction/give-up/resource-hit assertions, target-none-before-observation, determinism/death/cancel/legacy-note campaigns, SQLite mid-search restart, and integrated Sawmill/wall/skills overlap gates |
| Founding fog progression | The founding Leader sends a very fast first wood search and later resource/general scouts even before a Loremaster office exists; knowledge still becomes permanent only on shrine contact. | verified | Seeds 7/42/20,240,712 pass the checked-in four-hour Leader-requester gate and exact 48h one-second passive campaigns with permanent fog growth while research/ritual work stays absent; seed 7 records nine completed Leader scouts. Optimized browser and signed fresh-native founding runs both verify Explore, physical shrine return, and permanent growth from the exact 289-tile baseline |
| Canonical production contract | P19 is the resource/taxonomy authority: Logs are raw timber, Planks fine boards, Lumber structural timber, Stone raw rock, Blocks dressed stone, Bone is a distinct hunt byproduct, and stable `materials`/`refined` IDs remain the generic Supplies/Crafted Supplies chain. Raw Stone and Bone are defaulted independently across save, wire, finite storage, trade, cargo, HUD, and private Accountant reports. Quarry returns three Stone loads plus rubble/Supplies and mountain-only Ore; hunts return three Food loads followed by Hide and Bone, with aggregate credit only after delivery. Stone Prep consumes Stone. P12 owns officer/manual logistics and P16 owns three placement-available founding benches whose future research gates recipes rather than extra copies. All eleven maintained station recipes now have stable data-owned IDs/resource domains/default queues and catalog-derived availability. The six remaining benches still execute `aggregate_timer_compatibility` rather than station-local one-worker queue work, and finished equipment still has scalar compatibility fields plus incomplete finite-item chains. | in progress | Physical source manifests, descriptor uniqueness/resource sets, selected-recipe availability, rules-v0/v1 metadata, unchanged aggregate timers, interruption/death/full-storage conservation, legacy persistence, trade, HUD, and signed/passive deterministic campaigns are verified. Convert all six remaining benches to physical one-worker queue execution; add Bone/item recipe breadth without aliasing; migrate equipment authority without double-counting. |
| Physical workshop logistics | Four processors are physical. A staffed Sawmill carries finite-store Logs through local input/work/output into Lumber; a Mill does the same for Grain and intermediate Flour before Flour/Food leave for finite storage; Workshop and Smelter physically route Materials→Refined and Ore→Metal. Aggregate credit waits for final delivery, queues and transit/local inventories persist, a destination that fills en route triggers physical retargeting, and death/removal conserve cargo at its real location. An appointed Steward creates exact-resource local reserves and balances input deficits before output surplus through one conserved route; vacancy leaves dormant contents rather than teleporting them. Other benches still use aggregate inputs/outputs. | in progress | All-four-station provenance/fairness, capacity, partial-headroom, nearest-route, no-early-credit, full-destination recovery, cancellation/death/removal, SQLite restart, live-cadence, queue, signed HMAC, cadence-partition, and determinism coverage are verified; extend the contract to every remaining workshop chain |
| Physical scaffold inputs | Player-exact and Leader construction atomically pin each type-local escalating cost from finite visible piles, preferring Lumber with Planks fallback plus Blocks. One assigned living builder carries bounded source→transit→scaffold loads; no timer or progress exists before full delivery and consumption occurs exactly once. Reservations protect the bill from other physical/scalar consumers. Death spills at the real tile, source loss replans, removal exposes orphan ledgers, blocked routes suspend without fallback movement, and legacy `None` contracts remain already funded. | verified | Player/Leader, split-pile/fallback, partial, source-loss, empty-paw and loaded two-minute block/reopen at one-/five-second cadence, death/reassignment/removal, ready-input replacement arrival, pinned speed, gather/Steward/scalar reservation safety, deterministic cadence, SQLite restart/legacy, signed HMAC completion, protocol/client inspector coverage, 1,139+42+75+130 touched-crate tests, strict Clippy, and accepted 2048×1152 selected-scaffold own-framebuffer |
| Workshop inspector | Mill, Sawmill, Workshop, and Smelter snapshots and the Bevy inspector expose the real worker, travel direction, editable queue, local input/output, inbound/outbound cargo, progress, and blocked reason. Steward-managed pile inspectors expose active/dormant ownership, resolved station type, route phase, worker, and blocked recovery; unselected managed zones keep their typed overlays/props without persistent text plaques. | verified | Four station persistence/queue paths and Steward protocol/client coverage pass; accepted own-framebuffers show Mill local/outbound Flour, Workshop Refined in route, Smelter Metal in route, and a selected Steward-managed Sawmill Logs pile with readable provenance and no nine-label overlap |
| Accountant rounds | A staffed Accounting Tent runs a persisted physical round: its assigned cat returns to the tent, visits each reachable spatial stockpile in deterministic distance/ID order, dwells for five game-seconds to count it, refreshes only that pile's report, and returns. Blocked piles remain stale and are retried after topology changes; death, reassignment, and work pre-emption cancel motion without mutating physical truth. An unbuilt, vacant, or unassigned office never receives a background recount. The aggregate ledger is derived from per-pile reports, while colony and spatial-stockpile displays mark stale estimates with `~` or `uncounted`. | verified | Separate/offsetting piles, no-count-before-contact, deterministic visit order and cached reachability, blocked/removed targets, topology changes, death/reassignment, 24-game-hour vacancy cadence twins, one-time aggregate-only JSON migration without current-stock sampling, SQLite restart plus signed assign/release, four-crate gates, and accepted 1920×1080 own-framebuffers showing active round/count progress and indefinitely stale estimates |
| Accountant wire confidentiality | Exact completed snapshots remain trusted server state. The sole socket projection replaces physical colony totals, duplicate threat equipment totals, and pile contents with the Accountant's last reports; uncounted piles project zero. Aggregate/per-pile equality attestations are cleared and omitted, while exact blessings remain visible as non-stockpiled divine currency. | verified | Authenticated personal-owner vacant/blocked/uncounted fixtures and whole-JSON sentinels cover initial cache, broadcast tick, signed post-action refresh, and reconnect without mutating the canonical cache. Legacy accuracy fields deserialize conservatively; signed Accountant restart/release, 42 protocol, 76 server, 130 client, strict Clippy, and format gates pass. Future offer/block metadata may expose an attempted action's result but must never copy exact hidden totals or become an equality oracle. |
| Forester replanting | The first positive logging extraction persists the exact generated-tree anchor as a stump and retains the active logging reservation until every conserved load finishes. A signed manual order or appointed Forester then routes one living worker from the shrine to an unclaimed mapped/revealed stump; thirty on-site game-minutes consume that finite coppice/root stock into a persisted visible sapling. After 24 game-hours the same deterministic mature tree returns only when its complete 2×3 footprint is clear. Stumps/saplings suppress mature logging and canopy art, authored occupancy cannot overwrite growth, and temporary obstruction retains the sapling for deterministic retry. | verified | Manual/vacant-office and bounded Forester ownership, exact route/arrival clock, active/queued logging exclusion, death/cancel, obstruction/retry, cadence partition, SQLite restart, deterministic mature-tree restoration, protocol/client actions, and accepted own-framebuffer stump/sapling/canopy-suppression evidence |
| Skills breadth | Nineteen typed labors cover hunting, fishing, building, ritual, fighting/training, quarrying, woodcutting, foraging, water, milling/processing/crafting, textiles, metalwork, farming, hauling, research, and scouting. Only truthful work accrues tick-size-independent or completed-cycle XP; bounded skill effects apply to the corresponding production, movement, research, and combat paths. Skills persist and the cat inspector exposes the typed map with legacy role-XP compatibility. | verified | Exhaustive labor-source/effect, no-work, continuous/cycle, officer/manual, persistence, protocol/UI, four-processor/fishing compatibility, determinism, the current full simulation gate, and strict `cat-sim` Clippy |
| Farm labor truth | Every plot requires a living assigned cat. The worker follows a hard-reachable shrine→gate→plot route, plants and tends only while present, harvests into bounded crop baskets, carries them to a plot-local handoff, and a real mover delivers them into finite accepting storage before aggregate credit. Manual assignment and Farmer automation share the same state machine; work grants typed Farm/Haul skill. | verified | Signed no-cheat player campaign and unattended established campaign; vacancy/emergency pre-emption, inaccessible route, partial/full storage, multi-basket split, local handoff, missing destination, death conservation, restart, gate relocation, deterministic twins, protocol/client inspector, and own-framebuffer proof |
| Production breadth | Deterministic exterior farm plots, logging, grain→flour→food, logs→lumber, fibre/hide→cloth/leather, ore→metal, maintained staffing, tool productivity, lumber-first construction, persistence, and type-local escalating scaffold costs are verified. Wood Cutter, Stone Prep, Woodworking, Clothier, Tannery, and Smithy remain aggregate/parallel-cycle benches rather than one-worker physical queues; broader recipes and material/item variants also remain. | in progress | Build/staff/produce every remaining building and recipe from player actions; give each remaining bench one selected physical queue recipe; retain pinned plan-time cost and escalation boundaries |
| Fine-biome resource ecology | All 26 fine biomes affect travel, and climate data declares fertility/mining/resource hints, but world resources are still largely projected from coarse legacy biome roles. Bone now has a physical hunt source and final haul, but downstream variants are incomplete; Gem, clay, and sand do not yet have complete physical sources and chains. | open | Generated 26-biome source matrix, expected presence/absence, scout discovery from actual sources, depletion/regrowth, gathering and hauling, and no coarse-role leakage |
| Research effect truth | The 500-node dependency graph, purchases, persistence, daily Leader choice, and 169 global scalar modifiers are live. The legacy `gatherYieldMult` scales explicit fibre forage, while `materialYieldMult` scales physical logging and quarry loads before deterministic trip splitting; neither changes job completion time or bypasses hauling/capacity. The durability multiplier is consumed by material-backed item repair. Food Storage, Water Bowl, and Smithy capacity modifiers now have target-correct physical domains shared by clamp, routing, snapshot, trade, and persistence; no building bonus leaks globally. Eleven maintained station recipe IDs have exact catalog-derived descriptor availability. Four preparation payloads enforce the physical Mill/Sawmill/Workshop/Smelter queues; Textiles enforces fibre→cloth and hide→leather, while Weaponsmithing and Armorsmithing independently enforce Smithy weapon and armor output across both aggregate forge arms. Fresh rules-v1 production cannot advance those eight paths before entitlement; legacy rules-v0 saves are grandfathered. Carpentry Staples, Stonecraft Preparation, and Toolmaking Preparation own the Wood Cutter, Stone Prep, and Woodworking descriptor queues, but their aggregate compatibility timers deliberately remain behavior-unchanged until physical conversion. Sawmill→Gather Logs is the sole validated job entitlement and drives both signed work and Forester automation; false founding/non-runtime job claims are gone. Research Hut plus the three P16 benches are explicitly founding-placeable; `milling` is the sole Mill placement unlock and generated Mill Foundations is durability only, all through one catalog-derived resolver. The bench markers coexist with their first durability modifiers and never require purchase. The other 93 generated recipe IDs, all 64 generated resource IDs, all 25 worker-slot modifiers, and the other 22 building-capacity modifiers remain incomplete; only 33 of 156 building modifiers have a target-correct observable consumer. Mill/Sawmill/Workshop/Smelter local stores remain fixed at 10. Owning every node therefore does not yet produce every promised gameplay effect. | in progress | Upgraded-vs-control forage/logging/quarry yields, physical multi-trip campaign, exact completion time, durability-aware repair, target-isolated capacity, exact trade placement, catalog-wide resolver equivalence, deterministic twins, eleven descriptor bindings/eight enforced production gates, unique job/building sources, exact Leader authority/boundary/restart behavior, and the exhaustive payload/consumer map are verified. Add only sourced recipe/resource breadth; then implement remaining storage/resource domains and real multi-worker staffing with signed, persistence, daily-choice, and player-visible campaigns. |
| Founding stockpile contract | P16's live finite personal-village mix is canonical at founding and extinction recovery: 50 food, 100 water, 16 herbs, 60 general materials, 10 planks, and 10 blocks, doubled for the communal blueprint; every other maintained scalar resource starts at zero. | verified | Exact 25-field aggregate and finite general-storehouse content assertions, exact communal 2× scaling, single-store/no-legacy-shrine and repeated-reconciliation checks, bit-exact physical-pile conservation, and deterministic personal/communal extinction recovery |
| Client visibility | All 25 maintained scalar resources have unique semantic icons, labels, paths, tints, and live values without aliases. Stockpile totals, inspectors, Goods groups, farm stages, cargo, Fish habitat state, stations, and skills are visible. Stone uses a tracked block/ingot glyph and Bone a tracked fish-skeleton glyph; no HUD resource borrows terrain, farm, furniture, or world-prop art. | verified | The exhaustive 25-entry protocol/HUD bijection resolves every PNG under `public/images/game/icons/`; identity/value/tooltip/cargo tests and the inspected exact 1024×768 client-owned framebuffer verify all rows without overlap or clipping. |
| Transport and fishing | The first physical shoreline route is verified: signed designation, Farmer-only automation, worker travel/work/return, work-only timing, finite fresh-Fish cargo transfers, typed Fishing/Haul skill, cancellation/death conservation, hard reachability, exact restart state, actual general-storehouse-footprint delivery, and guided/unattended determinism. A persisted habitat keyed by canonical water tile holds at most 24 fish and regenerates by 0.5 per game-hour. Only successful on-site catches deplete it; removing/repainting the shore never resets it. Cargo at the village anchor is not credited, a full local target retargets cargo to another accepting store, and only the unstorable remainder of source-less fresh cargo is abandoned at its final store so a living worker cannot deadlock; no catch becomes generic Food. All 26 fine-biome movement factors drive inverse-cost A* and physical travel, composed with roads and soft obstacles from one prewarmed per-tick chunk cache. Stable Rail/Shipping capabilities are blueprint entitlements only: Shipping cannot make an ordinary walker enter water, and Rail ownership cannot accelerate a long walk without physical transport. Tracks, trains, docks, vessels, boarding, and routes do not yet exist. | in progress | Ownership-on/off water A*, exact 50-tile physical walking equivalence, deterministic twins, stable catalog IDs, signed full-prerequisite player purchases, truthful ledger copy, generated-biome/road composition, live-cadence founding, communal unattended, and guided/unattended fishing campaigns verify the neutral blueprint boundary. The fishing campaign remains fixture-assisted rather than worldgen evidence. Build rail/ship routes and vehicles before activating transport effects. |
| Item and recipe breadth | The item taxonomy is broad, but live crafting covers a small wood/stone/cloth/leather subset. Raw Bone is sourced physically and persists/trades independently, while bone/gem/clay/metal item variants and finished tool/weapon/armor item chains remain incomplete. Stable scalar equipment fields remain compatibility state while finite item units own condition, so the eventual authority migration must not double-count goods. | open | Source and craft every maintained material/category combination; prove one finite-object authority across quality, inventory, use, repair, trader, protocol, persistence, and UI campaigns |
| Finite item condition | Every item unit has a stable ID, weight, maximum/current durability, work-driven wear, and a persistent broken state at zero condition. Signed repair requires the appropriate completed staffed workshop, a living worker, and one visible matching material; durability research scales the restored condition. Each signed caravan sale accepts at most 20 kg of item weight. The Goods panel shows per-unit weight, condition range, damaged/broken counts, and a repair affordance. This does not imply complete recipe/material breadth. | verified | Unit identity/migration, wear/breakage, staffed material-backed repair and denial cases, research effect, trader capacity, deterministic twins, signed guided campaign, SQLite restart, protocol compatibility, and accepted Goods own-framebuffer |
| Physical finite visiting trader | Each NPC merchant owns a deterministic reachable exterior and follows ordinary obstacle-aware A* through the retained gate to the physical shrine; closed routes wait and replan, Shipping never grants water traversal, and Rail remains neutral. Trading starts only on shrine contact and ends at an exact persisted deadline. A visit carries a deterministic finite resource manifest, finite purse, 100 kg wagon, and exact stable item units bought from the colony. Purchases deplete exact stock and expose sold-out truth; sales conserve item identity and stop at purse/cargo capacity. Exact transition times are tick-partition invariant and never backdate travel after a blocked due boundary. Expansion revalidates and deterministically rehomes a now-claimed exterior. Arrival, stay, stock, purse, item cargo, departure target, blocked-route observation state, and visit counter survive SQLite restart, and the wagon is removed only after reaching a valid exterior. The fixed-height client panel opens only at shrine contact, pages all craft offers, and derives storage-block guidance solely from Accountant reports rather than private exact headroom. | verified | Route/passability/closure/reopen/contact/cadence/determinism tests; live/minute/hour/coarse partition twins; expansion rehoming; 5-seed × 3-colony × 1,024-visit cargo matrix; exact buy/sell conservation and sold-out tests; mid-phase SQLite restart; signed HMAC sale, purchase, restart, depletion, and denial; protocol/client phase/route/deadline/manifest/pagination/privacy tests; a seed-41 live-cadence 60h passive twin; 1,153 simulation tests; and 80 server tests verify behavior. The accepted client-owned 1024×768 logical framebuffer `/tmp/trader-physical-1024.png` shows the merchant at the shrine, page 2/2, finite stock, Food sold out, bounded controls, and report-derived storage guidance. |
| Physical road construction | Authored stone-road placement, shrine-network attachment, mapped-terrain validation, disjoint road surfaces, and movement effects are verified. The build action still paints every new tile and subtracts aggregate Materials immediately without a worker, carried load, or construction phase. | open (P2) | If promoted beyond current P16 acceptance, preserve exact placement/connectivity/surface rules while adding a conserved worker/cargo/build route with cancellation, obstruction, restart, and visual proof |
| Adventure UI skin | Tracked Adventure art drives sliced parchment, dark, and ornate panels; default, hovered, pressed, active, and disabled buttons; framed resource pills; need bars; the minimap ring; and pointer, interact, pressed, target, and disabled custom cursors across the HUD and research ledger. Native and optimized WASM framebuffers are verified at 1024×768, 1280×800, and 1920×1080; responsive wrapping and clipped research edge-pan behavior are covered. | verified | 117 client/web tests, native and wasm32 strict Clippy, decoded own-framebuffer/browser captures, a fresh signed personal-village Explore/shrine-return campaign, and exact optimized transfer measurements |
| Visual truth | Persistent map plaques are removed. Residential rooms retain roof silhouettes; every maintained building, including the Accounting Tent, has a tested typed open/roofed composition, and the fixed core hides procedural nature props. The maintained Adventure skin is native- and WASM-framebuffer verified. | verified | A legal 15/15 native village retains all three Dens while rendering the Accounting Tent as a separate open ledger station; accepted staged-wall and exterior-farm framebuffers close the integrated world gate |
| CI/hosting | Forgejo quality workflow and combined non-root server/WASM image are committed. | in progress | First pushed CI run remains; hosting live probes and deployment docs are verified |

## Player playtest feedback — 2026-07-13

These are current product requirements discovered by playing the native client. They remain
open until the real player path and, where applicable, Bevy framebuffers prove them.

| Feedback | Required behavior | Status | Verification required |
| --- | --- | --- | --- |
| Fog of war and scouting | A new village reveals its exact 13×13 claim plus a two-tile halo. Only purposeful scouts lift provisional fog; wood/food/water/stone and general missions commit knowledge only after a living scout physically returns to the shrine. The first Leader-dispatched wood mission is a bounded fast round trip even before a Loremaster exists. In-flight notebooks persist across SQLite restarts. | verified | Search/travel/return, 32-seed explicitly dispatched wood, determinism, death/cancel/restart, signed controls, exact 48-hour personal/communal campaigns, and optimized browser plus signed fresh-native own-framebuffers verify autonomous dispatch and permanent reveal only after shrine return |
| Open workshops | Houses retain roofs, while all current workshops and non-residential stations use explicit top-down floors and function-readable props. The shrine uses an altar, reliquary, candelabra, and brazier; fields use soil/crops rather than a market facade. Mill and Sawmill are distinct, and Accounting Tent has a tested open composition. | verified | Normal+dense own-framebuffer captures for the prior 24 variants at exact 1024×768, 1280×800, and 1920×1080; exhaustive 25-building visual grammar tests; integrated legal Accounting Tent framebuffer |
| Building labels | Persistent map-name plaques are gone; hover/click inspector names remain available. | verified | Label-free normal and dense client framebuffers at 1024×768, 1280×800, and 1920×1080 |
| Inspector render isolation | Dynamically opening the right-hand cat inspector must never clip the world render. Its panel explicitly uses visible overflow instead of the generic clipped-panel scissor. | verified | A 1920×1080 selected-cat framebuffer initially reproduced a black world outside a narrow strip; unchanged camera/fog/chunk counts isolated the inspector scissor, and a second own-framebuffer capture after the fix retained the full Grand Commons while showing skills and exact labor controls; regression test plus 115 client tests and strict Clippy |
| Clear village interior | Founding clears every settlement tile in authoritative simulation state: meadow ground, zero deposits/forage/ore/water/danger/wear, and no tree/rock overlay. Water is guaranteed outside the south wall. Persisted agricultural claims remain a distinct exterior subset excluded from wall derivation rather than becoming interior parcels. | verified | Multi-seed founding/expansion clearing, territory persistence, wall derivation, same-seed streamed-ground resynchronization, and accepted founding plus staged-expansion framebuffers |
| Global and personal villages | The secure foundation is live: one larger ownerless 30-cat/six-Den communal hub; one exact 15-cat/three-Den owner-only personal village per stable identity at a deterministic distant site; restart-persistent selection; returned-scout contact; and signed bounded barter. Durable scale keeps founding and extinction profiles distinct. Mutable terrain is still duplicated per colony, and meeting/trade are not physical. | in progress | Scale/ownership/privacy/discovery/barter, SQLite isolation, signed two-player control, deterministic 48-hour communal viability, and selector/census are verified; add shared spatial-state rules, physical meeting, and item/caravan trade |
| Sprite review tool | [`docs/sprite-review.html`](sprite-review.html) compares current art with three persisted/exportable proposals for all 22 current buildings plus Accounting Tent, Mill, Sawmill, and a global hall/market concept. | verified | Desktop/mobile browser runs: 26 rows, filters, favorites, reload persistence, path copying, JSON export, zero page/image errors |
| Research tree scale and UX | The complete 500-node catalog provides exactly 167 building, 167 recipe/resource, and 166 upgrade nodes across named families, with stable IDs, typed payloads, AND prerequisites, and layout coordinates. Every study is purchasable with research points and persists through SQLite. The living Leader deterministically chooses at most one affordable study per rolling real-life day without taking research labor or rituals from the Loremaster. The full-page client ledger renders and purchases the graph, labels the deterministic hint as “Leader priority,” gives maintained recipes human names, and keeps legacy blessing commissions limited to original nodes. Eleven maintained recipe descriptors, the sole Sawmill logging entitlement, and founding/researched building placement are authoritative; eight recipes enforce production today and three behavior-neutral descriptor queues await physical conversion. The remaining future-content registries are not yet gameplay. | verified | Exhaustive 500/500 dependency-order purchase, signed server action, legacy-column SQLite boundary restart, deterministic Leader selection/replacement/reset, research UI and framebuffer, eleven descriptor bindings/eight production gates, unique runtime job/building sources, and strict sim/server/client gates |
| Founding population and housing | Founding creates exactly 15 adult cats and three complete five-bed Dens. Slow pregnancy starts after establishment and reserves a permanent bed through 18 game-hours of gestation. Prosperity migration begins after 30 game-hours and checks every 12 hours. Each migrant visibly walks from a persisted dry exterior origin through the authoritative south gate before becoming a resident; only then does its 36-game-hour housing probation begin. An expired unhoused migrant releases work/cargo ownership, physically returns through the current gate, and is removed only at the exterior. Once breeding is established, permanent migration leaves the last real vacancy for a family unless a pregnancy already owns it. Extinction atomically restores the founding state with run-scoped identities. | verified | Five-seed 300h twins, signed guided Den campaign, family-vacancy/migration tests, blocked/reopened and relocated gates, water/mountain passability, cadence partition, cargo conservation, persistence/restart, exact 15/15 plus selected probation and arriving-status framebuffers, independent review, and four-crate gates |
| Roads | Authored stone roads and traffic-formed dirt roads are disjoint snapshot surfaces with cool-stone/warm-earth rendering. Stone roads move at 175%, dirt paths at 105%; mountain, cave, water, and authored stone ground cannot auto-form dirt. The founding cross connects the shrine to its single south gate and exterior approach. | verified | Wear-threshold and forbidden-surface boundaries, protocol compatibility, connectivity tests, and inspected own-framebuffer proof |
| Scout search semantics | Resource and exploration scouts search by deterministic knowledge-blind wander, only detect physically observed targets, change heading with bounded retries, and return successfully or empty when the mission ends; knowledge remains provisional until shrine contact. | verified | No-oracle target assertions, 32-seed first-wood bound, unsuccessful return, direction changes, determinism, death/cancel, and SQLite restart coverage |

## Current design-document traceability

| Document | Implementation status | Follow-up |
| --- | --- | --- |
| `docs/GAME_VISION.md` | current intent, partially implemented | Seven-role ownership, exact controls, Sawmill queue, physical farm labor, the complete 500-node research catalog/purchase/persistence/client ledger, and the distinct larger communal blueprint are verified; finish the unconsumed research payloads, broader physical workshop logistics, and authoritative shared-space/physical-trade depth |
| `docs/ARCHITECTURE.md` | reconciled | Keep phase/action/test inventories and physical four-processor/Steward/Accountant behavior synchronized with code |
| `docs/HANDOFF.md` | reconciled | Keep NEXT STEPS and verified evidence synchronized here |
| `docs/migration/BOARD.md` | core migration complete, expansion rollup reconciled | Close partial P12–P19 slices only after their feature campaigns |
| `p12-idle-cat-forest.md` | partial | Seven specialist officer roles plus the bounded Leader safety floor, role-building gates, signed manual work, shrine reachability, useful tools/costs, physical and wire-confidential Accountant reporting, farm/Mill/Sawmill/Workshop/Smelter work, Steward-managed local reserves, all 19 maintained skill gain/effect/UI paths, and physical Forester replanting/regrowth are verified; finish the remaining physical benches and local inventories under the P19-canonical source contract |
| `p14-spatial-placement.md` | verified placement slice | Atomic placement/reservations/scaffold recovery, finite delivered scaffold inputs, full occupancy, soft obstacles, exact roads, exterior agricultural claims, and staged atomic wall/gate cutover are verified in code and accepted native framebuffers. Physical authored-road labor is a P2 consistency enhancement under current P16 wording |
| `p15-playtest-feedback.md` | partial | Roads, exact footprints/depth, coordinate placement, governance plus between-term election timing, knowledge-blind shrine-return scouting in native and browser clients, the exact 48-hour baseline Leader safety-floor campaigns, and secure village foundations are verified; physical shared-world depth and broader station-local production remain |
| `p16-village-blueprint.md` | partial | The 15-adult/three-five-bed-Den lifecycle, canonical founding store mix, one logical 16px-art tile per authoritative grid cell, founding clearing, exterior water, exact roads/gate, selectable/removable gather controls, physical shoreline fishing, exterior agricultural territory, and research-free placement of Wood Cutter, Stone Prep, and Woodworking are verified; those benches retain their open-top identities but still need P19-canonical physical inputs, queues, and outputs |
| `p17-biome-generator.md` | simulation-heavy, product partial | Fine-biome movement is live; expose remaining biome resources/logistics and replace placeholder rail/shipping with real transport |
| `p18-visual-polish.md` | verified | Open stations, an integrated Accounting Tent, the label-free map, staged walls/exterior agriculture, the Adventure panel/button/progress/cursor skin, and unique semantic tracked glyphs for all 25 resource readouts are verified in native and optimized WASM campaigns; the exact 1024×768 semantic-HUD frame has no overlap or clipping |
| `p19-items-materials-trade.md` | canonical taxonomy, implementation partial | Its production table now resolves P12/P16 terminology while preserving stable IDs and every existing open-top station. Raw Stone and Bone are distinct defaulted physical cargo across save/wire/storage/trade/HUD; quarry rubble, mountain Ore, hunt Hide/Bone, finite fresh Fish, physical scaffold delivery, weighted item-unit wear/break/material-repair, and the physical finite shrine trader with exact controls are live. Remaining work is the six station-local queues, Bone and broader material/item recipes, finite functional-equipment authority, deeper inventories, and transport. |
| `docs/migration/WASM.md` | development and production packaging work | Optional transfer/performance campaign remains |
| `README.md`, `CLAUDE.md`, and `docs/assets/*.md` | reconciled | Keep cutover/CI/labels/WASM claims and current top-down Mill/Sawmill/cat-sheet selections synchronized with verified behavior |

The following are explicitly historical/superseded and do not create open features:
`ENGINE_FRONTEND.md`, `ENGINE_PLATFORM.md`, `LEADER_AI_DESIGN.md`, `ROADMAP.md`, `TASKS.md`,
`TERRAIN_DESIGN.md`, `TESTING.md`, `UI_CONCEPTS.md`, `plan.md`, and the old browser campaign
documents. `ENGINE_MCP_EVALUATION.md` carries the same historical banner.

The maintained expansion specs' stale fog, agricultural-territory, fishing, and founding-loadout
sentences were reconciled on 2026-07-14; do not reopen the shipped work from older revisions.

### Original TypeScript-design reconciliation

The frozen implementation and original documents remain on `archive/web-game` at tag
`web-final`; maintained copies of the design rationale remain under `docs/`. They were
checked by requirement group so “historical” does not conceal a dropped current promise:

| Original design group | Current disposition |
| --- | --- |
| Needs, autonomous survival, aging, genetics, breeding, skills, specialization, leaders, elections, raids, death, and extinction recovery | Carried at the world-tick level into deterministic `cat-sim`; shared colony food/water currently restore cats abstractly, rather than cats physically seeking food/drink/sleep. The current design deliberately replaces the prototype's 48/57.6-hour old-age thresholds with 240/288 hours and replaces its five-cat recovery roster with the 15-adult/three-Den invariant. Balance and role-aware guided campaigns remain part of this audit. |
| Dynamic colony grid plus a fixed 16×16 world map | Superseded by one flat, streamed, effectively infinite world containing multiple villages |
| Fog, expeditions, path wear, terrain travel cost, walls, and gates | Carried forward and verified; shrine-return resource/general scout movement, dirt/stone roads, fresh baseline Leader dispatch, and staged closed-perimeter expansion with an atomic one-gate cutover are implemented and native-framebuffer checked |
| Click-to-feed/heal/assign/fight and a browser task queue | Superseded by signed typed management actions and the verified seven-role manual-to-officer loop; coordinate placement, governance, variant/removal, durable per-cat labor preferences, and editable queue controls for all four physical processors are live |
| Blessings, buildings, and the original ~18-node research tree | Carried forward and expanded: active shrine faucets, useful tools, escalating costs, all 500 research-point purchases, deterministic Leader-owned full-catalog daily selection, persistence, and modeled effects are verified. The durability payload affects real item repair, four preparation studies gate physical recipes, and building-placement claims are authoritative. Research labor/building automation and rituals remain Loremaster-owned. Extra worker-slot, resource, recipe, and job records remain open where no corresponding runtime system exists. |
| Multiple colonies and inter-colony trade | Carried forward and extended: a larger durable communal hub, exact personal foundings, secure ownership, shrine-knowledge contact, and direct consensual resource barter are live alongside visiting traders. Shared mutable terrain and physical meeting/item/caravan trade remain open. |
| External sprite-render service, DOM/Pixi rendering, isometric/elevation experiments, and newspaper UI | Superseded or explicitly dropped by the Rust/Bevy top-down direction |
| Seasonal events, achievements, accessories, sound/music, and a mobile app | Listed only as non-MVP future ideas in the original document; not current commitments unless promoted into `GAME_VISION.md` |
| Historical roadmap stretch items such as traveler interception and elevation-aware zones | Not current commitments. Finite-source physical fishing was later promoted by P16/P19 and is verified above; real rail/ship transport was promoted by P17/P19 and remains open. |

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
Signed player actions and the founding Leader can dispatch general, wood, food, water, or stone
scouts; an established Loremaster owns broader research and ritual policy while the baseline
Leader retains deficit-driven exploration;
their per-cat notebooks remain provisional until the living cat physically reaches the shrine,
and are discarded on death/cancellation. The notebooks, mission, destination, and permanent map
round-trip through SQLite, with a legacy empty default.

The founding-wood guardrail covers 31 contiguous seeds plus the production seed that originally
took more than eight real minutes; every scout returns within 180 live seconds, with four
byte-identical determinism twins. Those tests explicitly dispatch or establish the office and
therefore do not prove fresh baseline Leader dispatch; that missing pre-Loremaster trigger is now
tracked separately above. The focused gate passed 26 scout tests and 119 server/client
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

Prosperity migration is now spatial rather than a census-only state change. A cohort starts at one
deterministic persisted dry tile beyond the south gate and must follow the same authoritative A*
wall topology as every other walker. `Arriving` and `Departing` cats remain visible but cannot
consume, work, research, trigger population gates, vote, fight, breed, or claim a bed. Physical
entry emits the sole arrival event and starts probation; expiry first conserves carried goods and
releases jobs/roles, then physical exterior arrival emits the sole departure event and removes the
cat. Blocked and relocated gates, cadence partitions, extinction, and SQLite restart preserve the
journey without enabling Shipping water-walking or Rail speed.

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
derived decoration cache. Staged outer-wall construction and exterior agricultural claims landed
after that earlier capture and are closed by the native campaign below.

### Native fog, station, and wall-cutover closeout — 2026-07-14

A new signed identity founded `Postfix Moss Hollow` through the production WebSocket path. Its
first authoritative snapshot contained exactly 15 adult cats, three complete Dens, 15/15 housing,
and the 289-tile founding reveal. The founding Leader completed autonomous exploration; permanent
terrain expanded only after physical shrine contact. The accepted native framebuffer records two
returned-scout dispatches carrying 128 and 110 newly mapped tiles and visibly shows the committed
known region beyond the wall. Browser and native now both prove the founding fog contract.

That distant personal village exposed three renderer bugs which focused tests now guard. Y-sort
depth is relative to the selected shrine rather than absolute world Y, so cats and 2×3 nature
props cannot rise above the fog plane at far coordinates. A tree or rock renders only when its
whole footprint is permanently known, and settlement claims suppress procedural decoration while
agricultural claims remain natural exterior ground. Authoritative cleared settlement meadow also
overrides generated water/snow tints, including when an already-streamed same-seed chunk changes
selection or gains a claim.

The spatial visual fixture used a real persisted personal colony, retained all three founding
Dens and 15/15 housing, granted the normal Accounting prerequisites, and added a separate legal
2×3 Accounting Tent beside the connected north road. Its accepted framebuffer shows the open
ledger/desk station without sacrificing housing. A mature 3×3 grain plot remains in the distinct
agricultural subset east of every active and prospective settlement wall.

The accepted wall sequence records exact authoritative geometry. Before work there are 55
complete effective edges. During the south-gate expansion, the old gate at
`(-713,3277,S)` and its complete perimeter remain authoritative while one new amber E face at the
still-unclaimed `(-713,3278)` raises the effective count to 56. After atomic completion the target
is claimed, no edge is marked under construction, its E/S/W outer faces are complete, the former
shared `(-713,3277,S)` face is absent, and the sole gate is the south edge at
`(-712,3277,S)`; the final effective count is 57. The exterior farm remains outside throughout.

Accepted local artifacts are `/tmp/cat-native-fresh-founding2.png`,
`/tmp/cat-native-wall-before-legal.png`, `/tmp/cat-native-wall-during-south-focus4.png`, and
`/tmp/cat-native-wall-after-south-focus2.png`. Rejected scissor-corrupted frames were not used.
Every screenshot and camera-focus system was temporary and removed before the final 120/120
client tests, strict all-target Clippy, and formatting gate.

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

All seven maintained offices now have exclusive specialist automation ownership: Steward,
Accountant, Forester, Farmer, Captain, Loremaster, and Cloth Leader. The founding Leader's bounded
hunt/water/scout safety floor is the only vacancy exception. With every office vacant, three seeds
survived 30 production-cadence real minutes plus a bounded accelerated continuation under repeated
signed manual guidance; the same longitudinal states completed research-point purchases, staffed a
Research Hut, paid both kinds of tithe, carried a material offering to the shrine, planned an
Accounting Tent, trained a warrior, and resolved a raid at exactly six damage per click. A separate
staged handoff proved manual-order frequency falls exactly `7, 6, 5, 4, 3, 2, 1, 0` as role stations
and their researched prerequisites are satisfied and offices are appointed one by one. Missing
prerequisites fail independently, dead holders receive deterministic living successors, and poor
guidance has bounded, visible, byte-identical consequences rather than being silently corrected by
vacant specialist automation.

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
Loremaster boundary. That balance slice's full `cat-sim` gate passed **906/906** tests with strict
Clippy and formatting. The later complete-research integration passed all **909** simulation tests,
500/500 dependency-order purchase, per-payload resolution, signed server actions, SQLite round
trips, client UI tests, and strict sim/server/client Clippy. The later physical-Sawmill slice
passed 913/913 simulation tests plus protocol/server/client gates; broader station-local logistics
remain tracked separately above. Staged walls and the then-current 18-labor skills raised the integrated
simulation gate to 921/921. A fresh post-integration 1920×1080 own-framebuffer capture against a
new SQLite database visibly reconfirmed the connected 15/15 founding village, finite storehouse,
roofed Dens, open stations, label-free map, stone/dirt roads, fog, exterior resources, and one
south gate. An initial capture using the server base URL without `/ws` was correctly rejected as a
404/black-world failure; the accepted PNG used `ws://127.0.0.1:8787/ws`. The temporary capture hook,
window override, server, and client were removed/stopped before the 113/113 client, strict Clippy,
and formatting gate.

### Canonical blessing fertility and legacy yield effects — 2026-07-14

The maintained blessing semantic is the current spendable god-currency balance, not a new
lifetime-earned counter. This follows the archived roadmap's “one tree, two ways to advance,” the
archived server paths that credit tithes and physically delivered ritual goods into
`globalUpgradePoints` and debit that value for god purchases, and P12.6's explicit definition of
the same balance as the god currency. The archived conception integration instead read the
unrelated `resources.blessings` scalar, which shrine work never credited. The Rust simulation now
uses `global_upgrade_points` for conception: earning a blessing raises the exact chance, spending
it lowers the chance immediately, cat-earned research points do neither, and extinction recovery
preserves the remaining balance. Colony resource and research snapshots expose the same canonical
value; physical stockpile/Accountant contents remain zero because blessings are not stored goods.

The two resolved legacy yield fields that had no work consumer are also live. Foraging Lore's
`gatherYieldMult` increases only the explicit fibre-forage reward. The Sawmill node's
`materialYieldMult` increases timber and quarry-material loads before the ordinary three-trip
split; quarry climate rules, cat skill, tool/haul composition, finite storage, and physical
delivery still apply. Neither effect shortens the job. Focused upgraded/control tests, a physical
logging-plus-quarry trip campaign and byte-identical twin, and an exhaustive consumer map cover
all legacy scalar effect keys. The complete simulation gate passed 987 tests with one intentional
skip.

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

### Fresh strict-officer live cadence — 2026-07-14

After the exact officer split and 15-cat founding landed, seeds 7, 42, and 20,240,712 each ran
48 game-hours at the real one-second cadence followed by a byte-identical silent repeat. All
three produced the same failing trajectory: 15 living cats, zero births/deaths, all cats idle for
the sampled 48 hours, and **eight unattended-collapse resets** near hours 3, 8, 14, 20, 25, 31,
37, and 42. Only one crisis job completed; no field, tool, item, shrine, raid, or research
lifecycle ran. This is not the expected consequence of specialist vacancies: it proves the strict
filter also removed the founding Leader's primitive safety floor.

The original binary's feature list and no-birth warning were stale for a full 15/15 founding and
are not used as gameplay evidence. The reset schedule, idle duration, resource samples, and
deterministic repeats are the accepted failure evidence. The harness now separates fresh passive
expectations from established guided features, supports `COMMUNAL=1`, and records permanent fog
growth plus shrine deliveries. Raw logs are `/tmp/cat-playtest-{7,42,20240712}.log`. Rerun this
exact matrix after restoring bounded Leader Hunt/FetchWater/Scout goals.

The focused repair now lets the baseline Leader allocate only deficit-scaled Hunt, FetchWater, and
Scout slots, capped at six, two, and one respectively at 15 cats and scaled proportionally beyond
that population. Vacancy cleanup preserves the same oldest
bounded set instead of cancelling every trip once per tick; Farmer
forage and Loremaster ritual jobs remain cancellable specialist work. Seeds 7, 42, and 20,240,712
then each crossed the original collapse boundary through the checked-in four-hour gate and an
external eight-hour true-live campaign with no reset, all three primitive job kinds observed,
permanent fog growth after shrine return, zero research
points, and no ritual/offering jobs. The signed guided campaign also proves a player can request
specialist fibre work while the Farmer office is vacant.

The complete personal matrix then ran 48 game-hours at the exact one-second cadence with zero
births, deaths, or resets on all three seeds; independent repeats were byte-identical. Each run
completed physical Leader hunts and water trips, expanded permanent knowledge through shrine
returns, and kept every specialist system dormant. A separate 30-cat communal 48-hour campaign
also passed with proportional 12/4/2 ceilings, 55 completed hunts, 41 completed water trips, and
18 discoveries. The rebuilt harness's final seed-7 run recorded 26 completed hunts, 22 completed
water trips, nine completed Leader scouts, and reveal growth from 289 to 1,299 tiles. Raw personal
twins are `/tmp/cat-playtest-dynamic-{7,42,20240712}-{a,b}.log`; communal evidence is
`/tmp/cat-playtest-dynamic-communal-7-a.log`; final requester evidence is
`/tmp/cat-playtest-final-7.log`. Optimized browser founding now verifies Explore, shrine return,
and permanent reveal growth. The later signed native campaign above closes the matching native
founding gate with two physical shrine returns.

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
and the then-current generated-node pending state remained readable at the required sizes. All
generated nodes are actionable now; the integrated purchasable-state framebuffer is tracked as a
follow-up visual gate. A first capture with an empty canvas was rejected; its centre-origin scaling
bug was corrected and covered by a transform test before the three accepted captures. All
temporary capture/window systems and compositor processes were removed before the 80-test
client gate, strict Clippy, and formatting checks.

### Optimized WASM founding and responsive UI — 2026-07-14

The exact `wasm-opt -Oz` artifact was served against a booted authoritative server. A fresh
signed personal village (`village-84aec654`) opened with 15 cats; Explore was observed, a scout
physically returned to the shrine with 18 newly mapped tiles, and permanent reveal grew from
289 to 394. Decoded client-owned/browser captures were inspected at 1024×768, 1280×800, and
1920×1080. The four-row narrow toolbar remained usable, custom cursors loaded without Winit
failure, and the 500-node research ledger clipped cards/connectors correctly while edge-panned.
The browser reported no console or asset-request errors.

The integrated gate passed 117 client/web tests plus strict native and wasm32 Clippy. The final
WASM payload is 29,730,887 bytes raw, 9,087,074 bytes with gzip `-9`, and 5,444,134 bytes with
Brotli quality 11. Temporary screenshot hooks and diagnostic build settings were removed before
commit.

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

### Physical farm labor and handoff campaign — 2026-07-14

Exterior farming now advances only through a living assigned worker's physical state machine.
The cat follows a hard-reachable shrine→retained-gate→plot route, works on site through planting,
tending, and harvest, then moves at most eight crop units per basket to a plot-adjacent temporary
handoff. A Steward mover carries the local backlog onward to finite compatible storage; crop does
not enter the aggregate ledger before that final delivery. Vacancy and food-emergency pre-emption
release the plot cleanly. Partial storage headroom, full output, repeated baskets, a dying carrier,
a missing destination, and changed wall/gate geometry conserve the exact uncredited crop.

The signed no-cheat player campaign starts from a fresh founding and uses only ordinary client
actions to establish a Field, designate reachable exterior grain ground, assign labor while the
Farmer office is vacant, and observe a physical harvest. Its deterministic twin reaches the same
state without direct simulation mutation. An independent established 48-game-hour unattended
campaign verifies Farmer automation. The integrated gate passed 1,019 simulation tests plus one
intentional skip, 35 protocol tests, 54 server tests, and 120 client tests, with strict Clippy on
all four crates and formatting clean.

A booted authoritative server supplied a 3×3 flowering grain plot south of the closed wall for
the visual acceptance frame. The native client captured its own exact 1920×1080 primary-window
framebuffer to `/tmp/cc.png`; decoded-pixel inspection clearly showed the full exterior plot, its
assigned pink worker, the green grain cargo glyph, and a separate adjacent handoff marker. The
simultaneously open inspector reported `Flowering · Hauling`, worker `colony-1-cat-1`, destination
`8,18`, `output: grain 10.0`, and `output_in_transit`. The temporary scene, camera, screenshot, and
tick-pause hooks were removed; the client gate was rerun afterward.

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
