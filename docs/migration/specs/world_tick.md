# P7 World Tick Port Spec

Sources read:
- `server/game.ts`, especially `workerTick` lines 2677-5058 and its local helpers.
- `server/elections.ts`, `server/raids.ts`, `server/zones.ts`, `server/worldMap.ts` call sites.
- `db/schema.ts` row shapes read/mutated by the tick.
- Pure modules used by the tick: `idleRules`, `lifeSim`, `movement`, `pathfinding`, `paths`, `leaderDirector`, `production`, `smithy`, `storage`, `survival`, `shrine`, `trips`, `threat`, `upgradeTree`, `housing`, `villageArea`, `roads`, `zones`, `depletion`.
- Direct `workerTick` coverage is in integration tests, not `tests/unit/game`: checked `tests/integration/serverGame.test.ts`, `serverRaids.test.ts`, `serverWorldPersistence.test.ts`, `serverUpgradeTree.test.ts`, `serverVillageArea.test.ts`, and `serverSurvivalBalance.test.ts`.

## Purpose

Port `server/game.ts:workerTick` into a pure deterministic multi-colony world tick. Target Rust module: `crates/cat-sim/src/world_tick.rs`.

The Rust tick should own orchestration only: it calls the already-ported pure modules, mutates in-memory world/colony state, appends events, and returns a tick result. It must not use DB, wall-clock reads, filesystem, threads, or raw RNG.

## Public Surface

Recommended Rust surface:

```rust
pub fn world_tick(world: &mut WorldState, now_ms: i64) -> WorldTickReport;

pub struct WorldTickReport {
    pub ok: bool,
    pub skipped: bool,
    pub colony_reports: Vec<ColonyTickReport>,
}

pub struct ColonyTickReport {
    pub colony_id: ColonyId,
    pub resources: ColonyResources,
    pub automation_tier: f64,
    pub global_upgrade_points: f64,
    pub policy_tier: PolicyTier,
    pub reset: bool,
    pub reset_reason: Option<RunResetReason>,
}
```

`world_tick` iterates `world.colonies` in stable colony id/order order. The old TS tick always calls `ensureGlobalColony` and processes exactly one global colony; Rust should instead tick every active colony independently with the same phase order. Each colony keeps independent `last_tick`, `test_rng_seed`, jobs, cats, events, zones, elections, buildings, raiders, and resources. Shared data is `world_seed` plus the world tile map if tiles are world-global; if tiles stay colony-scoped, keep the TS `colonyId` partition semantics.

## Runtime State Structs

Minimum state needed by `world_tick`:

```rust
pub struct WorldState {
    pub world_seed: u32,
    pub colonies: Vec<ColonyRuntime>,
}

pub struct ColonyRuntime {
    pub id: ColonyId,
    pub name: String,
    pub leader_id: Option<CatId>,
    pub status: ColonyStatus,
    pub resources: ColonyResources,
    pub cats: Vec<CatRuntime>,
    pub jobs: Vec<JobRuntime>,
    pub buildings: Vec<BuildingRuntime>,
    pub events: Vec<EventRuntime>,
    pub world_tiles: Vec<WorldTileRuntime>,
    pub zones: Vec<ZoneRuntime>,
    pub elections: Vec<ElectionRuntime>,
    pub votes: Vec<VoteRuntime>,
    pub raiders: Vec<RaiderRuntime>,
    pub upgrade_levels: UpgradeLevels,
    pub upgrade_tree: UpgradeTreeState,
    pub automation_tier: f64,
    pub global_upgrade_points: f64,
    pub ritual_requested_at: Option<i64>,
    pub critical_since: Option<i64>,
    pub claimed_tiles: Vec<TilePos>,
    pub threat_pressure: f64,
    pub last_raid_at: Option<i64>,
    pub active_raid_id: Option<RaidId>,
    pub raid_clicks: f64,
    pub run_number: u32,
    pub run_started_at: i64,
    pub created_at: i64,
    pub last_player_activity_at: Option<i64>,
    pub last_tick: i64,
    pub test_time_scale: f64,
    pub test_resource_decay_multiplier: f64,
    pub test_resilience_hours_override: Option<f64>,
    pub test_critical_ms_override: i64,
    pub test_rng_seed: Option<u32>,
}
```

