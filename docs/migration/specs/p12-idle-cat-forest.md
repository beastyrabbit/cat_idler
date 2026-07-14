# P12 — Idle Cat Forest sim expansion (cat-sim)

> **Living target spec.** Specialist manual-to-officer ownership (with a bounded founding Leader
> hunt/water/scout safety floor), all-labor skills, seeded spatial storehouses, and one complete
> physical Sawmill route are verified. Broader physical station/farm routes and recipes remain
> partial. Current evidence and exact follow-ups live in
> [`docs/IMPLEMENTATION_AUDIT.md`](../../IMPLEMENTATION_AUDIT.md).

The DF-texture depth from `docs/GAME_VISION.md`, decomposed into concrete, TDD-able Rust
cards grounded in the *current* `cat-sim` code. Each card is pure sim logic (no I/O),
unit-tested in `tests/`, and folds into `world_tick` as an ordered phase or an extension of
an existing one. Cards are ordered by dependency; P12.1 unblocks the rest.

## What already exists (reuse, don't rebuild)
- `Cat.role_xp: RoleXp { hunter, architect, ritualist, warrior }` — +1.0 per completed job
  (`world_tick.rs` complete_hunt/build/ritual/warrior), auto-promotes via
  `idle_engine::next_specialization`.
- `life_sim::trade_yield_multiplier(xp)` = `1 + 0.4·(1 − 1/(1+xp/30))` (→1.4× asymptote) and
  `trade_speed_multiplier(xp)` — the skill→multiplier curves. **Already the right shape**;
  P12.1 just applies them per-labor and to more labors.
- `idle_engine::get_duration_seconds` — specialization cuts job duration (hunter/architect 0.5,
  ritualist 0.6) + upgrade masteries, floor 5s. `stage_work_effectiveness` (life-stage weight).
- `leader_director` (utility-AI overseer, one budget) — the seed of the role/officer system.
- `production.rs` (workshops refine 5→1/10min, fields grow food), `storage.rs` (per-resource
  caps), `trips.rs`/`shrine.rs` (SC2-drone hauling to shrine), `zones.rs` (avoid/gather rects).
- `upgrade_tree.rs` (`resolveEffects` → flat modifier map) — role-buildings gate here.

## P12.1 — Skills (general per-labor proficiency) — FOUNDATION, do first
**Goal:** every labor a cat performs accrues a per-labor skill that scales its speed & yield,
so experts emerge. Generalizes `role_xp` from 4 roles to all labors.
- **Data:** add `Cat.skills: BTreeMap<Labor, f64>` (xp per labor), where
  `Labor = {Hunt, Build, Ritual, Fight, Quarry, FetchWater, Mill, Craft, Farm, Haul, Research}`.
  Keep `role_xp` for the 4 specialization roles + back-compat serde (`#[serde(default)]`, skip
  if empty). A skill `Labor` maps to the job `kind` that produced it.
- **Gain:** on job completion, `+SKILL_GAIN_PER_JOB` (start 1.0, same as role_xp) to that labor;
  hauling trips grant a small `Haul` gain. Diminishing via the existing curve, not the gain.
- **Effect:** in `get_scaled_duration_seconds` and the yield functions, multiply by
  `trade_speed_multiplier(skill)` / `trade_yield_multiplier(skill)` for that labor (today only
  hunt uses xp). Specialization keeps its discrete 0.5× cut; skill is the continuous layer.
- **Tests:** monotonic (more skill → shorter duration, higher yield, both bounded);
  determinism; serde round-trip incl. legacy rows with no `skills`; a cat that only hauls gains
  only `Haul`. Boundary: skill 0 = today's behavior (no regression on existing tests).

## P12.2 — Officer roles (split the monolith director)
**Goal:** the single leader director becomes assignable **officer roles**, each automating one
labor category; unfilled roles stay manual (player-triggered).

