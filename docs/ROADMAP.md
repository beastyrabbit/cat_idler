# Roadmap — "The Gods Shape, The Cats Live"

The target feel (from the 2026-07-02 design direction): **a self-running
simulation**. The cats live, age, breed, work, research, fight and die on
their own; players are gods who nudge and shape — zones, boosts, votes,
roads, and a slow upgrade tree — never micromanagers. Watching the
village should read as intentional: almost no cat idle, lineages forming,
the settlement physically growing through eras, threats scaling with
success. These are *directions*; implementers own the details.

**Status:** all six tracks below have landed on `feature/map-first-ui`.
The remaining work is terrain render integration (in flight) plus the
stretch items and the PR/push — see "Open items" at the bottom.

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

## Open items
- **Terrain render integration (in flight)** — the generator core landed
  (`e6f215b`; heightmap, oriented cliffs, stairs, rivers, biomes) with a
  `/dev/tiles` explorer, but wiring the isometric map render to the new
  terrain roles is still in progress.
- **Bridges** — let cats and roads cross rivers instead of only pathing
  around them.
- **Fishing** — a water-tile food source / job to complement hunting.
- **Traveler interception by raiders** — raiders currently march the gate;
  intercepting cats out on expeditions is unmodeled.
- **PR / push** — land `feature/map-first-ui` on `main`.
