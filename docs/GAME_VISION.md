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
   - **More workshops** placed around the forest; cats walk to them to work.
   - **Cats visibly haul items** between workshops and stockpiles (not just to the shrine).
   - **Stockpiles are real places in the world** — visible piles of wood / food / stone /
     refined goods that grow and shrink, that the player designates.

## What already exists in the sim (reuse, don't rebuild)
- Utility-AI **leader director** (one overseer that automates) — the seed of the role system.
- **Jobs/labors**, **workshops** (workshop/smithy/field/research_hut/school/barracks),
  **hauling** (cats carry yields in trips), **storage** capacities, **movement/pathfinding**,
  life sim, economy, upgrade tree. All ported to Rust (`cat-sim`), deterministic, tested.
- Authoritative server (`cat-server`) + Bevy client (`cat-client`) already stream the live world.

## What this vision adds (roadmap deltas)
- **Client: top-down renderer** (replaces the iso plan) — flat tile grid, cats + carried
  items, labelled workshops/buildings, **visible stockpiles**, camera, dashboard, manual
  action tools first.
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
- **Farms are in the overworld**: designate farm plots and **see the tiles being farmed** (DF
  farm plots). Grow **catnip**, **grain**, herbs, etc.
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

## Build order for this vision (phases after the visible world)
- **P9 (now):** top-down renderer of the current world (terrain, cats+carrying, workshops,
  visible storage, camera, dashboard, manual action tools). Foundation for all of the above.
- **P12 — Idle Cat Forest sim expansion (Rust `cat-sim`):**
  1. **Skills**: general per-labor skill/xp curve; skill affects speed/yield.
  2. **Role/officer system**: split the leader director into assignable roles, each gating a
     category of automation, each tied to a role-building + upgrade-tree unlock + escalating cost.
  3. **Spatial stockpiles**: designatable stockpile zones that physically hold items; hauling
     routes goods workshop↔stockpile↔workshop. Accountant improves accuracy/update-rate.
  4. **More workshops + production chains**: mill, clothier, sawmill, accounting tent, catnip/
     grain farms; the craft/haul graph between them.
  5. **Visible farm plots** in the overworld.
- **P13 — client:** designation tools (place stockpiles/farms/workshops), role assignment UI,
  manual-workshop controls, then the automation toggles as roles unlock.

## Fidelity note
The Rust sim is a "same idea" port of the TS game; this vision *extends* it, so new systems
(roles, spatial stockpiles) are designed fresh in Rust, not ported. The web game on
`archive/web-game` is the frozen predecessor, not the target.
