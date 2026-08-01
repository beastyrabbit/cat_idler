# Idle Cat Forest fix log

This is the maintained, evidence-backed log for fixes found during design review and player-guided
or unattended playtesting. The queue records reproduced problems before work begins. Move an item
to the verified log only after its behavior, persistence boundary, relevant Rust quality gates,
and any changed Bevy visuals have been verified.

## Open fix queue

No reproduced client-layout finding remains open after the 2026-07-22 unified research-tree and
feature-merge gate.

## Playtest cadence

**Current state:** the combined generalized gate is verified.

Focused tests run while a change is being built only as fast safety checks. They do not close a
finding and they are not counted as a playtest. Several related fixes are integrated first; then
one generalized campaign checks the whole game rather than replaying only the feature just changed.
That campaign must combine no-input controls with signed player guidance and cover survival,
housing/migration, connected roads and movement, production chains, research, shrine-return fog,
trade, expansion, persistence, and cross-feature conservation. For client changes it waits several
stable render frames after each state transition, then captures multiple frames from the booted
Bevy client's own framebuffer and inspects the complete scene,
including one body per authoritative cat and every visible road junction—not only the panel or
sprite that motivated the change.

## Verified fixes

## 2026-07-24 — Entry screen stages a mature showcase village

**Problem:** The old loading screen was one oversized parchment form. Input focus was a subtle
border change, selected destinations were easy to miss, server readiness was hidden behind failed
submissions, returning players could not see their remembered village, and compact layouts could
clip the required action. Showing the player's new founding village behind the charter was
truthful but visually communicated an empty beginning rather than the long-term fantasy.

**Fix:** `landing_showcase.rs` now owns a deterministic, client-only "two years later" settlement
at a dedicated off-map coordinate. It reuses the shipped grass, flower, road-autotile, wall,
residential, cutaway-station, prop, cat-atlas, hat, animation, and depth grammar without creating a
simulation snapshot. The 69×43-tile scene contains 42 authored lots across more than 18 building
types, four farm districts, warehouse yards, civic plazas, orchards, two gates, over 400 connected
street tiles, and 60 independently routed cats. It fills the 72-tile landing overview at both
1920×1080 and 3840×2160. Entering the game moves the camera back to the authoritative village and
stops showcase traffic; no showcase entity can send an action, mutate `LatestSnapshot`, advance
`cat-sim`, persist, or affect the player save.

The banner now contains only `IDLE CAT FOREST` in one explicitly bounded title row. Focus uses an
accent border, fill, label, rail, and visible cursor.
Destination cards show real village names and populations, an `AUSGEWÄHLT` badge, and an explicit
selection summary. Persisted destinations are preselected only after the live snapshot confirms
them and never auto-enter. Loading and every incomplete state have adjacent helper copy and a
disabled primary action. Compact layouts stack cards, give the body cursor-bounded wheel ownership,
and keep the helper/action in a fixed footer.

**Evidence:** Eight focused start-screen tests, three mature-showcase structure tests, and all 189
`cat-client` library tests pass. Strict
Clippy passes for `cat-client`, `cat-sim`, `cat-protocol`, and `cat-server`; formatting, the diff
check, all 76 workspace smoke tests, and the final optimized WASM build pass. Playwright exercised
the fresh development bundle at 3840×2160 and 1920×1080 against isolated custom ports. Captures
verified the full-screen mature settlement, title-only banner at 100% and 130% interface scale,
resolution-stable framing, absent gameplay chrome, and visibly changing cat traffic.
The final run reported zero application errors and one Chromium preload warning. The browser,
static host, authoritative server, both custom ports, and temporary captures were closed or
removed after inspection.

## 2026-07-23 — Research paths converge across disciplines

**Problem:** The vertical tree made progression direction readable, but most choices still had one
incoming line. A player could follow a branch downward without comparing dependencies elsewhere,
and cross-discipline gates on later stages of collapsed building and production tracks were hidden
from the player-facing graph.

**Fix:** Eight live junction studies now merge practical disciplines: Stone Tools, Metal Tools,
Precision Tools, Civil Engineering, Preservation Science, Organized Provisioning, Public
Administration, and Combined Arms. Early milestones and late transport/housing advances also use
logical multi-prerequisite gates. Collapsed family cards project external requirements from every
stage, while a regression guard keeps the displayed graph acyclic and requires at least 24 visible
convergence choices. The authoritative catalog now contains 495 raw studies, normalized to 88
catalog rows and 228 graph nodes. Every junction has distinct tracked art, a live modifier, and two
or three independently meaningful prerequisites.

**Evidence:** The focused catalog, track, queue, and research-UI suites pass 59 tests; all 176
`cat-client` library tests, strict Clippy for `cat-sim`, `cat-protocol`, `cat-server`, and
`cat-client`, formatting, the diff check, and all 76 workspace smoke tests pass. A fresh release
WASM bundle ran in Playwright at 1024×768 on isolated ports `18088` and `18796`. Settled captures
verified the 228-node overview; the two-input Stone Tools path; the three-input Metal Tools path;
semantic junction icons; readable forward connectors; fixed catalog/inspector columns; and
mouse-wheel tree panning. The browser reported zero application errors. Its five warnings were
one Chromium preload limitation and four expected WebGL screenshot readback stalls. The game page,
static host, authoritative server, custom ports, temporary database, and temporary captures were
closed or removed after inspection.

## 2026-07-23 — Research uses a vertical focusable progression tree

**Problem:** The normalized research planner still rendered one large information-heavy card per
track in a left-to-right graph. It hid the actual level progression, mixed details into every
node, made dependency direction difficult to follow, and let canvas wheel handling compete with
the independently scrollable catalog.

**Fix:** The canvas now uses compact icon-and-name nodes. Ordinary milestones, buildings, and
production families keep one recognizable node; global modifiers expose levels 1–10 followed by
their own infinite terminal. Prerequisites flow top-to-bottom at a fixed readable scale with
larger tier gaps. No selection shows the full graph; selection shows the complete ancestor-and-
descendant subgraph and moves all detail into the inspector. The **Full tree** action clears that
focus. Focus mode compacts the induced graph around the canvas centre instead of leaving relevant
cards at their full-tree coordinates. Wheel input pans the graph only while the pointer is over the
graph, so the catalog and inspector retain their own natural scroll ownership.

**Evidence:** The focused research regression suite passes all eight layout, closure, level,
icon, queue, and compact-focus tests; all 174 `cat-client` library tests and strict client Clippy
pass. A booted native client at 1024×768 verified the full 220-study overview, icon-and-name cards,
separate catalog, selected inspector, centered `Research Hut → Scholarship 1 → Scholarship 2 → …`
focus chain, and fast direct drag panning against the isolated game server on port `18795`.
Playwright also loaded the optimized WASM module and complete asset set from custom port `18087`;
the shared Chromium renderer was then lost under host resource pressure before it returned a
frame, without a Rust or JavaScript application error. The native game, Playwright pages, static
server, game server, both custom ports, test databases, and temporary captures were closed or
removed after inspection.

## 2026-07-23 — Research is a durable technology planner

**Problem:** Even after the dependency layout improvements, the screen exposed all 487 generated
ledger nodes. Players could not compare meaningful paths, could not plan unaffordable prerequisites,
and research completed as an instant currency purchase. Building research also had no physical
follow-through, so a permit and an upgraded building were effectively the same thing.

**Fix:** The stable raw ledger is now normalized into 80 player-facing tracks with semantic icons:
milestones, building families, production families, and finite-then-repeatable global modifiers.
The screen uses a scrollable catalog/queue, one connected reactive graph, and a selected-track
inspector. Selecting a target queues its complete missing path. Funding, partial timed progress,
reorder constraints, removal refunds, offline catch-up, daily Leader fallback, time/cost reductions,
and infinite levels persist authoritatively. Physical buildings have researched level permits but
must still be upgraded through an offline construction job from their inspector.

**Evidence:** Playwright exercised the WASM client at 1024×768 on isolated ports `18086` and
`18794`. It caught and drove the fix for one Bevy query-conflict panic, then the fresh final run
reported zero console errors. Settled captures verified the 80-track catalog scrollbar, contained
queue controls, disabled edge moves, connected overview, stronger selected-branch zoom, concise
inspector action, and physical **Upgrade to level 2** building button. Strict Clippy passes for the
four touched crates, the workspace smoke profile passes all 76 tests, focused normalized-track and
queue tests pass, formatting and the WASM build pass. The Playwright page, static server,
authoritative game server, custom ports, and temporary database were closed or removed afterward.

## 2026-07-23 — Research progression is spaced and icon-led

**Problem:** The persistent dependency backbone made the graph truthful, but dense layers still
read as one large text block. Branch changes had the same spacing as sibling stages, card
silhouettes were nearly identical, and players had to read every title before they could recognize
the route toward water, housing, craft, food, defense, or knowledge outcomes.

**Fix:** The prerequisite-derived layout now inserts large gaps between the four root branches and
smaller gaps when a visual parent or semantic family changes. Cards name their main branch and
step, lead with a semantic pixel-art icon, and reserve text for the study title and compact state.
All 487 studies resolve through `research_icon_path()` to tracked existing game art; more than 30
icons cover buildings, resources, work, travel, farming, scholarship, storage, and combat. Stages
within one family deliberately share an icon so they read as a progression track. The inspector
mirrors the selected icon, and the scale-aware root overview fits all four first-level branches at
100% and 130%.

**Evidence:** Playwright exercised the WASM client against isolated ports `18085` and `18793` at
1024×768. Root and selected-path captures at 100% and 130% showed distinct branch gaps, complete
first-level cards, readable icons, continuous routes, and a contained inspector. Selecting
Waterworks Sources visibly traced Research Hut → Water Carriers → Waterworks Sources with the
shared water icon. A fresh settled browser session reported zero application errors. All 186
`cat-client` tests and all 76 workspace smoke tests pass; strict `cat-client` Clippy, formatting,
the diff check, and the `cat-web` wasm32 build pass. The Playwright page and both temporary game
servers were closed after capture.

## 2026-07-23 — Research has a persistent visual backbone

**Problem:** The prerequisite-derived layout was technically one tree, but almost every connector
was hidden until its study was selected. Cards therefore looked like floating columns rather than
one hierarchy, filtering removed their context, and raw-window centering pushed the root toward
the inspector at enlarged interface scales.

**Fix:** Every non-root study now receives one deterministic primary visual parent from its nearest
prerequisites. Those 486 elbow routes remain visible as the tree's category-accented backbone;
additional DAG prerequisites appear as selected-path cross-links. Alternating depth bands and
`Step N` card labels expose progression, the root overview shows three levels and four main
branches, and search/category results retain muted prerequisite ancestors. Root centering and
overview zoom now use the shell's effective scaled viewport, preserving the same physical
readability and structural extent at 100% and 130%.

**Evidence:** Playwright exercised the WASM client against isolated ports `18084` and `18792` at
1024×768. Root, selected-path, filtered-context, and 130% captures showed a centered single tree,
three visible levels, persistent routes, a contained inspector, and no page overflow. A fresh
settled browser session reported zero application errors. All 185 `cat-client` tests and all 76
workspace smoke tests pass; strict `cat-client` Clippy, formatting, the diff check, and the
`cat-web` wasm32 build pass. The Playwright page and both temporary game servers were closed after
capture.

## 2026-07-22 — Research is one prerequisite-derived tree

**Problem:** The first Research-atlas correction improved contrast and navigation but retained the
catalog's authoring coordinates as four wide category regions. The resulting 20,000 px strip still
looked and behaved like several adjacent lists, obscured the single-root prerequisite structure,
and made distant studies unnecessarily laborious to reach.

**Fix:** Research presentation coordinates are now derived from the prerequisite DAG rather than
authoring categories. All 487 studies form one continuous, single-root tree across 17 topological
layers; category survives only as card and branch color. Opening the page or pressing **Home**
centers the root, search retains the complete catalog, and selecting a study highlights its full
ancestry to the root. Layer ordering minimizes crossings, cards have stable readable dimensions,
and the layout tests reject overlap or backward dependency edges. The P21 gameplay feature merge
retains this routed interface and the requested removal of Map, Help, Dispatches, the moving ticker,
and letter-key screen shortcuts while adding food ecology, caves, evergreen work, world job markers,
building interactions, employment/storage detail, Log filters, and signed world restart.

**Evidence:** Playwright exercised the merged WASM client on isolated ports `18083` and `18791` at
1024×768 with 100% and 130% interface scale. Settled captures confirmed one centered root, readable
cards and ancestry, a contained inspector, usable Stores/Village pages, and no page overflow; the
fresh browser session reported zero application errors. All 184 `cat-client` tests and all 76
workspace smoke tests pass. Strict Clippy for the four touched crates, formatting, the diff check,
and the `cat-web` wasm32 build pass. The Playwright page and both temporary game servers
were closed after capture.

## 2026-07-22 — Pointer panning is decisive and Research is a readable dependency atlas

**Problem:** Player follow-up after the client-surface correction found that dragging the world
still moved it much less than expected, while Research remained a flat beige node dump with weak
state, category, and dependency hierarchy.

**Fix:** World and Research panning now consume logical `CursorMoved` deltas, which are reliable in
both native winit and browser canvas builds, and apply a zoom-independent 1.65× screen-space gain.
World navigation retains middle-drag and Space-left-drag; Research adds direct left-drag anywhere
inside its computed canvas viewport. The Research page is now a cartographer-style dependency
atlas: a dark forest worktable separates the graph from UI chrome; progression regions have subtle
category rails and headers; larger paper cards carry category, title, state, era, and cost in a
stable hierarchy; owned, available, locked, selected, and selected-path states have distinct
contrast; and the plain paper inspector stays readable beside the map. The compact header hides
secondary leader-priority copy before controls wrap, preserving a clean two-row layout at 130%.

