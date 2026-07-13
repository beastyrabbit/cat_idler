# Idle Cat Forest — game vision

**"An idle version of Dwarf Fortress, played by cats, in a forest."**

## Pillars
1. **Top-down, single level.** A flat 2D grid world (no isometric, no z-layers). Read
   the map like DF: everything is a *place* — tiles, workshops, stockpiles, cats.
2. **Manual → automated via roles.** Early game you direct the colony by hand (what to
   build, who hauls what, where the stockpiles go). As the colony grows you unlock and
   assign **leadership roles / officers**, each of which *automates a whole category of
   work* — turning the game idle. The current single **Leader** (the utility-AI director)
   is role #1; the evolution is **more specialized roles**:
   - **Steward** — hauling + stockpile management (what goes where).
   - **Forester** — wood: felling, replanting, lumber.
   - **Farmer** — fields, foraging, food.
   - **Captain** — warriors, defense, raids.
   - **Loremaster/Ritualist** — research + shrine/rituals.
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

## What already exists in the sim (reuse, don't rebuild)
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
  gives you **more accurate stock values and a faster update rate** (à la DF bookkeeper).
- **Cloth Leader** (build a *Clothier's Workshop*) — cats can first be told **manually** to
  produce **cloth / armour / clothing**; once the Cloth Leader is unlocked, it runs automatically.
- **Steward** (hauling + stockpiles), **Forester** (wood), **Farmer** (fields/foraging),
  **Captain** (defense/warriors), **Loremaster** (research/rituals) — same manual→auto pattern.
Unlocking a role hands its slice of decisions to the AI (the existing leader director,
specialized per role); unfilled roles stay manual.

### Production chains + farming (lots to do, always slightly cat)
- **Farms are in the overworld outside the walled settlement interior**: designate farm
  plots and **see the tiles being farmed** (DF farm plots). Grow **catnip**, **grain**,
  herbs, etc. Trees, stone/deposits, and fields do not occupy the village interior.
- **Processing chains**: grain → **Mill** → flour → food; fibre → **Clothier** → cloth → armour;
  ore → **Smithy** → tools/weapons; wood → **Sawmill** → lumber → construction. Each step is a
  workshop cats walk to and haul between.
- Everything is lightly cat-flavoured (catnip, mouse-farms, naps) but there's a deep to-do list.

### Visible stockpiles
Stockpiles are **real places in the world** — piles of wood/food/stone/cloth that grow and
shrink and that the player designates. The **Accountant** improves how accurately + how often
their contents are reported.

### Buildings & upgrade tree
- **Role-buildings** and **workshops** are gated behind the **upgrade tree** and cost
  **escalating resources**, so expansion is the long-game economy: build → unlock a role →
  automate → free paws → build the next thing.
- Research uses a full-page dependency tree with about **500 data-driven nodes**: at least
  one third building-related unlocks and one third recipes/new resources, with the rest
  covering movement, labor, storage, defense, and similar upgrades. A player may buy any
  affordable nodes. The leader may autonomously choose at most one node per real-life day.

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

## Post-cutover completion order
The Rust/Bevy cutover and the original P9–P19 migration phases are complete. Remaining
product gaps are tracked in `docs/IMPLEMENTATION_AUDIT.md`; the intended dependency order is:

- **Simulation foundations:**
  1. **Skills**: general per-labor skill/xp curve; skill affects speed/yield.
  2. **Role/officer system**: split the leader director into assignable roles, each gating a
     category of automation, each tied to a role-building + upgrade-tree unlock + escalating cost.
  3. **Spatial stockpiles**: designatable stockpile zones that physically hold items; hauling
     routes goods workshop↔stockpile↔workshop. Accountant improves accuracy/update-rate.
  4. **More workshops + production chains**: mill, clothier, sawmill, accounting tent, catnip/
     grain farms; the craft/haul graph between them.
  5. **Visible farm plots** in the overworld.
- **Player paths:** designation tools (place stockpiles/farms/workshops), role assignment UI,
  manual-workshop controls, then automation as roles unlock.
- **World progression:** shrine-return scouting/fog, exact authored/traffic road rules,
  global and personal villages, meeting, and trade.
- **Long game:** the full research graph, migration/housing pressure, deeper production,
  and native/WASM interaction and framebuffer campaigns.

## Fidelity note
The Rust sim is a "same idea" port of the TS game; this vision *extends* it, so new systems
(roles, spatial stockpiles) are designed fresh in Rust, not ported. The web game on
`archive/web-game` is the frozen predecessor, not the target.