`CatRuntime` needs all TS `cats` fields used by the tick: id, name, parent ids, birth/death time, stats, needs, current task, position, destination, carrying, assigned building id, activity, pregnancy fields, age hours, sprite params/genetic traits, specialization, and role XP.

`JobRuntime` needs id, kind, status, requested by type/player, assigned cat id, duration/speed/yield/click fields, created/started/ends/completed timestamps, and metadata. Model metadata as typed variants where possible: construction phase/building type/building id/site, expansion target, hauling total yield/trips done/next trip, accepted flag.

`ColonyResources` must include `food`, `water`, `herbs`, `materials`, `blessings`, `refined`, `weapons`, and `armor`, defaulting missing optional TS fields to `0`.

Keep per-colony route cache if movement remains cached. The TS cache is process-global keyed by cat id and is cleared for a cat on journey end/no destination, and fully cleared on village expansion.

## Constants And Tuning Numbers

Worker-local constants:

| Name | Value | Notes |
| --- | ---: | --- |
| `QUARRY_TILE_TYPES` | `{"mountains", "cave_entrance"}` | Only these explored tile types are quarry sites. |
| `QUARRY_TOTAL_YIELD` | `15` | Materials per quarry job across all trips. |
| `WATER_TOTAL_YIELD` | `40` | Water per fetch job across all trips. |
| `SCOUT_RANGE` | `20` | Frontier max Chebyshev distance from `VILLAGE_ANCHOR`. |
| `ROAD_MATERIALS_RESERVE` | `30` | Materials the leader keeps before auto-paving. |
| `ROAD_MAX_PAVE_PER_BATCH` | `6` | Max auto-paved road tiles per minute. |
| `WALK_WEAR` | `8` | Path wear added for each trodden tile. |
| `MAX_PATH_DECAY_PER_TICK` | `2` | Cap on path wear decay in one tick. |
| `EVENT_KEEP` | `2_000` | Once per minute, keep newest 2000 events. |
| `VILLAGE_START_RADIUS` | `3` | Founding claimed square radius. |
| `LEGACY_AGE_GRACE_MS` | `5 * 60 * 1000` | Server migration helper only; omit if Rust has no legacy DB rows. |

Embedded thresholds/amounts in `workerTick`:
- Runtime defaults: `timeScale = max(1, testTimeScale ?? 1)`, `resourceDecayMultiplier = max(1, testResourceDecayMultiplier ?? 1)`, `criticalMsOverride = max(1_000, testCriticalMsOverride ?? 5 * 60 * 1000)`.
- RNG forks: movement `base_seed + 1_000_003`, life `base_seed + 2_000_003`, raids `base_seed + 3_000_003`.
- Stored food spoilage per minute: `0.0005`; overflow food spoilage per minute: `0.02`.
- Water crisis logs when previous water `> 3` and next water `<= 3`; recovery logs when previous water `<= 3` and patched water `> 6`.
- Minute cadence: true if `elapsedSec >= 60` or wall-clock minute number changes between `now` and previous `lastTick`.
- Explored tile: `pathWear > 62` or Chebyshev distance from anchor `<= 6`.
- Hunt target tile: food `>= 25`, explored, and distance `> 4`.
- Path decay: built roads (`overlayFeature == "road_built"`) never decay; wear `>= 70` decays to at least `63`; wear `> 62` and `< 70` is frozen; otherwise decay to at least `1`.
- Scaffold progress is `round(((now - startedAt) / max(1, endsAt - startedAt)) * 100)`, clamped `0..99` until completion.
- Committed den capacity: unfinished dens add `2 * max(1, level)`; active construct-den jobs add `2` each.
- Starving flag for war planning: `foodCapacity > 0 && nextResources.food / foodCapacity < 0.15`.
- School research trickle: each finished school counts as `0.25` researcher.
- Player supply completion adds `8` food or water.
- Build material-gather phase adds `12` materials.
- Build completion increases automation tier by `0.05`, rounded to two decimals and capped at `10`.
- Hunt/build role XP increments by `1`; hunting/building stat gain is `+0.4`, capped at `100`.
- Ritual blessing carry amount: `1 + floor(upgrades.ritual_mastery / 3)`.
- Warrior training stat gain: attack `+3`, defense `+3`, both capped at `100`; specialization becomes `warrior`; warrior XP increments by `1`.
- Wander chance: `min(0.08, 0.02 * elapsedSec)`.
- Built-road speed bonus in movement: `+0.6`; otherwise use `getPathSpeedBonus(pathWear)`.
- Exploration reveal radius: `2` for `currentTask == "explore"`, otherwise `1`.
- Route tile wear after movement: on route `max(addPathWear(pathWear, 8), 64)`; halo-only reveal `max(pathWear, 63)`.