**Evidence:** Playwright drove the WASM client on isolated ports `18081` and `18789`. At 1024×768,
a real 150×50 px left drag moved the Research content approximately 248×83 px, and a real
Space-left drag moved the world by the same accelerated screen distance. Settled screenshots at
100% and 130% confirmed readable cards, contained inspector content, and a non-wrapping compact
header. All 165 `cat-client` tests pass, including pure world/Research drag-gain checks and the
responsive Research contract; workspace smoke passes 74 tests. Strict Clippy, formatting,
`git diff --check`, and the `cat-web` wasm32 build pass. The Playwright page and both temporary
servers were closed after capture.

## 2026-07-22 — Redundant and overflowing client surfaces are removed

**Problem:** Player screenshots at the supported desktop size showed missing-character boxes,
long dispatch/log copy crossing panel bounds, a hover tooltip detached from the pointer, redundant
icon-plus-text resource rows, and visually weak Stores and Research pages. Map and Help did not
earn permanent navigation space, screen-opening letter shortcuts were easy to trigger by accident,
and world navigation had no direct return-to-village action. Underneath those symptoms, primary
pages and inspectors used independent fixed-pixel positioning, clipping hid excess content instead
of making it reachable, and multiple Boolean toggles could compete for full-window ownership.

**Fix:** `ClientUiShellPlugin` now owns five typed surface categories, one `UiRouter` state for the
four maintained destinations (Log, Stores, Village, and Research), centralized layer order,
whole-interface scaling, and reusable Bevy `ScrollArea`/`Scrollbar` bodies. Map, Help, Dispatches,
and the moving announcement ticker were removed. Primary pages open only from explicit top-bar
buttons; Log wraps inside a bounded scroll body; the permanent HUD uses text-only survival totals;
Stores owns the complete semantic resource-icon grid and structured crafted inventory; and
Research uses clearer category lanes, larger cards, a contained inspector, and middle-drag/wheel
navigation. Unsupported decorative glyphs were replaced with supported text. Tooltips convert the
physical cursor into scaled UI coordinates and flip at viewport edges. World controls now support
faster movement, Shift acceleration, Space-left or middle-button panning, cursor-centred zoom, and
an explicit **Center village** button. `docs/UI_ARCHITECTURE.md` defines the extension contract for
future panels, primary screens, scrolling, input ownership, and the required scale matrix.

**Evidence:** Playwright exercised the live WASM game against isolated ports `18080` (client) and
`8788` (server) at 1440×900 and the 1024×768 support floor, at both 100% and 130% interface scale.
Settled screenshots confirmed contained onboarding, Log, Stores, Research, HUD, command dock, and
near-pointer tooltips; visible scrollbars retained access to excess content. Sending `L/G/C/U/M`
left the world open. The current-origin browser run reported zero application console errors. The
`cat-client` suite passes all 163 tests, workspace smoke passes 74 tests, and strict `cat-client`
Clippy, formatting, `git diff --check`, and the `cat-web` wasm32 build pass. The Playwright page,
browser, WASM server, native client, and temporary authoritative server were closed afterward.

## 2026-07-16 — Comprehensive review findings are resolved and release-bounded

**Problem:** The five-pass review in `docs/reviews/` found a small set of cross-cutting defects
despite feature parity being complete: hostile queue/rail/path inputs could panic or over-allocate,
mutable non-finite state could poison a snapshot, wire evolution had no visible compatibility
boundary, public server deployments lacked firm client/session/message bounds, a disconnected
client could look live, and first-run/role guidance was too implicit. Workspace metadata, toolchain
policy, dependency auditing, and several implementation counts in the docs had also drifted. The
first consolidated long-horizon rerun then found a separate integration issue: established seeds
42 and 99 remained healthy but four physical water carriers could be in transit while a
migration-grown village consumed the last prosperity reserve.

**Fix:** Queue moves, rail endpoints, cardinal-line construction, and sparse A* now reject bounded
bad input without panic or dense allocation. Quarry staffing respects full Stone storage; d20 and
all mutable snapshot float families are finite-projected. Protocol v1 is explicit and first on the
wire, wire counts/indices are fixed-width, legacy trader fields default safely, and an incompatible
frame produces `UPDATE REQUIRED`. The server now binds limits to a strict effective client IP,
trusts forwarding only from an exact configured proxy allowlist, caps sockets/actions/sessions/
villages/messages, rotates expiring sessions without changing durable player identity, requires
Origin and secret configuration on public binds, fails readiness after repeated saves, avoids
readiness lock waits, caches hot statements,
and reports table/row/column context for every corrupt JSON field. The client keeps a
LIVE/STALE/OFFLINE status with retry time, adds first-run/Help guidance and distinct
specialization/officer cues, removes dead `FUTURE`
presentation, and performs typed snapshot decoding after the version discriminator. The mature
water transit reserve is now two units per cat; migration and research prosperity thresholds are
unchanged. Workspace metadata/MSRV are unified, Forgejo runs cargo-deny advisory/ban/source checks,
retired web build products are ignored, and every review disposition is recorded in
`docs/reviews/RESOLUTION.md`.

**Evidence:** `cargo nextest run --workspace` passed all 1,709 tests with two intentional skips.
That suite includes passive exact-cadence and proxy controls plus signed, observed-state guided
campaigns covering every public action, survival, migration/housing, research, roads, fog/scouts,
farming, fishing, industry, equipment, officers, combat, trade, and restart. The corrected seeds
42 and 99 each pass their established 300-hour campaign with positive water. Workspace strict
Clippy, formatting, the `cat-web` wasm32 check, `git diff --check`, and
`cargo deny check advisories bans sources` pass; the deny policy carries only narrow documented
upstream Bevy/Wayland parser exceptions.

Against a booted authoritative server, the native Bevy client captured its own 1906×1045 physical
framebuffer after settled frames (the requested logical window was 1024×768). This review inspected
the complete onboarding frame
`/tmp/idle-cat-review-final.png` (SHA-256
`db4cc9f898b9324f53c90a4e885c337b6645f6a203af35f3ef9ae0b5e0937ea5`) and complete world frame
`/tmp/idle-cat-review-world-final-b.png` (SHA-256
`e793993a16cb7085283477f2628d2f62c835e72918e4965dc0b9a62ac66c8a2c`). They visibly prove the
onboarding/Help surface, persistent LIVE chip, compact command dock, connected roads, open-top
stations, fog boundary, and one authoritative body per visible cat. Redundant captures exposed and
were rejected for the known intermittent partial-readback artifact. Temporary capture code was
removed before final gates.

## 2026-07-16 — Generalized correction gate closes the eight-player-feedback set

**Problem:** Focused regressions proved the individual corrections but did not establish that the
combined game still worked as an idle colony or under broad player guidance. Early exact-cadence
evidence also found that the Leader projected only six caller ticks of survival, so one-second
servers could starve while coarse tests passed. Initial Bevy screenshots after panel transitions
occasionally contained only part of a fully rendered swapchain frame, which could be mistaken for
a persistent layout defect.

**Fix:** Leader survival planning now projects a fixed four game-hour runway independent of caller
cadence. The physical Fibre→Thread→Cloth and four-material Mug routes, selectable shrine offerings,
complete 16-mask roads, single-body cats, resolved 2×3 decoration footprints, compact four-resource
HUD, complete Stores ledger, and categorized command dock are integrated. The screenshot gate takes
redundant settled frames and accepts a target state only after inspecting a complete client-owned
frame; no production workaround was added for the intermittent capture-readback artifact.

**Evidence:** Exact one-second, 48-game-hour passive campaigns on seeds 555, 20240712, and 7 all
finished with 15 cats, zero deaths, zero extinction resets, positive food/water, autonomous hunting,
water work, elections, scouting, and permanent shrine-return fog growth. Seed 555 improved from two
starvation deaths and 23 near-empty samples to zero deaths, a sampled food floor of 16, and final
Food 40/Water 105. An observed-state signed campaign exercised all 52 `ClientAction` variants,
including officer assignment/removal, construction and roads, research, offerings, every processor,
Fibre→Thread→Cloth, Mugs, rail/shipping, both trade systems, equipment, and SQLite restart.

The booted native client captured and this review inspected complete default, expanded Gather, and
Stores scenes at 1024×768 and 1920×1080. The accepted SHA-256 values are respectively
`6f8524c139a11098f664923d352b2ce8ec387e8a4d7ec8a683b27ec3719d613f`,
`128507859137483ece8a8c7894680930d01f8fe0814e45bac6e4160ddda22aa0`,
`e092606dfdf19c911a3532323c17419e3d36bcebe6592cd4698f4b3c2180ec3a`,
`476fb33b85c974fbd6ebe52912f1446797ceeb32ef26eba22f5697be59ef8890`,
`5d0f2d281981b613efad6e2df18edf27a9ff14f3090bd2a3e543047bcbcb6e89`, and
`152bded6f35b3d2d390bdeffa92641a6ab6d753a318857bcd423bcd0dfabbd45`.
They show connected junctions, exactly one body per visible cat, collision-free canopies, no natural
resources inside the village, a fully visible world beneath dynamic menus, every command reachable,
and all 32 resources available through Stores. Temporary capture code was removed before final gates.

## 2026-07-16 — Narrow HUD clears every command at the supported viewport floor

**Problem:** The first exact 1024×768 final-coverage capture exposed a real responsive-layout
defect: the truthful six-line colony header and wrapping goods/Accountant footer extended the
left HUD to y669 while the four-row command toolbar began around y604. The HUD hid the left side
of the first two command rows even though all 31 resource pills themselves were visible.

**Fix:** At the maintained narrow-layout breakpoint, only the redundant status prose switches to
a bounded four-line header and two-line treasury/ledger summary. Colony and leader labels are
bounded, all housing/threat/job/goods/ledger facts remain present, and the complete 31-resource
grid is unchanged. Wider windows retain the detailed copy. Window changes now refresh the layout
without waiting for a new snapshot.

**Evidence:** Responsive helper tests pin compact versus detailed selection, four/two-line bounds,
and the existing four-row toolbar geometry. The live native client captured its own RGB framebuffer
against a booted authoritative server at exactly 1024×768. The decoded HUD PNG
`/tmp/idle-cat-hud-1024.png` (SHA-256
`e26e3818c57e55901a647749ad7544f334f3926df8b7b44fca2d56a3fdb54cad`) shows all 31 rows,
the HUD ending at y580, and every toolbar control beginning at y604 with no overlap. The decoded
research PNG `/tmp/idle-cat-research-1024.png` (SHA-256
`329ea3a09db3837addaf73dfe3bc474eacbb37b2b6c11f17f5417da1c719e72a`) shows 487/487 nodes,
all filters and navigation, a readable inspector, and no `FUTURE` state. The temporary capture
system was removed before the final gates.

## 2026-07-16 — Subsistence and frontier research breadth has finite routes

**Problem:** Forty-eight nodes across Hunting, Foraging, Waterworks, Animal Husbandry, Field
Craft, and Expedition Supplies were still disabled `FUTURE` cards: 28 recipes had no station route
and twenty resource stages had no observable effect.

**Fix:** All thirty recipe-bearing studies in those families now map to exact finite item routes;
the two established Hunting bone goods are retained and 28 new operations consume Bone, Fibre,
Planks, Hide, Supplies, or Cloth at Woodworking, Clothier, Tannery, or Workshop. Every output is one
stable identity through local output, outbound cargo, storage, equipment where functional, and
trade otherwise. They do not claim hidden farm, water, husbandry, or scout automation. Sources,
Preservation, and Bulk respectively reduce exact family-recipe input, raise exact item durability,
and shorten only the matching physical cycle; Hunting and Foraging Reserves expand only their
listed finite storehouse domains. Research recipes remain absent from default automation queues.

**Evidence:** Exhaustive catalog tests make all fifty family nodes purchasable and pin their honest
descriptions and catalog-derived entitlements. Descriptor tests cover all thirty item mappings.
Every route consumes its exact aggregate/local input, creates one uncredited station identity,
moves that same ID in a carrier, and credits it only at storage in deterministic twins. Effect and
capacity tests cover all twenty resource stages, and signed player actions queue all thirty
recipes only at their actual station. At this slice boundary there were 74 physical recipes with
30 recipe and 27 resource payloads still pending. No protocol/client surface or art changed, so no
framebuffer recapture was required.

## 2026-07-16 — Industrial material research owns finite recipes and goods

**Problem:** All generated stages in Textile Work, Leatherworking, Carpentry, Stonecraft,
Metallurgy, Toolmaking, Weaponcraft, Armorcraft, and Trade Goods were still partly disabled
`FUTURE` promises. Thirty recipe stages lacked a physical selected operation and 27 resource
stages had no exact yield, cycle, or capacity consumer.

**Fix:** Thirty data-owned recipes now consume finite Fibre, Hide, Logs, Stone, Ore, or Metal at
the existing open-top station and deliver Cloth, Leather, Lumber, Planks, Blocks, Metal, or exact
equipment. The twelve researched equipment recipes mint one stable quality-mapped metal
Tool/Weapon/Armor identity—never a shadow scalar—and carry it through local output, paws, storage,
equipment, traders, and restart. All nine `sources`, `preservation`, and `bulk` families now change
only their exact input/yield, physical capacity, or selected cycle. Legacy default queues are
unchanged.

