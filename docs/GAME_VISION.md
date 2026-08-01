# Idle Cat Forest — game vision

**"An idle version of Dwarf Fortress, played by cats, in a forest."**

Idle Cat Forest is developed and distributed solely as a non-commercial game project.

## Colony intelligence

The colony is intentionally capable but imperfect. A deterministic founding Leader plans across
survival, families/housing, staged construction/storage, food/production, an endlessly hungry Hole,
two-lane research, defense, diplomacy, and material barter from
beliefs and officer reports—not hidden world truth. Experienced officers shorten review cadence,
improve estimates, and expose richer information; weak or absent officers can omit work, react
late, or select a locally bad resource such as scarce food for a Hole feed and then send many cats
to recover the deficit. The player-god receives the same report-safe view, so regeneration rates
remain unknown until an effective level-4 report makes them knowable.

Plans become real spatial work. A Hunt belongs to its cave, a Water job to its source and
bank/delivery endpoint, and a Workshop job highlights and reserves the entire canonical 3×3
footprint. The Hole's repeated physical feed pipeline and Void Insight are a main progression
engine alongside growing the village; physical scholar work creates Research Notes for ordinary
research. Families, care, governance, construction, storage, diplomacy, barter, and future systems use the
same plan/report/reservation/action architecture. The complete maintained contract lives in
[`leader-ai-overhaul/`](leader-ai-overhaul/README.md).

The LAI.34 cutover is historical baseline. The LAI.35–70 integration removes the utility director,
Shrine/Favor/Blessings, generic stored Food/Fish/Preserves, scholar Insight, coins, direct routine
control, semantic gameplay migration, and the old research/navigation surfaces. Progress and exact
remaining runtime/wire/persistence/client/art/acceptance work are recorded on the
[overhaul board](leader-ai-overhaul/BOARD.md).

## Pillars
1. **Top-down, single level.** A flat 2D grid world (no isometric, no z-layers). Read
   the map like DF: everything is a *place* — tiles, workshops, stockpiles, cats.
2. **Manual → automated via roles.** Early game you direct the colony by hand (what to
   build, who hauls what, where the stockpiles go). As the colony grows you unlock and
   assign **leadership roles / officers**, each of which *automates a whole category of
   work* — turning the game idle. The founding **Leader** owns cross-domain strategy and delegates
   through **specialized roles**:
   - **Steward** — hauling + stockpile management (what goes where).
   - **Accountant** — physical stockpile rounds and reported inventory freshness.
   - **Forester** — wood: felling, replanting, lumber.
   - **Farmer** — fields, foraging, food.
   - **Captain** — warriors, defense, raids.
   - **Loremaster/Ritualist** — research labor/building automation + shrine/rituals.
   - **Cloth Leader** — fibre, thread, hide, cloth, leather, and clothing stations.
   The following bounded safety-floor description is the historical P12 baseline that the new
   persistent cross-domain planner supersedes: the always-present founding Leader retains a
   narrow, deficit-scaled safety floor. At the
   15-cat founding population, at most six primitive hunters, two emergency water fetchers, and
   one scout; those ceilings scale proportionally as the population changes. Specialist
   vacancies still make farming, production,
   hauling policy, research labor, rituals, and defense manual; a fresh idle village must not repeatedly
   reset merely because its first officer buildings do not exist yet.
   Each unlocked role hands its slice of decisions to the AI; unfilled roles stay manual.
3. **A living, visible workplace (DF readability).**
   - **More workshops** placed around the forest; cats walk to them to work. Houses may
     retain roofs, but workshops are open-top/cutaway stations whose function reads from
     the art itself, without persistent map-name plaques.
   - **Cats visibly haul items** between workshops and stockpiles (not just to the shrine).
   - **Stockpiles are real places in the world** — visible piles of wood / food / stone /
     refined goods that grow and shrink, that the player designates.
4. **A shared world with private footholds.** One large global village is available to
   everyone. A player may also found a personal village at a distant deterministic
   location on the same world map. Villages can eventually discover one another and trade.