Imported module constants are not duplicated here in full; keep their module specs/tests authoritative. The tick relies on these values directly: `HUNT_TRIP_COUNT = 3`, `ROAD_PAVE_WEAR = 70`, `MOVE_SPEED_TILES_PER_SEC = 0.5`, `EXPLORE_SPEED_FACTOR = 0.35`, storage base/bonus caps, production/smithy cycle constants, election windows, threat thresholds, shrine deposit radius/grace, and village expansion thresholds.

## Determinism

The old TS code has four roll chains:

1. Base policy chain: starts at `runtime.rngSeed` (`colony.testRngSeed`), advances only through `nextRoll()`, and is persisted as `testRngSeed` at the end.
2. Movement chain: initialized once per tick from the current base seed after policy-tier roll and life fork setup: `movementSeed = rngSeed + 1_000_003`. It is not persisted.
3. Life chain: passed into `runLifeSimulation` as `lifeSeed = rngSeed + 2_000_003`. It is not persisted and does not feed back into base seed.
4. Raid chain: initialized near the end from the then-current base seed: `raidSeed = rngSeed + 3_000_003`. It is not persisted.

When `testRngSeed` is `null`, TS uses raw `Math.random()` in all chains. This is non-deterministic and should not be reproduced in `cat-sim` tests. Recommended Rust behavior: keep the optional unseeded mode only at a shell/server layer if needed, but pure `world_tick` should receive/store a seed for deterministic simulation.

TS also calls `Date.now()` inside helpers (`runLifeSimulation`, `birthKitten`, `queueJob`, `logEvent`, `resetGlobalRun`, raid spawning/logging), even though `workerTick` already computed `now`. For Rust, use the `now_ms` argument everywhere. This is a parity-improving normalization; generated fixtures should avoid asserting distinct timestamps inside one tick beyond “equals tick now” unless the TS fixture captures exact values.

Sort/tie behavior to preserve:
- `chooseLeader`: scans alive cats in storage order, picks strictly greater leadership; ties keep first.
- Quarry/water/frontier sites sort by Chebyshev distance only; equal-distance order stays original tile iteration order.
- `selectBestCat`: filters available cats in storage order; prefers matching specialization if any; picks strictly greater relevant stat, ties keep first.
- `matchCatsToSlots` owns labor assignment ordering; execute its returned assignments in order.
- Forest chopping picks first explored forest, then replaces only on strictly smaller Chebyshev distance; ties keep first.
- Raid casualty on sack picks weakest mustered defender by ascending power; ties depend on JS stable sort/input order. If no muster, random cat index is `min(len - 1, floor(roll * len))`.

## Ordered Tick Phases

The Rust port must execute these phases in order for each colony.