**Evidence:** Exhaustive tests pin all 72 family nodes, all 30 descriptors, exact catalog owners,
all 27 resource consumers, quality/durability mapping, signed purchase/queue editing, deterministic
physical input/output, exact-ID haul/equip/sale/restart conservation, and SQLite queue/ownership
restart. Combined with the subsistence/frontier slice, the maintained catalog now has 104 physical
recipes and no remaining generated recipe or resource payload marked `FUTURE`.

## 2026-07-16 — Every Building study has a truthful runtime consumer

**Problem:** Thirteen passive building families inherited the generic `Crews +1 worker` template
even though they had no safe concurrent station state. Those cards were disabled as `FUTURE`, and
several other generated Output, Cycle, or Durability studies resolved into fields that their
building never read. The Building branch therefore met its numeric one-third target while still
containing advertised dead ends.

**Fix:** The thirteen false worker promises keep their stable IDs and prerequisite positions but
now describe building-specific service practice: Den stewardship, Food Store rotation, Water Bowl
conservation, restful Beds, Herb Garden medicine, Nursery care, Elder protection, wall readiness,
Mouse Farm husbandry, Shrine ritual yield, Field stewardship, Barracks readiness, and Accounting
discipline. Every effect is inert until the matching completed building exists. Mouse Farm is the
one newly staffed physical service: one living keeper creates finite station-local Food, which an
ordinary carrier must haul into finite storage before aggregate credit. Research Hut and School
output/cycle studies now affect their matching staffed research service; otherwise-unread passive
Output/Cycle/Durability stages are mapped to the same truthful service or maintained output.

**Evidence:** Exhaustive catalog tests require all 165 Building-category nodes to be purchasable,
pin the exact thirteen service mappings, preserve their IDs and legacy prerequisite graph, and
reject fake worker-slot payloads. Consumer-level tests compare baseline, owned-without-building,
and owned-with-completed-building behavior for housing, spoilage, water debit, sleep recovery,
medicine, kitten aging, elder mortality, farm harvests, raid muster, blessings, accounting, and
research labor. Mouse Farm tests cover no worker, full destination, local output, physical haul,
death salvage, JSON restart, deterministic twins, and conservation; a signed tithe test covers the
public action boundary. The interim slice passes 1,351 simulation tests with two intentional skips,
147 client tests, strict all-target Clippy for both crates, formatting, and diff checks.

## 2026-07-16 — Useful labor pressure is measured, not inferred from idle labels

**Problem:** The vision's central “more useful work than cats” tension had no quantitative
guardrail. Existing playtests reported idle cats and officer vacancies, but could not distinguish a
deliberately manual specialist category, a cat temporarily satisfying a physical need, a worker
parked at an input-starved station, and a real shortage of paws for reachable work.

**Fix:** `observe_labor_pressure` now reports unique job/station/farm/transport ownership, visible
active and truly idle cats, sourced processor vacancies, workable exterior plots, and the seven
manual-only office states without creating automation. Its read-only station probe uses the same
authoritative recipe entitlement, physical input, output-headroom, and route block reason as the
inspector; paused, research-locked, missing-input, and full-output stations do not count as useful
demand. Player-guided tests issue ordinary signed actions from observed state and prove shortage by
filling the roster, then receiving `No available worker.` on one more real physical order.

**Evidence:** Seeds 7, 555, and 2024 each accept a mixed founding queue of six hunts, two water
routes, fibre forage, expansion, and five fog scouts for all 15 cats; one sourced founding
processor remains, measuring 16 useful slots for 15 paws. Three personal 200-game-hour guided
campaigns and byte-identical five-minute twins retain positive food/water and zero resets while
repeatedly reaching workforce-only denial. Three personal plus the communal 48-hour passive proxy
twins survive with all seven specialist offices correctly manual. The established action matrix
continues to exercise production, research, authored roads, exterior farms, scouts, and both
transport modes; the staged signed officer test observes every 7→0 manual-office state. The
existing exact one-second 48-hour personal/communal survival campaigns remain the live-cadence
control. No gameplay correction or renderer change was required.

## 2026-07-16 — Village barter follows one durable shared-world land route

**Problem:** Accepted village barter persisted arbitrary waypoint lists but production acceptance
still installed a direct shrine-to-shrine segment. The actor could cut through water, mountains,
or a closed section of either palisade, and shared terrain did not decide whether it could depart.

**Fix:** Acceptance now plans one deterministic orthogonal route before touching either cargo.
Bounded A* reads canonical mutable tiles first and deterministically generates only needed unknown
chunks; water, mountains, and every effective closed wall edge are hard blocks. Both shrine
interiors connect through their current real gate. Goal-distance tie-breaking keeps long journeys
close to linear work, while a fixed corridor and expansion ceiling bound hostile/no-route inputs.
The compressed outbound list and exact reverse persist once and never silently replan in flight.
Failure leaves the offer open and both physical cargoes at home. Planning cannot reveal fog.

**Evidence:** Tests cover a shared-water detour, flooded-gate signed rejection, both villages'
live wall/gate topology, deterministic 4,800-tile bounds, unchanged shared/fog ledgers, exact JSON
route restart, reverse-route durability, cancellation, full-storage waiting, exact equipment
identity, and one-second/coarse tick partition equivalence. Full simulation and strict gates pass.

## 2026-07-16 — Food and plant research breadth has physical consumers

**Problem:** Baking, Herbalism, Food Preservation, Brewing, and the later Grain Milling stages
were visible but non-purchasable `FUTURE` promises. They had neither selected finite recipes nor
observable consumers for their generated resource stages.

**Fix:** Twenty-three selected recipes now run through finite Mill or Workshop input, work,
local-output, carrier, and storage state. They cover quality Grain→Flour, Flour→Food baking,
Herbs→Medicine, Food→Preserves, and Grain/Catnip/Herbs→Brew. Preserves feed cats before
perishable Food, Medicine treats injured cats at a bounded cat-hour rate, and Brew replaces at
most one quarter of thirst so clean Water remains necessary. Seventeen exact resource stages now
raise the matching crop or batch yield, reduce Food spoilage, shorten only their family's batch,
or expand the exact Grain/Flour/Food/Preserves/Medicine/Brew capacity in a real pile. No generic
generated ID is accepted without one of those consumers.

**Evidence:** Exhaustive catalog and route tests cover all 42 nodes in the five families and all
23 new descriptor→physical-recipe mappings. Storage/effect tests pin each exact resource-stage
consumer. A no-input-after-setup passive twin produces finite local output deterministically; a
signed player purchase and queue edit reaches the same recipe. New goods cross serde/protocol,
stockpiles, cargo, Accountant projection, trader/village trade, SQLite audit restart, and the
31-entry unique client HUD/carrying maps. The future boundary is now 58 recipes and 47 resources.
The inspected client-owned 1090×2105 RGB framebuffer
`/tmp/food-plant-research-breadth.png` (SHA-256
`a4bd0fd829edb2021a8588c8e50bf21b85962f2030c69b387fbf021e114e0622`) shows all 31
resource rows, including distinct Preserves, Medicine, and Brew art and values, without clipping.
The temporary capture system and isolated client/server processes were removed.

## 2026-07-16 — Whole-game signed action campaign covers post-P19 controls

**Problem:** The deterministic `player_action_campaign` claimed to exercise every public
`ClientAction`, but its expected set stopped at 41 pre-transport actions. Ten subsequently shipped
paths were present in the protocol and client yet absent from that longitudinal gate: Rail
designation, Dock construction, transport-vehicle construction, route creation/cancellation,
village-trade offer/accept/cancel, and exact equipment equip/unequip. A regression across one of
those feature boundaries could therefore keep the “every action” campaign green.

**Fix:** The same deterministic campaign now covers all 51 action variants. It observes generated
terrain to select legal dry rail and land/water dock sites, uses signed construction actions,
authors both a constructed-track Rail route and a dock-to-dock Shipping route, and cancels each
through the public boundary. It equips and unequips the repaired finite Tool without changing its
identity. Its fresh global plus owner-personal villages establish explicit mutual contact (the
separate fog campaign owns physical shrine-return provenance), withdraw one offer, accept a
replacement, and then advance the exact escrow caravan at one-second cadence until its observed
round trip completes. The bound derives from the actual
generated distance rather than a scripted completion timestamp; final resource deltas and
byte-identical replay are asserted.

**Evidence:** The focused campaign passes twice within its own determinism assertion. The broader
player-guided matrix also covers three-seed manual survival, fog/shrine return, house/farm/building
placement, officer handoff, research purchase and staffed accrual, physical processor queues,
finite equipment/repair/trader controls, authored roads, Rail/Shipping cargo state machines, and
the signed two-player server founding/contact/exact-caravan path. Passive controls for seeds 7,
555, and 2024 run at the production one-second cadence as byte-identical four-game-hour twins,
growing fog from 289 to 459, 327, and 425 tiles respectively with three shrine deliveries each;
the seed-7 48-hour visible pass additionally held 15 cats with no death or reset while survival,
scouting, and raid work remained active. The final simulation gate passes all 1,306 tests (one
intentional skip), the authenticated server contact/trade test passes, and strict
Clippy/formatting/diff checks are clean. Exact commands are recorded in
`docs/IMPLEMENTATION_AUDIT.md`.

## 2026-07-16 — Physical Bone, Gem, Clay, and Sand goods

**Problem:** Four raw materials had real finite source and storage identities but no selected
consumer. Their research studies were therefore honest `FUTURE` cards, and a player could not turn
Bone into tools/decor/toys, Gem into jewelry, Clay into pottery/bricks, or Sand into glassy goods.

**Fix:** Ten catalog studies now own ten exact runtime recipes across Woodworking, Stone Prep, and
Workshop. Every batch reserves and carries its exact raw input, requires a living assigned worker,
creates one stable finite output identity in the station's local output, and carries that same ID
to finite storage before credit. Material, kind, quality, condition, location, death recovery,
SQLite persistence, trader visibility, and exact sale remain authoritative. Default founding
queues still contain only their safe baselines; the client now selects among all unlocked recipes
before adding one, so breadth beyond the first row is reachable. Steward pile keys are exhaustive,
preventing Gem/Sand input piles from colliding under an `unsupported` ID.

**Evidence:** An exhaustive route table proves all ten source→station→exact-output mappings. A
single physical lifecycle test produces and delivers every variant while preserving the same ID;
research denial/ownership, carrier death, passive deterministic Gem crafting, signed player queue
selection, trader sale, and SQLite mid-station/mid-carrier restart campaigns cover both unattended
and guided play. The inspected client-owned 1090×2105 RGB framebuffer
`/tmp/material-variant-breadth-final.png` (SHA-256
`8ef0ad5521d98128869e4ddd66992c5a1097faab1c74143ea258e15bc85e360f`) shows the selected
Stone Prep inspector, its independently cycled `Fired clay brick goods` add target, current queue,
staffing, local stores, and controls without overlap or clipping. The temporary fixture/capture
hook and isolated processes were removed. The catalog now pins 23 runtime recipes and 81 recipe/64
resource future payloads.

## 2026-07-15 — Finite fine-biome Gem, Clay, and Sand sources

**Problem:** Climate named rare minerals, wet earth, and sand, but runtime world tiles could not
hold them. Generic coarse quarry terrain therefore could not prove a finite, biome-specific source,
and the three identities disappeared across hauling, storage, persistence, trade, and the client.

**Fix:** Mountain tiles receive a small finite Gem deposit, marsh/swamp/badlands tiles finite Clay,
and beach/desert tiles finite Sand. Extraction drains the persisted tile only when a cat physically
picks up the exact cargo; final partial trips conserve integers and exhausted Clay/Sand-only sites
remain visibly and durably depleted. Generic Quarry selection protects the founding Stone/tool
chain before considering special-only sites. The new resources have independent aggregate and
stockpile capacity, wire/action/carrying kinds, SQLite-compatible defaulted tile JSON, trade and
Accountant projections, distinct public-pack HUD/cargo art, and exact repair-material matching.
Founding and expanded village interiors explicitly erase all three deposits.

**Evidence:** The exhaustive 26-biome deposit matrix proves both presence and absence without
inventing deposits from a coarse role. Physical multi-trip tests prove source decrement, exact
cargo identity, delivery-before-credit, exhaustion, and deterministic replay. Founding clearing,
storage clamping, protocol compatibility, SQLite round-trip, and full simulation survival/tool
campaigns pass. Client mapping tests cover all 28 resource and carrying identities with unique PNG
contents. The inspected client-owned `/tmp/fine-biome-resource-ecology.png` is a 1090×1046 RGB PNG
(SHA-256 `38f53853eca81a1836eb7d5de17129e3f0461a67042f645bb623982e89737cff`): all 28 HUD rows,
the top-down village, cats, roads, open stations, and bottom controls remain readable without
clipping or overlap. The temporary hook and isolated processes were removed. Downstream
Gem/Clay/Sand/Bone recipes remain explicitly open rather than being advertised as implemented.
## 2026-07-16 — Rail and Shipping are constructed, staffed physical logistics

**Problem:** Rail and Shipping correctly stopped granting magical walking effects, but the
blueprints had no tracks, rolling stock, docks, vessels, crews, or finite cargo routes.

