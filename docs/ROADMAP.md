# Roadmap — "The Gods Shape, The Cats Live"

> **SUPERSEDED.** Written for the TypeScript web game (frozen on branch `archive/web-game`).
> The design *feel* described here ("gods shape, cats live") carried over into the Rust rebuild
> — see [`docs/GAME_VISION.md`](GAME_VISION.md) for the current pillars and
> [`docs/migration/BOARD.md`](migration/BOARD.md) for what's actually built. This file's
> specific phase plan and implementation notes are TS-era and no longer current.

The target feel (from the 2026-07-02 design direction): **a self-running
simulation**. The cats live, age, breed, work, research, fight and die on
their own; players are gods who nudge and shape — zones, boosts, votes,
roads, and a slow upgrade tree — never micromanagers. Watching the
village should read as intentional: almost no cat idle, lineages forming,
the settlement physically growing through eras, threats scaling with
success. These are *directions*; implementers own the details.

**Status:** all six tracks below have landed on `feature/map-first-ui`, and
several refinement tracks have landed since (see "Landed since the six
tracks"). The map has settled on **flat Kenney "Isometric Miniature"** art:
the Isometric-Nature terrain/elevation experiment (heightmaps, cliffs,
stairs) was reverted by user decision — elevation read as stacked walls and
hurt the look — so terrain render is no longer an open item (`docs/TERRAIN_DESIGN.md`
is now historical). The remaining work is the PixiJS renderer cutover plus
the stretch items and the push/PR — see "Open items" at the bottom.

## 1. World credibility ✅ (`8ff27c0`, staged `7f4cf37`/`e6e976a`/`0c2de2d`, `4f8b96a`)
Expanding village clearing/fence as rings fill; no building on water;
visible roads on trafficked routes; every moving cat reveals 3x3
(explorers 5x5, but much slower walkers); the colony fetches its own
water from water tiles; storehouse-spam fixed and per-resource storage
buildings with per-resource capacity bars. Multi-trip hunt gathering,
tile depletion/regrowth, and quarry/scout expeditions shipped alongside.

## 2. Life simulation ✅ (`c7236d4`)
Aging through life stages, breeding with genetic inheritance (specialized
parents beget gifted kittens), elder death, starvation — a population
loop instead of a fixed 20 cats. Self-sustaining but fragile. Landed in
lib/game/{lifeSim,age,breeding,genetics} driving `ageHours` on the
accelerated clock.

## 3. Research & god upgrade tree ✅ (`d40de76` data model, `8705aad` live)
One tree, two ways to advance: gods spend blessings instantly; cats
unlock nodes slowly via a research building (a dedicated researcher ≈ 10
pts/week; nodes cost 5-25). ~18 nodes across 3 eras unlock buildings
(barracks, smithy, sawmill, school, fields, housing tiers) and jobs —
Age-of-Empires progression, tree-shaped. The tree persists on the colony
across run resets. The economic tension: research costs a mouth that
gathers nothing.

## 4. Military & threats ✅ (`6a70243`)
Smithies produce weapons/armor; warrior specialization musters and
consumes them at the gate; enemies raid, scaling with wealth, warrior
count, population, and playtime. Hunters fight badly; warriors fight well
and gain XP per defended raid. Threat pressure builds after a grace
window and launches a warband at threshold; a wipeout resets the run.

## 5. Specialization depth + lineage payoff ✅ (`c7236d4`, `6a70243`)
Jobs level cats visibly (trade level from XP; yields/speeds improve);
sitting leaders gain leadership over time; specialized cats breed more
and pass traits — born hunters, builders, warriors over generations.
Shown in the cat card. `warrior` joined hunter/architect/ritualist.

## 6. Leader AI v3 + real pathing ✅ (`608a4f5`)
Utility director (`lib/game/leaderDirector.ts`, per `LEADER_AI_DESIGN.md`)
scores every goal on one [0,1] scale and hands a shared employment budget
to the most urgent, balancing food/water/materials/research/defense with
near-zero idle cats; global greedy cat-to-slot assignment; deliberate
road paving; bounded A* pathing around walls/water through the village
gate.

## Landed since the six tracks
- **Survival demographics, persistence & bootstrap economy ✅ (`058e601`,
  `b0436c6`, `e8c5d8c`)** — staggered founder ages + breeding headroom so a
  founding cohort no longer ages out together into collapse; the world
  (tiles / fog / roads) persists across run resets; and a bootstrapped early
  economy (hunt yield 24, founding cap 22, starting food trimmed to 150) so
  an **unaided** colony reliably survives its founding window in the faithful
  survival sims.
- **Raid balance ✅ (`3e974a0`)** — militia / hunter / warrior muster weights
  by life stage; a starter colony repels its first raid with zero deaths and
  survives dozens of raids unaided (survival regression suite).
- **Terrain-cost pathfinding ✅ (`9f09e46`)** — A* over a real terrain cost
  model (built road < worn trail < open ground < forest < dense woods; water
  and the fence-off-gate impassable): cats visibly prefer roads and skirt the
  woods, so deliberate road-building is mechanically meaningful. Routes are
  deterministic and strictly contiguous, with a per-cat route cache.
- **Organic village core ✅ (`9baa58d`; stage-2 wiring in flight)** — the
  claimed village area grows organically with an auto-generated perimeter
  fence, replacing the fixed ring.
- **Flat-map polish ✅** — fog renders as dimmed terrain rather than blank
  diamonds (`35177f4`); real oriented road sprites with autotiling
  (end / clearing / corner / crossing), felled-forest stumps, and recolored
  water on the Kenney Miniature diamonds.
- **PixiJS renderer spike ✅ (`40ab476`, `/game/pixi`)** — a side-by-side
  WebGL renderer that validates the `docs/ENGINE_FRONTEND.md` verdict: the
  same live world, held at 60fps at the far zoom-out that stalls the DOM
  renderer, via chunk-level LOD. See that doc for the measured results and
  the cutover plan.

## Open items
- **PixiJS renderer cutover** — promote the `/game/pixi` spike to the real
  renderer per `docs/ENGINE_FRONTEND.md`. Prerequisite: hoist the pure render
  helpers (fog brightness, fence sprites, road autotiling, tile-sprite
  selection, `isExplored`) out of `components/map/TileLayer.tsx` into a shared
  lib module both renderers import — the spike had to reimplement them to stay
  decoupled from the in-flight organic-village rewiring, and the cutover
  should collapse that duplication.
- **Bridges** — let cats and roads cross rivers instead of only pathing
  around them.
- **Fishing** — a water-tile food source / job to complement hunting.
- **Traveler interception by raiders** — raiders currently march the gate;
  intercepting cats out on expeditions is unmodeled.
- **Elevation-aware zones** — a stretch item; would require reintroducing
  elevation (the walkgrid keeps inert `heightAt` / `hasStair` seams for it).
- **PR / push** — land `feature/map-first-ui` on `main` (the user's call).