| # | Phase | Purpose and exact work | Calls/modules | State read/mutated and DB replacement |
| ---: | --- | --- | --- | --- |
| 1 | Colony selection and elapsed time gate | TS ensures/loads the global colony, computes `elapsedSec = max(0, floor((now - lastTick)/1000))`, returns skipped without touching `lastTick` if `0`, and computes `processedThrough = lastTick + elapsedSec * 1000`. Rust iterates all colonies and applies this gate per colony. | server bootstrap only | Read `lastTick`. Mutate nothing on skip. Replace `ensureGlobalColony` with setup outside pure tick or an explicit initialization phase. |
| 2 | Runtime, upgrades, and effects | Load legacy upgrade levels, runtime test config, deserialize upgrade tree, resolve upgrade-tree effects. | `idleEngine`, `upgradeTree::deserialize/resolveEffects` | Read colony test config, upgrade levels, upgrade tree. No mutation. In Rust store upgrades directly in `ColonyRuntime`. |
| 3 | Base RNG and fork roots | Initialize base `rngSeed` from runtime. `nextRoll` uses seeded LCG and advances only base seed, or raw random in unseeded TS mode. Later fork roots are derived from the current base seed. | `seededRng::rollSeeded` | Read/mutate local base seed. Persist only base seed in phase 37. |
| 4 | Leader bootstrap and policy | If current `leaderId` is absent/dead, choose highest-leadership alive cat as interim, set `leaderId`, log `leader_change`. Then roll policy tier with `pickPolicyTier(leader leadership or 50, nextRoll())`, load config, define `canTakePolicyAction = nextRoll() <= actionReliability`. | local `chooseLeader`; `policy::pickPolicyTier/configForTier` | Read alive cats, leader id. Mutate `leaderId`, events. DB update/log becomes field assignment and event append. |
| 5 | Initial roster, buildings, caps | Snapshot alive cats. Load all buildings. Compute storage capacities with `effects.storagePerLevelMult`; set initial `foodCapacity`. | `storage::storageCapacities` | Read cats/buildings/resources. No mutation. |
| 6 | Life simulation | Age all alive cats by `(elapsedSec * timeScale)/3600`; old-age deaths use life fork; leaders gain leadership tenure; milestone events; births for due pregnant cats; adult conceptions while breeding gate passes. Roster is reloaded afterward. | `lifeSim`, `lifeMilestones`, genetics/naming helpers | Mutate cat age/stats/death/pregnancy/new kittens/jobs cancelled by death/events. DB inserts/updates become vector mutations. Use `lifeSeed = base_seed + 2_000_003`. |
| 7 | Consumption, spoilage, resource pre-patch, minute cadence | Compute `foodUse/waterUse` with cat count and `elapsedSec * resourceDecayMultiplier`. Subtract food, split stored/overflow by initial `foodCapacity`, apply spoilage, clamp water/herbs/materials/refined to caps, and copy into `nextResources`. Compute `minuteRolled`. | `idleRules::consumptionForTick`; `storage` | Read resources/caps/cat count. Mutate local `nextResources` only. |
| 8 | Water low crisis edge | If previous water was `> 3` and `nextResources.water <= 3`, append crisis event. | event helper | Read previous and next water. Mutate events. |
| 9 | Elections lifecycle | Resolve due open election, resolve due vote-kick, replace kicked leader and maybe open snap election, then open scheduled election if term due. Consumes no policy rolls. | `server/elections::runElectionLifecycle`; `lib/game/elections` | Mutate elections, votes-derived winner fields, `leaderId`, events. DB selects/updates/inserts become in-memory election/vote scans and appends. |
| 10 | Zones and event pruning | Delete expired zones, and once per minute keep only newest `EVENT_KEEP = 2000` events. Snapshot active zones as plain `Zone` rectangles for movement/destination steering. | `server/zones`; `zones` | Mutate zones/events. Active zone order follows stored zone order after expiry removal. |
| 11 | Path wear decay | Compute `decayAmount = min(MAX_PATH_DECAY_PER_TICK, elapsedSec * timeScale / 60)`. For colony tiles with `pathWear > 0`, skip built roads, decay worn roads `>= 70` down no lower than `63`, freeze `> 62`, else decay down no lower than `1`. | `paths` constants conceptually | Mutate `worldTiles.pathWear`. DB tile scan/update becomes mutable tile loop. |
| 12 | Resource regrowth | Once per minute, `regrowthAmount(elapsedSec * timeScale)`. For tiles with `lastDepleted > 0`, skip forests, and add food toward `maxResources.food`. | `depletion::regrowthAmount/isForestType` | Mutate `worldTiles.resources.food`. Chopped forests never regrow because their type is `field` with low cap and `lastDepleted > 0`; natural forest types are skipped. |
| 13 | Tick-local target caches | Create active `zoneList`, movement RNG fork, lazy query helpers: food tiles, colony tiles, quarry sites, water sites, frontier tiles, water-cell test, `drainHuntSite`. | `zones::pickTargetWithZones/isInZone`; local tile helpers | Local caches only, plus `drainHuntSite` later mutates tile food/lastDepleted. Movement fork is `base_seed + 1_000_003`. |
| 14 | Promote queued jobs and break ground | Iterate queued jobs in stored order. For construction `build_house` with metadata phase `construct_house`, choose scaffold type from allowed list (`workshop`, `field`, `food_storage`, `research_hut`, `school`, `smithy`, `barracks`, else `den`), choose a claimed free non-water site using one movement roll, insert scaffold building at progress `0`, and write `site/buildingId` metadata. Then set job `active`, `startedAt = job.startedAt || now`, keep `endsAt`. | `villageArea`, local `nextClaimedBuildingSite`, `movement` RNG | Mutate queued jobs to active and maybe buildings/job metadata. DB insert/update becomes vector push/mutation. |
| 15 | Assign promoted job destinations | For promoted jobs with assigned cats, compute target: hunts use zoned food pick; quarry nearest quarry; water nearest water; explore round-robins frontier by `scoutPromotions`; expansion uses metadata target. Then `destinationForJob` gets anchor/shrine/food/roll/site variants. If destination exists, update job metadata `{site, accepted:false}` and send cat to shrine anchor with `activity="traveling"`, `currentTask=job.kind`. | `movement::destinationForJob`; `zones::pickTargetWithZones` | Mutate job metadata and cat destination/activity/task. Movement rolls consumed for zoned pick, destination roll, construction site selection. |
| 16 | Active scaffold progress | For active `build_house` jobs with metadata `buildingId`, update matching building `constructionProgress` to rounded timer progress capped at `99`. | local math | Mutate buildings only. |
| 17 | Legacy emergency hunt | If `nextResources.food < policy.foodEmergencyThreshold`, no conflicting `leader_plan_hunt`/hunt active, and policy roll passes, queue `leader_plan_hunt` assigned to best hunter. This block predates the consolidated director and still runs before it. | `idleRules::hasConflictingStrategicJob`; local `queueJob/selectBestCat` | Mutate jobs/events and maybe clear production building assignment on queued cat. Base policy roll consumed only if threshold/conflict gates pass. |
| 18 | Leader snapshot assembly | Load current buildings; compute active hunt/quarry/scout/water job counts, den/storage in-flight, committed capacity, busy ids, work-capable/idle cats, workforce weight, unstaffed workshops/research huts/smithies, barracks, warriors, training count, threat band, starving flag. Build `LeaderSnapshot`. | `lifeSim::canWork/getLifeStage/workforceWeight`; `housing`; `storage`; `threat::threatBand` | Read cats/jobs/buildings/resources/caps/tiles. No mutation. |
| 19 | Leader cancellations | Run `directColony(snapshot)`. Execute cancellation decisions before spending labor: `cancel_hunts` cancels all active hunts and sends assigned cats home returning; `cancel_training` cancels active warrior training and idles recruits. Log if any cancelled. | `leaderDirector::directColony` | Mutate active jobs, cats, events. No policy roll for cancellations. |
| 20 | Leader labor assignments and staffing | Convert available idle cats to `CatBrief`, call `matchCatsToSlots(plan.slots, catBriefs, {excludeWarriorsFromTraining:true})`. For each assignment, consume one policy roll; if it passes, queue expedition/training jobs, or assign cat to next unstaffed workshop/research/smithy queue and log assignment. | `leaderDirector::matchCatsToSlots`; local `queueJob` | Mutate jobs, cats assigned buildings, workshop worker map, events. Base policy rolls consumed per attempted assignment. |
| 21 | Leader capital decisions and tithe | For each director decision: `build_storage` policy roll then queue `build_house` food_storage with best architect; `build_den` policy roll then queue `leader_plan_house`; `tithe` only when `minuteRolled`, subtract decision food/refined from `nextResources`, add decision blessings to colony global upgrade points, log shrine deposit. | `leaderDirector`; local `queueJob/selectBestCat` | Mutate jobs, resources, `globalUpgradePoints`, events. |
| 22 | Ritual approval | If `shouldStartRitual(ritualRequestedAt, nextResources, activeJobs)` and policy roll passes, queue `ritual` with best ritualist, clear `ritualRequestedAt`, and log approval. | `idleRules::shouldStartRitual`; local `queueJob/selectBestCat` | Mutate jobs, colony ritual flag, events. |
| 23 | Production | Copy `nextResources` into `patchedResources`. For each finished field, add passive `fieldYield(elapsedSec * timeScale)`. For each workshop, call `advanceWorkshop`; consume materials, add refined, log production, update building progress. For each smithy, call `advanceSmithy`; consume refined/materials, add weapons/armor, log, update progress. Workshop/smithy worker is from assigned-building map. | `production`; `smithy` | Mutate `patchedResources`, building `productionProgress`, events. |
| 24 | Research | Count assigned cats in finished research huts plus `0.25` per finished school. Accrue research points with `pointsPerTickFor`, then repeatedly `catAutoUnlock` cheapest affordable nodes until none; log each unlock and save tree if changed. | `upgradeTree` | Mutate upgrade tree and events. Effects used this tick do not refresh after auto-unlock; new effects apply next tick. |
| 25 | Survival, deaths, and carried-yield salvage | For each pre-survival alive cat, apply survival tick using `patchedResources.food/water`, elapsed resource decay, and policy needs multipliers. Update needs, log dehydration/recovery. If dead, salvage carried food/materials/water/blessings into patched resources/global points, retire cat, and log starvation/dehydration death. | `survival::applySurvivalTick`; local `retireCat` | Mutate cats, jobs cancelled by death, resources/global points, events. |
| 26 | Empty-colony reset | Reload alive cats. If none, reset run with reason `all-cats-dead` and return for this colony. Reset keeps completed buildings/world tiles, deletes jobs/raiders/unfinished buildings, resolves open elections, resets resources except blessings, threat/raid state, cats or creates starters. | local `resetGlobalRun` | Mutate almost all per-colony runtime fields. In pure Rust, return `reset=true` for that colony and do not continue later phases. |
| 27 | Due job prelude | Reload active jobs, select due jobs where `endsAt <= now`. Build `activeOrQueuedJobs` from active-at-completion plus the original `queuedJobs` snapshot from phase 14. | local status scans | Local lists only. Note the queued snapshot is from before promotion and may include jobs already promoted in this tick; preserve if golden parity demands it. |
| 28 | Due completion I: supplies and planner jobs | For each due job in active order: `supply_food/supply_water` add `8` and update requesting player lifetime contribution in TS; `leader_plan_hunt` with policy pass queues `hunt_expedition`; `leader_plan_house` calls `queuePlannedHouseJobs`. | `housePlanner`; local `queueJob` | Mutate resources, jobs/events. Player contribution is server/accounting only; omit from `cat-sim` unless protocol needs it. |
| 29 | Due completion II: gathering, explore, expansion | Hunts compute total yield from metadata or `huntYieldFor`, completion reward `remainingYield`, drain hunt site, increment hunter XP/stat/specialization, set food carrying. Quarry/fetch use total 15/40 and set materials/water carrying. Explore logs discovery. Expansion rounds target, ensures tile exists, and if target is outside but adjacent and not water, appends claimed tile, clears forest tile if needed, clears route cache, logs. | `trips`, `idleEngine`, `lifeSim`, `depletion`, `villageArea`, `worldMap` replacement | Mutate cats, tile depletion, jobs, claimed area, route cache, events. `ensureChunk` becomes “ensure tile/chunk exists in world state” or no-op if full map loaded. |
| 30 | Due completion III: build, ritual, training, return, mark done | Build construct phase consumes policy water/materials if available, finishes scaffold, automation tier `+0.05`; otherwise deletes scaffold and replans. Build gather phase adds `12` materials and chops nearest explored forest to `field` with food/herbs `0`, food cap `5`, `lastDepleted=now`. Ritual increments ritualist XP and sets blessing carrying. Warrior training specializes cat, increments warrior XP, attack/defense +3, idles and logs. For work jobs, send cat returning to shrine or a wander home spot for build jobs. Finally set job completed/completedAt and log `job_completed`. | `idleEngine`, `movement::pickWanderTarget`, `depletion`, local planners | Mutate resources, buildings, world tiles, cats, jobs, events, automation tier. Movement rolls are consumed for build-job home spot only. |
| 31 | Mid-job hauling | For each active hunt/quarry/fetch with assigned cat, accepted site, trips left before final, and due trip time: require worker alive, `activity == "working"`, and no carrying. Compute total yield once, share with `splitYield`, drain hunt site for hunts, update job metadata `totalYield/tripsDone/nextTripAt`, set cat carrying and returning to shrine. | `trips`, local `huntYieldFor/drainHuntSite` | Mutate jobs, cats, resources on tiles. |
| 32 | Movement setup and village expansion queue | Compute `movementElapsed`, wander chance, ring radius, claimed area, optionally queue one `expand_village` job if `shouldExpand`, no active/queued expansion exists, and policy roll passes. Compute gate from claimed area or legacy south gate. Build walk grid from colony tiles, water, fence, roads, and terrain generated from `worldSeed ?? createdAt`. | `villageArea`, `villageLayout`, `pathfinding`, `terrainGen` | Mutate jobs/events only for expansion queue. Build local `walkGrid`. Base policy roll may be consumed. |
| 33 | Movement deposits and no-destination/wander | For each alive cat in storage order, convert colony-local position to world. If carrying and `shouldDeposit`, credit resource/blessings, clear carrying, log, and if an ongoing active gather/fetch job exists, send cat back to its site and continue. If no destination, clear route cache; traveling/returning becomes idle; idle cats may roll to wander around assigned building or shrine, avoiding avoid-zones. | `shrine`; `movement::pickWanderTarget`; `zones::isInZone` | Mutate resources/global points, cats, events, route cache. Movement rolls consumed for idle wander chance and target coordinates. |
| 34 | Movement travel, job acceptance, reveal, path wear | For cats with destinations, load standing tile, compute speed, route using cached A*/fallback gate waypoint, walk with `walkPath`. On arrival while traveling to shrine for accepted=false job, mark job accepted and redirect to site. Otherwise update position and if arrived clear destination and set activity `working` when traveling else `idle`. For moved cats, reveal/wear nearby tiles outside village: route tiles become at least `64` and gain `WALK_WEAR`; halo becomes at least `63`. | `pathfinding`, `movement::walkPath`, `paths::addPathWear/getPathSpeedBonus`, `villageArea` | Mutate cats, jobs, route cache, world tile pathWear. Movement is cosmetic but path wear affects future economy/navigation. |
| 35 | Deliberate roads | Once per minute, if `patchedResources.materials > 30`, pave up to `min(6, materials - 30)` tiles from `selectRoadCorridor` among tiles with `pathWear >= 70`. Set each to `overlayFeature="road_built"` and `pathWear=100`, subtract one material per tile, log. | `roads::selectRoadCorridor`; `ROAD_PAVE_WEAR` | Mutate world tiles, resources, events. |
| 36 | Threat and raid director | Initialize raid fork from current base seed `+ 3_000_003`. Run raid director after cat movement, with accelerated elapsed seconds, current alive cats, combat effects, mutable resources, pressure, colony age, active raid id/clicks, and walk grid. It either accrues pressure/spawns raid, marches active raiders, resolves combat, mutates loot/gear/cat deaths, and returns pressure. If all cats die, reset reason `raid-wipeout` and return. | `server/raids` to pure `raid_director`; `threat`, `warriors`, `movement`, `pathfinding` | Mutate threat pressure, active raid id, raiders, resources, cats/jobs on casualties, events. Raid rolls only through raid fork. |
| 37 | Final clamp, critical collapse, status, persist | Recompute storage caps from latest buildings and clamp every resource to capacity. Compute unattended hours and resilience. Update/clear `criticalSince`; if critical too long, reset reason `unattended-collapse` and return. Log water recovery edge. Compute `nextColonyStatus`. Persist colony resources/status/automation/global points/critical/threat/`lastTick=processedThrough`/`testRngSeed=base rngSeed`. Return report. | `storage`, `idleEngine::getResilienceHours`, `idleRules` | Mutate colony resource/status/timers/seed/events or reset. This is the only phase that advances persisted base test RNG seed. |