**Fix:** Both studies remain neutral until a signed action designates exact infrastructure.
Track, dock, rolling-stock, and vessel projects reserve exact visible Metal/Lumber/Blocks, require
a living builder to visit their source piles and work at the project tiles, and consume nothing
before pickup. A route requires adjacent constructed track or a dock-to-dock water path, an idle
matching vehicle, two physical stockpiles, finite cargo, and a living crew cat. Its persisted
Boarding→Loading→Outbound→Unloading→Returning lifecycle debits cargo only on loading, waits behind
full destination storage, and releases the vehicle only after return. Cancellation, source loss,
crew death, and project death recover exact cargo; research ownership alone remains physically
neutral. The client renders connected dark track, timber docks, wagons, and vessels without map
labels.

**Evidence:** Typed wire round trips cover all designation/vehicle/route/cancel payloads and the
snapshot. A signed guided action authors a finite four-tile rail route. Passive deterministic
twins, staffed rail and vessel campaigns, paid on-site track construction, cancellation, crew
death, storage wait, and exact aggregate-plus-transit conservation pass. SQLite resumes an
in-flight loaded vehicle with its exact phase, path index, fractional segment progress, crew,
cargo, and repeat flag. The inspected client-owned 1090×1046 RGB framebuffer
`/tmp/physical-rail-shipping.png` (SHA-256
`63646bbf2db832afb0dc2f8103f0923ca8af6b56b52cb18d02a651da4362cba6`) shows the constructed
dark rail alignment and loaded wagon in the live top-down village. The temporary fixture/capture
hook and isolated client/server processes were removed.

## 2026-07-16 — Building research meets the one-third design floor

**Problem:** Removing thirteen fake `*_stores` purchases correctly left a truthful 487-study
catalog, but it also reduced the Building category to 154 studies (31.62%). That missed the game
vision's requirement that at least one third of the graph be building-related. Restoring inert
container studies merely to improve a ratio would have reopened the no-op bug.

**Fix:** The existing eleven-study Construction branch is now data-classified as Building
research. Every one of those stable studies already applies a positive `constructionSpeed`
payload to the authoritative physical scaffold timer; no node, ID, dependency, cost, layout,
priority, ownership record, effect, or save shape changed. The resulting split is 165 Building,
167 Recipe/Resource, and 155 Upgrade studies. Both required categories therefore independently
meet the mathematical one-third floor while the catalog remains exactly 487 studies.

**Evidence:** Catalog construction rejects either category falling below one third. Exhaustive sim
tests pin all eleven `construction_*` studies to the Building category, positive live scaffold
effects, and non-future purchase status. Client model tests pin the same 165/167/155 filter split
and both ratios. The pre-existing physical-scaffold campaign proves those exact studies shorten
real construction and preserve pinned progress across reassignment. The inspected client-owned
`/tmp/research-building-third.png` is a 1090×1046 RGB framebuffer (SHA-256
`bc5db26f323b28354ea4ffd24394d77a7618361db3ee5a7b1fdf4187879c1bb4`): its Building filter reads
`165 / 487 nodes`, the centered Construction branch is visible, and the selected inspector labels
Construction Basics `BUILDING` with `constructionSpeed add 0.03`; there is no black screen,
clipping, or overlap. The capture-only hook was removed before final full sim/client gates, strict
Clippy, and formatting. Final gates pass all 1,290 simulation tests (one intentional skip), all
146 client tests, strict `cat-sim` and `cat-client` Clippy, formatting, and diff checks.

## 2026-07-16 — Cats physically eat, drink, and sleep

**Problem:** Hunger, thirst, and rest were scalar background regeneration. Food and water vanished
from the colony ledger without a cat selecting a finite pile, walking, or consuming at a real
destination, and Dens supplied abstract rest without bed occupancy.

**Fix:** Eat, Drink, and Sleep are real deterministic tasks. A resident selects reachable revealed
Food/Fish/Water stock, removes a bounded finite serving only at pickup, walks to Food Storage,
Water Bowl, or the shrine fallback, and recovers only on arrival. Dens expose five reserved beds
each and restore rest only while the cat is physically there. Blocked routes suspend conserved
cargo; death returns a serving through finite storage. A durable marker in the persisted carrying
record preserves interrupted task, destination, and activity across restart, then routes the cat
back before work resumes. Empty stores release seekers so hunters and water carriers cannot
deadlock the colony. Fed-state decay remains the established balance curve without passive
restoration; serving sizes preserve the prior per-cat hourly Food/Water economy.

**Evidence:** Pickup/debit/arrival, blocked/death conservation, exact five-bed reservations,
serialization/resume, passive determinism/survival, live-second player-guided water, probationer
consumption, dehydration edge events, long population, farm/Mill, communal, and five-seed
300-game-hour campaigns cover the contract. This slice changes no client rendering; existing
inspector task/carrying projection shows the new wire literals, so no framebuffer recapture was
 required.

## 2026-07-15 — Palisades read as solid top-down fortress cells

**Problem:** The former 16×4 side-view rail was stretched into a square wall cell. At play scale it
looked like a thin root or stripe rather than a solid DF-style perimeter.

**Fix:** Wall cells now use the tracked 16×16 top-down sharpened-timber tile from the public Kenney
RTS Medieval Pixel sheet. Rotation, staged-work tint, authoritative segment topology, collision,
and the distinct open south gate are unchanged. `docs/sprite-review.html` now compares the chosen
solid timber cell with a lighter rail fence, a stone fortress alternative, and the open gate so
future art choices can be reviewed without guessing filenames.

**Evidence:** All 146 client tests and strict client Clippy pass. The accepted client-owned
framebuffer `/tmp/topdown-wall-art.png` is a 1090×2105 RGB PNG (SHA-256
`b9c939bf0287756276b80acd306895ef71d13ed6c0c1595a8d1f2f16e2879166`). It was captured against a
booted authoritative server and visually inspected: the complete timber perimeter reads as
upright solid stakes, corners join coherently, the south gate remains an obvious opening, and the
wall does not cover cats, roads, buildings, exterior water, fog, or UI. Capture-only code and both
processes were removed.

## 2026-07-16 — Mutable terrain and ecology have one shared-world authority

**Problem:** Every colony persisted its own mutable copy of a world coordinate. A road, worn path,
depleted source, felled/regrown tree, or Fish population changed by one village could remain
untouched for another village standing on the same coordinate. Simply sharing the maps would also
have leaked road/resource discoveries through fog.

**Fix:** `WorldState` now owns the canonical mutable tile and Fish ledgers. Colony tile/habitat maps
remain compatibility and bounded-view caches: signed actions and each deterministic colony tick
hydrate them from world truth, publish physical mutations back, and refresh overlapping caches.
Ecology aging and path-wear decay run once per coordinate per world tick even when multiple
colonies map it. Fresh founding plateaus register without overwriting existing authority. Legacy
per-colony copies merge in stable colony-id order while conservatively preserving authored roads,
maximum wear/latest depletion, minimum remaining source/Fish stock, and maximum Fish capacity.
Fog, provisional scout notes, known contacts, farms, claims, and village ownership remain private.
Snapshot road, dirt-road, stump, and sapling projections now require that colony's committed reveal.

**Evidence:** An overlapping signed fishing-designation campaign proves that one road/depletion/
wear/Fish mutation is immediately identical in the other colony cache while its unrevealed road and
habitat remain absent from the wire snapshot. SQLite owns a new world-level tile table plus Fish
ledger and rules marker; a focused test covers whole-world round trip, old per-colony migration,
private fog, and bit-identical post-restart replay. Existing passive/guided simulation and server
gates cover founding, physical roads, logging/regrowth, fishing, deterministic multi-village ticks,
and signed action breadth. The final gates execute 1,284 simulation tests (plus one intentional
skip) and 99 server tests, with strict Clippy and formatting clean. No renderer or protocol shape
changed, so framebuffer recapture was not required for this authority-only slice.
## 2026-07-16 — Inter-village barter uses physical caravans and exact equipment

**Problem:** Accepting a village offer atomically swapped aggregate resources despite villages
being far apart. No actor, travel time, restart state, source-pile provenance, storage wait, or
in-transit conservation existed.

**Fix:** Acceptance now debits exact deterministic source piles into a durable two-sided escrow and
creates an explicit caravan actor at the source shrine. It travels shared-world coordinates to the
known target shrine and home again at a deterministic rate. Final credit waits for both physical
receiving plans; removed capacity leaves the actor waiting rather than deleting cargo. Either
controlling village may cancel, restoring both manifests to their origins, including a conserved
shrine overflow recovery when storage was removed. The actor never changes fog or contact state.

**Player visibility and evidence:** Snapshots expose phase, world position, actor identity, both
finite manifests, and acceptance time without exposing foreign inventory. The selector shows live
progress/cancel control and the map renders a blue-packed walking cat. Cadence, cancellation,
storage, no-fog-leak, signed projection, SQLite migration/restart, protocol compatibility, client,
and own-framebuffer evidence cover the caravan slice. Tool/Weapon/Armor cargo is removed as exact
instances, retains material, quality, condition, and credit state, and gains an injective
length-qualified world identity before leaving its origin. Thus two villages' legacy `item-...`
serials cannot collide after ownership changes. SQLite mid-route restart round-trips the exact
instances, and the client exposes exact manifest counts. Durable waypoint lists can be replaced by
the shared-spatial planner before departure; direct shrine waypoints remain the default until that
world-scoped passability integration lands.

## 2026-07-15 — Authored roads require physical supplies, travel, and labor

**Problem:** A signed road action and the Steward's deliberate-road pass painted every selected
tile immediately and removed aggregate Supplies without a worker, source stack, carried load, or
on-site work. This bypassed the same physical contract used by scaffolds and processors.

**Fix:** `BuildRoad` is now a durable job. It reserves one exact visible Material from a named
pile per ordered tile, assigns a living Build worker, and keeps the aggregate committed against
scalar trade. The builder walks to the source, carries one real unit to the site, and performs a
skill/research/tool-adjusted one-minute base work stage before that single tile becomes stone road
and the unit is consumed. Future road tiles reserve their map cells from buildings, farms, and
stockpiles. Steward automation queues the identical job and never bypasses it. Death returns
in-transit cargo to the exact freed source slot or a persisted visible spill, requeues the project,
and recruits a replacement without duplication. A raced access road returns an unneeded unit.
SQLite preserves ordered tiles, exact reservations, partial work, and assignment state.

Post-implementation conservation review closed five edge cases: accessibility paving now spends
only above both the safety floor and every active road commitment; stockpile reconciliation
protects exact road reservations; accessibility routes cannot race a tile already reserved by a
physical road; an externally paved target returns its carried unit to the source or a visible
spill even if that source disappeared; and replacement builders recompute the pinned duration
from their own skill and exact equipped tool. Leader-authored roads now carry Steward provenance:
a vacancy parks the durable project, salvages in-flight cargo, and resumes after reappointment,
while player-authored roads remain untouched.

**Evidence:** The signed all-actions campaign observes action→queued labor→physical cargo→paved
tile; the passive Steward test proves designation without instant painting or spending. Exact
pickup/arrival/work/debit, death/reassignment, map reservation, one-second versus one-minute
partition invariance, persistence/restart, and Build skill/tool provenance tests pass. Placement,
shrine-network attachment, mapped-terrain, stone/dirt surface distinction, and movement-speed
rules remain unchanged. The accepted client-owned framebuffer `/tmp/physical-road-labor.png` is a
1090×2105 RGB PNG (SHA-256
`2c66f3076e90302e7b26331106b29bb97c52963195044aa711841fbfce916eb0`). It was captured against an
isolated booted server and visually inspected: the top-down village, stone/dirt road network,
cats, open stations, reveal boundary, and road control render without black-screen clipping or
world/UI overlap. The temporary capture hook and both processes were removed afterward. The
conservation correction gate passes all 1,289 simulation tests (one intentional
instrumentation skip), including focused aggregate/pile/reservation, competing-route,
removed-source spill, restart-shaped replacement, and Steward vacancy/resume regressions. Strict
`cat-sim` Clippy, formatting, and diff checks are clean. The prior protocol, server, client, and
framebuffer evidence is unchanged because this correction alters no wire or visual contract.

## 2026-07-15 — Unsupported catalog breadth no longer sells no-ops

**Problem:** The 500-study ledger offered 93 generated recipe IDs and 64 generated resource IDs
that had no physical gameplay consumer. The Mill also exposed one combined operation that could
grind and bake implicitly, while the documented Smithy Metal→Tool route had no selected recipe.

**Fix:** The catalog now treats any recipe payload without a runtime station descriptor, and every
generic resource-registry payload without a physical source entitlement, as visible future
content. Those studies cannot spend research points or be selected by the Leader. The Mill owns
separate `grain_to_flour` and `flour_to_food` recipes with independent studies and physical
input/work/output/delivery. The Smithy owns `smithy_tool`; two Metal become one stable metal Tool
after selected Metalwork and outbound delivery. Persisted combined Mill queues migrate once at
station-rules v7 without losing authored order, repeat, pause, progress, or empty state.

**Evidence:** `RECIPE_RESOURCE_MATRIX.md` records the design-backed source/station/output boundary.
Regression tests pin 13 live descriptors and exactly 91 unsupported recipe plus 64 unsupported
resource payloads. Separate-operation tests prevent implicit Mill work; a signed deterministic
farm→Grain→Flour→Food campaign proves player guidance, a signed Ore→Smelter→Metal→Smithy→exact
metal Tool campaign proves item identity and delivery, and passive Captain twins forge metal Tools
at one- and five-minute cadence. SQLite v6→v7 and signed HMAC restart campaigns preserve the new
queue entitlements. The research ledger displays unsupported promises as disabled `FUTURE` cards.
The client-owned framebuffer `/tmp/research-future-1024.png` is a 1090×2105 RGB PNG (SHA-256
`1201f2cea5327c4da640fe5d0fe3ad289d61cf342e71e82941137a7f14c2e642`) captured from the live
client against a booted server and visually inspected: the Baking family is filtered into view,
every card is marked FUTURE, the selected inspector says “Planned content — not yet researchable,”
and the frame is rendered rather than black. The temporary staging/capture systems were removed.
Final gates pass 1,267 simulation (one intentional skip), 46 protocol, 98 server, and 145 client
tests plus strict four-crate Clippy, formatting, and diff checks.