**Superseded design note (2026-07-10, replaced 2026-07-14):** the initial additive-officer proposal
kept every base-Leader category automated. Playtesting rejected that because it made vacancies
meaningless; the later fully manual split then made a fresh founding repeatedly collapse. The
maintained contract is specialist-manual with one bounded exception: the always-present founding
Leader may keep at most six hunts, two emergency water trips, and one scout in flight per 15
living cats. Farms,
production, hauling policy, research, rituals, defense, and expansion remain manual while their
offices are vacant. The 8 `LaborGoalKind`s otherwise partition into 5 roles:
`Farmer{Hunt,FetchWater}`, `Forester{Quarry}`, `Captain{TrainWarrior,AssignSmithy}`,
`Loremaster{AssignResearch,Scout}`, `Steward{AssignWorkshop}` (+stockpiles in P12.3). The deeper
"unfilled ⇒ fully manual" comes once P12.3/P12.4 make manual control meaningful.
- **Data:** `Colony.officers: BTreeMap<OfficerRole, Option<CatId>>` where
  `OfficerRole = {Steward(haul/stockpiles), Forester(wood), Farmer(fields/forage),
  Captain(defense), Loremaster(research/ritual)}` (+ the existing Leader as role #0).
- **Gate:** each role is unlocked by an upgrade-tree node + a built **role-building** (P12.4)
  with escalating cost. `leader_director` splits into per-role goal scorers reading the same
  `LeaderSnapshot`; an unfilled role emits no specialist auto-goals beyond the bounded founding
  hunt/water/scout safety floor (its other labors await manual actions).
- **Actions:** `assignOfficer{role, catId}` / `unassignOfficer{role}` (new ClientActions,
  add to cat-protocol + apply_action). Manual fallback = existing `requestJob`.
- **Tests:** filled role auto-issues its category's goals; unfilled roles emit only the bounded
  Leader safety floor; assigning a
  dead/foreign cat rejected; determinism of multi-role budget split; a water crisis still pulls
  labor cross-role (extends the existing leaderDirector trade-off test).

## P12.3 — Spatial stockpiles (visible piles that hold items)
**Goal:** designatable stockpile zones that physically hold resources; hauling routes goods
workshop↔stockpile↔workshop (extends trips/shrine, which today only credit at the shrine).
- **Data:** `Colony.stockpiles: Vec<Stockpile { id, rect, accepts: ResourceSet, contents:
  Resources }>`. Extends `zones.rs` (reuse rect + player-cap machinery).
- **Logic:** haul targets the nearest accepting stockpile (not only the shrine); a stockpile's
  contents count toward colony storage (respect `storage.rs` caps). Workshops pull inputs from /
  push outputs to the nearest stockpile.
- **Accountant (P12.4 building):** without it, stockpile totals in the snapshot are stale
  (updated slowly / fuzzed); a staffed Accounting Tent walks the piles and makes totals exact +
  fast-refresh (à la DF bookkeeper). Model as a per-stockpile `last_counted` freshness the
  snapshot reports.
- **Actions:** `designateStockpile{rect, accepts}` / `removeStockpile`. **Renders** as visible
  piles (props already sliced: barrel/crate/sack/log_pile/stone_pile/ore_pile/gold_pile).
- **Tests:** haul chooses nearest accepting pile; caps enforced; contents survive tick;
  accountant improves freshness; determinism.

## P12.4 — More workshops + production chains + role-buildings
**Goal:** the craft/haul graph. Each is a workshop cats walk to; role-buildings gate officers.
- **Buildings (extend `buildings.type`):** `mill` (grain→flour), `clothier` (fibre→cloth→armour),
  `sawmill` (logs→lumber), `accounting_tent` (Accountant), plus role-buildings for each officer.
- **Chains:** catnip/grain → Mill → flour → food; fibre → Clothier → cloth → armour; logs →
  Sawmill → lumber → construction; ore → Smithy (exists) → tools/weapons. Each = a
  `production.rs`-style staffed cycle moving items between stockpiles.
- **Cost:** role-buildings + workshops gated behind upgrade-tree nodes with **escalating**
  resource costs (build → unlock role → automate → free paws → build next).
- **Tests:** each chain converts inputs→outputs at the right rate only when staffed + inputs
  present; escalating cost math; unlock gating; determinism.

## P12.5 — Visible farm plots
**Goal:** designate farm plots; cats plant/tend/harvest; crops grow through visible stages.
- **Data:** `Colony.farm_plots: Vec<FarmPlot { rect, crop: CropKind, planted_at, stage }>`;
  `CropKind = {Catnip, Grain, Herb}`. Stage advances on the game-hour clock (like breeding
  gestation). Harvest yields the crop resource into the nearest stockpile.
- **Render:** farm/ sprites (soil + crop_sprout→growing→mature→flowering) already sliced.
- **Actions:** `designateFarm{rect, crop}` / `clearFarm`. **Tests:** stage progression on the
  accelerated clock; harvest yield; only farmable (flat, claimed, non-water) tiles; determinism.

## P12.6 — Logistics, general/limited stockpiles & shrine offerings (user direction 2026-07-10)
The refined storage/shrine model. Three linked pieces:

### (a) Cats carry to the closest accepting stockpile
Delivered by the **haul-fill** card (carrying cats walk to the nearest stockpile whose `accepts`
contains the carried resource, else the reservoir). Prereq for everything below.

### (b) General vs limited stockpiles + the Logistics Master
- `Stockpile.accepts` already models it: **general** = all `ResourceKind`s, **limited** = a subset.
  Needs (client) a designation affordance to pick general vs a specific resource, and (sim) the
  routing to respect `accepts` (haul-fill does).
- **Logistics Master = the Steward officer** (P12.2; vision: Steward = hauling + stockpiles). When
  appointed, Steward **auto-creates + manages** stockpiles: e.g. one general reservoir near the
  storehouse + per-resource limited piles near the workshops that consume/produce them, and keeps
  hauling prioritised. Officer-automation of the stockpile category (additive: no Steward ⇒ only
  player-designated piles, as today).

### (c) Shrine becomes an offering site (not the default reservoir)
Today (P12.3) the **shrine is the default balancing reservoir**. Change it:
- Seed the starter village with a **general "storehouse" stockpile** that becomes the balancing
  reservoir (keeps the `sum(piles)==resources` invariant intact); the **shrine stops being general
  storage**.
- Two new jobs: **`carry_offering`** — haul a chosen resource (food/herbs/materials — an "offering")
  from a stockpile to the shrine; **`perform_offering`** — a cat performs the offering ritual at the
  shrine, consuming the offered goods and producing **blessings** (`global_upgrade_points`, the god
  currency). This reuses/extends the existing ritual→blessing path but makes it a visible two-step
  haul-then-ritual loop at the shrine.
- Sequencing note: (c) touches the same shrine/deposit/hauling code as **haul-fill**, so do it
  **after** haul-fill lands (not in parallel). Keep the reservoir invariant; regression-guard so an
  un-modified colony (no offerings queued) is byte-identical.

## Cross-cutting
- **Protocol:** every new action + snapshot field goes in `cat-protocol` (camelCase) and
  `apply_action`/`build_snapshot`; the client (P13) adds designation/assignment UI.
- **Determinism:** any new RNG use forks the seeded chain (never `Math.random`); add the
  determinism-twice assertion to each card's tests (per the testing contract).
- **No parallel tick paths:** everything hangs off the single `world_tick` phase order.
- **Sequencing:** P12.1 → (P12.2 ∥ P12.3) → P12.4 → P12.5. P12.1 is the only hard prerequisite
  for the others; do it first.