## DB Operations To Replace

Replace these TS DB operations with in-memory operations:

- `select().from(cats).where(colonyId && deathTime null)`: filter `colony.cats` by `death_time.is_none()`, preserving vector order.
- `select jobs by status`: filter `colony.jobs`, preserving vector order.
- `insert jobs/buildings/events/cats/raiders/elections`: push to the relevant vector with deterministic id generation from a fixture id allocator or stable counter. Do not use random ids inside `cat-sim`.
- `update rows`: mutate the matching struct by id.
- `delete jobs/raiders/buildings/zones/events`: retain/filter vectors.
- `events` append/prune: append `EventRuntime { timestamp: now_ms, ... }`; for prune, sort/order exactly as stored if timestamps tie, then keep newest 2000 by descending timestamp semantics.
- `ensureChunk`/`initializeWorldMap`: should not run hidden IO. Either pre-generate tiles in `WorldState` or make chunk generation a deterministic pure helper using `worldSeed`.
- Player lifetime contribution writes are not simulation state. Exclude from `cat-sim` unless the protocol crate explicitly needs them.

## Golden Fixtures To Generate

Generate fixtures from TS by setting `testRngSeed`, replacing wall-clock with controlled `advanceTime`, then snapshotting only simulation fields. Use stable ids in the fixture serializer if possible.