## 2026-07-15 — Crews research owns real independent work stations

**Problem:** Every generated building family advertised a `worker_slots +1` Crews study, but
the runtime, save, socket, and client could represent only one worker. Buying any of those 25
studies therefore produced no observable effect.

**Fix:** The twelve building families which already own real physical labor now gain one durable
station when their exact Crews study is owned: Workshop, Wood Cutter, Stone Prep, Woodworking,
Smithy, Clothier, Tannery, Smelter, Mill, Sawmill, Research Hut, and School. Each station owns its
worker, automation provenance, selected recipe queue, pause state, and fractional progress.
Physical processor stations share the building's finite input/output stores and contend in stable
slot order, so a second paw can neither duplicate inputs nor mint output. Player assignment,
officer automation, death, release, reassignment, old saves, SQLite restart, snapshots, and the
Bevy station inspector all use that same state. The inspector lets the player select a station and
edit only its queue.

The other thirteen catalog nodes remain visible and explicitly say `(future)`, but cannot be
purchased and cannot block the following study: Den, Food Storage, Water Bowl, Beds, Herb Garden,
Nursery, Elder Corner, Walls, Mouse Farm, Shrine, Field, Accounting Tent, and Barracks do not yet
own an independent labor station. This is deliberate truthfulness, not invented work for passive
storage, housing, walls, or ritual structures. Field and Accounting Tent retain their existing
single physical route until those state machines can safely support concurrent ownership.

**Evidence:** Research-cap isolation, independent same/different queues, finite shared-input
conservation, per-worker skill/tool attribution, death promotion without lost work, deterministic
tick partitioning, signed assignment and exact-station controls, legacy/default wire shapes, and
SQLite restart are regression-covered. Passive officer and signed player-guided campaigns exercise
the no-input and directed paths. The accepted client-owned 1090×2105 RGB framebuffer
`/tmp/crews-work-stations.png` (SHA-256
`9b4082ce125d7ef64420ed8c66c003c5070b8ed95f12b69ea6feb730b0125b75`) was captured against a
booted authoritative server and visually inspected: the selected Woodworking inspector shows
`staffed: 2/2`, two independently progressed stations (35% and 68%, the second paused), and the
worker selector plus exact queue controls without clipping. The capture-only staging system and
both processes were removed before the final gates.

## 2026-07-15 — Functional equipment has one finite physical authority

**Problem:** Tool, Weapon, and Armor recipes ended as scalar counters while the item store also
held weighted, durable units. Crafting, carrying, wearing, work wear, combat wear, repair, sale,
death, and restart therefore could not name one authoritative object without either losing it or
counting it twice.

**Fix:** Every functional unit now has one stable identity and one explicit location: station
input/output, carrier, stockpile, equipped cat, trader wagon, or the legacy migration boundary.
The old scalar fields remain readable save/wire compatibility projections derived from credited
finite units; they are no longer a second inventory. Woodworking and Smithy create the exact unit
in local output, the same ID travels outbound, and compatibility credit occurs only on first final
delivery. Signed equip/unequip targets one living owned cat and one exact item, with one slot per
kind. Work and raids wear only the contributing/equipped IDs. Captain issue is physical,
priority-ordered, non-sticky, and cannot interrupt a cat's survival or station route. Broken gear
stays visible and repairable. Death, departure, source removal, and extinction recover an ID only
into accepting capacity or a persisted world spill. Exact trader sale debits the real source and
moves that same ID atomically. Rules-v1 migration and SQLite persistence preserve location,
condition, credit, auto-issue, and active-job provenance. Exact equipment detail is redacted unless
the signed selected-colony controller has an accurate physically counted Accountant ledger.

**Evidence:** The end-to-end exact-ID campaign follows one Tool through local craft output,
outbound carrier, credited stockpile, JSON restart, retrieval/equip, exact wear to broken,
capacity-aware unequip, material repair, and exact trader sale while asserting that identity and
scalar/pile projections never duplicate. Passive Captain campaigns at one-minute and five-minute
cadence and signed player-guided equip/unequip/repair/sale campaigns cover both unattended and
directed play. Conservation tests cover carrier death, old age, migration, reset, full former
sources, spill recovery, multi-raid wear, reversed-ID priority, and non-preemption of hunt, water,
and station jobs. Server tests cover HMAC ownership, foreign/dead/wrong-bearer denial, restart,
redaction, stale Accountant books, and exact-source sale rollback. The accepted client-owned
framebuffer `/tmp/finite-equipment-1024.png` is an exact 1024×768 RGB PNG (SHA-256
`cc18adfdab2d00b43fccaf95b44784a6142c47f043ec9eb6e26aea1f19c1ff9d`). It was captured against a
booted server and visually inspected: stable IDs and condition are readable for equipped,
carried, workshop-output, stored damaged/broken, and trader units; repair and exact unequip are
visible; Goods, Cat, top bar, and bottom toolbar do not overlap. The minimap deliberately yields
only while Goods and Cat jointly need its corner. All capture-only staging was removed. Final
gates pass 1,259 simulation tests (one intentional instrumentation skip), 46 protocol tests, 97
server tests, and 144 client tests; strict Clippy is green for all four crates, with formatting and
diff checks clean.

## 2026-07-15 — Smithy physically forges one selected scalar gear output

**Problem:** Smithy still spent colony-global Metal through two hidden parallel timers. Its visible
selected queue, worker, local inventories, and cargo did not own the batch, and one cycle minted a
Weapon and Armor independently of physical delivery.

**Fix:** One Metalwork worker now carries exactly two Metal into the selected Smithy, works one
900-game-second `smithy_weapon` or `smithy_armor` batch, leaves one whole selected output in local
storage, and carries that unit to finite storage before aggregate credit. Queue order, one-shot,
repeat, pause, and deliberate emptiness are authoritative. Captain automation is comfort-gated and
non-sticky, but may finish exactly one committed batch; signed manual staffing bypasses that
comfort gate. Full output storage, missing input, research lock, vacancy, death, removal, and worker
replacement conserve whole cargo. Orphan recovery floors both gear amount and destination
headroom, so a whole Weapon/Armor never becomes fractional cargo when less than one unit fits.
The legacy aggregate forge timers remain bit-frozen, and this
slice deliberately does not mint a second finite `ItemStore` identity for scalar gear. Station
rules v6 is version-only and preserves exact v5 authored Smithy state.

**Evidence:** Deterministic one-, five-, and 60-second cadence tests cover both selected outputs,
whole-unit capacity/headroom (including removed-Smithy recovery at 0.5 headroom), skill gating,
recovery, automation, and frozen compatibility state.
A signed guided pure-sim campaign proves Ore→Smelter→Metal→Smithy→Weapon through every
inbound/local/outbound boundary. An authentic HMAC server campaign preserves authored paused
queues across SQLite restart, reconnects the bearer, resumes production, and credits exactly one
Weapon only after delivery. Established no-input Captain twins produce at both one-minute and
five-minute cadence without a reset. Protocol/client tests cover the additive `weapons` and
`armor` cargo literals, their existing semantic tracked icons, and the selected station's truthful
worker/progress/inventory/queue presentation. The mandatory selected-Smithy client-owned
framebuffer `/tmp/physical-smithy-c2.png` is an exact 1024×768 RGB PNG (SHA-256
`833082b06e6b95172bc1afe1e22a4d3e2e34787381538cce629f675466226429`). It was captured from
the live client against a booted server and visually inspected: the open Smithy, assigned hauling
cat, 50% Weapons batch, two Metal inbound/local, one whole Weapon local/outbound, repeat queue,
block reason, and queue controls are all visible without a black frame. The temporary staging and
capture systems were removed afterward. Final gates pass 1,233 simulation tests (one intentional
instrumentation skip), 44 protocol tests, 93 server tests, and 136 client tests; strict Clippy is
green for all four crates, with formatting and diff checks clean.

## 2026-07-15 — Fibre and Clothier production are finite physical routes

**Problem:** Fibre forage credited the colony at job completion, while Clothier spent that global
counter and minted Cloth plus hidden Clothing through parallel timers. Queue, worker, cargo, and
inspector state therefore did not own the visible batch.

**Fix:** A completed forage now leaves bounded Fibre in the gatherer's paws and credits it only on
finite pile delivery. One Textile worker carries exactly five Fibre into the selected Clothier,
works one 600-game-second `fibre_to_cloth` batch, and carries one Cloth to finite storage. The
legacy Clothing timer is bit-frozen. Cloth Leader staffing is comfort-gated, non-sticky, bounded
before director fill, and deterministically alternates need across Clothier and Tannery while
committed cargo/work may finish. Rules-v5 persistence is version-only and preserves exact v4
queue order, repeat, pause, progress, and intentional emptiness.

**Evidence:** The complete physical route is verified at one-second, five-second, and 60-second
cadence. Five signed guided forages exercise physical Fibre delivery into the chain. An authentic
HMAC campaign crosses SQLite restart without losing authored queue or route state, and established
no-input runs produce Cloth at both 60-second and five-minute cadence. The accepted client-owned
1024×768 framebuffer `/tmp/physical-clothier-c2.png` (SHA-256
`2c00b7edaacb24037c81acb3d8bc262e9a4f165163c1ccf64326288f4e649ccd`) shows the selected
open-top Clothier, Moss and distinct Fibre/Cloth carrier art, 40% progress, inbound/local Fibre 5,
local/outbound Cloth 1, and every queue control without clipping or overlap.
Final gates pass 1,226 simulation, 44 protocol, 91 server, and 136 client tests plus strict
four-crate Clippy, formatting, and diff checks.

## 2026-07-15 — Tannery physically converts hunted Hide into Leather

**Problem:** Tannery still consumed colony-global Hide through a hidden parallel timer. Its visible
`hide_to_leather` queue, worker, and local inventories did not own the batch, so selected queue
intent could not account for the source Hide or resulting Leather.

**Fix:** One assigned Textile worker now fetches exactly five Hide from finite storage, works one
selected 600-game-second batch, leaves one Leather in station-local output, and physically carries
that unit to finite storage before aggregate credit. Queue order, repeat, one-shot, pause, and empty
state are authoritative. Full Leather storage, missing Hide, research lock, vacancy, low comfort,
death, and building removal preserve worker and cargo truth. The legacy aggregate Tannery timer is
frozen. Station-rules v4 changes only the marker and does not seed or rewrite authored queue state.

**Evidence:** Deterministic 1s/5s/60s route tests cover every physical boundary, conservation,
release/replacement, multi-station staffing bounds, and frozen compatibility state. Passive play
proves a Cloth Leader can run an established Tannery without further input. A guided campaign
attributes real Hide-in-paws to the exact signed player Hunt, then proves Hide inbound/local and
Leather local/outbound/final delivery. The server version repeats that HMAC route across SQLite
restart, including authored pause/queue/no-seed assertions and bearer reconnection. Protocol and
client tests cover the additive `leather` carrying literal, its unique semantic icon, and truthful
worker/progress/inventory/queue controls. The accepted exact client-owned 1024×768 framebuffer
`/tmp/physical-tannery-c2.png` is recorded with the P19.C2 board evidence. Final gates pass 1,218
simulation, 44 protocol, 89 server, and 136 client tests plus strict four-crate Clippy.

## 2026-07-15 — Woodworking physically combines Planks and Blocks into scalar Tools

**Problem:** Woodworking still spent colony-global Planks and Blocks through an invisible parallel
timer. Its visible `planks_and_blocks_to_tools` queue, worker, and local inventories did not own
the batch, so one selected recipe could not account for the two inputs or the resulting Tool.

**Fix:** One assigned Craft worker now fetches exactly two Planks and then two Blocks from finite
storage, waits until both loads are local, consumes them atomically in one selected
600-game-second batch, leaves one whole scalar Tool in local output, and carries that whole unit to
finite storage before aggregate credit. Queue repeat, one-shot, pause, and deliberately empty state
are authoritative. Planks and Blocks independently preserve the four-unit construction reserve;
low-comfort automation releases a worker that has only local inputs, while real cargo, positive
progress, or local output retains the worker until conserved work finishes. Full Tool capacity
prevents both early reservation and fallback staffing. The legacy `wood_craft_progress` field is
persisted bit-for-bit but never advances. Station-rules v3 changes only the version marker: unlike
the older C2.0/C2.2 migrations, it never seeds or rewrites a Woodworking queue.

This bounded C2 slice originally produced the existing scalar Tool only. P19.C3 subsequently
completed finite Tool identity, condition authority, and compatibility-scalar migration without
minting a duplicate object; see the verified finite-equipment entry above.