5. **Knowledge must come home.** A founding village knows only its viable interior and a
   roughly two-tile halo. Resource-targeted and general scouts tentatively lift fog while
   away; discoveries become permanent only when the scout returns to and touches the shrine.
   Baseline deficit-driven scout dispatch belongs to the founding Leader (especially the fast
   first wood search) and is not gated behind the later Loremaster office; research labor/building
   automation and rituals remain Loremaster-owned, while the Leader retains the daily strategic
   study choice described below.

## Historical P12 inventory (reuse, don't rebuild)

This section records the pre-overhaul product baseline. Where it says “utility-AI leader
director,” read it as migration history; new planning work follows
[`leader-ai-overhaul/`](leader-ai-overhaul/README.md).

- Utility-AI **leader director** (one overseer that automates) — the seed of the role system.
- **Jobs/labors**, **workshops** (workshop/smithy/field/research_hut/school/barracks),
  **hauling** (cats carry yields in trips), **storage** capacities, **movement/pathfinding**,
  life sim, economy, upgrade tree. All ported to Rust (`cat-sim`), deterministic, tested.
- Authoritative server (`cat-server`) + Bevy client (`cat-client`) already stream the live world.

## What this vision adds (roadmap deltas)
- **Client: top-down renderer** (replaces the iso plan) — flat tile grid, cats + carried
  items, readable open workshops/buildings without map plaques, **visible stockpiles**,
  camera, dashboard, and manual action tools.
- **Sim: role/officer system** — split the monolithic leader director into assignable roles,
  each gating a category of automation; a "manual" default when a role is unfilled.
- **Sim: spatial stockpiles** — designate stockpile zones that physically hold items; hauling
  routes goods workshop↔stockpile↔workshop (extends the existing trip/haul + storage systems).
- **Sim: more workshops** + the crafting/hauling chains between them.

## Core loop & mechanics (the DF texture)

### More jobs than cats (the central tension)
Like DF, there is always **more work than labor**. You never have enough paws. This is
what makes prioritization — and eventually roles — matter, and what makes it a satisfying
idle-management game rather than a solved economy.

### Cats get better at what they do (skills)
Cats **improve at labors over time** (extends the existing `roleXp`/specialization). A cat
that hauls a lot becomes a faster hauler; one that mills becomes a better miller. Skilled
cats are faster / higher-yield, so *who* does *what* accumulates value — you grow experts.

### Manual → role-automation, one building at a time
Each production/management category starts **manual** (a god/player triggers it), then you
unlock a **role-officer** that automates it. Roles live in **role-buildings**, which are
**unlocked by the upgrade tree** and cost **escalating resources** to build:
- **Accountant** (build an *Accounting Tent*) — a cat walks the **stockpiles "counting stuff"**;
  only physically visited piles receive a fresh exact report (à la DF bookkeeper).