1. **Sub-second skip**: colony `lastTick = now - 999`, any seed. Expected: skipped report, `lastTick` unchanged, no resources/cats/jobs/events changed.
2. **One-cat consumption and status**: one adult idle cat, no jobs, resources `food=100 water=100 herbs=16 materials=0`, base caps only, `elapsedSec=60`, seed `12345`. Expected hand checks: food and water decrease by `consumptionForTick(1, 60, upgrades)`, food also gets stored spoilage, `lastTick += 60000`, base seed advanced by policy rolls.
3. **Queued hunt promotion**: one queued `hunt_expedition` with assigned cat, one explored food tile with `food>=25`, seed `12345`, elapsed `1`. Expected: job active, metadata has shrine-accepted `false` and selected site, cat destination is `VILLAGE_ANCHOR`, activity `traveling`, current task `hunt_expedition`.
4. **Mid-job haul split**: active accepted hunt with total yield `10`, `HUNT_TRIP_COUNT=3`, `tripsDone=0`, worker at site and working, next trip due. Expected share `4`, tripsDone `1`, nextTripAt for trip 2, cat carrying food `4`, hunt site food drained by `4`.
5. **Build completion and scaffold**: active due construct `build_house` with building id, enough water/materials, policy requirements from selected tier. Expected water/materials decremented, building progress `100`, automation tier `+0.05` rounded to two decimals, architect XP/stat increment.
6. **Movement reveal**: cat walks across at least two outside-village tiles with no job effects. Expected route tiles pathWear at least `64` and halo at least `63`, inside-village tiles untouched.
7. **Road paving**: minute tick, materials `> 30`, at least eight worn outside tiles with `pathWear>=70`. Expected at most six selected corridor tiles become `road_built`/`100`, materials subtract corridor length, reserve stays at least `30`.
8. **Raid fork isolation**: same colony and base seed with/without an active raid, no earlier policy differences. Expected base `testRngSeed` after tick is unchanged by raid rolls; raid outcomes differ only in raid/raider/resource/casualty fields.
9. **Critical unattended collapse**: food or water `0`, unattended hours beyond resilience, `criticalSince` older than threshold. Expected reset reason `unattended-collapse`, jobs/raiders cleared, unfinished buildings removed, world tiles preserved.
10. **Multi-colony order**: two colonies with different ids/seeds and one due job each. Expected both tick in stable order, no cross-colony mutation, each persists its own base seed/lastTick/resources/events.