**Evidence:** Conservation tests cover ordered two-input fetch, atomic consumption, whole-unit
output/headroom, no early credit, pause/empty/nonrepeat queues, low-comfort release, full-capacity
automation, death salvage, building-removal recovery, construction reserves, and stable complete
routes at 1s/5s/60s cadence. SQLite tests preserve both local inputs, local output, transit cargo,
queue/pause/progress, and the frozen legacy timer exactly; v2→v3 migration preserves an explicitly
empty or authored queue. The signed HMAC guidance/restart campaign assigns a worker and edits the
Woodworking queue itself. The accepted client-owned 1024x768 framebuffer
`/tmp/physical-woodworking-c2-stable.png` shows the selected station's two local inputs, local and
outbound whole Tool, active worker/progress, editable queue controls, and a distinct semantic Tool
glyph on the shrine-adjacent carrier without inspector/minimap overlap.

## 2026-07-15 — Stone Prep is a conserved physical Stone-to-Blocks station

**Problem:** Stone Prep still consumed colony-global Stone through an aggregate timer while its
visible `stone_to_blocks` queue, worker, and local inventories were decorative. A second hidden
Blocks-to-trade-good timer could also run beside the declared recipe.

**Fix:** One assigned Process worker now carries exactly five finite Stone to local input, performs
one selected 600-game-second batch, leaves one Block in local output, and carries it to finite
storage before aggregate credit. Queue order, repeat, one-shot, pause, and empty state are
authoritative. The founding recipe requires no study. The legacy `stone_craft_progress` value is
persisted bit-for-bit but never advances or spends Blocks. Station-rules v2 seeds only pre-v2 empty
Stone Prep queues once, so a player-cleared v2 queue remains empty across restart. Non-sticky
Forester staffing retains inbound/outbound station cargo, positive batch progress, and local output
until conserved delivery completes. Local-input-only stock stays dormant below the per-capita
comfort runway, then resumes when food and water recover; the same safe boundary applies to Wood
Cutter.

**Evidence:** Conservation tests cover exact 5:1 conversion, no early credit, one worker, full
storage, death, removal/orphan salvage, pause/empty release, progress and queue restart, and stable
full-route projections at 1s/5s/60s. Signed HMAC server tests persist assignment and queue edits.
A signed fresh-zero-Stone deterministic campaign observes quarry Stone in paws, ordinary deposit,
station-in cargo, local Stone, local Blocks, station-out Blocks, and final delivery; a separate
valid-office Forester twin produces local and banked Blocks for 45 minutes with no further input.
The accepted client-owned framebuffer `/tmp/physical-stone-prep-1024.png` is exactly 1024×768 and
shows staffed `1/1`, the repeating queue, local/input/output and transit truth, finite-storage
blocking, all queue controls, and nonoverlapping panels. Temporary capture fixtures were removed.
Final acceptance is green: `cat-sim` 1201/1201 (one configured skip), `cat-protocol` 44/44,
`cat-server` 84/84, and `cat-client` 136/136; strict all-target Clippy passes for all four
crates, with formatting and diff checks clean.

## 2026-07-15 — Every carried resource has a semantic tracked glyph

**Problem:** Only Materials and Blessings loaded their declared carry art. The other eighteen
`CarryingKind` values rendered as colored squares, including Stone even though it declared a
world-prop path that `SpriteSheets` never loaded. A moving cat therefore did not truthfully show
what it was hauling.

**Fix:** All twenty-one physical cargo identities now use the exact corresponding maintained HUD
glyph under `public/images/game/icons/`. One exhaustive mapping owns both path and tint, every
handle loads at startup, and the square fallback no longer exists. Food, Fish, Water, Materials,
Stone, Refined, Blessings, Logs, Lumber, Planks, Blocks, Tools, Catnip, Grain, Flour, Herbs, Hide,
Bone, Leather, Ore, and Metal remain visually distinct; notably Lumber and Planks retain separate tracked
symbols rather than sharing a generic wood mark.

**Evidence:** An exhaustive client test rejects a missing cargo kind, path aliases, non-icon
paths, missing files, and non-PNG files. The client's own exact 1024×768 framebuffer at
`/tmp/semantic-cargo-icons-1024.png` shows ten simultaneous truthful loads, including Food, Fish,
Water, Stone, Logs, Lumber, Planks, Grain, Bone, and Metal, without fallback squares or clipping.

## 2026-07-15 — Wood Cutter is a conserved physical Logs-to-Planks station

**Problem:** The founding Wood Cutter minted Planks from the colony-wide Supplies pool through a
hidden aggregate timer. Its visible `logs_to_planks` queue, local inventory fields, and
queue did not control work, so the canonical production table and
inspector could not tell the truth about any particular batch.

**Fix:** One assigned Process worker now carries exactly five finite Logs from an accepting pile to
the Wood Cutter input, performs one 600-game-second selected queue batch, leaves one Plank in local
output, and physically delivers it before aggregate credit. Ordered repeat, one-shot, pause, empty,
and locked queue states are authoritative. Full output storage suspends without loss; worker death
and building removal leave every unit recoverable. The baseline recipe is founding-available in
fresh rules-v1 colonies; Carpentry studies gate later recipes, not this early chain. A monotonic
`physicalStationRulesVersion` migration seeds an empty pre-column C2.0 queue once. That legacy shape
cannot distinguish default-empty from intentionally-empty future intent; after the version is set,
a player-cleared queue stays empty across every restart.

**Evidence:** Focused conservation tests cover no-early-credit, finite headroom, tool wear, truthful
Process/Haul XP, one-worker continuity across fetch/work/output, and a stable full-route projection
at 1s/5s/60s cadence. Queue controls, death/removal, SQLite restart, and signed HMAC assignment plus
queue restoration are covered. A deterministic unattended founding run banks Planks under Forester
automation. A separate signed player campaign chooses the worker and repeating recipe, orders a real
tree harvest, observes the stump, Logs in paws and station input, local Planks, and final finite-store
delivery. The client's own exact 1024×768 framebuffer at
`/tmp/physical-wood-cutter-1024.png` shows the selected station's five local Logs, one local Plank,
one outbound Plank, paused queue, all seven queue controls, a nonoverlapping minimap, and Whiskers
visibly carrying the semantic Planks glyph after the connection toast clears.

The final acceptance gate passed all 1,460 four-crate tests (one intentionally skipped), strict
all-target Clippy for `cat-sim`, `cat-protocol`, `cat-server`, and `cat-client`, formatting, and
diff hygiene. That gate also corrected adjacent legacy fixtures so Supplies can never silently
fund the physical Wood Cutter in tests or production.

## 2026-07-15 — All 25 HUD resources use semantic tracked icons

**Problem:** Thirteen resource rows still borrowed map terrain, farm stages, furniture, world props,
or the generic-goods glyph. Stone and Bone were especially misleading: raw Stone reused its map
pile and Bone reused a catch-all sack, despite the HUD contract requiring an icon readable before
the adjacent label.

**Fix:** Every maintained resource now resolves to a unique PNG under
`public/images/game/icons/`. Stone uses the tracked block/ingot glyph and Bone uses Kenney Fish
Pack's 128 px blue fish-skeleton glyph. The remaining substitutions were replaced with the
documented Board Game/Fish symbols for Fish, Catnip, Grain, Flour, Logs, Lumber, Fibre, Hide,
Cloth, Leather, Ore, and Metal. The client keeps the distinct labels, values, and resource-specific
tints while an exhaustive test locks the 25-entry protocol/HUD bijection and rejects paths outside
the semantic icon directory.

**Evidence:** All 135 `cat-client` tests, strict all-target Clippy, formatting, and diff checks
pass. The client's own exact 1024×768 framebuffer at `/tmp/semantic-hud-25-final.png` shows all
25 icon/value rows within the parchment panel without overlap or clipping.

## 2026-07-15 — Remaining stations own behavior-neutral recipe descriptors

**Problem:** Wood Cutter, Stone Prep, Woodworking, Clothier, Tannery, and Smithy had no common
recipe/queue/resource-domain authority to receive the physical station contract. Moving each route
directly would have repeated IDs, research maps, and block-reason logic while risking an accidental
change to established aggregate production.

**Fix:** One data module now owns all eleven maintained station recipe IDs and canonical resource
sets. The six remaining benches receive deterministic default queues, generic signed queue and
persistence state, exact catalog-derived rules-v1 availability, rules-v0 grandfathering, and
selected-recipe snapshot/block metadata. Stone Prep's descriptor consumes the new raw Stone kind;
Materials remain Supplies. The compatibility benches report `aggregate_timer_compatibility`, and
an exact six-bench twin proves queue order, empty state, and pause state do not alter their existing
aggregate timers. Additive trade-craft timers remain outside this bounded descriptor slice.

**Evidence:** Descriptor uniqueness/resource-domain tests, exact catalog reverse-index tests,
locked/owned/grandfathered block-reason tests, multi-recipe Smithy selection, signed snapshot and
protocol round trips, all 1,175 executed simulation tests (one intentional skip), all 44 protocol
tests, strict sim/protocol Clippy, formatting, and diff checks pass. This verified C2.0 scaffolding
only; later cards closed the then-open station-local cargo/work/output conversion.

## 2026-07-15 — Raw Stone and source byproducts are finite physical cargo

**Problem:** Quarry rock was still credited through the stable generic `materials` field, Stone
Prep dressed that same Supplies pool, and quarry/hunt byproducts could appear as aggregate stock
without a carried load. Adding Stone naively would also have stranded the renewable Supplies route
used by the Workshop and shrine offerings. Partial logging had a related conservation hole: the
tree was marked felled only on the final trip, so an interrupted early haul could target it again.

**Fix:** Stone is now its own defaulted save, wire, storage, trade, stockpile, cargo, HUD, and
Accountant-projection field. Missing legacy Stone defaults to zero while existing `materials`
remains bit-exact Supplies. Quarry workers carry three raw-Stone loads and one distinct positive
rubble/Supplies load; only a persisted Mountains site adds a fifth Ore load. Hunts now carry three
Food loads followed by separate Hide and Bone loads. Bone has its own defaulted save, wire,
storage, stockpile, cargo, trader, HUD, and private Accountant identity. Stone Prep consumes only
Stone. Every result credits aggregate stock only
after finite delivery, and the loaded-site Ore manifest avoids terrain generation in the active-job
hot path. The first positive logging extraction now writes the stump immediately, while an active
job reservation prevents premature replant/retarget until the job ends.

**Evidence:** Legacy JSON and SQLite migration, full persistence, trade depletion, stockpile
capacity, death/cancel/full-storage conservation, private Accountant reports, ordinary-versus-
mountain manifests, logging interruption/retry, and deterministic unattended and signed guided
campaigns cover the boundary. The guided fresh village begins at exactly zero Stone, physically
quarries it, and grows Blocks without debiting Supplies. The inspected client-owned framebuffer
`/tmp/raw-stone-bone-final.png` is exactly 1024×768, renders the top-down village normally, and
shows truthful counted Stone `~12/100` and Bone `~3/100` rows with distinct icons and no clipping.
The final gates pass 1,169 simulation, 43 protocol, 82 server, and 134 client tests plus strict
Clippy for all four crates. Bone item variants and downstream recipes were open at this dated slice
boundary and were closed by later breadth cards; the raw hunt source and scalar were already live.

## 2026-07-15 — Guided founding economy reaches farms, offerings, and bounded route planning

**Problem:** Broad player-guided and unattended runs exposed three integration failures hidden by
focused fixtures. The material-offering decision protected too much Supplies to ever reach its own
pickup threshold, the Farmer could treat a promised but unpaid Field as satisfying the essential
food floor, and the guided farm→Mill smoke sometimes appointed a Steward before demonstrating the
vacant-office manual road dependency. Repeated exact construction-road A* probes also dominated
long fishing campaigns even when village topology had not changed.

**Fix:** The offering decision bar is 20 Supplies: ten for the carried offering and ten retained for
operations. Essential Field demand now counts only completed physical Fields and reopens Supplies
for a fundable plank bill; comfort can stop only discretionary Fields after the minimum floor.
Non-sticky raw benches restaff while buffers are deficient without reserving dormant offerings.
The signed campaign now plans and manually paves a second Workshop while the Steward seat is vacant,
asserts the visible paved-road event immediately, appoints the Steward afterward, and continues to
physical Field harvest and Mill delivery. Construction access-road candidates are cached by a
deterministic topology signature, bounding unchanged exact route probes without caching mutable
claims or future sites.

**Evidence:** Five unattended 200-game-hour founding-economy seeds each produce tithes, one physical
offering, and blessings while the established population matrix retains its essential Field floor.
The signed pre-earned-research replay twin purchases its dependencies, lays the manual road, harvests
a staffed Field, and delivers both Flour and Food through the Mill bit-for-bit. Guided and unattended
fishing campaigns cover all maintained seeds with at most eight exact construction-road queries;
the authoritative full simulation gate and final touched-crate gates are recorded on P19.C1.

## 2026-07-15 — Visiting traders are physical and finite

**Problem:** Visiting merchants were a timer-backed menu with effectively infinite stock. They did
not walk into the village, own conserved cargo, wait behind a closed route, or visibly leave.

**Fix:** Each visit now owns a deterministic reachable exterior, follows ordinary obstacle-aware
A* through the retained gate to physical shrine contact, trades only while present, and returns to
a still-valid exterior before despawning. The merchant carries a finite deterministic resource
manifest, finite purse, 100 kg wagon, and exact stable item units. Purchases deplete exact stock and
show sold-out truth; sales preserve item identity and stop at purse or cargo capacity. Phase, route,
deadline, inventory, and cargo persist across restart. Expansion rehomes an exterior that becomes
claimed. Exact transition times agree across one-second, minute, hourly, and coarse tick partitions
without granting backdated travel after a blocked boundary. The bounded panel pages every offer and
uses only Accountant reports when explaining storage blocks.