- **Cloth Leader** (build a *Clothier's Workshop*) — cats can first be told **manually** to
  produce **cloth / armour / clothing**; once the Cloth Leader is unlocked, it runs automatically.
- **Steward** (hauling + stockpiles), **Forester** (wood), **Farmer** (fields/foraging),
  **Captain** (defense/warriors), **Loremaster** (research labor/rituals) — same manual→auto pattern.
Unlocking a role hands its slice of decisions to the AI (the existing leader director,
specialized per role); unfilled roles stay manual.

### Production chains + farming (lots to do, always slightly cat)
- **Farms are in the overworld outside the walled settlement interior**: designate farm
  plots and **see the tiles being farmed** (DF farm plots). Grow **catnip**, **grain**,
  herbs, etc. Trees, stone/deposits, and fields do not occupy the village interior.
- **Processing chains**: grain → **Mill** → flour → food; fibre → **Clothier** → thread → cloth/clothing;
  hide → **Tannery** → leather; ore → **Smelter** → metal → **Smithy** → weapons/armor;
  planks + blocks → **Woodworking** → tools;
  logs → **Wood Cutter** → fine planks or **Sawmill** → structural lumber. Stone is dressed into
  blocks before construction. Each step is an open-top workshop cats walk to and haul between;
  one worker advances one selected recipe rather than several invisible parallel cycles.
- Everything is lightly cat-flavoured (catnip, mouse-farms, naps) but there's a deep to-do list.

### Visible stockpiles
Stockpiles are **real places in the world** — piles of wood/food/stone/cloth that grow and
shrink and that the player designates. The **Accountant** physically visits and counts them;
unvisited or unreachable piles remain visibly stale.

### Buildings & upgrade tree
- **Role-buildings** and later workshops are gated behind the **upgrade tree** and cost
  **escalating resources**, so expansion is the long-game economy: build → unlock a role →
  automate → free paws → build the next thing. Wood Cutter, Stone Prep, Woodworking, and
  Research Hut are explicit founding-placement exceptions that establish the first physical chains.
- Research uses one validated graph whose finite total is derived from canonical content,
  fourteen curated tracks at levels 1–10, repeatable level 11+, and real AND/convergence junctions.
  The God lane queues and physically researches ordinary Notes-funded or Hole-axis Void-funded
  studies; the free instant Leader lane uses a persisted rolling-seven-day quota, normally avoids
  the God target, and may collide only for critical need or the exact bounded mistake band.
  Building studies grant permits; the Leader still starts the timed physical upgrade.

### Founding, housing, breeding, and migration
- A new village starts with **15 adult cats and three early Dens**; each Den provides exactly
  five permanent beds. The founding blueprint therefore starts full, not with spare breeding
  capacity.
- Breeding is deliberately slow: a pregnancy may begin only after the establishment window and
  only when a permanent bed can be reserved for the future kitten. Gestation takes 18 game-hours,
  and the reservation prevents a migrant or another pregnancy from silently overbooking it.
- A prosperous village attracts migrants after a 30-game-hour establishment window. Arrivals
  are real cats who work, consume resources, and may temporarily exceed housing capacity. An
  unhoused arrival remains on 36 game-hours of probation, then physically leaves unless a
  permanent bed becomes available first.
- Extinction recovery is deterministic and atomic: a reset clears the failed run's transient
  work and restores the complete 15-adult/three-Den founding state without ghost cats, stale
  housing reservations, or reused migrant identities.
- Water recovery remains physical even in an emergency. The director may pre-empt a worker and
  dispatch a real fetch from a known water source, but the simulation never conjures an
  emergency bucket directly into storage.
- Cat lives are paced for an idle game. Ordinary old-age mortality begins at **240 game-hours**;
  leaders and healers receive the same 20% extension and begin at **288 game-hours**. This is a
  deliberate current-design replacement for the archived web prototype's 48/57.6-hour values.

## Platform/P12–P19 completion status

The Rust/Bevy cutover and maintained P12–P19 design are complete. Skills, seven specialist roles,
spatial stockpiles and physical Accountant rounds, visible exterior farms, all ten processor types,
108 physical recipes, shrine-return scouting, authored/traffic roads, global and personal villages,
physical trade, the 487-study research graph, housing/migration pressure, and native/WASM player
paths are implemented. The focused and generalized acceptance evidence lives in
`docs/IMPLEMENTATION_AUDIT.md` and `docs/FIX_LOG.md`; the integrated correction gate is verified.

That statement closes the Rust/Bevy platform migration and P12–P19 baseline. The subsequent
leader-intelligence cutover expanded the catalog to 531 studies and replaced the bounded director,
legacy currencies, and old offering/action model as described above. Its integrated evidence is
recorded on the [overhaul board](leader-ai-overhaul/BOARD.md); only the deliberately external
release matrix and publication shards remain to run before a release is published.

P9–P19 remain useful historical delivery groupings, not an active backlog. The integrated
correction set in `docs/FIX_LOG.md` passed its generalized passive, player-guided, persistence,
and multi-frame visual gate. The first pushed
Forgejo quality run and optional WASM transfer tuning remain external follow-ups. New design work
should be promoted here deliberately rather than inferred from old phase prose.

## Fidelity note
The Rust sim is a "same idea" port of the TS game; this vision *extends* it, so new systems
(roles, spatial stockpiles) are designed fresh in Rust, not ported. The web game on
`archive/web-game` is the frozen predecessor, not the target.
