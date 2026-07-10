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

## Fidelity note
The Rust sim is a "same idea" port of the TS game; this vision *extends* it, so new systems
(roles, spatial stockpiles) are designed fresh in Rust, not ported. The web game on
`archive/web-game` is the frozen predecessor, not the target.