**Evidence:** Focused route, passability, closure/reopen, shrine-contact, cadence, rehoming,
conservation, sold-out, SQLite restart, signed buy/sell/depletion/denial, protocol, pagination, and
privacy tests pass. A deterministic seed-41 live-cadence 60-hour passive twin observed arrival,
the complete shrine trading window, and physical departure without deaths or reset. The accepted
client-owned 1024×768 logical framebuffer `/tmp/trader-physical-1024.png` shows the merchant at the
shrine, page 2/2, finite quantities, Food sold out, bounded controls, and report-derived storage
guidance without exposing exact private headroom. The first broad gate exposed a blocked-route
timestamp regression that focused tests had not exercised: reopening could reuse elapsed time from
the unavailable window and collapse arrival, trading, and departure into one coarse call. A
defaulted persisted blocked-route marker now gives the reopened route only the new tick's movement
while timestamping contact/departure at that observation boundary. Both exact reopen guardrails,
all 23 trader tests, the full 1,153 simulation tests, all 80 server tests, strict touched-crate
Clippy, formatting, and diff checks pass.

## 2026-07-15 — Foresters physically replant felled trees

**Problem:** Logging converted every generated tree into a permanent stump. The maintained design
assigns felling, replanting, and lumber to the Forester, but there was no replant job, growth clock,
or visual/persisted route back to the deterministic forest.

**Fix:** `ReplantTree` is one signed manual order and one Forester-owned automation job. Manual
orders remain available while the office is vacant; the founding Leader never plants. A worker
must have a real route from the shrine, walk to the exact mapped/revealed stump, accept it, and
complete thirty game-minutes on site. The finite input is that stump's surviving coppice/root
stock: completion atomically changes the existing persisted tile from `stump` to `sapling` and
records `planted_at` in its existing `last_depleted` clock without inventing a seed resource.
After 24 game-hours the sapling restores the same seed-derived mature tree. Buildings, farms,
stockpiles, agriculture, water, mountains, roads, perimeter walls, village interiors, and rocks
delay growth without deleting it. Stumps and saplings cannot be logged as mature trees.

**Player visibility and evidence:** Snapshots expose separate stump/sapling anchor sets; Bevy hides
the mature canopy and renders tracked top-down stump/sprout art until growth completes. Manual and
officer campaign tests cover vacancy ownership, bounded automation, reachable-site denial,
arrival-controlled work timing, death/cancellation, finite mutation, delayed and obstructed retry,
reveal-clock idempotence, cadence partition determinism, SQLite job/tile restart, and the final
`overlay = None` becoming the exact original generated logging target again. The integrated full
gates pass 1,176 simulation, 43 protocol, 82 server, and 135 client tests. The later conflict-free
recipe-descriptor rebase passes 18 focused Forester/source-cargo/descriptor tests, all 44 protocol
tests, strict four-crate Clippy, formatting, and diff checks. The accepted own-framebuffer capture
used temporary render-only
lifecycle anchors inside the visible village to
isolate stump/sapling readability and canopy suppression; it does not imply that simulation-valid
replanting or regrowth may bypass the authoritative village-interior and occupancy safeguards.

## 2026-07-15 — Accountant reports are the player-wire inventory boundary

**Problem:** The client displayed the Accountant's stale aggregate and per-pile reports, but the
same owner WebSocket payload also carried authoritative colony resources, exact pile contents,
and aggregate/per-pile `accurate` comparisons. Reading JSON bypassed every physical counting round.

**Fix:** The server retains its exact completed snapshot cache, then applies one mandatory socket
projection for initial connections, broadcast ticks, and post-action/reconnect snapshots. Physical
`resources`, the duplicate threat weapon/armor totals, and every visible pile's `contents` now come
from the last report; uncounted piles project zero contents. Equality attestations are cleared and
omitted. Exact blessings remain visible because they are a non-stockpiled divine currency, not part
of the Accountant's physical inventory remit. New offer/block metadata must not copy an exact
resource total or reintroduce an equality oracle; whole-payload sentinel tests guard numeric leaks.

**Evidence:** Authenticated personal-owner tests cover vacant and blocked books, a wholly uncounted
pile, exact trusted-cache preservation, initial cached delivery, tick broadcast, signed post-action
refresh, and reconnect. Unique authoritative sentinels are absent from the complete serialized JSON,
not merely from the documented resource fields. Protocol compatibility defaults omitted accuracy
fields to conservative `false`; signed Accountant restart/release coverage remains green. The full
42 protocol, 76 server, and 130 client suites plus strict touched-crate Clippy and formatting pass.

## 2026-07-15 — Scaffold construction uses conserved physical inputs

**Problem:** Exact and autonomous construction subtracted Planks/Blocks from aggregate resources
when a scaffold appeared. No cat fetched the pinned bill, construction could progress without a
delivery, and restart/removal/death had no durable source→transit→scaffold accounting.

**Fix:** Placement now atomically reserves the exact type-local escalating bill from finite visible
piles, preferring Lumber and filling any shortfall with Planks. One living builder carries bounded
loads from each deterministic source into persisted transit and scaffold-input stores. The build
timer remains absent and progress stays zero until every pinned unit is physically delivered, at
which point the exact input is consumed once. Reservations protect the bill from crafting, repair,
trade, gather-spot movers, Steward balancing, station hauling, and the late-removal recovery
window. Reconciliation drains ordinary surplus before reserved sources or exact scaffold inputs;
aggregate restoration is limited to construction-local physical goods as legacy corruption safety.
Death spills at the cat's real tile; source loss replans to recovered visible goods; cancellation,
reassignment, scaffold removal, and restart conserve every unit. Legacy incomplete scaffolds with
no contract remain grandfathered as already funded.

**Evidence:** Player and Leader placement, split multi-pile Lumber/Planks fallback, partial loads,
no-early-progress, source loss, death/reassignment, removal and same-window spend protection,
blocked empty-paw source and loaded return routes, ready-input replacement arrival, pinned speed
across mid-build research/reassignment, deterministic one-/five-second cadence, SQLite
mid-haul/legacy restart, signed HMAC completion, and client inspector tests pass. The selected-
scaffold inspector always exposes required, delivered, in-transit, and blocked/building stage.
The final gates passed 1,139 simulation, 42 protocol, 75 server, and 130 client tests plus strict
Clippy for all four crates. The accepted live 2048×1152 own-framebuffer
`/tmp/scaffold-physical-inputs.png` shows the intact Grand Commons world and selected open
Woodworking scaffold at 0% with Lumber/Blocks required, partial Lumber delivered, no cargo in
transit, and its truthful blocked state.

## 2026-07-15 — Founding benches no longer require Basic Tools for placement

**Problem:** Wood Cutter, Stone Prep, and Woodworking are fixed P16 founding benches whose later
copies remain placement-available, but signed construction had a second hard-coded `basic_tools`
check after the catalog resolver. That study models hunt yield, not station placement, so a fresh
personal or communal village could see each bench in its blueprint yet could not request another
copy.

**Fix:** The duplicate action-layer gate is removed. The three exact building-family records now
declare `availableAtFounding` in catalog data, producing one non-purchase
`BuildingAvailableAtFounding` marker alongside each first durability modifier. The marker changes
neither study prerequisites, cost, order, category, modifier, nor daily Leader choice. Missing and
owned `basic_tools` states therefore have identical placement behavior; future research remains a
recipe entitlement, not permission to place another bench.

**Evidence:** Catalog validation pins one founding source and no `UnlockBuilding` competitor for
each bench while retaining the exact 167 Building / 167 RecipeResource / 166 Upgrade split and the
first durability payload. Deterministic signed personal/communal plans, exact scaffold payment,
reservation/overlap and mutation-free denial, authenticated server HMAC, and SQLite restart tests
cover placement without altering production behavior at that slice. At that time all three
founding queues were physical; the aggregate Smithy's two selected recipes and finite-equipment
authority were still open.

## 2026-07-15 — Prosperity migrants physically enter and leave through the gate

**Problem:** Prosperity migration created or removed cats directly in colony state. A new migrant
could count as a resident, consume stores, take work, vote, fight, or claim housing before walking
into the village; an expired unhoused migrant disappeared without releasing work or visibly
leaving. The client therefore showed a probation countdown without a truthful physical journey.

**Fix:** Each cohort now receives one deterministic dry/passable exterior origin near the south
gate. The origin and `Arriving`/`Probationary`/`Departing` phase persist in the existing migration
JSON, with legacy records defaulting to already-present probationers. Authoritative A* movement
alone carries cats through the current gate, so a blocked route waits and resumes without
consuming the cohort, Shipping never permits water walking, Rail remains neutral, and gate
relocation uses the new wall topology. Only physical entry emits `MigrationArrived` and starts the
36-game-hour housing clock. Expiry conserves cargo, cancels jobs and role ownership, then routes
the cat back to its persisted origin; only physical exit removes it and emits
`MigrationDeparted`. Arriving/departing cats are visible but excluded from needs, survival,
labor, research, rituals, officer dispatch, elections, combat, housing, and signed assignments.

**Evidence:** Focused migration, exact water/mountain passability, 120-second blocked-route
performance/reopen, moved-gate, cadence partition, cargo/job recovery, death/reset, legacy serde,
SQLite mid-arrival/mid-departure restart, protocol/client status, personal guided Den retention,
signed server guided-vs-unattended, and three-seed organic 150-game-hour campaigns pass on the
integrated transport/entitlement/Accountant base. The exact live-cadence 48-hour passive run kept
all 15 cats healthy while completing hunt, water, Leader scout, shrine-delivery, and fog-expansion
loops. The final touched-crate gate passed 1,363/1,363 tests with one intentional skip; strict
Clippy and formatting passed. An inspected 3,826×2,105 client framebuffer selected an arriving
cat walking through the south gate and showed the truthful `MIGRATION: ARRIVING THROUGH THE SOUTH
GATE — NOT YET A RESIDENT` inspector state with needs, skills, and controls intact.

## 2026-07-15 — Truthful catalog job and aggregate-recipe entitlements

**Problem:** Nine catalog job payloads were false: three founding/building capabilities were
already usable before their advertised studies, and six IDs were not runtime jobs at all. Only
Sawmill→Gather Logs matched live behavior, but both the signed action and Forester automation
duplicated that entitlement as hardcoded ownership checks. Textiles and the two Smithy design
studies also lacked recipe payloads for aggregate production that already existed.

**Fix:** Sawmill→Gather Logs is now the sole `UnlockJob`. A validated one-time catalog reverse
index drives both signed logging and Forester automation; denial is mutation-free and names
Sawmill. Fetch Water, Explore, manual research, and Barracks training remain founding/building
capabilities. Water Carriers, Textiles, and Den Insulation were reclassified without changing
stable identity, cost, prerequisites, order, or the live `housingPerDen` effect, preserving the
exact 167 Building / 167 RecipeResource / 166 Upgrade split. At that slice Textiles first received
the Clothier and Tannery recipe entitlements, while Weaponsmithing and Armorsmithing independently
owned the two Smithy outputs. By the end of that historical slice, Tannery and Clothier had moved
to entitlement-backed physical queues while Smithy remained aggregate with two selected recipes. Fresh rules-v1 production cannot
advance or spend inputs before its exact study; rules-v0 saves remain grandfathered. Smithy's
editable queue persists selected intent but does not replace its aggregate timers until the next
physical-station slice.

**Evidence:** Catalog uniqueness/runtime-ID/category and the then-full 500-node purchase checks, signed
logging denial/success, Forester non-bypass, founding personal/communal capability regressions,
fresh-v1 versus rules-v0 Clothier/Tannery/Smithy neither/one/both cycles, both Smithy forge arms,
SQLite restart, Leader daily-choice determinism, client-copy/no-queue-leak tests, guided and
unattended campaigns, touched-crate gates, and strict Clippy cover the slice. The accepted
own-framebuffers `/tmp/research-job-payload-truth-sawmill.png` and
`/tmp/research-job-payload-truth-textiles.png` show the sole job line `Unlocks logging` and the
human recipe names `Cloth weaving` and `Leather tanning`; the temporary capture hook and processes
were removed.

## 2026-07-15 — Transport research no longer conjures vehicles

**Problem:** Owning Shipping made every water tile walkable by an ordinary cat, while owning Rail
gave any cat three-times speed whenever its remaining destination was at least 40 tiles away.
Neither effect required a dock, vessel, track, train, boarding step, or physical route, so research
ownership bypassed the game's physical-logistics contract.

**Fix:** The stable `water_travel` and `rail_logistics` capability IDs now mean blueprint
entitlements only. Water remains a hard obstacle for ordinary walking pathfinding regardless of
Shipping ownership. Rail ownership and remaining route length are neutral in physical movement
until tracks, vehicles, and routes exist. The two study descriptions and research-ledger payload
lines state that they grant blueprints rather than immediate travel.

**Evidence:** Ownership-on/off A* reaches exactly the same result across a full-width water
barrier; a real phase-34 cat advances exactly the same distance on a 50-tile route with or without
Rail; both comparisons have deterministic twins. A signed player campaign purchases the complete
prerequisite chain through Rail and Shipping, preserves the stable capability IDs, and rechecks
both physical denials. Existing fine-biome, stone-road, live-cadence founding, communal unattended,
and guided/unattended fishing campaigns remain green. Real rail and ship construction was open at
this slice boundary and is now verified. The accepted live own-framebuffers `/tmp/transport-rail-blueprint.png` and
`/tmp/transport-shipping-blueprint.png` visibly prove the two blueprint-only study descriptions
and payload labels; the temporary capture hook and isolated processes were removed.

