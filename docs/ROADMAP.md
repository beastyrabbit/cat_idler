# Roadmap — "The Gods Shape, The Cats Live"

The target feel (from the 2026-07-02 design direction): **a self-running
simulation**. The cats live, age, breed, work, research, fight and die on
their own; players are gods who nudge and shape — zones, boosts, votes,
roads, and a slow upgrade tree — never micromanagers. Watching the
village should read as intentional: almost no cat idle, lineages forming,
the settlement physically growing through eras, threats scaling with
success. These are *directions*; implementers own the details.

## 1. World credibility (bugs + polish, do first)
Expanding village clearing/fence as rings fill; no building on water;
visible roads on trafficked routes; every moving cat reveals 3x3
(explorers 5x5, but much slower walkers); the colony fetches its own
water from water tiles (water economy currently starves at 6 while food
sits at ~10k); storehouse-spam fixed and per-resource storage buildings
with per-resource capacity bars.

## 2. Life simulation
Aging through life stages, breeding with genetic inheritance (specialized
parents beget gifted kittens), elder death, starvation — a population
loop instead of a fixed 20 cats. Self-sustaining but fragile. Dormant
modules exist: lib/game/{age,breeding,genetics,catTraits,lifeMilestones}.

## 3. Research & god upgrade tree
One tree, two ways to advance: gods spend blessings instantly; cats
unlock nodes slowly via a research building (a dedicated researcher ≈ 10
pts/week; nodes cost 5-25). Nodes unlock buildings and eras (barracks,
smithy, sawmill/Sägewerk, school where kittens generate points, housing
tiers) — Age-of-Empires progression, tree-shaped. The economic tension:
research costs a mouth that gathers nothing.

## 4. Military & threats
Workshops produce weapons/armor; warrior specialization consumes them;
enemies raid, scaling with wealth, warrior count, population, and
playtime. Hunters can fight badly; warriors fight well and improve.

## 5. Specialization depth + lineage payoff
Jobs level cats visibly (yields/speeds improve); leaders improve at
leading; specialized cats breed more and pass traits — born hunters,
builders, scholars over generations. Shown in the cat card.

## 6. Leader AI v3 + real pathing
With all the new job types, near-zero idle cats; portfolio balancing
across food/water/materials/research/defense; deliberate road building;
A* pathing around walls/water with the village gate.

Ordering: 1 first; 2 and 3 can proceed in parallel after; 4 needs 2+3;
5 rides on 2; 6 lands last on top of everything.