## Dependencies

Port or stub these cat-sim modules before `world_tick` can be finished:

- Required pure rule modules: `rng`, `idle_engine`, `idle_rules`, `policy`, `life_sim`, `life_milestones`, `genetics`, `naming`, `housing`, `storage`, `leader_director`, `house_planner`, `movement`, `pathfinding`, `paths`, `zones`, `depletion`, `production`, `smithy`, `survival`, `shrine`, `trips`, `upgrade_tree`, `village_area`, `village_layout`, `terrain_gen`, `roads`, `threat`, `warriors`.
- Server orchestration to make pure: elections lifecycle, raid director, world/chunk tile generation, event append/prune, job queueing, run reset.
- Protocol/state types: colony resources, cats, jobs, buildings, events, world tiles, zones, elections/votes, raiders, upgrade tree, route cache.

## Known TS Quirks To Preserve Or Deliberately Normalize

- Preserve phase order exactly; many decisions use snapshots from earlier phases (`activeJobs`, original `queuedJobs`, initial caps) rather than freshly reloading after every mutation.
- Preserve fork behavior: movement/life/raid fork seeds do not feed back into persisted seed.
- Normalize `Date.now()` to the `now_ms` argument in Rust. This is safer and required for pure determinism.
- Do not port newspaper/flavor generators. Tick event messages are enough for parity where they are part of gameplay feedback.
- Avoid raw `Math.random` equivalent in `cat-sim`; require deterministic seeded state for tests and simulation.