## 2026-07-15 — Leader-owned daily research choice

**Problem:** The design assigns the once-per-rolling-real-day strategic study choice to the living
Leader, but runtime incorrectly required an appointed Loremaster and the ledger presented its
priority hint as an already active study.

**Fix:** A living Leader now deterministically spends existing research points on at most one
affordable study per rolling 24 hours. The colony-wide clock survives Leader replacement, run
reset, and SQLite restart through the legacy `lastLoremasterUnlockAt` column, so upgrades cannot
mint a free choice. Missing/null legacy values remain ready; zero points or no affordable target
never stamps the clock; clock rollback and multi-day offline jumps never create backlog. Signed
player purchases remain unlimited and outside that clock. Research labor/building automation,
comfort release, and every ritual path remain Loremaster-owned. The ledger now truthfully labels
the deterministic hint as `Leader priority`.

**Evidence:** Focused living/dead/bootstrap/succession/reset, exact-boundary/rollback/no-backlog,
positive-points/no-target, recipe/building entitlement, seven-role vacancy, ritual, signed manual
purchase, legacy-column restart, deterministic twin, and client-copy tests cover the authority
split without changing entitlement resolvers. The final touched-crate gate passed 1,334/1,334
tests (one intentionally skipped), strict Clippy, formatting, and diff checks; after removing the
temporary capture hook, all 126 client tests passed again. A live server/client framebuffer at
`/tmp/leader-daily-research.png` visibly proves the full ledger labels its hint `Leader priority`
and explains the Leader/Loremaster/player authority split in the selected-study inspector.

## 2026-07-15 — Truthful catalog-derived building placement research

**Problem:** The Research Hut was intentionally placeable before research could begin, but its
root study falsely claimed to unlock the hut. The generated `mill_foundations` study also claimed
to unlock the Mill even though the legacy `milling` study remained separately mandatory.

**Fix:** The catalog now marks the Research Hut explicitly available from founding. `milling` is
the sole Mill placement unlock, while `mill_foundations` is once again its declared durability
study with the same stable ID, cost, prerequisites, and daily-selection order. One catalog-derived
resolver drives signed placement acceptance and names the actual missing study in denial text;
catalog validation rejects competing founding/research placement sources.

**Evidence:** Exhaustive placement-source, catalog, signed deterministic, guided purchase/place,
unchanged daily-selection order/boundary, 48-hour unattended, server HMAC, SQLite restart, and client-copy tests
pass without changing the four live recipe entitlements or rules-v0 save grandfathering. The
accepted own-framebuffer `/tmp/research-building-unlock-truth.png` shows the full-page ledger's
Research Hut inspector saying `Available from founding: Research Hut`; the temporary capture hook
and processes were removed. Remaining daily Leader authority is tracked separately above.

## 2026-07-15 — Target-correct storage-capacity research

**Problem:** A capacity study on any completed building leaked its largest multiplier into every
resource. Simulation clamps, snapshots, physical pile routing, and village-trade checks also used
different capacity calculations, so the UI could advertise space that delivery could not use.

**Fix:** One authoritative calculation now drives the aggregate clamp, general-storehouse
headroom, snapshots, signed trade validation/deposit, and persistence. `food_storage_stores`
changes only Food/Fish/Herbs/Materials/Refined capacity, `water_bowl_stores` only Water, and
`smithy_stores` only Weapons/Armor. Unsupported building targets cannot leak globally. Trade
acceptance builds an exact physical deposit plan before mutation and therefore cannot lose goods
behind a debug-only assertion. Consumption refreshes the pile ledger before returning carriers;
source-less fresh expedition excess that genuinely cannot fit is explicitly abandoned and the
living worker released, while farm/gather/station cargo and death spills remain conserved.

**Evidence:** The capacity-only runtime resolver is bit-identical to full effect resolution across
the catalog. Target-isolation, legacy shrine/designated migration, exact physical
headroom, signed purchase/trade, adversarial no-route trade, persistence, fresh-overflow
termination, physical-cargo conservation, deterministic twins, and the untouched no-cheat guided
farm→Mill campaign pass. Remaining scope is the 22 unsupported `*_stores` targets and the fixed
10-unit station-local stores listed in the open queue. The accepted 3826×2105 own-framebuffer
shows researched Food/Fish capacity at 680 (baseline 600 plus the targeted 80) while Water remains
independently 200. The final sim/server gate passes 1,130 tests with one explicit skip, plus strict
Clippy, formatting, and diff checks.

## 2026-07-15 — Truthful processor-local capacity research

**Problem:** Nine physical processor families persisted real station input, output, and transit
stores, but every compartment stayed fixed at 10 units even after its targeted `stores` study.
Thirteen more generated `stores` studies belonged to buildings with no routed physical container,
so players could spend research points on capacity that did not exist.

**Fix:** Workshop, Mill, Sawmill, Wood Cutter, Stone Prep, Woodworking, Smelter, Tannery, and
Clothier now resolve one target-specific per-resource station capacity. Their `stores` study raises
all three owning compartments from 10 to 12, and the same authority controls input delivery,
finished-output production clamps, recovery headroom, snapshots, and the selected-building
inspector. General storage and trade headroom remain unchanged. The thirteen containerless
studies are omitted from the purchasable graph instead of becoming fake global warehouses; the
result is a truthful 487-study graph, still within the vision's “about 500” target. Existing saves
that paid for one of the retired inert studies remove the unknown ownership and refund its exact
historical point cost once, using removal itself as the restart-safe migration marker.

**Evidence:** An exhaustive catalog guardrail permits exactly twelve capacity payloads—the three
existing storehouse domains plus these nine station domains—and rejects every former inert target.
All nine upgraded/control and unrelated-target pairs cover input and output headroom. A passive
900-second deterministic processor twin stays within its 12-unit compartments and delivers real
output. A signed `mill_stores` purchase survives the same research/stockpile JSON used by SQLite,
changes wire-visible local capacity, and leaves global storage/trade capacity bit-identical; the
client inspector renders both local ledgers as `amount / 12.0 per resource`. An inspected
client-owned framebuffer showed both capacity rows in the live selected-building panel without
clipping or overlap. Final gates pass 1,264 sim tests (one explicit skip), 46 protocol tests, and
144 client tests, plus strict Clippy, formatting, and diff checks.

## 2026-07-15 — Physical Workshop and Smelter refining

**Problem:** Workshop Materials→Refined and Smelter Ore→Metal conversion mutated aggregate
resources without requiring the assigned cat to fetch inputs, work at the open station, or carry
finished output to finite storage.

**Fix:** Both refiners now use editable repeating queues, durable station-local input/output and
transit ledgers, physical source and destination routes, delivery-before-credit, per-completed-cycle
skill and tool wear, and deterministic partial-load handling. Removing a station leaves its local
goods at the former footprint for a living salvage trip; full storage, a filled destination while
en route, carrier death, and restart never teleport, create, or lose cargo.

**Player visibility and evidence:** The inspector exposes local and in-flight Materials/Refined and
Ore/Metal, queue state, progress, blocked reason, and worker travel. Signed player guidance,
cadence partitioning, legacy queue migration, SQLite restart, removal/death/full-storage recovery,
protocol/client coverage, 1,286 four-crate tests, strict Clippy, and formatting pass. Accepted
1920×1080 own-framebuffers show the selected roofless Workshop and Smelter with their real local
ledgers, outbound cargo, repeating recipes, and workers in transit.

## 2026-07-15 — Steward-managed physical station stockpiles

**Problem:** Physical processors still pulled inputs directly from the general storehouse. The
Steward did not create the limited local reserves promised by the role, and a naive implementation
could teleport goods, consume the player's stockpile budget, or lose cargo when an office, station,
route, carrier, or destination disappeared.

**Fix:** An appointed Steward now creates provenance-tracked, one-tile, exact-resource piles beside
every physical Mill, Sawmill, Workshop, and Smelter. Nine distinct piles cover one of each station;
the separate automation budget is sixteen. One durable source→transit→destination job balances
input deficits before output surplus, never rewrites player pile definitions, and never changes the
aggregate resource total. Vacating the office or removing a station leaves its pile and contents
dormant. Cancellation restores only available source headroom and persists any blocked remainder.

**Player visibility and evidence:** Active/dormant ownership, station provenance, carried resource,
route phase, and recovery blockage are visible in the stockpile inspector; unsafe removal is denied.
Deterministic all-four-station, priority, full-storage, partial-recovery, vacancy, station-removal,
death, signed HMAC, SQLite restart, protocol-compatibility, and client tests pass. Own-framebuffer
`/tmp/steward-stockpiles-final.png` shows a selected Sawmill Logs pile with resolved station
provenance; the other managed zones remain sprite/overlay-readable without persistent text plaques
over the open workshops. The unchanged no-cheat farm→Mill guided campaign passes in 87.793s after
topology-gated, minute-bounded, one-route-at-a-time planning.

## 2026-07-14 — Finite item wear, breakage, and material-backed repair

**Problem:** Material variants had value and quality, but individual units did not complete the
DF-like condition loop: no stable unit identity, physical load pressure, work wear, broken state,
or repair action was visible to a player.

**Fix:** Every item unit now has a stable ID, weight, and finite condition. Relevant work causes
wear and zero condition leaves the item broken. Signed repair requires the appropriate completed,
staffed workshop, a living worker, and one visible matching material; durability research scales
the repair. Traders respect a 20 kg item-load cap. State survives SQLite restart.

**Player visibility and evidence:** The Goods panel shows weight, condition range,
damaged/broken counts, and a repair control. Unit, migration, determinism, denial, research,
trader-cap, protocol, signed guided, persistence, client, and accepted own-framebuffer coverage
pass. This verifies condition behavior, not full material or recipe breadth.

## 2026-07-14 — Authoritative election timing between terms

**Problem:** The governance panel could show an open election but could not answer when the next
one would begin while the colony was between terms.

**Fix:** Snapshots now carry the authoritative term start, next election boundary, term length,
and remaining duration. The governance panel renders the server-derived countdown even when no
election is open, and the schedule survives persistence/restart.

**Evidence:** Boundary, protocol, persistence, client-formatting, and integrated governance UI
coverage pass.

## 2026-07-14 — Physical Mill production and truthful local inventory

**Problem:** Grain processing could mutate aggregate resources without proving that a worker
carried inputs to an open Mill, worked there, and returned its outputs.

**Fix:** The Mill now follows the same conserved physical contract as the Sawmill: finite-store
pickup, station-local input, on-site progress, station-local output, and finite-store delivery.
Its editable queue, cargo, progress, blocked reason, and local ledgers persist through restart;
death and cancellation do not create or lose goods.

**Player visibility and evidence:** The accepted 1920×1080 own-framebuffer shows a selected open
Mill with one worker, repeating grain processing at 50%, Grain 4.0 local input, Flour 2.0 local
output, and Flour 1.5 outbound. Simulation, protocol, server, client, persistence, determinism,
and strict lint gates pass. Temporary capture fixtures and processes were removed afterward.

## 2026-07-14 — Physical shrine material offerings

**Problem:** A material offering could be credited as a scalar threshold without a cat moving
visible stockpile goods to the shrine, so the player could receive a blessing before delivery.

**Fix:** The offering is now two conserved physical stages: reserve and carry material from a
reachable visible pile into shrine escrow, then perform the ritual. Cancellation, death, blocked
routes, and restart preserve inventory exactly; credit occurs once, only after both stages.

**Evidence:** Conservation, reachability, cancellation/death, determinism, restart, and integrated
shrine-faucet campaigns pass alongside the maintained full quality gate.

## 2026-07-14 — Physical Accountant rounds and truthful stale stock reports

**Problem:** A staffed Accounting Tent refreshed the colony-wide ledger without a cat visiting
the spatial stockpiles. The client could therefore present exact authoritative quantities that no
cat had physically counted, and separate or unreachable piles had no independent freshness.

**Fix:** Accounting is now a durable physical route. An assigned tent worker returns to the tent,
visits reachable piles in deterministic distance/ID order, counts each for five game-seconds, and
returns. Only the visited pile's report changes; blocked piles remain stale, topology changes are
replanned, and cancellation never changes physical inventory. Active routes and reports persist
through SQLite restart. Existing aggregate-only saves migrate additively.

**Vacancy correction (2026-07-15):** The first physical-round slice retained a hidden
30-game-second fallback that copied authoritative colony resources into every report whenever no
tent worker was assigned. That contradicted both the manual-before-officer design and the physical
bookkeeper contract. The fallback is removed: an unbuilt tent, a vacant Accountant office, or a
completed but unassigned tent may remain stale indefinitely. Founding and extinction recovery still
seed one exact baseline, and aggregate-only legacy JSON attributes its already-persisted historical
total to the founding general storehouse once without sampling current pile contents.

**Player visibility:** Resource HUD, Goods ledger, stockpile text, and inspectors use reported
values and mark stale estimates with `~` or `uncounted`. The Accounting Tent inspector shows the
active physical phase, target, reachable progress, and count dwell.

**Evidence:** Unit, determinism, protocol compatibility, signed assign/release server action,
persistence/restart, one-shot and hourly 24-game-hour vacancy partitions, and client display tests
pass across `cat-sim`, `cat-protocol`, `cat-server`, and `cat-client`. The booted client/server
own-framebuffer at 1920×1080 was inspected and accepted with an active counting round and visibly
stale HUD/ledger estimates; the 2026-07-15 recapture verifies the corrected indefinitely stale
vacancy behavior.
