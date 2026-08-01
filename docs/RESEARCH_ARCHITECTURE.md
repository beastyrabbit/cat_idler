# Research and building progression

This is the maintained contract for technology progression. It exists to prevent the research
screen, save format, and physical building system from drifting apart.

## Authority layers

1. `research_catalog` is the stable 495-node ledger. Raw node ids are persisted and their payloads
   remain the effect authority. Removing or renaming one requires an explicit save migration.
2. `research_tracks` groups that ledger into player-facing technologies. One track owns an ordered
   list of raw node indices, a type, one display name, and cross-track prerequisites.
3. `UpgradeTreeState` owns completed raw ids, research points, the durable queue, reserved costs,
   partial elapsed work, and repeatable global levels.
4. `ResearchSnapshot` is a projection for UI. The client never calculates an authoritative
   completion, discount, refund, or repeatable level.

Old saves without queue or repeatable fields load with empty defaults. Existing raw ownership is
never rewritten, so the normalized UI is a presentation/progression migration rather than a save
reset.

## Track types

- Milestone: one finite completion.
- Building: physical level 1 is baseline; grouped research permits levels 2–10. Research never
  upgrades an existing building by itself.
- Recipe: grouped finite progress is displayed on a ten-level scale. Existing catalog payloads
  remain the source of actual recipes and modifiers.
- Global modifier: ten finite levels, then repeatable forever. Each repeatable level adds 3% to
  the track's final scalar effect.

The current catalog normalizes to 88 catalog rows (25 building, 19 recipe, 14 global modifier, and
30 milestone tracks) while retaining every raw node. The dependency canvas contains 228 compact
study nodes: one per milestone, building, and production family, plus levels 1–10 and one infinite
terminal for each global modifier.

Prerequisites are logical AND, not alternatives. The player-facing graph deliberately has at least
24 visible convergence points. Curated junctions such as Stone Tools, Metal Tools, Precision Tools,
Civil Engineering, Preservation Science, Organized Provisioning, Public Administration, and
Combined Arms merge otherwise independent disciplines before opening later routes. This makes a
target a planning question—finish one missing branch or redirect into another—instead of a row of
unrelated linear upgrades.

## Dependency-canvas interaction

- The graph is vertical and fixed-scale. Nodes contain only a semantic icon and name; selection
  owns all descriptive text.
- No selection means the full tree. A selection means the transitive prerequisite and downstream
  unlock subgraph for that study. The induced graph is compacted per depth layer around the canvas
  centre, so a relevant prerequisite cannot remain hidden at its old full-tree coordinate.
- **Full tree** clears selection and returns to the root overview.
- Wheel input is routed by the hovered region: catalog, tree, and inspector scroll independently.
  Tree wheel pans vertically, Shift-wheel pans horizontally, and direct drag pans both axes.
- Global-modifier levels 1–10 are individual graph nodes. Infinite research is always a separate
  level-11 terminal, never folded into level 10 or represented only by inspector copy. Ordinary
  building and production families remain one stable graph node; their current and next level are
  shown in the inspector.

## Queue and time

- Selecting a finite technology queues every missing prerequisite in deterministic topological
  order, up to 64 entries.
- An entry reserves no points until it reaches the front. At the front it waits for prerequisites,
  enough points, and at least one operational staffed Research Hut or School.
- Once funded, its cost is durable. Partial elapsed work is durable across reorder, disconnect,
  restart, and offline catch-up.
- Removing an entry removes queued descendants that depend on it and refunds every funded removed
  entry. Reordering cannot cross a prerequisite.
- The Leader may add one available study per rolling day only while the queue is empty.
- Base duration is `max(60 seconds, base_cost × 12 seconds)`.
- Operational staffed Research Hut and School levels form an infrastructure score. Cost uses
  `max(0.80, 0.9975^score)` and time uses `max(0.60, 0.995^score)`.
- Scholarship finite levels multiply both by `0.99^level`; infinite Scholarship uses
  `0.9975^repeatable_levels`. Combined floors are 50% cost and 30% time.
- Repeatable cost is the final finite node cost doubled once per level after 10.

## Physical building upgrades

Every completed physical building has levels 1–10. A building's matching technology track is only
a permit. The player must select the building and press **Upgrade to level N**.

An upgrade:

- validates the researched level permit and level-10 cap;
- consumes scaled timber and blocks, then introduces tools, metal, refined materials, and a gem at
  later levels;
- releases assigned workers;
- marks the building incomplete/offline for all systems that already honor completion;
- uses the existing construction job, architect travel, persistence, and completion path;
- takes `8 hours × (target_level − 1)^1.25`;
- increments the physical level only when construction completes.

Do not add a separate instant-upgrade counter or client-side level mutation.

## Adding content safely

For a new technology family:

1. Add stable raw nodes and payloads to `research_catalog`; never repurpose an old id.
2. Add the family prefix to exactly one track list in `research_tracks`.
3. Give a node at most three direct prerequisites. Each must describe a distinct required
   discipline; do not add a prerequisite already implied by another prerequisite merely to draw
   another line.
4. Use `rootPrerequisites` for entry gates. Use `stagePrerequisites` only when a named later stage
   truly crosses into another discipline. Every prerequisite is required.
5. Remember that building and recipe families collapse to one player-facing card. Their external
   stage gates are projected onto that card. Do not make a late stage depend on a junction that
   already depends on an earlier stage of the same family; that would turn a valid raw sequence
   into a misleading visual cycle.
6. Map a semantic tracked icon in `research_icon_path`.
7. Verify catalog-row count, finite/infinite graph-node count, complete raw-node coverage, vertical
   forward connectors, no overlap, focused ancestor/descendant closure, queue path order, save
   round-trip, icon existence, and at least 24 player-visible convergence points.

For a new physical building:

1. Add its building track prefix or explicitly document why it cannot upgrade.
2. Ensure runtime benefits read `BuildingRuntime.level`.
3. Ensure every service treats `construction_progress < 100` as offline.
4. Add inspector/action coverage for its upgrade permit and material tiers.

Never add screen coordinates, a new research panel, global wheel capture, variable tree zoom, or a
keyboard opener.
