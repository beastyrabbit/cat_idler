# P14 — Spatial placement, walls & road accessibility

User direction (2026-07-10): the world must obey proper tile-based spatial rules. Today
buildings render overlapping and placement/pathfinding don't enforce occupancy or
accessibility. This spec is the target model; cards are grounded by the `spatial-survey`
map of what already exists (village_area fence+gate, pathfinding WalkGrid/is_blocked,
roads paving, stockpile footprints) vs what's missing.

## Target invariants (the "logic requirements")
1. **One occupant per tile.** A tile holds at most ONE of: a building-footprint cell, a
   tree/rock decoration, a wall segment, water/river, or open ground. No overlaps —
   buildings must not overlap each other, workshops, houses, or stockpiles.
2. **No building on an occupied tile.** Placement is rejected if any target footprint tile
   has a tree, water, another building, or a wall. (Clearing a tree first frees the tile.)
3. **Multi-tile footprints.** Buildings occupy an NxM footprint (e.g. 2x3, 3x3), not a
   point. Stockpiles already have rect footprints — buildings need the same.
4. **Buildings, walls, and trees block movement.** Cats cannot walk through a building, a
   wall, a tree, or water. Pathfinding routes around them. The **shrine is passable**
   (you can walk through it) and is the road/path anchor.
5. **Always-closed perimeter wall with exactly one gate.** The village is enclosed by a
   wall; there is a single gate. Growth rule: when the village expands, **build the new
   outer wall first, then remove the now-interior old wall**, so the perimeter is never
   open. (Extends the organic edge-derived fence in `village_area`.)
6. **Road accessibility (hard requirement).** Every building footprint must be **adjacent
   to a road**, and roads form a **connected network that reaches the shrine** (every road
   ultimately leads to the shrine; the shrine is walk-through). So every workshop/house/
   building is reachable. Placement of a building must guarantee (or create) a road path
   from its entrance to the shrine.

## Decomposition (refined after the spatial-survey report)
- **P14.1 Tile occupancy + footprints.** A tile→occupant index; `Footprint{x,y,w,h}` on
  buildings; `can_place(footprint)` rejecting overlap with buildings/trees/water/walls.
  Building site-selection uses free non-overlapping footprints.
- **P14.2 Cost-based pathfinding with SOFT obstacles** (revised 2026-07-10 per user). Not
  hard-blocking for structures/trees — instead A* cost = **inverse effective move speed**, so
  cats get A→B fast, **prefer roads**, and **avoid slow tiles**:
  - **Slow-passable (~25% speed, high cost, NOT blocked)**: **trees** and **buildings/workshops**
    — a cat CAN cut through but it's a bad idea, so A* routes around unless forced.
  - **Hard-blocked (`is_blocked`)**: **walls** (except the single gate), **water**, **mountains**
    (until unlocked). The shrine footprint stays fully passable (the hub).
  - Cost tiers mirror the movement-speed model (stone 1.0 / grass 0.75 / sand 0.5 / dirt-road
    1.05 / stone-road 1.75 / tree+building 0.25) → cost ∝ 1/speed. Keep A* + the deterministic
    heap; feed the building-footprint + tree tiles (via terrain_gen, like P14.1) into the cost,
    not the blocked set.
- **P14.3 Closed perimeter wall + single gate, grown outward.** village_area expansion
  builds the outer ring before deleting the interior wall; guarantee exactly one gate; the
  wall blocks (P14.2) except at the gate.
- **P14.4 Road network + accessibility.** Roads connect building entrances to the shrine;
  an accessibility invariant/validation (every building footprint touches a road that
  reaches the shrine). Building placement extends the road to reach the site.
- **P14.5 Renderer.** Draw building footprints on their actual tiles WITHOUT overlap, the
  wall ring + gate, and the road network; trees/decoration as blockers; the shrine as the
  passable hub. Fixes the visible overlap.

## Sequencing notes
- P14.1 (occupancy/footprints) is the foundation; P14.2 (blocking) depends on it; P14.3
  (wall) and P14.4 (roads/accessibility) build on both; P14.5 (render) surfaces all of it.
- This interacts with in-flight hauling (haul-fill) — once structures block, cat routing to
  stockpiles/shrine must path around them (pathfinding already supports obstacles).
- Regression discipline as elsewhere: keep the deterministic tick + existing fixtures green;
  where placement/pathfinding output changes, update fixtures deliberately (this is a
  behavior change, not a silent break) and re-verify survival/determinism.
