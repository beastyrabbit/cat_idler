/**
 * Game server logic — 1:1 port of convex/game.ts onto Drizzle + SQLite.
 *
 * Pure game rules stay in lib/game/; this module reads/writes the DB and
 * orchestrates. All entry points are synchronous (better-sqlite3) and the
 * mutating ones run inside a transaction.
 *
 * The worker process calls `workerTick` every second; Next.js route
 * handlers call the rest.
 */

import { and, desc, eq, gt, gte, inArray, isNull, lt, lte } from "drizzle-orm";
import { nanoid } from "nanoid";
import type { GameDb } from "@/db/client";
import {
	buildings,
	type CatRow,
	type ColonyRow,
	cats,
	colonies,
	elections,
	events,
	globalUpgrades,
	type JobRow,
	jobs,
	players,
	type RoleXpJson,
	raiders,
	runHistory,
	votes,
	type WorldTileRow,
	worldTiles,
} from "@/db/schema";
import type { CatSpriteParams } from "@/lib/cat-renderer/types";
import {
	CHOPPED_FOREST_FOOD_CAP,
	isForestType,
	regrowthAmount,
} from "@/lib/game/depletion";
import {
	KICK_THRESHOLD,
	tallyVotes,
	voteIdentityKey,
} from "@/lib/game/elections";
import {
	extractGeneticTraits,
	type GeneticTraits,
	inheritTraits,
	traitsToSpriteParams,
} from "@/lib/game/genetics";
import { planHousePipeline } from "@/lib/game/housePlanner";
import {
	housingCapacity,
	housingPressure,
	villageLevel,
} from "@/lib/game/housing";
import {
	applyClickBoostSeconds,
	type CatSpecialization,
	getHuntReward,
	getResilienceHours,
	getScaledDurationSeconds,
	getUpgradeCost,
	type JobKind,
	nextSpecialization,
	type UpgradeLevels,
} from "@/lib/game/idleEngine";
import {
	consumptionForTick,
	hasConflictingStrategicJob,
	nextColonyStatus,
	ritualRequestIsFresh,
	shouldResetFromCritical,
	shouldStartRitual,
	shouldTrackCritical,
} from "@/lib/game/idleRules";
import type { LeaderSnapshot } from "@/lib/game/leaderAI";
import {
	type CatBrief,
	directColony,
	matchCatsToSlots,
} from "@/lib/game/leaderDirector";
import {
	detectLifeStageTransition,
	generateMilestoneAnnouncement,
} from "@/lib/game/lifeMilestones";
import {
	canWork,
	colonyCanBreed,
	conceptionProbability,
	GESTATION_GAME_HOURS,
	getLifeStage,
	inheritStats,
	leadershipAfterTenure,
	oldAgeDeathProbability,
	stageWorkEffectiveness,
	tradeSpeedMultiplier,
	tradeYieldMultiplier,
	workforceWeight,
} from "@/lib/game/lifeSim";
import {
	destinationForJob,
	EXPLORE_SPEED_FACTOR,
	MOVE_SPEED_TILES_PER_SEC,
	pickWanderTarget,
	type WorldPos,
	walkPath,
} from "@/lib/game/movement";
import { generateName } from "@/lib/game/naming";
import {
	buildColonyWalkGrid,
	findPath,
	type WalkGrid,
} from "@/lib/game/pathfinding";
import { addPathWear, getPathSpeedBonus } from "@/lib/game/paths";
import { configForTier, pickPolicyTier } from "@/lib/game/policy";
import {
	advanceWorkshop,
	fieldUnlocked,
	fieldYield,
	workshopUnlocked,
} from "@/lib/game/production";
import { ROAD_PAVE_WEAR, selectRoadCorridor } from "@/lib/game/roads";
import { rollSeeded } from "@/lib/game/seededRng";
import { shouldDeposit } from "@/lib/game/shrine";
import { advanceSmithy } from "@/lib/game/smithy";
import {
	countStorehouses,
	storageCapacities,
	storehouseCap,
} from "@/lib/game/storage";
import { applySurvivalTick } from "@/lib/game/survival";
import {
	terrainHeightAt,
	terrainStairAt,
	WORLD_TERRAIN_OPTIONS,
} from "@/lib/game/terrainGen";
import { configForPreset } from "@/lib/game/testAcceleration";
import { colonyWealth, threatBand } from "@/lib/game/threat";
import {
	HUNT_TRIP_COUNT,
	remainingYield,
	splitYield,
	tripDueAt,
} from "@/lib/game/trips";
import {
	accrueResearch,
	catAutoUnlock,
	createUpgradeTreeState,
	deserializeUpgradeTreeState,
	getNode,
	godPurchase,
	isOwned,
	nextResearchTarget,
	pointsPerTickFor,
	resolveEffects,
	serializeUpgradeTreeState,
	type UpgradeTreeState,
} from "@/lib/game/upgradeTree";
import {
	type GatePlacement,
	isInsideVillage,
	SIDE_DELTA as VILLAGE_SIDE_DELTA,
	type VillageArea,
	type Pos as VillagePos,
	expandVillage as villageExpand,
	fromTiles as villageFromTiles,
	gatePlacement as villageGate,
	shouldExpand as villageShouldExpand,
	toTiles as villageToTiles,
} from "@/lib/game/villageArea";
import {
	colonyToWorld,
	nextBuildingSite,
	ringCells,
	SHRINE_LOCAL,
	VILLAGE_ANCHOR,
	villageRingRadius,
} from "@/lib/game/villageLayout";
import { isInZone, pickTargetWithZones, type Zone } from "@/lib/game/zones";
import {
	DEFEND_CLICK_DAMAGE,
	gatePosition,
	runRaidDirector,
	spawnRaid,
} from "@/server/raids";
import type { CatStats } from "@/types/game";

import { runElectionLifecycle } from "./elections";
import { countOnlinePlayers, upsertPlayer } from "./players";
import { ensureChunk, initializeWorldMap } from "./worldMap";
import { activeZones, sweepExpiredZones } from "./zones";

/** Tile types a quarry expedition can mine for materials. */
const QUARRY_TILE_TYPES: ReadonlySet<string> = new Set([
	"mountains",
	"cave_entrance",
]);
/** Materials one quarry expedition hauls home across its trips. */
const QUARRY_TOTAL_YIELD = 15;
/** Water one fetch expedition hauls home across its trips. */
const WATER_TOTAL_YIELD = 40;
/** Explore jobs only target fogged frontier tiles within this range. */
const SCOUT_RANGE = 20;
/** The leader keeps this many materials in reserve before paving roads. */
const ROAD_MATERIALS_RESERVE = 30;
/** Most tiles the leader paves into road in a single (once-a-minute) batch. */
const ROAD_MAX_PAVE_PER_BATCH = 6;

/**
 * Wear a single traversal lays on a trodden tile. The first pass reveals the
 * tile (clamped to 64); a second pass over the same tile crosses the road
 * threshold (>=70), so shared corridors between shrine and work sites harden
 * into roads while a one-off scouting crossing stays bare explored ground.
 */
const WALK_WEAR = 8;

/**
 * Per-cat route memo, keyed by cat id and shared across ticks (the worker is a
 * single long-lived process). A* is cheap but runs for every traveller every
 * tick; caching the last route lets a cat that is still trudging toward the same
 * destination reuse it instead of re-searching. The route is re-planned only
 * when its target changes or a tile it was about to step on has become blocked —
 * the two things that can make a cached route wrong. Entries are dropped when a
 * cat arrives or loses its destination, so the map tracks only live travellers.
 */
interface CachedRoute {
	destKey: string;
	route: WorldPos[];
}
const routeCache = new Map<string, CachedRoute>();

/**
 * Route from the cat's current tile to `destination`, reusing the cached plan
 * when it is still valid. A reused route is the *remaining tail* of the cached
 * plan from the cat's current tile — a sub-path of an optimal route is itself
 * optimal, so this returns the same tiles a fresh search would walk, just
 * without paying for the search. Falls back to (and caches) a fresh A* search on
 * a target change, when the cat has strayed off its cached route, or when an
 * upcoming tile is now blocked.
 */
function routeForCat(
	catId: string,
	worldPos: WorldPos,
	destination: WorldPos,
	grid: WalkGrid,
): WorldPos[] | null {
	const curX = Math.round(worldPos.x);
	const curY = Math.round(worldPos.y);
	const destKey = `${Math.round(destination.x)},${Math.round(destination.y)}`;
	const cached = routeCache.get(catId);
	if (cached && cached.destKey === destKey) {
		const idx = cached.route.findIndex((p) => p.x === curX && p.y === curY);
		if (idx >= 0) {
			const remaining = cached.route.slice(idx);
			// Interior tiles (everything but the always-enterable goal) must still be
			// walkable, and every edge must still be crossable. The organic fence is
			// an edge blocker, so checking tiles alone would let stale routes keep
			// using a gate that moved after village expansion.
			const blockedAhead = remaining.slice(1).some((p, i) => {
				const prev = remaining[i];
				const isGoal = i === remaining.length - 2;
				return (
					(!isGoal && grid.isBlocked(p.x, p.y)) ||
					(grid.fenceBlocksStep?.(prev.x, prev.y, p.x, p.y) ?? false)
				);
			});
			if (!blockedAhead) {
				return remaining;
			}
		}
	}
	const fresh = findPath(worldPos, destination, grid);
	if (fresh) {
		routeCache.set(catId, { destKey, route: fresh });
	} else {
		routeCache.delete(catId);
	}
	return fresh;
}
/**
 * Most path wear that can fade in one tick. Traversal adds {@link WALK_WEAR}
 * per pass, so a route walked repeatedly outpaces this cap and hardens into a
 * road, while a one-off crossing fades back to bare explored ground.
 */
const MAX_PATH_DECAY_PER_TICK = 2;

/** A world tile holds drawable water (river channel or a resource pool). */
function tileHasWater(tile: {
	type: string;
	overlayFeature?: string | null;
	resources: { water: number };
}): boolean {
	return (
		tile.type === "river" ||
		tile.overlayFeature === "river" ||
		(tile.resources?.water ?? 0) > 0
	);
}

const UPGRADE_DEFAULTS = [
	{
		key: "click_power",
		maxLevel: 20,
		baseCost: 2,
		description: "Increase click speed-up power.",
	},
	{
		key: "supply_speed",
		maxLevel: 10,
		baseCost: 3,
		description: "Reduce player supply action time.",
	},
	{
		key: "hunt_mastery",
		maxLevel: 10,
		baseCost: 5,
		description: "Improve hunting speed and yield.",
	},
	{
		key: "build_mastery",
		maxLevel: 10,
		baseCost: 5,
		description: "Improve planning and build speed.",
	},
	{
		key: "ritual_mastery",
		maxLevel: 10,
		baseCost: 6,
		description: "Improve ritual cadence and timing.",
	},
	{
		key: "resilience",
		maxLevel: 10,
		baseCost: 7,
		description: "Survive unattended for longer.",
	},
] as const;

type UpgradeKey = (typeof UPGRADE_DEFAULTS)[number]["key"];

export type PlayerJobKind =
	| "supply_food"
	| "supply_water"
	| "leader_plan_hunt"
	| "leader_plan_house"
	| "ritual";

interface RuntimeConfig {
	timeScale: number;
	resourceDecayMultiplier: number;
	resilienceHoursOverride: number | null;
	criticalMsOverride: number;
	rngSeed: number | null;
}

function getRuntimeConfig(colony: ColonyRow): RuntimeConfig {
	return {
		timeScale: Math.max(1, colony.testTimeScale ?? 1),
		resourceDecayMultiplier: Math.max(
			1,
			colony.testResourceDecayMultiplier ?? 1,
		),
		resilienceHoursOverride:
			typeof colony.testResilienceHoursOverride === "number"
				? colony.testResilienceHoursOverride
				: null,
		criticalMsOverride: Math.max(
			1_000,
			colony.testCriticalMsOverride ?? 5 * 60 * 1000,
		),
		rngSeed: typeof colony.testRngSeed === "number" ? colony.testRngSeed : null,
	};
}

const DEFAULT_ROLE_XP = {
	hunter: 0,
	architect: 0,
	ritualist: 0,
	warrior: 0,
} as const;

/**
 * Stocked general storage for a fresh settlement, plus starter materials. Food
 * carries the colony through the opening gap before the first hunt returns (a
 * hunt takes 8 game-hours). It need not be a large surplus: the per-capita
 * breeding gate (see colonyCanBreed) lets the young founders start a replacement
 * generation as long as the store holds a few units per cat, which this clears
 * from turn one. Water self-sustains quickly (river fetching outpaces drinking).
 */
const STARTING_RESOURCES = {
	food: 150,
	water: 100,
	herbs: 16,
	materials: 24,
	blessings: 0,
	refined: 0,
	weapons: 0,
	armor: 0,
} as const;

const STARTER_CAT_COUNT = 20;

function defaultRoleXp(cat: CatRow): {
	hunter: number;
	architect: number;
	ritualist: number;
	warrior: number;
} {
	return { ...DEFAULT_ROLE_XP, ...(cat.roleXp ?? {}) };
}

function randomStat(min: number, max: number): number {
	return min + Math.floor(Math.random() * (max - min + 1));
}

/** The role-experience track a job kind trains, if any. */
function tradeForJob(kind: JobKind): keyof RoleXpJson | null {
	if (kind === "hunt_expedition" || kind === "leader_plan_hunt") {
		return "hunter";
	}
	if (kind === "build_house" || kind === "leader_plan_house") {
		return "architect";
	}
	if (kind === "ritual") {
		return "ritualist";
	}
	return null;
}

/**
 * Duration scaling for a cat taking on a job, folding in life stage (elders and
 * young cats are slower than adults) and trade experience (a seasoned hunter or
 * architect finishes quicker). Returns 1 for unassigned/player jobs. Clamped so
 * neither effect can make a job absurdly long or instant.
 */
function capabilityDurationFactor(cat: CatRow | null, kind: JobKind): number {
	if (!cat) {
		return 1;
	}
	const effectiveness = stageWorkEffectiveness(getLifeStage(cat.ageHours ?? 0));
	if (effectiveness <= 0) {
		return 1;
	}
	const trade = tradeForJob(kind);
	const xp = trade ? (defaultRoleXp(cat)[trade] ?? 0) : 0;
	const factor = tradeSpeedMultiplier(xp) / effectiveness;
	return Math.max(0.4, Math.min(2.5, factor));
}

function starterNames(): string[] {
	const names = ["Whiskers", "Shadow", "Luna", "Max", "Bella"];
	const seen = new Set(names);

	// Fill out the founding population with warrior-style names; walk the
	// seed until we have enough unique ones.
	let seed = 424_242;
	while (names.length < STARTER_CAT_COUNT) {
		const name = generateName(seed);
		seed += 1;
		if (seen.has(name)) {
			continue;
		}
		seen.add(name);
		names.push(name);
	}

	return names;
}

/**
 * Colony-local resting spots inside the village clearing — rings 2 and 3
 * around the shrine, past the ring-1 building sites.
 */
function starterCatSpot(index: number): { x: number; y: number } {
	const spots = [...ringCells(2), ...ringCells(3)];
	return spots[index % spots.length];
}

/**
 * Founding age for a starter cat, in game-hours. Spread *evenly* across a young
 * band (6-30h) so the colony opens with a working-age population — no kittens
 * that can't work, and none near the 48h old-age cliff. Elders, births and
 * deaths all emerge from the simulation over time.
 *
 * The even spread is load-bearing for survival: cats age on a shared clock and
 * face a hard old-age mortality cliff at {@link getDeathChance}'s 48h threshold,
 * so a roster bunched into a few ages would all cross that cliff together and
 * wipe the colony in a single die-off wave. Fanning the founders across the band
 * staggers their deaths into a steady trickle the birth rate can replace. (The
 * previous `12 + (i*17)%34` collapsed to just two ages — 12 and 29 — because
 * 17*2 = 34, re-creating exactly the cohort cliff it meant to avoid.)
 *
 * The band is kept *young* (top at 30h, not 44h) so every founder has a long
 * adult breeding window (24-48h) ahead of it. An unaided colony must breed a
 * replacement generation before its founders age out; starting them close to the
 * cliff cut that window short and let the roster die faster than it could be
 * replaced.
 */
const STARTER_AGE_MIN_HOURS = 6;
const STARTER_AGE_MAX_HOURS = 30;
function starterAgeHours(index: number): number {
	const span = STARTER_AGE_MAX_HOURS - STARTER_AGE_MIN_HOURS;
	const denom = Math.max(1, STARTER_CAT_COUNT - 1);
	return STARTER_AGE_MIN_HOURS + Math.round((index / denom) * span);
}

function createStarterCats(db: GameDb, colonyId: string) {
	const names = starterNames();
	for (let i = 0; i < STARTER_CAT_COUNT; i += 1) {
		const spot = starterCatSpot(i);
		db.insert(cats)
			.values({
				_id: nanoid(),
				colonyId,
				name: names[i] ?? `Cat ${i + 1}`,
				parentIds: [null, null],
				birthTime: Date.now(),
				ageHours: starterAgeHours(i),
				pregnancyDueAgeHours: null,
				pregnancyMateId: null,
				deathTime: null,
				stats: {
					attack: randomStat(30, 60),
					defense: randomStat(30, 60),
					hunting: randomStat(30, 60),
					medicine: randomStat(20, 50),
					cleaning: randomStat(25, 55),
					building: randomStat(20, 50),
					leadership: randomStat(20, 60),
					vision: randomStat(30, 60),
				},
				needs: { hunger: 100, thirst: 100, rest: 100, health: 100 },
				currentTask: null,
				position: { map: "colony", x: spot.x, y: spot.y },
				isPregnant: false,
				pregnancyDueTime: null,
				spriteParams: traitsToSpriteParams(inheritTraits(null, null)) as Record<
					string,
					unknown
				>,
				specialization: null,
				roleXp: { ...DEFAULT_ROLE_XP },
			})
			.run();
	}
}

/**
 * Hunt yield for one expedition, folding the base reward together with the
 * hunter's life stage (young/elder haul less) and trade experience (a seasoned
 * hunter hauls more, with diminishing returns).
 */
function huntYieldFor(
	cat: CatRow,
	hunterXp: number,
	upgrades: UpgradeLevels,
	/** Upgrade-tree `huntYieldMult` (Basic Tools, etc.); default 1. */
	huntYieldMult = 1,
): number {
	const base = getHuntReward(
		cat.stats.hunting,
		cat.specialization ?? null,
		hunterXp,
		upgrades,
	);
	const stageMult = stageWorkEffectiveness(getLifeStage(cat.ageHours ?? 0));
	const yieldMult = tradeYieldMultiplier(hunterXp);
	return Math.max(
		1,
		Math.floor(base * stageMult * yieldMult * Math.max(0, huntYieldMult)),
	);
}

/**
 * Retire a cat that has died of old age: cancel any job it was on, vacate its
 * workplace, and mark it dead. Keeps the job/assignment state clean so no
 * expedition or workshop is left pointing at a corpse.
 */
function retireCat(db: GameDb, colonyId: string, cat: CatRow, now: number) {
	db.update(jobs)
		.set({ status: "cancelled", completedAt: now })
		.where(and(eq(jobs.assignedCatId, cat._id), eq(jobs.status, "active")))
		.run();
	db.update(jobs)
		.set({ status: "cancelled", completedAt: now })
		.where(and(eq(jobs.assignedCatId, cat._id), eq(jobs.status, "queued")))
		.run();

	db.update(cats)
		.set({
			deathTime: now,
			currentTask: null,
			carrying: null,
			assignedBuildingId: null,
			destination: null,
			activity: "idle",
			isPregnant: false,
			pregnancyDueAgeHours: null,
			pregnancyMateId: null,
		})
		.where(eq(cats._id, cat._id))
		.run();

	logEvent(
		db,
		colonyId,
		"death",
		`${cat.name} died peacefully of old age.`,
		[cat._id],
		{ cause: "old_age" },
	);
}

/**
 * Pick a co-parent for a conceiving cat: another eligible adult, preferring one
 * with the same specialization so lineages of a trade concentrate, then the
 * strongest available. Deterministic — no RNG. Returns null if no partner.
 */
function pickMate(candidates: CatRow[], cat: CatRow): CatRow | null {
	const others = candidates.filter((c) => c._id !== cat._id);
	if (others.length === 0) {
		return null;
	}
	const sameTrade = cat.specialization
		? others.filter((c) => c.specialization === cat.specialization)
		: [];
	const pool = sameTrade.length > 0 ? sameTrade : others;
	return [...pool].sort(
		(a, b) =>
			b.stats.leadership +
			b.stats.hunting +
			b.stats.building -
			(a.stats.leadership + a.stats.hunting + a.stats.building),
	)[0];
}

/**
 * Birth a kitten to a pregnant mother. Coat and stats are inherited from both
 * parents (biased toward their strengths, so born hunters beget hunters), the
 * mother's pregnancy is cleared, and a birth notice is logged.
 */
function birthKitten(
	db: GameDb,
	colonyId: string,
	mother: CatRow,
	now: number,
	roll: () => number,
) {
	const father = mother.pregnancyMateId
		? (db
				.select()
				.from(cats)
				.where(
					and(eq(cats._id, mother.pregnancyMateId), isNull(cats.deathTime)),
				)
				.get() ?? null)
		: null;

	const motherTraits: GeneticTraits | null = extractGeneticTraits(
		(mother.spriteParams ?? null) as unknown as CatSpriteParams | null,
	);
	const fatherTraits: GeneticTraits | null = father
		? extractGeneticTraits(
				(father.spriteParams ?? null) as unknown as CatSpriteParams | null,
			)
		: null;

	const kittenStats = inheritStats(
		mother.stats,
		father ? father.stats : null,
		roll,
	);
	const kittenName = generateName(Math.floor(roll() * 1_000_000_000));

	db.insert(cats)
		.values({
			_id: nanoid(),
			colonyId,
			name: kittenName,
			parentIds: [mother._id, father?._id ?? null],
			birthTime: now,
			ageHours: 0,
			pregnancyDueAgeHours: null,
			pregnancyMateId: null,
			deathTime: null,
			stats: kittenStats,
			needs: { hunger: 100, thirst: 100, rest: 100, health: 100 },
			currentTask: null,
			position: { ...mother.position },
			isPregnant: false,
			spriteParams: traitsToSpriteParams(
				inheritTraits(motherTraits, fatherTraits),
			) as Record<string, unknown>,
			specialization: null,
			roleXp: { ...DEFAULT_ROLE_XP },
		})
		.run();

	db.update(cats)
		.set({
			isPregnant: false,
			pregnancyDueTime: null,
			pregnancyDueAgeHours: null,
			pregnancyMateId: null,
		})
		.where(eq(cats._id, mother._id))
		.run();

	logEvent(
		db,
		colonyId,
		"birth",
		father
			? `${kittenName} was born to ${mother.name} and ${father.name}.`
			: `${kittenName} was born to ${mother.name}.`,
		father ? [mother._id, father._id] : [mother._id],
		{ motherId: mother._id, fatherId: father?._id ?? null },
	);
}

interface LifeSimContext {
	aliveCats: CatRow[];
	elapsedGameHours: number;
	housingCap: number;
	foodRatio: number;
	waterRatio: number;
	/** Absolute stored food/water, for the per-capita breeding fallback. */
	food: number;
	water: number;
	/** Seed for the isolated life-sim roll chain, or null for unseeded. */
	lifeSeed: number | null;
}

/**
 * The population loop: age every cat, retire elders that lose the old-age roll,
 * grow the leader's leadership with tenure, deliver kittens whose gestation has
 * finished, and — while the village is fed, watered and has spare beds — pair
 * adults into new pregnancies. All randomness runs on a forked chain so the
 * seeded policy/movement chains stay byte-stable.
 */
function runLifeSimulation(
	db: GameDb,
	colony: ColonyRow,
	ctx: LifeSimContext,
): void {
	const { elapsedGameHours } = ctx;
	if (elapsedGameHours <= 0) {
		return;
	}
	const now = Date.now();

	let lifeSeed = ctx.lifeSeed;
	const nextLifeRoll = () => {
		if (lifeSeed === null) {
			return Math.random();
		}
		const roll = rollSeeded(lifeSeed);
		lifeSeed = roll.nextSeed;
		return roll.value;
	};

	// 1. Aging, old-age mortality, leadership tenure, life-stage milestones.
	for (const cat of ctx.aliveCats) {
		const prevAge = cat.ageHours ?? 0;
		const newAge = prevAge + elapsedGameHours;
		const isLeader = cat._id === colony.leaderId;

		const deathChance = oldAgeDeathProbability(
			newAge,
			isLeader,
			elapsedGameHours,
		);
		if (deathChance > 0 && nextLifeRoll() < deathChance) {
			retireCat(db, colony._id, cat, now);
			continue;
		}

		db.update(cats)
			.set({
				ageHours: newAge,
				...(isLeader
					? {
							stats: {
								...cat.stats,
								leadership: leadershipAfterTenure(
									cat.stats.leadership,
									elapsedGameHours,
								),
							},
						}
					: {}),
			})
			.where(eq(cats._id, cat._id))
			.run();

		const transition = detectLifeStageTransition(prevAge, newAge);
		if (transition) {
			const announcement = generateMilestoneAnnouncement(
				transition,
				cat.name,
				cat.stats,
			);
			logEvent(
				db,
				colony._id,
				"milestone",
				`${announcement.headline} — ${announcement.body}`,
				[cat._id],
				{ from: transition.from, to: transition.to },
			);
		}
	}

	// 2. Births: any mother whose gestation (tracked in her own age) is up.
	const postAge = getAliveCats(db, colony._id);
	for (const cat of postAge) {
		if (
			cat.isPregnant &&
			cat.pregnancyDueAgeHours != null &&
			(cat.ageHours ?? 0) >= cat.pregnancyDueAgeHours
		) {
			birthKitten(db, colony._id, cat, now, nextLifeRoll);
		}
	}

	// 3. Conceptions: adults pair off while the colony is healthy and has room.
	// The housing headroom check is the soft population cap — growth tracks the
	// village's shelter instead of running away.
	const roster = getAliveCats(db, colony._id);
	const blessings = colony.resources.blessings ?? 0;
	let pregnantCount = roster.filter((c) => c.isPregnant).length;
	const population = roster.length;
	const adults = roster.filter(
		(c) => !c.isPregnant && getLifeStage(c.ageHours ?? 0) === "adult",
	);
	for (const cat of adults) {
		if (
			!colonyCanBreed({
				foodRatio: ctx.foodRatio,
				waterRatio: ctx.waterRatio,
				food: ctx.food,
				water: ctx.water,
				population: population + pregnantCount,
				housingCapacity: ctx.housingCap,
			})
		) {
			break;
		}
		const chance = conceptionProbability(
			cat.specialization ?? null,
			blessings,
			elapsedGameHours,
		);
		if (nextLifeRoll() >= chance) {
			continue;
		}
		const mate = pickMate(adults, cat);
		db.update(cats)
			.set({
				isPregnant: true,
				pregnancyDueAgeHours: (cat.ageHours ?? 0) + GESTATION_GAME_HOURS,
				pregnancyDueTime: now + GESTATION_GAME_HOURS * 3_600_000,
				pregnancyMateId: mate?._id ?? null,
			})
			.where(eq(cats._id, cat._id))
			.run();
		pregnantCount += 1;
		logEvent(
			db,
			colony._id,
			"breeding",
			mate
				? `${cat.name} and ${mate.name} are expecting a litter.`
				: `${cat.name} is expecting a litter.`,
			mate ? [cat._id, mate._id] : [cat._id],
		);
	}
}

/**
 * Grace window separating a genuine newborn (ageHours 0, just created) from a
 * cat that predates the ageHours column (ageHours 0 but born long ago). Legacy
 * rows would otherwise read as kittens who can't work and stall the colony.
 */
const LEGACY_AGE_GRACE_MS = 5 * 60 * 1000;

/**
 * One-time backfill for colonies migrated before cats carried an age: any cat
 * still at ageHours 0 but born more than the grace window ago is a legacy row,
 * so seed it a working-age adult age. Idempotent — once seeded (> 0) it's never
 * touched again, and true newborns inside the grace window are left alone.
 */
function backfillLegacyAges(db: GameDb, aliveCats: CatRow[]) {
	const now = Date.now();
	let index = 0;
	for (const cat of aliveCats) {
		if (cat.ageHours !== 0 || now - cat.birthTime < LEGACY_AGE_GRACE_MS) {
			continue;
		}
		db.update(cats)
			.set({ ageHours: starterAgeHours(index) })
			.where(eq(cats._id, cat._id))
			.run();
		index += 1;
	}
}

function getGlobalColony(db: GameDb): ColonyRow | undefined {
	return db.select().from(colonies).where(eq(colonies.isGlobal, true)).get();
}

function getColony(db: GameDb, colonyId: string): ColonyRow {
	const colony = db
		.select()
		.from(colonies)
		.where(eq(colonies._id, colonyId))
		.get();
	if (!colony) {
		throw new Error("Colony not found");
	}
	return colony;
}

function getUpgradeRows(db: GameDb, colonyId: string) {
	return db
		.select()
		.from(globalUpgrades)
		.where(eq(globalUpgrades.colonyId, colonyId))
		.all();
}

function upgradesToLevels(
	rows: Array<{ key: string; level: number }>,
): UpgradeLevels {
	const map: Record<string, number> = {};
	for (const row of rows) {
		map[row.key] = row.level;
	}
	return {
		click_power: map.click_power ?? 0,
		supply_speed: map.supply_speed ?? 0,
		hunt_mastery: map.hunt_mastery ?? 0,
		build_mastery: map.build_mastery ?? 0,
		ritual_mastery: map.ritual_mastery ?? 0,
		resilience: map.resilience ?? 0,
	};
}

/** Deserialize the colony's upgrade-tree progress (fresh tree on null). */
function getUpgradeTree(colony: ColonyRow): UpgradeTreeState {
	return deserializeUpgradeTreeState(colony.upgradeTree ?? null);
}

/** Persist upgrade-tree progress back onto the colony row. */
function saveUpgradeTree(
	db: GameDb,
	colonyId: string,
	state: UpgradeTreeState,
): void {
	db.update(colonies)
		.set({ upgradeTree: serializeUpgradeTreeState(state) })
		.where(eq(colonies._id, colonyId))
		.run();
}

function ensureGlobalUpgrades(db: GameDb, colonyId: string) {
	const existing = getUpgradeRows(db, colonyId);
	const existingKeys = new Set(existing.map((u) => u.key));

	for (const upgrade of UPGRADE_DEFAULTS) {
		if (existingKeys.has(upgrade.key)) {
			continue;
		}

		db.insert(globalUpgrades)
			.values({
				_id: nanoid(),
				colonyId,
				key: upgrade.key,
				level: 0,
				maxLevel: upgrade.maxLevel,
				baseCost: upgrade.baseCost,
				description: upgrade.description,
			})
			.run();
	}
}

export function ensureGlobalColony(db: GameDb): ColonyRow {
	const now = Date.now();
	let colony = getGlobalColony(db);

	if (!colony) {
		const colonyId = nanoid();
		db.insert(colonies)
			.values({
				_id: colonyId,
				name: "Global Cat Colony",
				leaderId: null,
				status: "starting",
				resources: { ...STARTING_RESOURCES },
				gridSize: 3,
				createdAt: now,
				lastTick: now,
				lastAttack: now,
				worldSeed: now,
				isGlobal: true,
				runNumber: 1,
				runStartedAt: now,
				lastPlayerActivityAt: now,
				lastResetAt: now,
				automationTier: 0,
				globalUpgradePoints: 0,
				upgradeTree: serializeUpgradeTreeState(createUpgradeTreeState()),
				ritualRequestedAt: null,
				criticalSince: null,
				claimedTiles: foundingClaimedTiles(),
				threatPressure: 0,
				lastRaidAt: null,
				activeRaidId: null,
				raidClicks: 0,
				testTimeScale: 1,
				testResourceDecayMultiplier: 1,
				testResilienceHoursOverride: null,
				testCriticalMsOverride: 5 * 60 * 1000,
				testRngSeed: null,
			})
			.run();

		createStarterCats(db, colonyId);
		ensureGlobalUpgrades(db, colonyId);

		colony = getColony(db, colonyId);
	} else {
		ensureGlobalUpgrades(db, colony._id);
	}

	const aliveCats = getAliveCats(db, colony._id);
	if (aliveCats.length === 0) {
		createStarterCats(db, colony._id);
	} else {
		backfillLegacyAges(db, aliveCats);
	}

	ensureShrineAndWorld(db, colony._id);
	colony = ensureClaimedTiles(db, getColony(db, colony._id));

	return colony;
}

/**
 * Interior radius (Chebyshev) of the founding village. The auto-fence sits one
 * tile beyond, at the historical ring radius (4), so a fresh colony's footprint
 * and fence match the old square exactly before it grows organically.
 */
const VILLAGE_START_RADIUS = 3;

/** The founding claimed footprint: the interior square around the anchor. */
function foundingClaimedTiles(): VillagePos[] {
	const out: VillagePos[] = [];
	for (let dy = -VILLAGE_START_RADIUS; dy <= VILLAGE_START_RADIUS; dy++) {
		for (let dx = -VILLAGE_START_RADIUS; dx <= VILLAGE_START_RADIUS; dx++) {
			out.push({ x: VILLAGE_ANCHOR.x + dx, y: VILLAGE_ANCHOR.y + dy });
		}
	}
	return out;
}

/**
 * The colony's claimed organic village area (lib/game/villageArea.ts), backfilled
 * from the founding square for legacy rows so the fence/clearing/walkability keep
 * working before the first save writes `claimedTiles`.
 */
function getClaimedArea(colony: ColonyRow): VillageArea {
	return villageFromTiles(colony.claimedTiles ?? foundingClaimedTiles());
}

/** The village gate for a claimed area — opens onto the busiest worn corridor,
 * else the historical south side. */
function claimedGate(area: VillageArea): GatePlacement | null {
	return villageGate(area);
}

function ensureClaimedTiles(db: GameDb, colony: ColonyRow): ColonyRow {
	if (colony.claimedTiles) {
		return colony;
	}
	const claimedTiles = foundingClaimedTiles();
	db.update(colonies)
		.set({ claimedTiles })
		.where(eq(colonies._id, colony._id))
		.run();
	return { ...colony, claimedTiles };
}

export function getVillagePayload(db: GameDb, colonyId: string) {
	const colony = getColony(db, colonyId);
	const colonyBuildings = db
		.select()
		.from(buildings)
		.where(eq(buildings.colonyId, colonyId))
		.all();
	const area = getClaimedArea(colony);
	return {
		villageRadius: villageRingRadius(colonyBuildings.length),
		claimedTiles: villageToTiles(area),
		villageGate: claimedGate(area),
	};
}

function worldToVillageLocal(pos: VillagePos): VillagePos {
	return { x: pos.x - VILLAGE_ANCHOR.x, y: pos.y - VILLAGE_ANCHOR.y };
}

function nextClaimedBuildingSite(
	area: VillageArea,
	occupied: VillagePos[],
	roll: number,
	isBlocked?: (world: VillagePos) => boolean,
): VillagePos | null {
	const taken = new Set(
		occupied.map((p) => `${VILLAGE_ANCHOR.x + p.x},${VILLAGE_ANCHOR.y + p.y}`),
	);
	taken.add(`${VILLAGE_ANCHOR.x},${VILLAGE_ANCHOR.y}`);
	const free = villageToTiles(area).filter(
		(pos) => !taken.has(`${pos.x},${pos.y}`) && !(isBlocked?.(pos) ?? false),
	);
	if (free.length === 0) {
		return null;
	}
	const clamped = Math.min(Math.max(roll, 0), 0.999999);
	return worldToVillageLocal(free[Math.floor(clamped * free.length)]);
}

function isAdjacentToArea(pos: VillagePos, area: VillageArea): boolean {
	return (
		isInsideVillage({ x: pos.x + 1, y: pos.y }, area) ||
		isInsideVillage({ x: pos.x - 1, y: pos.y }, area) ||
		isInsideVillage({ x: pos.x, y: pos.y + 1 }, area) ||
		isInsideVillage({ x: pos.x, y: pos.y - 1 }, area)
	);
}

function chunkCoordForTile(n: number): number {
	return Math.floor(n / 12);
}

function getTileAt(
	db: GameDb,
	colonyId: string,
	pos: VillagePos,
): WorldTileRow | undefined {
	return db
		.select()
		.from(worldTiles)
		.where(
			and(
				eq(worldTiles.colonyId, colonyId),
				eq(worldTiles.x, pos.x),
				eq(worldTiles.y, pos.y),
			),
		)
		.get();
}

function clearClaimedTile(
	db: GameDb,
	colonyId: string,
	pos: VillagePos,
	now: number,
) {
	ensureChunk(db, colonyId, chunkCoordForTile(pos.x), chunkCoordForTile(pos.y));
	const tile = getTileAt(db, colonyId, pos);
	if (!tile || !isForestType(tile.type)) {
		return;
	}
	db.update(worldTiles)
		.set({
			type: "field",
			resources: { ...tile.resources, food: 0, herbs: 0 },
			maxResources: {
				...tile.maxResources,
				food: CHOPPED_FOREST_FOOD_CAP,
			},
			lastDepleted: now,
		})
		.where(eq(worldTiles._id, tile._id))
		.run();
}

function ensureShrineAndWorld(db: GameDb, colonyId: string) {
	const shrine = db
		.select({ _id: buildings._id })
		.from(buildings)
		.where(and(eq(buildings.colonyId, colonyId), eq(buildings.type, "shrine")))
		.limit(1)
		.get();

	if (shrine) {
		return;
	}

	db.insert(buildings)
		.values({
			_id: nanoid(),
			colonyId,
			type: "shrine",
			level: 1,
			position: { ...SHRINE_LOCAL },
			constructionProgress: 100,
		})
		.run();

	// Founding village around the shrine: five dens plus a stocked general
	// storage, all pre-built. Four of the dens are raised longhouses (level 2,
	// 4 beds); the fifth is an ordinary den (2 beds). With the shrine's 4 beds
	// that shelters 4*4 + 2 + 4 = 22 cats. Fixed rolls keep the layout
	// deterministic while looking organic.
	//
	// This headroom is load-bearing for unaided survival. Conception is blocked
	// while population >= housing capacity (see colonyCanBreed), so a cap at or
	// below the 20-cat founding roster left the founders unable to breed during
	// their adult window (24-48h): by the time old-age deaths dropped the
	// population below the cap, the survivors were already elders, no replacement
	// generation ever formed, and the colony collapsed of old age. Seating the
	// cap *above* the roster lets the founders breed immediately, so a
	// replacement generation matures before they age out. The extra beds come
	// from den *levels* rather than more den *buildings* so the founding
	// footprint — and the fence ring / walkability grid derived from the building
	// count — is unchanged.
	const starterBuildings: Array<{
		type: "den" | "food_storage";
		roll: number;
		level: number;
	}> = [
		{ type: "den", roll: 0.05, level: 2 },
		{ type: "den", roll: 0.3, level: 2 },
		{ type: "den", roll: 0.55, level: 2 },
		{ type: "den", roll: 0.8, level: 2 },
		{ type: "den", roll: 0.95, level: 1 },
		{ type: "food_storage", roll: 0.4, level: 1 },
	];

	const occupied: Array<{ x: number; y: number }> = [];
	for (const starter of starterBuildings) {
		const site = nextBuildingSite(occupied, starter.roll);
		if (!site) {
			break;
		}
		occupied.push(site);
		db.insert(buildings)
			.values({
				_id: nanoid(),
				colonyId,
				type: starter.type,
				level: starter.level,
				position: site,
				constructionProgress: 100,
			})
			.run();
	}

	// First run for this colony: seed the starting 3x3 world chunks
	// (idempotent — skips chunks that already exist).
	initializeWorldMap(db, colonyId);

	// Seed the organic village footprint (lib/game/villageArea.ts) with the
	// founding square. From here it grows one tile at a time; the fence, clearing,
	// building sites and walkability all derive from this set.
	db.update(colonies)
		.set({ claimedTiles: foundingClaimedTiles() })
		.where(eq(colonies._id, colonyId))
		.run();
}

function getAliveCats(db: GameDb, colonyId: string): CatRow[] {
	return db
		.select()
		.from(cats)
		.where(and(eq(cats.colonyId, colonyId), isNull(cats.deathTime)))
		.all();
}

function getJobsByStatus(
	db: GameDb,
	colonyId: string,
	status: JobRow["status"],
): JobRow[] {
	return db
		.select()
		.from(jobs)
		.where(and(eq(jobs.colonyId, colonyId), eq(jobs.status, status)))
		.all();
}

function pendingJobCatIds(db: GameDb, colonyId: string): Set<string> {
	return new Set(
		db
			.select({ assignedCatId: jobs.assignedCatId })
			.from(jobs)
			.where(
				and(
					eq(jobs.colonyId, colonyId),
					inArray(jobs.status, ["active", "queued"]),
				),
			)
			.all()
			.map((job) => job.assignedCatId)
			.filter((id): id is string => typeof id === "string" && id.length > 0),
	);
}

function canTakeNewJob(cat: CatRow, busyIds: Set<string>): boolean {
	return (
		canWork(getLifeStage(cat.ageHours ?? 0)) &&
		!busyIds.has(cat._id) &&
		!cat.assignedBuildingId &&
		(cat.activity ?? "idle") === "idle" &&
		!cat.currentTask &&
		!cat.carrying &&
		!cat.destination
	);
}

function chooseLeader(db: GameDb, colonyId: string): CatRow | null {
	const aliveCats = getAliveCats(db, colonyId);
	if (aliveCats.length === 0) {
		return null;
	}

	let best = aliveCats[0];
	for (const cat of aliveCats) {
		if (cat.stats.leadership > best.stats.leadership) {
			best = cat;
		}
	}
	return best;
}

const SPECIALIZATION_STAT: Record<
	Exclude<CatSpecialization, null>,
	keyof CatStats
> = {
	hunter: "hunting",
	architect: "building",
	ritualist: "leadership",
	warrior: "attack",
};

function selectBestCat(
	db: GameDb,
	colonyId: string,
	specialization: CatSpecialization,
): CatRow | null {
	const busyIds = pendingJobCatIds(db, colonyId);
	const availableCats = getAliveCats(db, colonyId).filter((cat) =>
		canTakeNewJob(cat, busyIds),
	);
	if (availableCats.length === 0) {
		return null;
	}

	const preferred = availableCats.filter(
		(cat) => (cat.specialization ?? null) === specialization,
	);
	const pool = preferred.length > 0 ? preferred : availableCats;

	const statKey = specialization
		? SPECIALIZATION_STAT[specialization]
		: "leadership";

	let best = pool[0];
	for (const cat of pool) {
		if (cat.stats[statKey] > best.stats[statKey]) {
			best = cat;
		}
	}

	return best;
}

function logEvent(
	db: GameDb,
	colonyId: string,
	type: string,
	message: string,
	involvedCatIds: string[] = [],
	metadata: Record<string, unknown> = {},
) {
	db.insert(events)
		.values({
			_id: nanoid(),
			colonyId,
			catId: involvedCatIds[0] ?? null,
			timestamp: Date.now(),
			type,
			message,
			involvedCatIds,
			metadata,
		})
		.run();
}

function queueJob(
	db: GameDb,
	colonyId: string,
	kind: JobKind,
	requestedByType: "player" | "leader" | "system",
	upgrades: UpgradeLevels,
	runtime: RuntimeConfig,
	requestedByPlayerId: string | null,
	assignedCat: CatRow | null,
	metadata: Record<string, unknown> = {},
): string {
	const specialization: CatSpecialization = assignedCat?.specialization ?? null;
	const baseDuration = getScaledDurationSeconds(
		kind,
		specialization,
		upgrades,
		runtime.timeScale,
	);
	// Life stage and trade experience of the assigned cat stretch or compress
	// the job: elders and kittens-just-grown work slower, seasoned tradescats
	// work faster.
	const duration = Math.max(
		1,
		Math.round(baseDuration * capabilityDurationFactor(assignedCat, kind)),
	);
	const now = Date.now();

	const jobId = nanoid();
	if (assignedCat?.assignedBuildingId) {
		const assignedBuilding = db
			.select({ type: buildings.type })
			.from(buildings)
			.where(eq(buildings._id, assignedCat.assignedBuildingId))
			.get();
		const isProductionAssignment =
			assignedBuilding?.type === "workshop" ||
			assignedBuilding?.type === "research_hut" ||
			assignedBuilding?.type === "smithy";
		if (isProductionAssignment) {
			db.update(cats)
				.set({ assignedBuildingId: null })
				.where(eq(cats._id, assignedCat._id))
				.run();
		}
	}
	db.insert(jobs)
		.values({
			_id: jobId,
			colonyId,
			kind,
			status: "queued",
			requestedByType,
			requestedByPlayerId: requestedByPlayerId ?? null,
			assignedCatId: assignedCat?._id ?? null,
			baseDurationSec: duration,
			speedMultiplier: 1,
			yieldMultiplier: 1,
			clickTimeReducedSec: 0,
			createdAt: now,
			startedAt: now,
			endsAt: now + duration * 1000,
			metadata,
		})
		.run();

	logEvent(
		db,
		colonyId,
		"job_queued",
		`Queued ${kind.replace(/_/g, " ")}`,
		assignedCat ? [assignedCat._id] : [],
		{ jobId, kind },
	);

	return jobId;
}

function queuePlannedHouseJobs(
	db: GameDb,
	colonyId: string,
	patchedResources: { water: number; materials: number },
	policy: { houseWaterRequired: number; houseMaterialsRequired: number },
	activeOrQueuedJobs: Array<{
		kind: JobKind;
		metadata?: Record<string, unknown>;
	}>,
	upgrades: UpgradeLevels,
	runtime: RuntimeConfig,
	policyGate?: () => boolean,
) {
	const plannedJobs = planHousePipeline({
		resources: {
			water: patchedResources.water,
			materials: patchedResources.materials,
		},
		activeOrQueuedJobs,
		waterRequired: policy.houseWaterRequired,
		materialsRequired: policy.houseMaterialsRequired,
	});

	for (const planned of plannedJobs) {
		if (policyGate && !policyGate()) {
			continue;
		}
		const architect =
			planned.kind === "build_house"
				? selectBestCat(db, colonyId, "architect")
				: null;
		if (planned.kind === "build_house" && !architect) {
			continue;
		}
		queueJob(
			db,
			colonyId,
			planned.kind,
			"leader",
			upgrades,
			runtime,
			null,
			architect,
			planned.metadata,
		);
		activeOrQueuedJobs.push({
			kind: planned.kind,
			metadata: planned.metadata,
		});
	}
}

function resetGlobalRun(db: GameDb, colony: ColonyRow, reason: string) {
	const now = Date.now();

	const activePlayers = countOnlinePlayers(db, now);

	db.insert(runHistory)
		.values({
			_id: nanoid(),
			colonyId: colony._id,
			runNumber: colony.runNumber ?? 1,
			startedAt: colony.runStartedAt ?? colony.createdAt,
			endedAt: now,
			durationSec: Math.max(
				1,
				Math.floor((now - (colony.runStartedAt ?? colony.createdAt)) / 1000),
			),
			reason,
			finalResources: colony.resources,
			activePlayers,
		})
		.run();

	db.delete(jobs).where(eq(jobs.colonyId, colony._id)).run();
	// A collapse also scatters any in-progress raid and resets threat pressure.
	db.delete(raiders).where(eq(raiders.colonyId, colony._id)).run();

	// Half-finished construction dies with the run: the jobs driving it are
	// gone (above), so any building still under construction would otherwise
	// linger as an unbuildable orphan. Completed buildings — and the shrine —
	// survive as the standing village. Crucially, the WORLD is untouched here:
	// the colony row keeps its id and worldSeed, and worldTiles (terrain,
	// explored pathWear, revealed fog, depletion) are never deleted, so a new
	// run continues on exactly the same map. See serverWorldPersistence tests.
	db.delete(buildings)
		.where(
			and(
				eq(buildings.colonyId, colony._id),
				lt(buildings.constructionProgress, 100),
			),
		)
		.run();

	// A collapse dissolves any open polls — the new run elects fresh.
	db.update(elections)
		.set({ status: "resolved" })
		.where(
			and(eq(elections.colonyId, colony._id), eq(elections.status, "open")),
		)
		.run();

	const aliveCats = getAliveCats(db, colony._id);
	if (aliveCats.length === 0) {
		createStarterCats(db, colony._id);
	} else {
		aliveCats.forEach((cat, index) => {
			const spot = starterCatSpot(index);
			db.update(cats)
				.set({
					needs: { hunger: 100, thirst: 100, rest: 100, health: 100 },
					currentTask: null,
					position: { map: "colony", x: spot.x, y: spot.y },
					destination: null,
					activity: "idle",
				})
				.where(eq(cats._id, cat._id))
				.run();
		});
	}

	db.update(colonies)
		.set({
			status: "starting",
			resources: {
				...STARTING_RESOURCES,
				blessings: colony.resources.blessings,
			},
			runNumber: (colony.runNumber ?? 1) + 1,
			runStartedAt: now,
			lastResetAt: now,
			lastTick: now,
			criticalSince: null,
			ritualRequestedAt: null,
			threatPressure: 0,
			activeRaidId: null,
			raidClicks: 0,
			lastRaidAt: null,
			testRngSeed: colony.testRngSeed ?? null,
		})
		.where(eq(colonies._id, colony._id))
		.run();

	logEvent(
		db,
		colony._id,
		"run_reset",
		`The colony collapsed and started run ${(colony.runNumber ?? 1) + 1}.`,
		[],
		{ reason },
	);
}

/**
 * Player-requested construction of an unlockable building (Phase 7).
 * Reuses the build_house pipeline with a buildingType override.
 */
export function planBuilding(
	db: GameDb,
	args: {
		sessionId: string;
		nickname: string;
		type:
			| "workshop"
			| "field"
			| "research_hut"
			| "school"
			| "smithy"
			| "barracks";
	},
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = ensureGlobalColony(tx);
		const now = Date.now();
		upsertPlayer(tx, args.sessionId, args.nickname, now);

		const colonyBuildings = tx
			.select()
			.from(buildings)
			.where(eq(buildings.colonyId, colony._id))
			.all();
		const level = villageLevel(colonyBuildings);
		if (args.type === "workshop" && !workshopUnlocked(level)) {
			throw new Error("Workshops unlock at village level 2");
		}
		if (args.type === "field" && !fieldUnlocked(level)) {
			throw new Error("Fields unlock at village level 4");
		}
		// Era gating: research_hut, school, smithy and barracks are unlocked by
		// their tree nodes (whose ids match the building type). Reject until
		// owned.
		if (
			args.type === "research_hut" ||
			args.type === "school" ||
			args.type === "smithy" ||
			args.type === "barracks"
		) {
			const tree = getUpgradeTree(colony);
			if (!isOwned(tree, args.type)) {
				const node = getNode(args.type);
				throw new Error(
					`${node?.name ?? args.type} must be researched or granted by the gods first`,
				);
			}
		}

		const pending = [
			...getJobsByStatus(tx, colony._id, "active"),
			...getJobsByStatus(tx, colony._id, "queued"),
		].some(
			(job) =>
				job.kind === "build_house" &&
				(job.metadata as Record<string, unknown> | null)?.buildingType ===
					args.type,
		);
		if (pending) {
			return { ok: false, reason: "already_in_progress" };
		}

		const upgrades = upgradesToLevels(getUpgradeRows(tx, colony._id));
		const runtime = getRuntimeConfig(colony);
		const architect = selectBestCat(tx, colony._id, "architect");
		if (!architect) {
			return { ok: false, reason: "no_available_worker" };
		}
		const jobId = queueJob(
			tx,
			colony._id,
			"build_house",
			"player",
			upgrades,
			runtime,
			null,
			architect,
			{ phase: "construct_house", buildingType: args.type },
		);

		tx.update(colonies)
			.set({ lastPlayerActivityAt: now })
			.where(eq(colonies._id, colony._id))
			.run();

		return { ok: true, jobId };
	});
}

/** Assign (or unassign with buildingId null) a cat to a workshop. */
export function assignWorker(
	db: GameDb,
	args: {
		sessionId: string;
		nickname: string;
		catId: string;
		buildingId: string | null;
	},
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = ensureGlobalColony(tx);
		const now = Date.now();
		upsertPlayer(tx, args.sessionId, args.nickname, now);

		const cat = tx
			.select()
			.from(cats)
			.where(
				and(
					eq(cats._id, args.catId),
					eq(cats.colonyId, colony._id),
					isNull(cats.deathTime),
				),
			)
			.get();
		if (!cat) {
			throw new Error("That cat is not available");
		}

		if (args.buildingId === null) {
			tx.update(cats)
				.set({ assignedBuildingId: null })
				.where(eq(cats._id, cat._id))
				.run();
			return { ok: true };
		}

		const activeOrQueuedJob = tx
			.select({ _id: jobs._id })
			.from(jobs)
			.where(
				and(
					eq(jobs.assignedCatId, cat._id),
					inArray(jobs.status, ["active", "queued"]),
				),
			)
			.limit(1)
			.get();
		if (
			activeOrQueuedJob ||
			(cat.activity ?? "idle") !== "idle" ||
			cat.currentTask ||
			cat.carrying ||
			cat.destination
		) {
			throw new Error("That cat is busy");
		}

		const building = tx
			.select()
			.from(buildings)
			.where(
				and(
					eq(buildings._id, args.buildingId),
					eq(buildings.colonyId, colony._id),
				),
			)
			.get();
		if (
			!building ||
			(building.type !== "workshop" &&
				building.type !== "research_hut" &&
				building.type !== "smithy") ||
			building.constructionProgress < 100
		) {
			throw new Error("That building cannot take a worker");
		}

		// One worker per building — displace any current occupant.
		const occupant = getAliveCats(tx, colony._id).find(
			(candidate) => candidate.assignedBuildingId === building._id,
		);
		if (occupant && occupant._id !== cat._id) {
			tx.update(cats)
				.set({ assignedBuildingId: null })
				.where(eq(cats._id, occupant._id))
				.run();
		}

		tx.update(cats)
			.set({ assignedBuildingId: building._id })
			.where(eq(cats._id, cat._id))
			.run();

		tx.update(colonies)
			.set({ lastPlayerActivityAt: now })
			.where(eq(colonies._id, colony._id))
			.run();

		return { ok: true };
	});
}

/** Lay a permanent road along an L-path (x leg then y), 1 material/tile. */
export function buildRoad(
	db: GameDb,
	args: {
		sessionId: string;
		nickname: string;
		a: { x: number; y: number };
		b: { x: number; y: number };
	},
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = ensureGlobalColony(tx);
		const now = Date.now();
		upsertPlayer(tx, args.sessionId, args.nickname, now);

		// Reject non-finite/absurd endpoints BEFORE any loop — NaN/Infinity
		// or huge coords would otherwise spin inside the write transaction
		// and stall the whole game (review finding: DoS).
		const coords = [args.a.x, args.a.y, args.b.x, args.b.y];
		if (!coords.every((c) => Number.isFinite(c) && Math.abs(c) <= 1_000)) {
			throw new Error("Invalid road endpoints");
		}
		const ax = Math.round(args.a.x);
		const ay = Math.round(args.a.y);
		const bx = Math.round(args.b.x);
		const by = Math.round(args.b.y);
		if (Math.abs(bx - ax) + Math.abs(by - ay) > 24) {
			throw new Error("Roads are limited to 24 tiles per build");
		}
		const path: Array<{ x: number; y: number }> = [];
		const xStep = Math.sign(bx - ax);
		for (let x = ax; x !== bx; x += xStep) {
			path.push({ x, y: ay });
		}
		path.push({ x: bx, y: ay });
		const yStep = Math.sign(by - ay);
		for (let y = ay + yStep; yStep !== 0 && y !== by + yStep; y += yStep) {
			path.push({ x: bx, y });
		}
		if (path.length > 24) {
			throw new Error("Roads are limited to 24 tiles per build");
		}
		if (colony.resources.materials < path.length) {
			throw new Error(
				`Not enough materials (${path.length} needed, one per tile)`,
			);
		}

		let paved = 0;
		for (const pos of path) {
			const tile = tx
				.select()
				.from(worldTiles)
				.where(
					and(
						eq(worldTiles.colonyId, colony._id),
						eq(worldTiles.x, pos.x),
						eq(worldTiles.y, pos.y),
					),
				)
				.get();
			if (!tile || tile.type === "river" || tile.overlayFeature === "river") {
				continue;
			}
			tx.update(worldTiles)
				.set({ overlayFeature: "road_built", pathWear: 100 })
				.where(eq(worldTiles._id, tile._id))
				.run();
			paved += 1;
		}

		tx.update(colonies)
			.set({
				resources: {
					...colony.resources,
					materials: colony.resources.materials - paved,
				},
				lastPlayerActivityAt: now,
			})
			.where(eq(colonies._id, colony._id))
			.run();

		logEvent(
			tx,
			colony._id,
			"road_built",
			`A paved road was laid (${paved} tiles).`,
		);
		return { ok: true, paved };
	});
}

export function ensureGlobalState(db: GameDb): string {
	return db.transaction((tx) => ensureGlobalColony(tx as unknown as GameDb))
		._id;
}

export function setTestAcceleration(
	db: GameDb,
	preset: "off" | "fast" | "turbo" | "hyper" | "ludicrous",
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = ensureGlobalColony(tx);

		const config = configForPreset(preset);
		tx.update(colonies)
			.set({
				testTimeScale: config.timeScale,
				testResourceDecayMultiplier: config.resourceDecayMultiplier,
				testResilienceHoursOverride: config.resilienceHoursOverride,
				testCriticalMsOverride: config.criticalMsOverride,
			})
			.where(eq(colonies._id, colony._id))
			.run();
		return { preset };
	});
}

export function setTestRngSeed(db: GameDb, seed: number | null) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = ensureGlobalColony(tx);

		tx.update(colonies)
			.set({
				testRngSeed:
					typeof seed === "number" ? Math.max(1, Math.floor(seed)) : null,
			})
			.where(eq(colonies._id, colony._id))
			.run();

		return { seed };
	});
}

export function advanceTime(db: GameDb, seconds: number) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = ensureGlobalColony(tx);

		const advance = Math.max(1, Math.floor(seconds));
		tx.update(colonies)
			.set({ lastTick: colony.lastTick - advance * 1000 })
			.where(eq(colonies._id, colony._id))
			.run();

		return { advancedSeconds: advance };
	});
}

export function getGlobalDashboard(db: GameDb) {
	const colony = getGlobalColony(db);
	if (!colony) {
		return null;
	}

	const now = Date.now();

	const aliveCats = getAliveCats(db, colony._id);

	const jobsQueued = getJobsByStatus(db, colony._id, "queued");
	const jobsActive = getJobsByStatus(db, colony._id, "active");
	const allJobs = [...jobsActive, ...jobsQueued].sort(
		(a, b) => a.endsAt - b.endsAt,
	);

	const recentEvents = db
		.select()
		.from(events)
		.where(eq(events.colonyId, colony._id))
		.orderBy(desc(events.timestamp))
		.limit(30)
		.all();

	const upgrades = getUpgradeRows(db, colony._id);

	const onlineCount = countOnlinePlayers(db, now);

	const leader = colony.leaderId
		? (aliveCats.find((cat) => cat._id === colony.leaderId) ?? null)
		: null;

	const colonyBuildings = db
		.select()
		.from(buildings)
		.where(eq(buildings.colonyId, colony._id))
		.all();

	// Upgrade-tree state + modifiers so the HUD's caps match the tick's.
	const tree = getUpgradeTree(colony);
	const effects = resolveEffects(tree.ownedNodeIds);
	const caps = storageCapacities(colonyBuildings, effects.storagePerLevelMult);
	const houseCap = housingCapacity(colonyBuildings, effects.housingPerDen);
	const researchHutIds = new Set(
		colonyBuildings
			.filter((b) => b.type === "research_hut" && b.constructionProgress >= 100)
			.map((b) => b._id),
	);
	const researcherCount = aliveCats.filter(
		(cat) =>
			cat.assignedBuildingId && researchHutIds.has(cat.assignedBuildingId),
	).length;
	const nextTarget = nextResearchTarget(tree);

	// Active-raid raiders and the current threat reading for the map + HUD.
	const activeRaiders = colony.activeRaidId
		? db
				.select()
				.from(raiders)
				.where(
					and(
						eq(raiders.colonyId, colony._id),
						eq(raiders.raidId, colony.activeRaidId),
					),
				)
				.all()
				.filter((r) => r.status !== "dead" && r.hp > 0)
		: [];
	const warriorCount = aliveCats.filter(
		(cat) => cat.specialization === "warrior",
	).length;
	const claimedArea = getClaimedArea(colony);

	return {
		now,
		colony,
		leader,
		cats: aliveCats.sort((a, b) => b.stats.leadership - a.stats.leadership),
		jobs: allJobs,
		upgrades: [...upgrades].sort((a, b) => a.key.localeCompare(b.key)),
		events: recentEvents,
		onlineCount,
		anchor: VILLAGE_ANCHOR,
		// The renderer regenerates the Isometric-Nature terrain client-side from
		// this seed (terrainGen is pure), so map visuals match the gameplay tiles
		// server-side worldgen derives from the very same seed.
		worldSeed: colony.worldSeed ?? colony.createdAt,
		villageRadius: villageRingRadius(colonyBuildings.length),
		claimedTiles: villageToTiles(claimedArea),
		villageGate: claimedGate(claimedArea),
		buildings: colonyBuildings.map((building) => ({
			...building,
			worldPosition: colonyToWorld(building.position),
		})),
		storage: {
			// Per-resource caps derived from the finished storehouses, plus a
			// `foodCapacity` alias kept for the existing HUD.
			capacities: caps,
			foodCapacity: caps.food,
			titheRates: { food: 20, refined: 5 },
		},
		housing: {
			population: aliveCats.length,
			capacity: houseCap,
			pressure: housingPressure(aliveCats.length, houseCap),
			villageLevel: villageLevel(colonyBuildings),
		},
		// God/cat upgrade tree: owned nodes + accrued research so the UI can
		// render the tree, blessings-buy buttons, and the research progress bar.
		research: {
			ownedNodeIds: tree.ownedNodeIds,
			researchPoints: tree.researchPoints,
			researcherCount,
			blessings: colony.globalUpgradePoints ?? 0,
			nextTarget: nextTarget
				? { id: nextTarget.id, name: nextTarget.name, cost: nextTarget.cost }
				: null,
		},
		...electionPayloads(db, colony._id, aliveCats),
		zones: activeZones(db, colony._id, now).map((zone) => ({
			_id: zone._id,
			kind: zone.kind,
			x1: zone.x1,
			y1: zone.y1,
			x2: zone.x2,
			y2: zone.y2,
			expiresAt: zone.expiresAt,
		})),
		// Military: the current threat reading, the standing guard, and any
		// raiders on the map so the HUD can warn and the map can draw them.
		threat: {
			pressure: colony.threatPressure ?? 0,
			band: threatBand(colony.threatPressure ?? 0),
			raidActive: Boolean(colony.activeRaidId),
			warriors: warriorCount,
			weapons: colony.resources.weapons ?? 0,
			armor: colony.resources.armor ?? 0,
		},
		raiders: activeRaiders.map((r) => ({
			_id: r._id,
			position: r.position,
			hp: r.hp,
			strength: r.strength,
			status: r.status,
		})),
	};
}

/** Open election + vote-kick payloads for the dashboard. */
function electionPayloads(db: GameDb, colonyId: string, aliveCats: CatRow[]) {
	const openPolls = db
		.select()
		.from(elections)
		.where(and(eq(elections.colonyId, colonyId), eq(elections.status, "open")))
		.all();

	const poll = openPolls.find((election) => election.kind === "election");
	const kick = openPolls.find((election) => election.kind === "vote_kick");

	let election = null;
	if (poll) {
		const ballots = db
			.select()
			.from(votes)
			.where(eq(votes.electionId, poll._id))
			.all()
			.map((vote) => ({
				playerId: vote.playerId,
				subscriberHash: vote.subscriberHash,
				catId: vote.catId,
			}));
		election = {
			_id: poll._id,
			endsAt: poll.endsAt,
			tally: tallyVotes(ballots),
			totalBallots: new Set(ballots.map(voteIdentityKey)).size,
			candidates: poll.candidateCatIds
				.map((catId) => aliveCats.find((cat) => cat._id === catId))
				.filter((cat): cat is CatRow => Boolean(cat))
				.map((cat) => ({
					_id: cat._id,
					name: cat.name,
					leadership: cat.stats.leadership,
					specialization: cat.specialization ?? null,
				})),
		};
	}

	let voteKick = null;
	if (kick?.targetCatId) {
		const signatures = new Set(
			db
				.select()
				.from(votes)
				.where(eq(votes.electionId, kick._id))
				.all()
				.map((vote) =>
					voteIdentityKey({
						playerId: vote.playerId,
						subscriberHash: vote.subscriberHash,
						catId: vote.catId,
					}),
				),
		).size;
		const target = aliveCats.find((cat) => cat._id === kick.targetCatId);
		voteKick = {
			_id: kick._id,
			endsAt: kick.endsAt,
			targetCatId: kick.targetCatId,
			targetName: target?.name ?? "the leader",
			signatures,
			needed: KICK_THRESHOLD,
		};
	}

	return { election, voteKick };
}

export function requestJob(
	db: GameDb,
	args: { sessionId: string; nickname: string; kind: PlayerJobKind },
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = ensureGlobalColony(tx);
		const now = Date.now();

		const player = upsertPlayer(tx, args.sessionId, args.nickname, now);
		const upgrades = upgradesToLevels(getUpgradeRows(tx, colony._id));
		const runtime = getRuntimeConfig(colony);

		// Any job request counts as player activity for unattended-time tracking
		tx.update(colonies)
			.set({ lastPlayerActivityAt: now })
			.where(eq(colonies._id, colony._id))
			.run();

		// Fetch active+queued jobs once for conflict checks on strategic kinds.
		const isStrategicKind =
			args.kind !== "supply_food" && args.kind !== "supply_water";
		if (isStrategicKind) {
			const allJobs = [
				...getJobsByStatus(tx, colony._id, "active"),
				...getJobsByStatus(tx, colony._id, "queued"),
			];

			if (hasConflictingStrategicJob(args.kind as JobKind, allJobs)) {
				return {
					ok: false,
					reason: "already_in_progress",
					message: "That request is already in progress.",
				};
			}

			if (args.kind === "ritual") {
				const alreadyRequested = ritualRequestIsFresh(
					colony.ritualRequestedAt,
					now,
				);
				const activeRitual = allJobs.some((job) => job.kind === "ritual");
				if (alreadyRequested || activeRitual) {
					return {
						ok: false,
						reason: "ritual_pending",
						message: "Ritual request already pending or active.",
					};
				}
			}
		}

		const bumpJobsRequested = () => {
			tx.update(players)
				.set({
					lifetimeContribution: {
						...player.lifetimeContribution,
						jobsRequested: player.lifetimeContribution.jobsRequested + 1,
					},
				})
				.where(eq(players._id, player._id))
				.run();
		};

		if (args.kind === "ritual") {
			tx.update(colonies)
				.set({ ritualRequestedAt: now })
				.where(eq(colonies._id, colony._id))
				.run();

			bumpJobsRequested();

			logEvent(
				tx,
				colony._id,
				"ritual_ready",
				`${args.nickname} requested a ritual. Leader will schedule it when conditions are safe.`,
			);

			return { requested: true };
		}

		const jobId = queueJob(
			tx,
			colony._id,
			args.kind as JobKind,
			"player",
			upgrades,
			runtime,
			player._id,
			null,
			{},
		);

		bumpJobsRequested();

		return { jobId };
	});
}

export function clickBoostJob(
	db: GameDb,
	args: { sessionId: string; nickname: string; jobId: string },
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const now = Date.now();

		const job = tx.select().from(jobs).where(eq(jobs._id, args.jobId)).get();
		if (!job || (job.status !== "active" && job.status !== "queued")) {
			throw new Error("This job cannot be boosted.");
		}

		const colony = ensureGlobalColony(tx);
		if (colony._id !== job.colonyId) {
			throw new Error("Invalid colony");
		}

		const player = upsertPlayer(tx, args.sessionId, args.nickname, now);
		const upgrades = upgradesToLevels(getUpgradeRows(tx, colony._id));

		const inSameWindow = now - player.clickWindowStart < 60_000;
		const clicksInWindow = inSameWindow ? player.clicksInWindow + 1 : 1;
		const windowStart = inSameWindow ? player.clickWindowStart : now;
		const reduceSeconds = applyClickBoostSeconds(
			clicksInWindow,
			upgrades.click_power,
		);

		const minEnd = now + 5_000;
		const nextEnd = Math.max(minEnd, job.endsAt - reduceSeconds * 1000);

		tx.update(players)
			.set({
				clickWindowStart: windowStart,
				clicksInWindow,
				lifetimeClicks: player.lifetimeClicks + 1,
			})
			.where(eq(players._id, player._id))
			.run();

		tx.update(jobs)
			.set({
				endsAt: nextEnd,
				clickTimeReducedSec: (job.clickTimeReducedSec ?? 0) + reduceSeconds,
			})
			.where(eq(jobs._id, job._id))
			.run();

		tx.update(colonies)
			.set({ lastPlayerActivityAt: now })
			.where(eq(colonies._id, colony._id))
			.run();

		return {
			reducedBySec: reduceSeconds,
			newEndsAt: nextEnd,
		};
	});
}

export function purchaseUpgrade(
	db: GameDb,
	args: { sessionId: string; nickname: string; key: UpgradeKey },
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = ensureGlobalColony(tx);
		const now = Date.now();
		const player = upsertPlayer(tx, args.sessionId, args.nickname, now);

		const upgrade = tx
			.select()
			.from(globalUpgrades)
			.where(
				and(
					eq(globalUpgrades.colonyId, colony._id),
					eq(globalUpgrades.key, args.key),
				),
			)
			.get();

		if (!upgrade) {
			throw new Error("Upgrade not found.");
		}

		if (upgrade.level >= upgrade.maxLevel) {
			throw new Error("Upgrade already maxed.");
		}

		const cost = getUpgradeCost(upgrade.baseCost, upgrade.level);
		const points = colony.globalUpgradePoints ?? 0;
		if (points < cost) {
			throw new Error("Not enough ritual points.");
		}

		tx.update(globalUpgrades)
			.set({ level: upgrade.level + 1 })
			.where(eq(globalUpgrades._id, upgrade._id))
			.run();

		tx.update(colonies)
			.set({
				globalUpgradePoints: points - cost,
				lastPlayerActivityAt: now,
			})
			.where(eq(colonies._id, colony._id))
			.run();

		tx.update(players)
			.set({
				lifetimeContribution: {
					...player.lifetimeContribution,
					upgradesPurchased: player.lifetimeContribution.upgradesPurchased + 1,
				},
			})
			.where(eq(players._id, player._id))
			.run();

		logEvent(
			tx,
			colony._id,
			"upgrade_purchased",
			`${args.nickname} upgraded ${args.key.replace(/_/g, " ")} to level ${upgrade.level + 1}.`,
		);

		return { level: upgrade.level + 1, remainingPoints: points - cost };
	});
}

/**
 * God purchase of an upgrade-tree node: spend the colony's blessings
 * (`globalUpgradePoints`) to unlock a node instantly, provided its
 * prerequisites are met. Soft-fails (returns `{ ok: false, reason }`) on
 * bad node, already-owned, unmet prerequisites, or insufficient blessings —
 * the client surfaces the reason inline.
 */
export function unlockNode(
	db: GameDb,
	args: { sessionId: string; nickname: string; nodeId: string },
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = ensureGlobalColony(tx);
		const now = Date.now();
		upsertPlayer(tx, args.sessionId, args.nickname, now);

		const tree = getUpgradeTree(colony);
		const result = godPurchase(tree, args.nodeId);
		if (!result.ok) {
			return { ok: false, reason: result.reason };
		}

		const blessings = colony.globalUpgradePoints ?? 0;
		if (blessings < result.blessingsCost) {
			return { ok: false, reason: "insufficient-blessings" };
		}

		saveUpgradeTree(tx, colony._id, result.state);
		tx.update(colonies)
			.set({
				globalUpgradePoints: blessings - result.blessingsCost,
				lastPlayerActivityAt: now,
			})
			.where(eq(colonies._id, colony._id))
			.run();

		const node = getNode(args.nodeId);
		logEvent(
			tx,
			colony._id,
			"research_unlocked",
			`The gods granted ${node?.name ?? args.nodeId} to the colony (−${result.blessingsCost} blessings).`,
		);

		return {
			ok: true,
			nodeId: args.nodeId,
			remainingBlessings: blessings - result.blessingsCost,
		};
	});
}

/**
 * Player-requested warrior training. Needs a finished barracks; enrolls a
 * chosen cat (or the sturdiest idle adult) into a `train_warrior` job.
 * Kittens and existing warriors are rejected. Soft-fails so the client can
 * surface the reason inline.
 */
export function trainWarrior(
	db: GameDb,
	args: { sessionId: string; nickname: string; catId?: string | null },
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = ensureGlobalColony(tx);
		const now = Date.now();
		upsertPlayer(tx, args.sessionId, args.nickname, now);

		const colonyBuildings = tx
			.select()
			.from(buildings)
			.where(eq(buildings.colonyId, colony._id))
			.all();
		const hasBarracks = colonyBuildings.some(
			(b) => b.type === "barracks" && b.constructionProgress >= 100,
		);
		if (!hasBarracks) {
			return { ok: false, reason: "no_barracks" };
		}

		const alive = getAliveCats(tx, colony._id);
		const busyIds = new Set(
			[
				...getJobsByStatus(tx, colony._id, "active"),
				...getJobsByStatus(tx, colony._id, "queued"),
			]
				.map((job) => job.assignedCatId)
				.filter(Boolean),
		);
		const eligible = (cat: CatRow): boolean =>
			canWork(getLifeStage(cat.ageHours ?? 0)) &&
			cat.specialization !== "warrior" &&
			!busyIds.has(cat._id);

		let recruit: CatRow | null = null;
		if (args.catId) {
			const chosen = alive.find((cat) => cat._id === args.catId) ?? null;
			if (!chosen || !eligible(chosen)) {
				return { ok: false, reason: "ineligible" };
			}
			recruit = chosen;
		} else {
			recruit =
				alive
					.filter(eligible)
					.sort(
						(a, b) =>
							b.stats.attack +
							b.stats.defense -
							(a.stats.attack + a.stats.defense),
					)[0] ?? null;
		}
		if (!recruit) {
			return { ok: false, reason: "no_recruit" };
		}

		const upgrades = upgradesToLevels(getUpgradeRows(tx, colony._id));
		const runtime = getRuntimeConfig(colony);
		const jobId = queueJob(
			tx,
			colony._id,
			"train_warrior",
			"player",
			upgrades,
			runtime,
			null,
			recruit,
		);
		tx.update(colonies)
			.set({ lastPlayerActivityAt: now })
			.where(eq(colonies._id, colony._id))
			.run();
		return { ok: true, jobId, catId: recruit._id };
	});
}

/**
 * Player defense click against the active raid — the muster's answer to
 * {@link clickBoostJob}. Each click deals damage to the frontmost raider
 * (nearest the gate); a raider at zero hp is cut down. Soft-fails when no raid
 * is in progress.
 */
export function defendRaid(
	db: GameDb,
	args: { sessionId: string; nickname: string },
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = ensureGlobalColony(tx);
		const now = Date.now();
		upsertPlayer(tx, args.sessionId, args.nickname, now);

		const raidId = colony.activeRaidId ?? null;
		if (!raidId) {
			return { ok: false, reason: "no_raid" };
		}

		const living = tx
			.select()
			.from(raiders)
			.where(and(eq(raiders.colonyId, colony._id), eq(raiders.raidId, raidId)))
			.all()
			.filter((r) => r.status !== "dead" && r.hp > 0);
		if (living.length === 0) {
			return { ok: false, reason: "no_raid" };
		}

		// Frontmost = closest to the gate the warband is marching on.
		const gate = gatePosition(
			villageRingRadius(
				tx
					.select()
					.from(buildings)
					.where(eq(buildings.colonyId, colony._id))
					.all().length,
			),
		);
		const target = [...living].sort(
			(a, b) =>
				Math.hypot(a.position.x - gate.x, a.position.y - gate.y) -
				Math.hypot(b.position.x - gate.x, b.position.y - gate.y),
		)[0];

		const nextHp = target.hp - DEFEND_CLICK_DAMAGE;
		tx.update(raiders)
			.set(nextHp <= 0 ? { hp: 0, status: "dead" } : { hp: nextHp })
			.where(eq(raiders._id, target._id))
			.run();

		tx.update(colonies)
			.set({
				raidClicks: (colony.raidClicks ?? 0) + 1,
				lastPlayerActivityAt: now,
			})
			.where(eq(colonies._id, colony._id))
			.run();

		return {
			ok: true,
			raiderId: target._id,
			raiderHp: Math.max(0, nextHp),
			killed: nextHp <= 0,
		};
	});
}

/**
 * Force a raid for tests / live sanity. Spawns a warband (at the gate by
 * default so the next tick resolves it) using the current colony snapshot.
 */
export function spawnRaidForTest(
	db: GameDb,
	opts: { atGate?: boolean; count?: number; strength?: number } = {},
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = ensureGlobalColony(tx);
		if (colony.activeRaidId) {
			return { ok: false, reason: "raid_in_progress" };
		}
		const alive = getAliveCats(tx, colony._id);
		const buildingCount = tx
			.select()
			.from(buildings)
			.where(eq(buildings.colonyId, colony._id))
			.all().length;
		const ringRadius = villageRingRadius(buildingCount);
		const snapshot = {
			wealth: colonyWealth(colony.resources),
			population: alive.length,
			warriors: alive.filter((c) => c.specialization === "warrior").length,
			colonyAgeSec:
				(Date.now() - (colony.runStartedAt ?? colony.createdAt)) / 1000,
		};
		const raidId = spawnRaid(tx, colony._id, snapshot, ringRadius, () => 0.5, {
			atGate: opts.atGate ?? true,
			plan:
				opts.count != null || opts.strength != null
					? {
							count: opts.count ?? 3,
							strengthEach: opts.strength ?? 30,
						}
					: undefined,
		});
		return { ok: true, raidId };
	});
}

export function upsertPresence(
	db: GameDb,
	sessionId: string,
	nickname: string,
): string {
	return upsertPlayer(db, sessionId, nickname, Date.now(), {
		recordPresence: true,
	})._id;
}

export function workerTick(db: GameDb) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = ensureGlobalColony(tx);

		const now = Date.now();
		const elapsedSec = Math.max(0, Math.floor((now - colony.lastTick) / 1000));
		if (elapsedSec === 0) {
			// Sub-second tick — keep lastTick untouched so fractional elapsed time
			// accumulates instead of being discarded by early/jittery workers.
			return { ok: true, skipped: true };
		}
		const processedThrough = colony.lastTick + elapsedSec * 1000;

		const upgrades = upgradesToLevels(getUpgradeRows(tx, colony._id));
		const runtime = getRuntimeConfig(colony);

		// Upgrade-tree state + resolved modifiers for this tick. `effects`
		// feeds capacities, hunt yields, movement and research below; the
		// tree itself accrues research points and may auto-unlock a node.
		const upgradeTree = getUpgradeTree(colony);
		const effects = resolveEffects(upgradeTree.ownedNodeIds);

		let rngSeed = runtime.rngSeed;
		const nextRoll = () => {
			if (rngSeed === null) {
				return Math.random();
			}
			const roll = rollSeeded(rngSeed);
			rngSeed = roll.nextSeed;
			return roll.value;
		};

		// Leadership is player-elected (Phase 4). The tick only auto-picks an
		// interim leader when the seat is empty — leader death or bootstrap —
		// and otherwise leaves the elected cat in charge, good or bad.
		let leaderCat = colony.leaderId
			? (tx
					.select()
					.from(cats)
					.where(and(eq(cats._id, colony.leaderId), isNull(cats.deathTime)))
					.get() ?? null)
			: null;
		if (!leaderCat) {
			const interim = chooseLeader(tx, colony._id);
			if (interim) {
				leaderCat = interim;
				tx.update(colonies)
					.set({ leaderId: interim._id })
					.where(eq(colonies._id, colony._id))
					.run();
				logEvent(
					tx,
					colony._id,
					"leader_change",
					`${interim.name} is now leading the colony.`,
					[interim._id],
				);
			}
		}

		const policyTier = pickPolicyTier(
			leaderCat?.stats.leadership ?? 50,
			nextRoll(),
		);
		const policy = configForTier(policyTier);
		const canTakePolicyAction = () => nextRoll() <= policy.actionReliability;

		let aliveCats = getAliveCats(tx, colony._id);

		// Storage: base stores plus each finished food storehouse.
		const colonyBuildingsEarly = tx
			.select()
			.from(buildings)
			.where(eq(buildings.colonyId, colony._id))
			.all();
		let caps = storageCapacities(
			colonyBuildingsEarly,
			effects.storagePerLevelMult,
		);
		let foodCapacity = caps.food;

		// --- Life simulation: aging, mortality, births, conceptions --------
		// The colony is a living population. Everyone ages on the accelerated
		// game-clock; elders roll against old-age death; sitting leaders get
		// better at leading; and when the village is fed, watered and has spare
		// housing, adults pair off and birth trait-inheriting kittens. Runs on
		// a forked roll chain so the policy/movement chains (and their
		// deterministic tests) are untouched.
		runLifeSimulation(tx, colony, {
			aliveCats,
			elapsedGameHours: (elapsedSec * runtime.timeScale) / 3600,
			housingCap: housingCapacity(colonyBuildingsEarly, effects.housingPerDen),
			foodRatio: caps.food > 0 ? colony.resources.food / caps.food : 0,
			waterRatio: caps.water > 0 ? colony.resources.water / caps.water : 0,
			food: colony.resources.food,
			water: colony.resources.water,
			lifeSeed: rngSeed === null ? null : rngSeed + 2_000_003,
		});
		// Births and deaths this tick change the roster the rest of the tick
		// runs against.
		aliveCats = getAliveCats(tx, colony._id);

		const { foodUse, waterUse } = consumptionForTick(
			aliveCats.length,
			elapsedSec * runtime.resourceDecayMultiplier,
			upgrades,
		);

		// Spoilage: food inside storage keeps well; overflow "in the open"
		// rots fast — offer it to the shrine before it goes off.
		const rawFood = Math.max(0, colony.resources.food - foodUse);
		const stored = Math.min(rawFood, foodCapacity);
		const overflow = Math.max(0, rawFood - foodCapacity);
		const decayedFood =
			stored * (1 - 0.0005 * (elapsedSec / 60)) +
			overflow * (1 - 0.02 * (elapsedSec / 60));

		const nextResources = {
			...colony.resources,
			food: Math.max(0, decayedFood),
			water: Math.min(
				caps.water,
				Math.max(0, colony.resources.water - waterUse),
			),
			herbs: Math.min(caps.herbs, colony.resources.herbs),
			materials: Math.min(caps.materials, colony.resources.materials),
			refined: Math.min(caps.refined, colony.resources.refined ?? 0),
		};

		// Leader tithe cadence: surplus goes to the gods at most once a
		// minute. The amount is decided in the consolidated leader pass
		// below; this flag just gates when it may fire.
		// Deterministic under advanceTime: a >=60s tick always counts as a
		// minute, so skip-time tests don't depend on wall-clock boundaries.
		const minuteRolled =
			elapsedSec >= 60 ||
			Math.floor(now / 60_000) !== Math.floor(colony.lastTick / 60_000);

		if (colony.resources.water > 3 && nextResources.water <= 3) {
			logEvent(
				tx,
				colony._id,
				"crisis",
				"CRISIS: WATER RESERVES DANGEROUSLY LOW",
			);
		}

		// --- Elections: resolve due polls, then open the next one when the
		// term expires. Uses no policy rolls, so the seeded chain is stable.
		runElectionLifecycle(tx, colony, aliveCats, runtime, now);

		// --- Player zones: drop expired ones, snapshot the rest for
		// destination steering below.
		sweepExpiredZones(tx, colony._id, now);

		// Events are append-only; without pruning they grow ~100k+/day.
		// Once a game-minute, keep only the newest rows.
		if (minuteRolled) {
			const EVENT_KEEP = 2_000;
			const cutoff = tx
				.select({ timestamp: events.timestamp })
				.from(events)
				.where(eq(events.colonyId, colony._id))
				.orderBy(desc(events.timestamp))
				.limit(1)
				.offset(EVENT_KEEP)
				.get();
			if (cutoff) {
				tx.delete(events)
					.where(
						and(
							eq(events.colonyId, colony._id),
							lt(events.timestamp, cutoff.timestamp),
						),
					)
					.run();
			}
		}

		// Unused paths decay (~1 wear/min). Wear floors at 1 so explored
		// terrain stays revealed even after the road itself fades. The decay
		// is capped per tick (MAX_PATH_DECAY_PER_TICK) so an accelerated preset
		// can't erase a whole route in one tick before traffic re-lays it —
		// that cap is what lets roads persist under the hyper/ludicrous presets.
		const decayAmount = Math.min(
			MAX_PATH_DECAY_PER_TICK,
			(elapsedSec * runtime.timeScale) / 60,
		);
		if (decayAmount > 0) {
			const wornTiles = tx
				.select()
				.from(worldTiles)
				.where(
					and(eq(worldTiles.colonyId, colony._id), gt(worldTiles.pathWear, 0)),
				)
				.all();
			for (const worn of wornTiles) {
				if (worn.overlayFeature === "road_built") {
					continue; // built roads are permanent
				}
				let next = worn.pathWear;
				if (worn.pathWear >= 70) {
					// A worn road slowly fades back toward a bare trail once the
					// traffic that made it stops.
					next = Math.max(63, worn.pathWear - decayAmount);
				} else if (worn.pathWear > 62) {
					// Revealed-but-not-road ground stays put: it's explored terrain
					// and a faint trail, and freezing it here is what lets repeated
					// traversals accumulate into a road under accelerated presets.
					continue;
				} else {
					// Worldgen's faint seeded trails fade to nothing.
					next = Math.max(1, worn.pathWear - decayAmount);
				}
				if (next !== worn.pathWear) {
					tx.update(worldTiles)
						.set({ pathWear: next })
						.where(eq(worldTiles._id, worn._id))
						.run();
				}
			}
		}
		// Regrowth: depleted non-forest tiles slowly refill their food back
		// toward the cap. Chopped forests keep their new low cap and never
		// regain the forest type. Runs at most once a game-minute over just
		// the tiles a haul has touched (lastDepleted > 0) to stay cheap.
		if (minuteRolled) {
			const regrow = regrowthAmount(elapsedSec * runtime.timeScale);
			if (regrow > 0) {
				const depletedTiles = tx
					.select()
					.from(worldTiles)
					.where(
						and(
							eq(worldTiles.colonyId, colony._id),
							gt(worldTiles.lastDepleted, 0),
						),
					)
					.all();
				for (const tile of depletedTiles) {
					if (isForestType(tile.type)) {
						continue;
					}
					const maxFood = tile.maxResources.food ?? 0;
					const food = tile.resources.food ?? 0;
					if (food >= maxFood) {
						continue;
					}
					tx.update(worldTiles)
						.set({
							resources: {
								...tile.resources,
								food: Math.min(maxFood, food + regrow),
							},
						})
						.where(eq(worldTiles._id, tile._id))
						.run();
				}
			}
		}

		const zoneList: Zone[] = activeZones(tx, colony._id, now).map((zone) => ({
			kind: zone.kind,
			x1: zone.x1,
			y1: zone.y1,
			x2: zone.x2,
			y2: zone.y2,
		}));

		// Movement randomness runs on a forked chain so the policy/planning
		// roll order (and its deterministic tests) stays untouched.
		let movementSeed = rngSeed === null ? null : rngSeed + 1_000_003;
		const nextMovementRoll = () => {
			if (movementSeed === null) {
				return Math.random();
			}
			const roll = rollSeeded(movementSeed);
			movementSeed = roll.nextSeed;
			return roll.value;
		};

		// Known food-rich tiles outside the village, loaded lazily when a
		// hunt starts (tile snapshot at job start, never per-tick).
		let cachedFoodTiles: WorldPos[] | null = null;
		const foodTilesNearVillage = (): WorldPos[] => {
			if (cachedFoodTiles) {
				return cachedFoodTiles;
			}
			cachedFoodTiles = tx
				.select()
				.from(worldTiles)
				.where(eq(worldTiles.colonyId, colony._id))
				.all()
				.filter((tile) => {
					const distance = Math.max(
						Math.abs(tile.x - VILLAGE_ANCHOR.x),
						Math.abs(tile.y - VILLAGE_ANCHOR.y),
					);
					// Cats only hunt land they know: scouted (cat-worn paths
					// beat worldgen's <=60 seeds) or within village sight.
					const explored = tile.pathWear > 62 || distance <= 6;
					return (tile.resources?.food ?? 0) >= 25 && explored && distance > 4;
				})
				.map((tile) => ({ x: tile.x, y: tile.y }));
			return cachedFoodTiles;
		};

		const chebFromAnchor = (pos: { x: number; y: number }): number =>
			Math.max(
				Math.abs(pos.x - VILLAGE_ANCHOR.x),
				Math.abs(pos.y - VILLAGE_ANCHOR.y),
			);
		// Same fog rule the rest of the tick uses: a tile is known once a
		// cat has worn a path across it, or it sits within village sight.
		const tileIsExplored = (tile: {
			x: number;
			y: number;
			pathWear: number;
		}): boolean => tile.pathWear > 62 || chebFromAnchor(tile) <= 6;

		// All colony tiles, loaded once and shared by the quarry/frontier
		// lookups below (one indexed select, cached for the tick).
		let cachedColonyTiles: WorldTileRow[] | null = null;
		const colonyTiles = (): WorldTileRow[] => {
			if (!cachedColonyTiles) {
				cachedColonyTiles = tx
					.select()
					.from(worldTiles)
					.where(eq(worldTiles.colonyId, colony._id))
					.all();
			}
			return cachedColonyTiles;
		};

		// Explored stone country, nearest first — the leader quarries the
		// closest known mountains/cave tile for materials.
		let cachedQuarrySites: WorldPos[] | null = null;
		const quarrySitesNearVillage = (): WorldPos[] => {
			if (!cachedQuarrySites) {
				cachedQuarrySites = colonyTiles()
					.filter(
						(tile) => QUARRY_TILE_TYPES.has(tile.type) && tileIsExplored(tile),
					)
					.sort((a, b) => chebFromAnchor(a) - chebFromAnchor(b))
					.map((tile) => ({ x: tile.x, y: tile.y }));
			}
			return cachedQuarrySites;
		};

		// Explored water country, nearest first — the colony draws water from
		// the closest known river/pond tile.
		let cachedWaterSites: WorldPos[] | null = null;
		const waterSitesNearVillage = (): WorldPos[] => {
			if (!cachedWaterSites) {
				cachedWaterSites = colonyTiles()
					.filter((tile) => tileHasWater(tile) && tileIsExplored(tile))
					.sort((a, b) => chebFromAnchor(a) - chebFromAnchor(b))
					.map((tile) => ({ x: tile.x, y: tile.y }));
			}
			return cachedWaterSites;
		};

		// A claimed build cell sits on water when its world tile is a river/pond
		// — scaffolds must never rise there.
		const worldCellIsWater = (world: WorldPos): boolean => {
			const tile = colonyTiles().find(
				(t) => t.x === world.x && t.y === world.y,
			);
			return tile ? tileHasWater(tile) : false;
		};

		// Frontier tiles: still fogged, within scouting range, and touching
		// explored land — the edge the leader sends scouts to reveal.
		let cachedFrontierTiles: WorldPos[] | null = null;
		const frontierTilesNearVillage = (): WorldPos[] => {
			if (!cachedFrontierTiles) {
				const tiles = colonyTiles();
				const exploredKeys = new Set(
					tiles.filter(tileIsExplored).map((tile) => `${tile.x},${tile.y}`),
				);
				cachedFrontierTiles = tiles
					.filter((tile) => {
						if (tileIsExplored(tile)) {
							return false;
						}
						if (chebFromAnchor(tile) > SCOUT_RANGE) {
							return false;
						}
						for (let dy = -1; dy <= 1; dy++) {
							for (let dx = -1; dx <= 1; dx++) {
								if (dx === 0 && dy === 0) {
									continue;
								}
								if (exploredKeys.has(`${tile.x + dx},${tile.y + dy}`)) {
									return true;
								}
							}
						}
						return false;
					})
					.sort((a, b) => chebFromAnchor(a) - chebFromAnchor(b))
					.map((tile) => ({ x: tile.x, y: tile.y }));
			}
			return cachedFrontierTiles;
		};

		// Every haul off a hunt site eats into that tile's food. Drained by
		// the integer share hauled (floored at 0); marks the tile depleted so
		// the regrowth sweep above will slowly refill it (unless it's forest).
		const drainHuntSite = (site: WorldPos | undefined, amount: number) => {
			if (!site || amount <= 0) {
				return;
			}
			const tile = tx
				.select()
				.from(worldTiles)
				.where(
					and(
						eq(worldTiles.colonyId, colony._id),
						eq(worldTiles.x, Math.round(site.x)),
						eq(worldTiles.y, Math.round(site.y)),
					),
				)
				.get();
			if (!tile) {
				return;
			}
			tx.update(worldTiles)
				.set({
					resources: {
						...tile.resources,
						food: Math.max(0, (tile.resources.food ?? 0) - Math.floor(amount)),
					},
					lastDepleted: now,
				})
				.where(eq(worldTiles._id, tile._id))
				.run();
		};

		// Promote queued jobs to active; send assigned cats to the job site.
		const queuedJobs = getJobsByStatus(tx, colony._id, "queued");
		// Spread scouts promoted this tick across the frontier instead of
		// stacking them on the single nearest tile.
		let scoutPromotions = 0;
		for (const job of queuedJobs) {
			let jobMetadata = job.metadata ?? null;

			// A construction job breaks ground when it starts: pick a free
			// site next to the village and raise a scaffold there.
			const isConstruction =
				job.kind === "build_house" &&
				String((jobMetadata as Record<string, unknown> | null)?.phase) ===
					"construct_house";
			let constructionSite: WorldPos | null = null;
			if (isConstruction) {
				const requestedType = String(
					(jobMetadata as Record<string, unknown> | null)?.buildingType ??
						"den",
				);
				const scaffoldType =
					requestedType === "workshop" ||
					requestedType === "field" ||
					requestedType === "food_storage" ||
					requestedType === "research_hut" ||
					requestedType === "school" ||
					requestedType === "smithy" ||
					requestedType === "barracks"
						? (requestedType as
								| "workshop"
								| "field"
								| "food_storage"
								| "research_hut"
								| "school"
								| "smithy"
								| "barracks")
						: ("den" as const);
				const occupied = tx
					.select()
					.from(buildings)
					.where(eq(buildings.colonyId, colony._id))
					.all()
					.map((b) => b.position);
				const siteLocal = nextClaimedBuildingSite(
					getClaimedArea(getColony(tx, colony._id)),
					occupied,
					nextMovementRoll(),
					worldCellIsWater,
				);
				if (siteLocal) {
					const buildingId = nanoid();
					tx.insert(buildings)
						.values({
							_id: buildingId,
							colonyId: colony._id,
							type: scaffoldType,
							level: 1,
							position: siteLocal,
							constructionProgress: 0,
						})
						.run();
					jobMetadata = {
						...(jobMetadata ?? {}),
						site: siteLocal,
						buildingId,
					};
					constructionSite = colonyToWorld(siteLocal);
				}
			}

			tx.update(jobs)
				.set({
					status: "active",
					startedAt: job.startedAt || now,
					metadata: jobMetadata,
				})
				.where(eq(jobs._id, job._id))
				.run();

			if (!job.assignedCatId) {
				continue;
			}
			// Zones steer hunts: avoid tiles are excluded (unless nothing else
			// exists), gather tiles are twice as likely.
			let huntTiles: WorldPos[] = [];
			if (job.kind === "hunt_expedition") {
				const preferred = pickTargetWithZones(
					foodTilesNearVillage(),
					zoneList,
					nextMovementRoll(),
				);
				huntTiles = preferred ? [preferred] : [];
			}
			// Quarry heads to the nearest explored stone tile; scouts pick a
			// frontier tile, rotating so a batch fans out across the edge.
			let quarrySite: WorldPos | undefined;
			if (job.kind === "quarry") {
				quarrySite = quarrySitesNearVillage()[0];
			}
			let waterSite: WorldPos | undefined;
			if (job.kind === "fetch_water") {
				waterSite = waterSitesNearVillage()[0];
			}
			let exploreSite: WorldPos | undefined;
			if (job.kind === "explore") {
				const frontier = frontierTilesNearVillage();
				if (frontier.length > 0) {
					exploreSite = frontier[scoutPromotions % frontier.length];
					scoutPromotions += 1;
				}
			}
			let expansionSite: WorldPos | undefined;
			if (job.kind === "expand_village") {
				const target = (jobMetadata as Record<string, unknown> | null)
					?.target as WorldPos | undefined;
				if (typeof target?.x === "number" && typeof target?.y === "number") {
					expansionSite = { x: target.x, y: target.y };
				}
			}
			const jobDestination = destinationForJob(job.kind, {
				anchor: VILLAGE_ANCHOR,
				shrine: VILLAGE_ANCHOR,
				foodTiles: huntTiles,
				roll: nextMovementRoll(),
				site: constructionSite ?? undefined,
				quarrySite,
				waterSite,
				exploreSite,
				expansionSite,
			});
			if (jobDestination) {
				// Jobs are accepted at the shrine: the cat reports there first,
				// then heads out to the recorded site.
				jobMetadata = {
					...(jobMetadata ?? {}),
					site: jobDestination,
					accepted: false,
				};
				tx.update(jobs)
					.set({ metadata: jobMetadata })
					.where(eq(jobs._id, job._id))
					.run();
				tx.update(cats)
					.set({
						destination: { map: "world", ...VILLAGE_ANCHOR },
						activity: "traveling",
						currentTask: job.kind,
					})
					.where(eq(cats._id, job.assignedCatId))
					.run();
			}
		}

		// Leader auto-plans hunt/build when resources are low, gated by policy reliability.
		const activeJobs = getJobsByStatus(tx, colony._id, "active");

		// Scaffolds rise with their job's progress.
		for (const job of activeJobs) {
			if (job.kind !== "build_house") {
				continue;
			}
			const meta = job.metadata as Record<string, unknown> | null;
			const buildingId = meta?.buildingId;
			if (typeof buildingId !== "string") {
				continue;
			}
			const duration = Math.max(1, job.endsAt - job.startedAt);
			const progress = Math.min(
				99,
				Math.max(0, Math.round(((now - job.startedAt) / duration) * 100)),
			);
			tx.update(buildings)
				.set({ constructionProgress: progress })
				.where(eq(buildings._id, buildingId))
				.run();
		}

		if (
			nextResources.food < policy.foodEmergencyThreshold &&
			!hasConflictingStrategicJob("leader_plan_hunt", activeJobs) &&
			canTakePolicyAction()
		) {
			queueJob(
				tx,
				colony._id,
				"leader_plan_hunt",
				"leader",
				upgrades,
				runtime,
				null,
				selectBestCat(tx, colony._id, "hunter"),
			);
		}

		// --- Leader brain: one coherent decision pass over a colony
		// snapshot (replaces the old scattered hunt / cancel / storage / den /
		// workshop / tithe blocks). The pure planner in lib/game/leaderAI.ts
		// decides *what* to do; here we execute each decision, keeping the
		// seeded policy-reliability roll at the call site so leader tiers still
		// skip and limit actions.
		const colonyBuildings = tx
			.select()
			.from(buildings)
			.where(eq(buildings.colonyId, colony._id))
			.all();

		const jobBuildingType = (job: JobRow): string =>
			String(
				(job.metadata as Record<string, unknown> | null)?.buildingType ?? "den",
			);

		// All queued jobs were promoted to active above, so activeJobs is the
		// complete in-flight set.
		const activeHunts = activeJobs.filter(
			(job) => job.kind === "hunt_expedition",
		).length;
		const activeQuarries = activeJobs.filter(
			(job) => job.kind === "quarry",
		).length;
		const activeScouts = activeJobs.filter(
			(job) => job.kind === "explore",
		).length;
		const activeWaterFetchers = activeJobs.filter(
			(job) => job.kind === "fetch_water",
		).length;
		const denPlansInFlight = activeJobs.filter(
			(job) =>
				job.kind === "leader_plan_house" ||
				(job.kind === "build_house" && jobBuildingType(job) === "den"),
		).length;
		const storagePlansInFlight = activeJobs.filter(
			(job) =>
				job.kind === "build_house" && jobBuildingType(job) === "food_storage",
		).length;

		// Committed shelter: dens still under construction plus dens being
		// raised by an in-flight construct job.
		const committedCapacity =
			colonyBuildings
				.filter((b) => b.type === "den" && b.constructionProgress < 100)
				.reduce((sum, b) => sum + 2 * Math.max(1, b.level), 0) +
			2 *
				activeJobs.filter(
					(job) =>
						job.kind === "build_house" &&
						jobBuildingType(job) === "den" &&
						(job.metadata as Record<string, unknown> | null)?.phase ===
							"construct_house",
				).length;

		const busyIds = new Set(
			activeJobs.map((job) => job.assignedCatId).filter(Boolean),
		);
		// Kittens stay in the nursery — they're never dispatched to work.
		const workCapableCats = aliveCats.filter((cat) =>
			canWork(getLifeStage(cat.ageHours ?? 0)),
		);
		const idleCatRows = workCapableCats.filter(
			(cat) =>
				!busyIds.has(cat._id) &&
				!cat.assignedBuildingId &&
				(cat.activity ?? "idle") === "idle",
		);
		// Stage-aware workforce: kittens count for nothing, elders partially.
		// The leader employs a fraction of this, not of the raw head count.
		const workforce = aliveCats.reduce(
			(sum, cat) => sum + workforceWeight(getLifeStage(cat.ageHours ?? 0)),
			0,
		);

		// Completed workshops with no assigned worker, for staffing below.
		const staffedBuildingIds = new Set(
			aliveCats
				.map((cat) => cat.assignedBuildingId)
				.filter((id): id is string => Boolean(id)),
		);
		const workshopsNeedingWorkers = colonyBuildings.filter(
			(building) =>
				building.type === "workshop" &&
				building.constructionProgress >= 100 &&
				!staffedBuildingIds.has(building._id),
		);
		// Completed research huts with no assigned researcher, for staffing.
		const researchHutsNeedingWorkers = colonyBuildings.filter(
			(building) =>
				building.type === "research_hut" &&
				building.constructionProgress >= 100 &&
				!staffedBuildingIds.has(building._id),
		);

		// Completed smithies with no assigned smith, for staffing.
		const smithiesNeedingWorkers = colonyBuildings.filter(
			(building) =>
				building.type === "smithy" &&
				building.constructionProgress >= 100 &&
				!staffedBuildingIds.has(building._id),
		);

		// --- Military readiness for the leader's war planning ---------------
		const hasBarracks = colonyBuildings.some(
			(b) => b.type === "barracks" && b.constructionProgress >= 100,
		);
		const warriorCount = aliveCats.filter(
			(cat) => cat.specialization === "warrior",
		).length;
		const trainingInFlight = activeJobs.filter(
			(job) => job.kind === "train_warrior",
		).length;
		// Threat band the raid director will act on — the leader trains toward a
		// bigger guard as pressure climbs, and stands down when the larder empties.
		const currentThreatBand = threatBand(colony.threatPressure ?? 0);
		const starving =
			foodCapacity > 0 && nextResources.food / foodCapacity < 0.15;

		const snapshot: LeaderSnapshot = {
			population: aliveCats.length,
			workforce,
			idleCats: idleCatRows.length,
			employedCats: workCapableCats.length - idleCatRows.length,
			resources: {
				food: nextResources.food,
				refined: nextResources.refined ?? 0,
			},
			foodCapacity,
			// This tick's consumption feeds the director's projection curve, so a
			// still-full but fast-draining store scores urgent at high time scales.
			foodDrainPerTick: foodUse,
			materials: nextResources.materials,
			materialsCapacity: caps.materials,
			water: nextResources.water,
			waterCapacity: caps.water,
			waterDrainPerTick: waterUse,
			housing: {
				capacity: housingCapacity(colonyBuildings, effects.housingPerDen),
				committed: committedCapacity,
			},
			activeHunts,
			activeQuarries,
			activeScouts,
			activeWaterFetchers,
			hasQuarrySite: quarrySitesNearVillage().length > 0,
			hasWaterSite: waterSitesNearVillage().length > 0,
			hasFrontier: frontierTilesNearVillage().length > 0,
			denPlansInFlight,
			storagePlansInFlight,
			storehouseCount: countStorehouses(colonyBuildings),
			storehouseCap: storehouseCap(aliveCats.length),
			workshopsNeedingWorkers: workshopsNeedingWorkers.length,
			researchHutsNeedingWorkers: researchHutsNeedingWorkers.length,
			smithiesNeedingWorkers: smithiesNeedingWorkers.length,
			hasBarracks,
			warriorCount,
			trainingInFlight,
			threatBand: currentThreatBand,
			starving,
		};

		// Worker map the later production pass consumes.
		const workshopWorkers = new Map<string, CatRow>();
		for (const cat of aliveCats) {
			if (cat.assignedBuildingId) {
				workshopWorkers.set(cat.assignedBuildingId, cat);
			}
		}

		// Idle cats still free to take work this tick (mutated as we assign).
		const availableIdle = [...idleCatRows];
		const claimIdle = (cat: CatRow) => {
			const idx = availableIdle.findIndex((c) => c._id === cat._id);
			if (idx >= 0) {
				availableIdle.splice(idx, 1);
			}
		};

		// The IAUS director scores every goal on one scale and hands the shared
		// employment budget to the highest-urgency goals first; here we execute
		// its plan. The seeded policy-reliability roll stays at each execution
		// site, so leader tiers still skip and cap actions.
		const plan = directColony(snapshot);

		// --- Cancellations first: they free labour rather than spend it. -----
		for (const decision of plan.decisions) {
			if (decision.kind === "cancel_hunts") {
				// Overflowing stores: call the hunts off; the cats walk home and
				// only pick up new work back at the shrine.
				const pointlessHunts = activeJobs.filter(
					(job) => job.kind === "hunt_expedition",
				);
				for (const hunt of pointlessHunts) {
					tx.update(jobs)
						.set({ status: "cancelled", completedAt: now })
						.where(eq(jobs._id, hunt._id))
						.run();
					if (hunt.assignedCatId) {
						tx.update(cats)
							.set({
								destination: { map: "world", ...VILLAGE_ANCHOR },
								activity: "returning",
								currentTask: null,
							})
							.where(eq(cats._id, hunt.assignedCatId))
							.run();
					}
				}
				if (pointlessHunts.length > 0) {
					logEvent(
						tx,
						colony._id,
						"job_cancelled",
						`The leader called off ${pointlessHunts.length} hunt${pointlessHunts.length === 1 ? "" : "s"} — the stores are overflowing.`,
					);
				}
			} else if (decision.kind === "cancel_training") {
				// A starving colony pulls its recruits back to work.
				const training = activeJobs.filter(
					(job) => job.kind === "train_warrior",
				);
				for (const job of training) {
					tx.update(jobs)
						.set({ status: "cancelled", completedAt: now })
						.where(eq(jobs._id, job._id))
						.run();
					if (job.assignedCatId) {
						tx.update(cats)
							.set({ activity: "idle", currentTask: null })
							.where(eq(cats._id, job.assignedCatId))
							.run();
					}
				}
				if (training.length > 0) {
					logEvent(
						tx,
						colony._id,
						"job_cancelled",
						`The leader called ${training.length} recruit${training.length === 1 ? "" : "s"} back from the barracks — the larder is bare.`,
					);
				}
			}
		}

		// --- Assignment: one greedy skill-fit pass matches idle able cats to
		// the director's open labour slots (highest-urgency slot first). This
		// single global pass replaces the old per-goal sort loops, so a great
		// hunter is never burned on a scout slot while a scrub takes the hunt.
		const catBriefs: CatBrief[] = availableIdle.map((c) => ({
			id: c._id,
			specialization: c.specialization ?? null,
			stats: {
				hunting: c.stats.hunting,
				building: c.stats.building,
				vision: c.stats.vision,
				medicine: c.stats.medicine,
				attack: c.stats.attack,
				defense: c.stats.defense,
				leadership: c.stats.leadership,
			},
		}));
		const idleById = new Map(availableIdle.map((c) => [c._id, c]));
		const workshopQueue = [...workshopsNeedingWorkers];
		const researchQueue = [...researchHutsNeedingWorkers];
		const smithyQueue = [...smithiesNeedingWorkers];
		const staffBuilding = (
			building: (typeof workshopsNeedingWorkers)[number] | undefined,
			cat: CatRow,
			message: string,
		) => {
			if (!building) {
				return;
			}
			tx.update(cats)
				.set({ assignedBuildingId: building._id })
				.where(eq(cats._id, cat._id))
				.run();
			workshopWorkers.set(building._id, cat);
			claimIdle(cat);
			logEvent(tx, colony._id, "worker_assigned", message, [cat._id]);
		};

		for (const assignment of matchCatsToSlots(plan.slots, catBriefs, {
			excludeWarriorsFromTraining: true,
		})) {
			const cat = idleById.get(assignment.catId);
			if (!cat) {
				continue;
			}
			// One policy roll per intended action — a fallible leader skips it.
			if (!canTakePolicyAction()) {
				continue;
			}
			switch (assignment.goal) {
				case "hunt":
				case "fetch_water":
				case "quarry":
				case "scout":
				case "train_warrior": {
					const kind =
						assignment.goal === "hunt"
							? "hunt_expedition"
							: assignment.goal === "scout"
								? "explore"
								: assignment.goal;
					queueJob(
						tx,
						colony._id,
						kind,
						"leader",
						upgrades,
						runtime,
						null,
						cat,
					);
					claimIdle(cat);
					break;
				}
				case "assign_workshop":
					staffBuilding(
						workshopQueue.shift(),
						cat,
						`The leader put ${cat.name} to work at the workshop.`,
					);
					break;
				case "assign_research":
					staffBuilding(
						researchQueue.shift(),
						cat,
						`The leader sent ${cat.name} to study at the research hut.`,
					);
					break;
				case "assign_smithy":
					staffBuilding(
						smithyQueue.shift(),
						cat,
						`The leader set ${cat.name} to work the forge at the smithy.`,
					);
					break;
			}
		}

		// --- Capital projects and offerings the director scheduled. ----------
		for (const decision of plan.decisions) {
			if (decision.kind === "build_storage") {
				if (canTakePolicyAction()) {
					const architect = selectBestCat(tx, colony._id, "architect");
					if (!architect) {
						continue;
					}
					queueJob(
						tx,
						colony._id,
						"build_house",
						"leader",
						upgrades,
						runtime,
						null,
						architect,
						{ phase: "construct_house", buildingType: "food_storage" },
					);
				}
			} else if (decision.kind === "build_den") {
				if (canTakePolicyAction()) {
					queueJob(
						tx,
						colony._id,
						"leader_plan_house",
						"leader",
						upgrades,
						runtime,
						null,
						selectBestCat(tx, colony._id, "architect"),
					);
				}
			} else if (decision.kind === "tithe") {
				// Surplus offering is capped to once a minute.
				if (!minuteRolled) {
					continue;
				}
				nextResources.food -= decision.food;
				nextResources.refined = (nextResources.refined ?? 0) - decision.refined;
				const points = (colony.globalUpgradePoints ?? 0) + decision.blessings;
				tx.update(colonies)
					.set({ globalUpgradePoints: points })
					.where(eq(colonies._id, colony._id))
					.run();
				colony.globalUpgradePoints = points;
				logEvent(
					tx,
					colony._id,
					"shrine_deposit",
					`The leader offered surplus stores to the gods (+${decision.blessings} blessing${decision.blessings === 1 ? "" : "s"}).`,
				);
			}
		}

		// Ritual from player request, only if resources are stable and policy roll passes.
		if (
			shouldStartRitual(colony.ritualRequestedAt, nextResources, activeJobs) &&
			canTakePolicyAction()
		) {
			queueJob(
				tx,
				colony._id,
				"ritual",
				"leader",
				upgrades,
				runtime,
				null,
				selectBestCat(tx, colony._id, "ritualist"),
			);
			tx.update(colonies)
				.set({ ritualRequestedAt: null })
				.where(eq(colonies._id, colony._id))
				.run();
			logEvent(
				tx,
				colony._id,
				"ritual_ready",
				"Leader approved a ritual window.",
			);
		}

		const patchedResources = { ...nextResources };

		// Workshops refine materials, fields grow food (Phase 7). Runs once
		// resource patching is available; worker staffing happened above.
		const productionElapsed = elapsedSec * runtime.timeScale;
		for (const building of colonyBuildings) {
			if (building.constructionProgress < 100) {
				continue;
			}
			if (building.type === "field") {
				patchedResources.food += fieldYield(productionElapsed);
			}
			if (building.type === "workshop") {
				const worker = workshopWorkers.get(building._id) ?? null;
				const step = advanceWorkshop(
					building.productionProgress ?? 0,
					productionElapsed,
					{
						hasWorker: worker !== null,
						workerIsArchitect: worker?.specialization === "architect",
						materialsAvailable: patchedResources.materials,
					},
				);
				if (step.refinedProduced > 0) {
					patchedResources.materials = Math.max(
						0,
						patchedResources.materials - step.materialsUsed,
					);
					patchedResources.refined =
						(patchedResources.refined ?? 0) + step.refinedProduced;
					logEvent(
						tx,
						colony._id,
						"production",
						`${worker?.name ?? "The workshop"} refined ${step.materialsUsed} materials into ${step.refinedProduced} refined good${step.refinedProduced === 1 ? "" : "s"}.`,
						worker ? [worker._id] : [],
					);
				}
				if (step.nextProgress !== (building.productionProgress ?? 0)) {
					tx.update(buildings)
						.set({ productionProgress: step.nextProgress })
						.where(eq(buildings._id, building._id))
						.run();
				}
			}
			// A staffed smithy forges refined + materials into weapons and armor
			// for the armory — the arsenal the warriors draw on when raiders come.
			if (building.type === "smithy") {
				const worker = workshopWorkers.get(building._id) ?? null;
				const step = advanceSmithy(
					building.productionProgress ?? 0,
					productionElapsed,
					{
						hasWorker: worker !== null,
						workerIsFast: worker?.specialization === "architect",
						refinedAvailable: patchedResources.refined ?? 0,
						materialsAvailable: patchedResources.materials,
					},
				);
				if (step.weaponsProduced > 0 || step.armorProduced > 0) {
					patchedResources.refined = Math.max(
						0,
						(patchedResources.refined ?? 0) - step.refinedUsed,
					);
					patchedResources.materials = Math.max(
						0,
						patchedResources.materials - step.materialsUsed,
					);
					patchedResources.weapons =
						(patchedResources.weapons ?? 0) + step.weaponsProduced;
					patchedResources.armor =
						(patchedResources.armor ?? 0) + step.armorProduced;
					logEvent(
						tx,
						colony._id,
						"production",
						`${worker?.name ?? "The smith"} forged ${step.weaponsProduced} weapon${step.weaponsProduced === 1 ? "" : "s"} and ${step.armorProduced} armor at the smithy.`,
						worker ? [worker._id] : [],
					);
				}
				if (step.nextProgress !== (building.productionProgress ?? 0)) {
					tx.update(buildings)
						.set({ productionProgress: step.nextProgress })
						.where(eq(buildings._id, building._id))
						.run();
				}
			}
		}
		// --- Research: staffed research huts (and, faintly, schools) accrue
		// points toward the upgrade tree; the cats then auto-unlock the
		// cheapest affordable node on their own. Persisted on the colony so it
		// survives run resets, like the god upgrades.
		let tree = upgradeTree;
		const researchHutIds = new Set(
			colonyBuildings
				.filter(
					(b) => b.type === "research_hut" && b.constructionProgress >= 100,
				)
				.map((b) => b._id),
		);
		const researcherCount = aliveCats.filter(
			(cat) =>
				cat.assignedBuildingId && researchHutIds.has(cat.assignedBuildingId),
		).length;
		const schoolCount = colonyBuildings.filter(
			(b) => b.type === "school" && b.constructionProgress >= 100,
		).length;
		// Each school trickles a little research from kittens at their books —
		// a quarter-scholar apiece.
		const researchWorkforce = researcherCount + schoolCount * 0.25;
		const researchGained = pointsPerTickFor(
			researchWorkforce,
			productionElapsed,
			effects.researchRateMult,
		);
		if (researchGained > 0) {
			tree = accrueResearch(tree, researchGained);
		}
		let autoUnlock = catAutoUnlock(tree);
		while (autoUnlock.ok) {
			tree = autoUnlock.state;
			const node = autoUnlock.nodeId ? getNode(autoUnlock.nodeId) : null;
			logEvent(
				tx,
				colony._id,
				"research_unlocked",
				`The cats discovered ${node?.name ?? autoUnlock.nodeId}!`,
			);
			autoUnlock = catAutoUnlock(tree);
		}
		if (tree !== upgradeTree) {
			saveUpgradeTree(tx, colony._id, tree);
		}

		let automationTier = colony.automationTier ?? 0;
		let globalUpgradePoints = colony.globalUpgradePoints ?? 0;

		for (const cat of aliveCats) {
			const survival = applySurvivalTick(
				cat.needs,
				{
					food: patchedResources.food,
					water: patchedResources.water,
				},
				elapsedSec * runtime.resourceDecayMultiplier,
				{
					needsDecayMultiplier: policy.needsDecayMultiplier,
					needsDamageMultiplier: policy.needsDamageMultiplier,
				},
			);

			tx.update(cats)
				.set({ needs: survival.nextNeeds })
				.where(eq(cats._id, cat._id))
				.run();

			if (survival.dehydratingStarted) {
				logEvent(tx, colony._id, "crisis", `${cat.name} started dehydrating.`, [
					cat._id,
				]);
			}

			if (survival.recoveredFromDehydration) {
				logEvent(
					tx,
					colony._id,
					"recovery",
					`${cat.name} recovered from dehydration.`,
					[cat._id],
				);
			}

			if (survival.died) {
				// A dying carrier's yield is salvaged rather than lost.
				if (cat.carrying) {
					if (cat.carrying.kind === "food") {
						patchedResources.food += cat.carrying.amount;
					} else if (cat.carrying.kind === "materials") {
						patchedResources.materials += cat.carrying.amount;
					} else if (cat.carrying.kind === "water") {
						patchedResources.water += cat.carrying.amount;
					} else {
						globalUpgradePoints += cat.carrying.amount;
					}
				}
				retireCat(tx, colony._id, cat, now);
				logEvent(
					tx,
					colony._id,
					"death",
					`${cat.name} died from ${
						survival.nextNeeds.thirst === 0 && survival.nextNeeds.hunger === 0
							? "starvation and dehydration"
							: survival.nextNeeds.thirst === 0
								? "dehydration"
								: "starvation"
					}.`,
					[cat._id],
				);
			}
		}

		const livingCats = getAliveCats(tx, colony._id);
		if (livingCats.length === 0) {
			resetGlobalRun(
				tx,
				{
					...colony,
					resources: patchedResources,
					automationTier,
					runStartedAt: colony.runStartedAt ?? colony.createdAt,
				},
				"all-cats-dead",
			);
			return { ok: true, reset: true };
		}

		// Complete due jobs.
		const activeJobsAtCompletion = getJobsByStatus(tx, colony._id, "active");
		const dueJobs = activeJobsAtCompletion.filter((job) => job.endsAt <= now);
		const activeOrQueuedJobs: Array<{
			kind: JobKind;
			metadata?: Record<string, unknown>;
		}> = [
			...activeJobsAtCompletion.map((job) => ({
				kind: job.kind,
				metadata: job.metadata ?? undefined,
			})),
			...queuedJobs.map((job) => ({
				kind: job.kind,
				metadata: job.metadata ?? undefined,
			})),
		];

		for (const job of dueJobs) {
			const assignedCat = job.assignedCatId
				? (tx
						.select()
						.from(cats)
						.where(and(eq(cats._id, job.assignedCatId), isNull(cats.deathTime)))
						.get() ?? null)
				: null;

			if (job.kind === "supply_food" || job.kind === "supply_water") {
				const resourceKey = job.kind === "supply_food" ? "food" : "water";
				patchedResources[resourceKey] += 8;

				if (job.requestedByPlayerId) {
					const player = tx
						.select()
						.from(players)
						.where(eq(players._id, job.requestedByPlayerId))
						.get();
					if (player) {
						tx.update(players)
							.set({
								lifetimeContribution: {
									...player.lifetimeContribution,
									[resourceKey]: player.lifetimeContribution[resourceKey] + 8,
								},
							})
							.where(eq(players._id, player._id))
							.run();
					}
				}
			}

			if (job.kind === "leader_plan_hunt" && canTakePolicyAction()) {
				const hunter = selectBestCat(tx, colony._id, "hunter");
				queueJob(
					tx,
					colony._id,
					"hunt_expedition",
					"leader",
					upgrades,
					runtime,
					null,
					hunter,
				);
				activeOrQueuedJobs.push({ kind: "hunt_expedition" });
			}

			if (job.kind === "leader_plan_house") {
				queuePlannedHouseJobs(
					tx,
					colony._id,
					patchedResources,
					policy,
					activeOrQueuedJobs,
					upgrades,
					runtime,
					canTakePolicyAction,
				);
			}

			if (job.kind === "hunt_expedition" && assignedCat) {
				const roleXp = defaultRoleXp(assignedCat);
				const meta = (job.metadata as Record<string, unknown> | null) ?? {};
				// Mid-job trips may already have hauled shares home; the
				// completion haul carries exactly what's left of the catch.
				const total =
					typeof meta.totalYield === "number"
						? meta.totalYield
						: huntYieldFor(
								assignedCat,
								roleXp.hunter,
								upgrades,
								effects.huntYieldMult,
							);
				const tripsDone =
					typeof meta.tripsDone === "number" ? meta.tripsDone : 0;
				const reward = remainingYield(total, HUNT_TRIP_COUNT, tripsDone);

				// The completion haul drains the last of the catch from the site.
				drainHuntSite(meta.site as WorldPos | undefined, reward);

				const nextRoleXp = { ...roleXp, hunter: roleXp.hunter + 1 };
				tx.update(cats)
					.set({
						roleXp: nextRoleXp,
						specialization: nextSpecialization(
							"hunter",
							nextRoleXp.hunter,
							assignedCat.specialization ?? null,
						),
						stats: {
							...assignedCat.stats,
							hunting: Math.min(100, assignedCat.stats.hunting + 0.4),
						},
						// The last share is carried home and credited at the shrine.
						carrying:
							reward > 0
								? { kind: "food", amount: reward, jobEndedAt: now }
								: null,
					})
					.where(eq(cats._id, assignedCat._id))
					.run();
			}

			if (job.kind === "quarry" && assignedCat) {
				const meta = (job.metadata as Record<string, unknown> | null) ?? {};
				// Mirrors the hunt haul: trips may already have carried shares
				// home, so completion picks up exactly what's left of the load.
				const total =
					typeof meta.totalYield === "number"
						? meta.totalYield
						: QUARRY_TOTAL_YIELD;
				const tripsDone =
					typeof meta.tripsDone === "number" ? meta.tripsDone : 0;
				const reward = remainingYield(total, HUNT_TRIP_COUNT, tripsDone);
				tx.update(cats)
					.set({
						// The final load is carried to the shrine and banked there.
						carrying:
							reward > 0
								? { kind: "materials", amount: reward, jobEndedAt: now }
								: null,
					})
					.where(eq(cats._id, assignedCat._id))
					.run();
			}

			if (job.kind === "fetch_water" && assignedCat) {
				const meta = (job.metadata as Record<string, unknown> | null) ?? {};
				// Mirrors the quarry haul: trips may already have carried shares
				// home, so completion picks up exactly what's left of the load.
				const total =
					typeof meta.totalYield === "number"
						? meta.totalYield
						: WATER_TOTAL_YIELD;
				const tripsDone =
					typeof meta.tripsDone === "number" ? meta.tripsDone : 0;
				const reward = remainingYield(total, HUNT_TRIP_COUNT, tripsDone);
				tx.update(cats)
					.set({
						carrying:
							reward > 0
								? { kind: "water", amount: reward, jobEndedAt: now }
								: null,
					})
					.where(eq(cats._id, assignedCat._id))
					.run();
			}

			if (job.kind === "explore" && assignedCat) {
				const site = (job.metadata as Record<string, unknown> | null)?.site as
					| WorldPos
					| undefined;
				logEvent(
					tx,
					colony._id,
					"discovery",
					site
						? `${assignedCat.name} mapped the lands around (${Math.round(site.x)}, ${Math.round(site.y)}).`
						: `${assignedCat.name} mapped the lands around the village.`,
					[assignedCat._id],
				);
			}

			if (job.kind === "expand_village") {
				const meta = (job.metadata as Record<string, unknown> | null) ?? {};
				const rawTarget = (meta.target ?? meta.site) as WorldPos | undefined;
				if (
					typeof rawTarget?.x === "number" &&
					typeof rawTarget?.y === "number"
				) {
					const target = {
						x: Math.round(rawTarget.x),
						y: Math.round(rawTarget.y),
					};
					ensureChunk(
						tx,
						colony._id,
						chunkCoordForTile(target.x),
						chunkCoordForTile(target.y),
					);
					const tile = getTileAt(tx, colony._id, target);
					const currentColony = getColony(tx, colony._id);
					const currentArea = getClaimedArea(currentColony);
					if (
						!isInsideVillage(target, currentArea) &&
						isAdjacentToArea(target, currentArea) &&
						(!tile || !tileHasWater(tile))
					) {
						const grown = [...villageToTiles(currentArea), target];
						tx.update(colonies)
							.set({ claimedTiles: grown })
							.where(eq(colonies._id, colony._id))
							.run();
						clearClaimedTile(tx, colony._id, target, now);
						routeCache.clear();
						logEvent(
							tx,
							colony._id,
							"village_expanded",
							assignedCat
								? `${assignedCat.name} cleared new ground for the village at (${target.x}, ${target.y}).`
								: `The village claimed new ground at (${target.x}, ${target.y}).`,
							assignedCat ? [assignedCat._id] : [],
							{ target },
						);
					}
				}
			}

			if (job.kind === "build_house" && assignedCat) {
				const phase = String(
					(job.metadata as Record<string, unknown> | null)?.phase ??
						"gather_materials",
				);
				if (phase === "construct_house") {
					const buildingId = (job.metadata as Record<string, unknown> | null)
						?.buildingId;
					if (
						patchedResources.water >= policy.houseWaterRequired &&
						patchedResources.materials >= policy.houseMaterialsRequired
					) {
						patchedResources.water = Math.max(
							0,
							patchedResources.water - policy.houseWaterRequired,
						);
						patchedResources.materials = Math.max(
							0,
							patchedResources.materials - policy.houseMaterialsRequired,
						);
						automationTier =
							Math.round(Math.min(10, automationTier + 0.05) * 100) / 100;

						// The scaffold becomes a finished building.
						if (typeof buildingId === "string") {
							tx.update(buildings)
								.set({ constructionProgress: 100 })
								.where(eq(buildings._id, buildingId))
								.run();
							const builtType = String(
								(job.metadata as Record<string, unknown> | null)
									?.buildingType ?? "den",
							).replaceAll("_", " ");
							logEvent(
								tx,
								colony._id,
								"building_completed",
								`${assignedCat.name} finished building a new ${builtType}.`,
								[assignedCat._id],
							);
						}
					} else {
						// Not enough resources — abandon the scaffold and re-plan.
						if (typeof buildingId === "string") {
							tx.delete(buildings).where(eq(buildings._id, buildingId)).run();
						}
						queuePlannedHouseJobs(
							tx,
							colony._id,
							patchedResources,
							policy,
							activeOrQueuedJobs,
							upgrades,
							runtime,
						);
					}
				} else {
					patchedResources.materials += 12;

					// Materials come from felled timber: chop the nearest explored
					// forest tile down to a permanent field. Chopped trees never
					// grow back — the tile keeps a low food cap and the "field"
					// type forever.
					const exploredForests = tx
						.select()
						.from(worldTiles)
						.where(eq(worldTiles.colonyId, colony._id))
						.all()
						.filter((tile) => {
							if (!isForestType(tile.type)) {
								return false;
							}
							const dist = Math.max(
								Math.abs(tile.x - VILLAGE_ANCHOR.x),
								Math.abs(tile.y - VILLAGE_ANCHOR.y),
							);
							return tile.pathWear > 62 || dist <= 6;
						});
					if (exploredForests.length > 0) {
						let nearest = exploredForests[0];
						let nearestDist = Math.max(
							Math.abs(nearest.x - VILLAGE_ANCHOR.x),
							Math.abs(nearest.y - VILLAGE_ANCHOR.y),
						);
						for (const tile of exploredForests) {
							const dist = Math.max(
								Math.abs(tile.x - VILLAGE_ANCHOR.x),
								Math.abs(tile.y - VILLAGE_ANCHOR.y),
							);
							if (dist < nearestDist) {
								nearest = tile;
								nearestDist = dist;
							}
						}
						tx.update(worldTiles)
							.set({
								type: "field",
								resources: { ...nearest.resources, food: 0, herbs: 0 },
								maxResources: { ...nearest.maxResources, food: 5 },
								lastDepleted: now,
							})
							.where(eq(worldTiles._id, nearest._id))
							.run();
						logEvent(
							tx,
							colony._id,
							"forest_chopped",
							`${assignedCat.name} chopped the forest at (${nearest.x}, ${nearest.y}) for lumber.`,
							[assignedCat._id],
						);
					}
				}

				const roleXp = defaultRoleXp(assignedCat);
				const nextRoleXp = { ...roleXp, architect: roleXp.architect + 1 };
				tx.update(cats)
					.set({
						roleXp: nextRoleXp,
						specialization: nextSpecialization(
							"architect",
							nextRoleXp.architect,
							assignedCat.specialization ?? null,
						),
						stats: {
							...assignedCat.stats,
							building: Math.min(100, assignedCat.stats.building + 0.4),
						},
					})
					.where(eq(cats._id, assignedCat._id))
					.run();
			}

			if (job.kind === "ritual" && assignedCat) {
				const blessings = 1 + Math.floor(upgrades.ritual_mastery / 3);

				const roleXp = defaultRoleXp(assignedCat);
				const nextRoleXp = { ...roleXp, ritualist: roleXp.ritualist + 1 };
				tx.update(cats)
					.set({
						roleXp: nextRoleXp,
						specialization: nextSpecialization(
							"ritualist",
							nextRoleXp.ritualist,
							assignedCat.specialization ?? null,
						),
						// Blessings beam up once the ritualist reaches the shrine.
						carrying: {
							kind: "blessings",
							amount: blessings,
							jobEndedAt: now,
						},
					})
					.where(eq(cats._id, assignedCat._id))
					.run();
			}

			// Training graduates a cat into a warrior — the barracks turns a
			// recruit into a fighter who'll draw gear from the armory when
			// raiders come. Kittens are never enrolled (guarded at queue time).
			if (job.kind === "train_warrior" && assignedCat) {
				const roleXp = defaultRoleXp(assignedCat);
				tx.update(cats)
					.set({
						specialization: "warrior",
						roleXp: { ...roleXp, warrior: (roleXp.warrior ?? 0) + 1 },
						stats: {
							...assignedCat.stats,
							attack: Math.min(100, assignedCat.stats.attack + 3),
							defense: Math.min(100, assignedCat.stats.defense + 3),
						},
						activity: "idle",
						currentTask: null,
					})
					.where(eq(cats._id, assignedCat._id))
					.run();
				logEvent(
					tx,
					colony._id,
					"warrior_trained",
					`${assignedCat.name} completed warrior training and joined the village guard.`,
					[assignedCat._id],
				);
			}

			// Working cats head back when the job wraps up — carriers
			// (hunters, quarriers, ritualists) make for the shrine to deposit;
			// scouts just walk home.
			if (
				assignedCat &&
				(job.kind === "hunt_expedition" ||
					job.kind === "build_house" ||
					job.kind === "ritual" ||
					job.kind === "quarry" ||
					job.kind === "explore" ||
					job.kind === "fetch_water" ||
					job.kind === "expand_village")
			) {
				const homeSpot =
					job.kind === "build_house"
						? pickWanderTarget(
								VILLAGE_ANCHOR,
								nextMovementRoll(),
								nextMovementRoll(),
							)
						: VILLAGE_ANCHOR;
				tx.update(cats)
					.set({
						destination: { map: "world", ...homeSpot },
						activity: "returning",
						currentTask: null,
					})
					.where(eq(cats._id, assignedCat._id))
					.run();
			}

			tx.update(jobs)
				.set({ status: "completed", completedAt: now })
				.where(eq(jobs._id, job._id))
				.run();

			logEvent(
				tx,
				colony._id,
				"job_completed",
				`Completed ${job.kind.replace(/_/g, " ")}.`,
				assignedCat ? [assignedCat._id] : [],
			);
		}

		// --- Mid-job hauling: hunters and quarriers at their site depart for
		// the shrine with a share of the load when a trip comes due (SC2-drone
		// style, idle-paced: trips are spread across the job duration).
		for (const job of getJobsByStatus(tx, colony._id, "active")) {
			const isQuarry = job.kind === "quarry";
			const isWaterFetch = job.kind === "fetch_water";
			if (
				(job.kind !== "hunt_expedition" && !isQuarry && !isWaterFetch) ||
				!job.assignedCatId
			) {
				continue;
			}
			const meta = (job.metadata as Record<string, unknown> | null) ?? {};
			if (meta.accepted !== true || !meta.site) {
				continue;
			}
			const tripsDone = typeof meta.tripsDone === "number" ? meta.tripsDone : 0;
			if (tripsDone >= HUNT_TRIP_COUNT - 1) {
				continue; // only the completion haul remains
			}
			const nextTripAt =
				typeof meta.nextTripAt === "number"
					? meta.nextTripAt
					: tripDueAt(job.startedAt, job.endsAt, tripsDone + 1);
			if (now < nextTripAt || job.endsAt <= now) {
				continue;
			}
			const worker = tx
				.select()
				.from(cats)
				.where(and(eq(cats._id, job.assignedCatId), isNull(cats.deathTime)))
				.get();
			if (!worker || worker.activity !== "working" || worker.carrying) {
				continue;
			}

			// The full load is sized once so trips + completion sum exactly.
			const workerRoleXp = defaultRoleXp(worker);
			const total =
				typeof meta.totalYield === "number"
					? meta.totalYield
					: isWaterFetch
						? WATER_TOTAL_YIELD
						: isQuarry
							? QUARRY_TOTAL_YIELD
							: huntYieldFor(
									worker,
									workerRoleXp.hunter,
									upgrades,
									effects.huntYieldMult,
								);
			const share = splitYield(total, HUNT_TRIP_COUNT, tripsDone);

			// Hunt shares eat into the site's food; quarry stone and river
			// water are inexhaustible.
			if (!isQuarry && !isWaterFetch) {
				drainHuntSite(meta.site as WorldPos | undefined, share);
			}

			tx.update(jobs)
				.set({
					metadata: {
						...meta,
						totalYield: total,
						tripsDone: tripsDone + 1,
						nextTripAt: tripDueAt(job.startedAt, job.endsAt, tripsDone + 2),
					},
				})
				.where(eq(jobs._id, job._id))
				.run();
			tx.update(cats)
				.set({
					carrying: {
						kind: isWaterFetch ? "water" : isQuarry ? "materials" : "food",
						amount: share,
						jobEndedAt: now,
					},
					destination: { map: "world", ...VILLAGE_ANCHOR },
					activity: "returning",
				})
				.where(eq(cats._id, worker._id))
				.run();
		}

		// --- Movement pass: cats walk to job sites, come home, or wander.
		// Cosmetic only — the economy stays on job timers above.
		const movementElapsed = elapsedSec * runtime.timeScale;
		const wanderChance = Math.min(0.08, 0.02 * elapsedSec);
		// Fence/clearing radius grows as the village fills — the gate, fence
		// crossing, and "outside the village" reveal all key off it so the
		// server and client agree on where the palisade sits.
		const ringRadius = villageRingRadius(colonyBuildings.length);
		// Organic village footprint: the palisade and gate derive from the claimed
		// shape (lib/game/villageArea.ts), so the fence hugs the actual village.
		const movementColony = getColony(tx, colony._id);
		const claimedArea = getClaimedArea(movementColony);
		// Organic growth: when buildable ground runs low or the settlement gets
		// crowded, the leader queues a small cat job to claim ONE adjacent, dry
		// tile. Completion performs the mutation; this phase only selects and
		// records the target. Water is excluded via the known tiles.
		if (
			villageShouldExpand(
				aliveCats.length,
				claimedArea.size,
				colonyBuildings.length,
			) &&
			![
				...getJobsByStatus(tx, colony._id, "active"),
				...getJobsByStatus(tx, colony._id, "queued"),
			].some((job) => job.kind === "expand_village") &&
			canTakePolicyAction()
		) {
			const waterKeys = new Set(
				colonyTiles()
					.filter((t) => tileHasWater(t))
					.map((t) => `${t.x},${t.y}`),
			);
			const next = villageExpand(claimedArea, {
				isWater: (p) => waterKeys.has(`${p.x},${p.y}`),
			});
			if (next) {
				queueJob(
					tx,
					colony._id,
					"expand_village",
					"leader",
					upgrades,
					runtime,
					null,
					selectBestCat(tx, colony._id, "architect"),
					{ target: next },
				);
			}
		}
		const areaGate = claimedGate(claimedArea);
		// Gate passage tile (the tile just outside the gate edge). Used by the
		// straight-walk fallback and the "at the gate" check; at founding it equals
		// the historical south gate, and it tracks the organic gate as the village
		// grows. Falls back to the old south gate if the area has no perimeter.
		const gate = areaGate
			? {
					x: areaGate.x + VILLAGE_SIDE_DELTA[areaGate.side].x,
					y: areaGate.y + VILLAGE_SIDE_DELTA[areaGate.side].y,
				}
			: { x: VILLAGE_ANCHOR.x, y: VILLAGE_ANCHOR.y + ringRadius };
		// Walkability for real pathing this tick: rivers block, the palisade blocks
		// crossing the claimed boundary except at the gate, roads are cheap. Built
		// once from the tiles already cached above and shared by cats and raiders.
		// Terrain floors/stairs (same seed the client renders) so cliffs block a
		// route unless a staircase bridges them; a mesa with no stairs just falls
		// back to the straight walk, so cats never freeze.
		const terrainSeed = colony.worldSeed ?? colony.createdAt;
		const walkGrid = buildColonyWalkGrid({
			tiles: colonyTiles(),
			anchor: VILLAGE_ANCHOR,
			ringRadius,
			gate,
			area: claimedArea,
			areaGate,
			terrain: {
				heightAt: (x, y) =>
					terrainHeightAt(x, y, terrainSeed, WORLD_TERRAIN_OPTIONS),
				hasStair: (x, y) =>
					terrainStairAt(x, y, terrainSeed, WORLD_TERRAIN_OPTIONS),
			},
		});
		for (const cat of getAliveCats(tx, colony._id)) {
			const worldPos: WorldPos =
				cat.position.map === "world"
					? { x: cat.position.x, y: cat.position.y }
					: colonyToWorld({ x: cat.position.x, y: cat.position.y });
			const destination = cat.destination
				? { x: cat.destination.x, y: cat.destination.y }
				: null;
			const activity = cat.activity ?? "idle";

			// Carried yields deposit at the shrine — or force-credit once the
			// grace window runs out, so cosmetics can't lose resources.
			if (
				cat.carrying &&
				shouldDeposit(cat.carrying, worldPos, VILLAGE_ANCHOR, now)
			) {
				if (cat.carrying.kind === "food") {
					patchedResources.food += cat.carrying.amount;
				} else if (cat.carrying.kind === "materials") {
					patchedResources.materials += cat.carrying.amount;
				} else if (cat.carrying.kind === "water") {
					patchedResources.water += cat.carrying.amount;
				} else {
					globalUpgradePoints += cat.carrying.amount;
				}
				tx.update(cats)
					.set({ carrying: null })
					.where(eq(cats._id, cat._id))
					.run();
				logEvent(
					tx,
					colony._id,
					"shrine_deposit",
					cat.carrying.kind === "food"
						? `${cat.name} delivered ${Math.round(cat.carrying.amount)} food to the shrine.`
						: cat.carrying.kind === "materials"
							? `${cat.name} hauled ${Math.round(cat.carrying.amount)} materials to the shrine.`
							: cat.carrying.kind === "water"
								? `${cat.name} carried ${Math.round(cat.carrying.amount)} water to the shrine.`
								: `${cat.name}'s ritual beamed ${cat.carrying.amount} blessing${cat.carrying.amount === 1 ? "" : "s"} up to the players.`,
					[cat._id],
					{ kind: cat.carrying.kind, amount: cat.carrying.amount },
				);

				// Mid-job haulers head straight back to their site for the
				// next collection instead of settling down at the shrine.
				const ongoingJob = getJobsByStatus(tx, colony._id, "active").find(
					(job) =>
						job.assignedCatId === cat._id &&
						(job.kind === "hunt_expedition" ||
							job.kind === "quarry" ||
							job.kind === "fetch_water") &&
						job.endsAt > now,
				);
				const ongoingSite = (
					ongoingJob?.metadata as Record<string, unknown> | null
				)?.site as WorldPos | undefined;
				if (ongoingJob && ongoingSite) {
					tx.update(cats)
						.set({
							destination: { map: "world", ...ongoingSite },
							activity: "traveling",
						})
						.where(eq(cats._id, cat._id))
						.run();
					continue;
				}
			}

			if (!destination) {
				routeCache.delete(cat._id); // no journey → drop any stale plan
				if (activity === "traveling" || activity === "returning") {
					// Lost its destination (legacy row / interrupted job) — settle.
					tx.update(cats)
						.set({ activity: "idle" })
						.where(eq(cats._id, cat._id))
						.run();
				} else if (activity === "idle" && nextMovementRoll() < wanderChance) {
					// Assigned workers linger around their workplace.
					const assignedShop = cat.assignedBuildingId
						? (colonyBuildings.find(
								(building) => building._id === cat.assignedBuildingId,
							) ?? null)
						: null;
					const wanderAnchor = assignedShop
						? colonyToWorld(assignedShop.position)
						: VILLAGE_ANCHOR;
					const target = pickWanderTarget(
						wanderAnchor,
						nextMovementRoll(),
						nextMovementRoll(),
					);
					const blocked = zoneList.some(
						(zone) => zone.kind === "avoid" && isInZone(target, zone),
					);
					if (
						!blocked &&
						(target.x !== worldPos.x || target.y !== worldPos.y)
					) {
						tx.update(cats)
							.set({ destination: { map: "world", ...target } })
							.where(eq(cats._id, cat._id))
							.run();
					}
				}
				continue;
			}

			const standingTile = tx
				.select()
				.from(worldTiles)
				.where(
					and(
						eq(worldTiles.colonyId, colony._id),
						eq(worldTiles.x, Math.round(worldPos.x)),
						eq(worldTiles.y, Math.round(worldPos.y)),
					),
				)
				.get();
			// Explorers pick their way slowly through the fog on the way out;
			// once they're done and heading home they move at normal pace.
			const exploreSlowdown =
				cat.currentTask === "explore" && activity === "traveling"
					? EXPLORE_SPEED_FACTOR
					: 1;
			const speed =
				MOVE_SPEED_TILES_PER_SEC *
				(1 +
					(standingTile?.overlayFeature === "road_built"
						? 0.6
						: getPathSpeedBonus(standingTile?.pathWear ?? 0))) *
				exploreSlowdown *
				effects.moveSpeedMult;
			// Real pathing: A* over the walkability grid finds the route around
			// rivers and out through the fence's one gate. On open ground it
			// returns the straight x-before-y L, so ordinary trips are unchanged
			// and just as cheap. The intermediate tiles become walkPath's
			// waypoints, so the whole tick's budget is still spent tile-by-tile
			// (and every tile worn) even on a huge accelerated step. If no route
			// fits the search budget, fall back to a straight walk to the gate.
			const atGate =
				Math.abs(worldPos.x - gate.x) < 1 && Math.abs(worldPos.y - gate.y) < 1;
			const route = routeForCat(cat._id, worldPos, destination, walkGrid);
			const crossesFence =
				isInsideVillage(
					{ x: Math.round(worldPos.x), y: Math.round(worldPos.y) },
					claimedArea,
				) !==
				isInsideVillage(
					{ x: Math.round(destination.x), y: Math.round(destination.y) },
					claimedArea,
				);
			const waypoints =
				route && route.length > 2
					? route.slice(1, -1)
					: crossesFence && !atGate
						? [gate]
						: [];
			const walk = walkPath(
				worldPos,
				destination,
				movementElapsed * speed,
				waypoints,
			);
			const step = { position: walk.position, arrived: walk.arrived };
			let arrived = walk.arrived;

			// Reporting in at the shrine: pick up the job and head out.
			if (arrived && activity === "traveling") {
				const activeJob = getJobsByStatus(tx, colony._id, "active").find(
					(job) => job.assignedCatId === cat._id,
				);
				const meta = activeJob?.metadata as Record<string, unknown> | null;
				const site = meta?.site as WorldPos | undefined;
				if (activeJob && site && meta?.accepted === false) {
					tx.update(jobs)
						.set({ metadata: { ...meta, accepted: true } })
						.where(eq(jobs._id, activeJob._id))
						.run();
					tx.update(cats)
						.set({
							position: {
								map: "world",
								x: step.position.x,
								y: step.position.y,
							},
							destination: { map: "world", ...site },
						})
						.where(eq(cats._id, cat._id))
						.run();
					continue;
				}
				arrived = true;
			}
			const moved =
				step.position.x !== worldPos.x || step.position.y !== worldPos.y;
			if (!moved && !arrived) {
				continue;
			}

			if (arrived) {
				routeCache.delete(cat._id); // journey done → free the cached plan
			}
			tx.update(cats)
				.set({
					position: { map: "world", x: step.position.x, y: step.position.y },
					...(arrived
						? {
								destination: null,
								activity:
									activity === "traveling"
										? ("working" as const)
										: ("idle" as const),
							}
						: {}),
				})
				.where(eq(cats._id, cat._id))
				.run();

			// Exploration: every tile the cat trod this tick wears toward a
			// visible road, and a fog halo around the whole route is revealed.
			// Walking the full segment (not just the landing tile) is what lets
			// roads form under the accelerated presets, where a cat can cross
			// many tiles in a single tick.
			if (moved) {
				// walkPath already listed every integer tile this walk crossed —
				// one source of truth shared with the interception helper.
				const walked = walk.tiles;

				// Ordinary cats reveal a 3x3; explorers sweep a wide 5x5.
				const revealRadius = cat.currentTask === "explore" ? 2 : 1;
				const walkedKeys = new Set(walked.map((w) => `${w.x},${w.y}`));
				const xs = walked.map((w) => w.x);
				const ys = walked.map((w) => w.y);
				const nearby = tx
					.select()
					.from(worldTiles)
					.where(
						and(
							eq(worldTiles.colonyId, colony._id),
							gte(worldTiles.x, Math.min(...xs) - revealRadius),
							lte(worldTiles.x, Math.max(...xs) + revealRadius),
							gte(worldTiles.y, Math.min(...ys) - revealRadius),
							lte(worldTiles.y, Math.max(...ys) + revealRadius),
						),
					)
					.all();
				for (const tile of nearby) {
					const outsideVillage = !isInsideVillage(
						{ x: tile.x, y: tile.y },
						claimedArea,
					);
					if (!outsideVillage) {
						continue; // the clearing inside the fence is already open ground
					}
					const onRoute = walkedKeys.has(`${tile.x},${tile.y}`);
					// Tiles actually trodden climb toward a road (>=70 renders a road
					// sprite). Halo tiles are only revealed (>=63 clears the >62
					// explored threshold) so reveal never spawns phantom roads.
					let nextWear = tile.pathWear;
					if (onRoute) {
						nextWear = Math.max(addPathWear(tile.pathWear, WALK_WEAR), 64);
					} else {
						const nearRoute = walked.some(
							(w) =>
								Math.max(Math.abs(w.x - tile.x), Math.abs(w.y - tile.y)) <=
								revealRadius,
						);
						if (nearRoute) {
							nextWear = Math.max(tile.pathWear, 63);
						}
					}
					if (nextWear !== tile.pathWear) {
						tx.update(worldTiles)
							.set({ pathWear: nextWear })
							.where(eq(worldTiles._id, tile._id))
							.run();
					}
				}
			}
		}

		// --- Deliberate roads: once a minute, if the colony can spare the
		// materials, the leader paves the most-trafficked trodden corridor
		// outside the fence into a permanent road. Routine leaves a mark — a
		// route walked day after day earns a road, which is then cheaper to walk
		// (the A* cost model) so it entrenches itself.
		if (minuteRolled && patchedResources.materials > ROAD_MATERIALS_RESERVE) {
			const paveBudget = Math.min(
				ROAD_MAX_PAVE_PER_BATCH,
				patchedResources.materials - ROAD_MATERIALS_RESERVE,
			);
			const wornTiles = tx
				.select()
				.from(worldTiles)
				.where(
					and(
						eq(worldTiles.colonyId, colony._id),
						gte(worldTiles.pathWear, ROAD_PAVE_WEAR),
					),
				)
				.all();
			const corridor = selectRoadCorridor(wornTiles, {
				anchor: VILLAGE_ANCHOR,
				ringRadius,
				maxTiles: paveBudget,
			});
			if (corridor.length > 0) {
				for (const pos of corridor) {
					tx.update(worldTiles)
						.set({ overlayFeature: "road_built", pathWear: 100 })
						.where(
							and(
								eq(worldTiles.colonyId, colony._id),
								eq(worldTiles.x, pos.x),
								eq(worldTiles.y, pos.y),
							),
						)
						.run();
				}
				patchedResources.materials -= corridor.length;
				logEvent(
					tx,
					colony._id,
					"road_built",
					`The leader had a well-worn trail paved into a road (${corridor.length} tile${corridor.length === 1 ? "" : "s"}).`,
				);
			}
		}

		// --- Threat director: build raid pressure, march the active warband,
		// and resolve the fight at the gate. Runs on its own forked roll chain
		// so the policy/movement/life chains stay byte-stable, and after the
		// movement pass so raiders and cats have both moved this tick. Loot is
		// taken straight from patchedResources before the storage clamp below.
		let raidSeed = rngSeed === null ? null : rngSeed + 3_000_003;
		const nextRaidRoll = () => {
			if (raidSeed === null) {
				return Math.random();
			}
			const roll = rollSeeded(raidSeed);
			raidSeed = roll.nextSeed;
			return roll.value;
		};
		const raidResult = runRaidDirector(
			tx,
			{ _id: colony._id },
			{
				now,
				elapsedGameSec: elapsedSec * runtime.timeScale,
				ringRadius,
				aliveCats: getAliveCats(tx, colony._id),
				effects: {
					combatPowerMult: effects.combatPowerMult,
					defenseMult: effects.defenseMult,
				},
				resources: patchedResources,
				roll: nextRaidRoll,
				pressure: colony.threatPressure ?? 0,
				colonyAgeSec:
					((now - (colony.runStartedAt ?? colony.createdAt)) / 1000) *
					runtime.timeScale,
				activeRaidId: colony.activeRaidId ?? null,
				raidClicks: colony.raidClicks ?? 0,
				walkGrid,
			},
		);
		const threatPressure = raidResult.pressure;
		// A raid that wiped the colony ends the run cleanly.
		if (getAliveCats(tx, colony._id).length === 0) {
			resetGlobalRun(
				tx,
				{
					...colony,
					resources: patchedResources,
					automationTier,
					runStartedAt: colony.runStartedAt ?? colony.createdAt,
				},
				"raid-wipeout",
			);
			return { ok: true, reset: true };
		}

		// Storage caps are the final word: deposits, field yields, and player
		// supplies this tick can push a store past its cap, so clamp every
		// resource down to what the buildings can actually hold.
		caps = storageCapacities(
			tx
				.select()
				.from(buildings)
				.where(eq(buildings.colonyId, colony._id))
				.all(),
			effects.storagePerLevelMult,
		);
		foodCapacity = caps.food;
		patchedResources.food = Math.min(patchedResources.food, caps.food);
		patchedResources.water = Math.min(patchedResources.water, caps.water);
		patchedResources.herbs = Math.min(patchedResources.herbs, caps.herbs);
		patchedResources.materials = Math.min(
			patchedResources.materials,
			caps.materials,
		);
		patchedResources.refined = Math.min(
			patchedResources.refined ?? 0,
			caps.refined,
		);
		patchedResources.weapons = Math.min(
			patchedResources.weapons ?? 0,
			caps.weapons,
		);
		patchedResources.armor = Math.min(patchedResources.armor ?? 0, caps.armor);

		const unattendedHours =
			(now - (colony.lastPlayerActivityAt ?? now)) / 3_600_000;
		const resilienceHours =
			runtime.resilienceHoursOverride ??
			getResilienceHours(upgrades, automationTier);

		let criticalSince = colony.criticalSince ?? null;
		if (
			shouldTrackCritical(patchedResources, unattendedHours, resilienceHours)
		) {
			if (!criticalSince) {
				criticalSince = now;
			}

			// Collapse if critical state exceeds configured threshold (default 5 min).
			if (
				shouldResetFromCritical(criticalSince, now, runtime.criticalMsOverride)
			) {
				resetGlobalRun(
					tx,
					{
						...colony,
						resources: patchedResources,
						automationTier,
						runStartedAt: colony.runStartedAt ?? colony.createdAt,
					},
					"unattended-collapse",
				);
				return { ok: true, reset: true };
			}
		} else {
			criticalSince = null;
		}

		if (colony.resources.water <= 3 && patchedResources.water > 6) {
			logEvent(
				tx,
				colony._id,
				"recovery",
				"Water reserves restored to safe levels.",
			);
		}

		const nextStatus = nextColonyStatus(patchedResources);

		tx.update(colonies)
			.set({
				resources: patchedResources,
				status: nextStatus,
				automationTier,
				globalUpgradePoints,
				criticalSince,
				threatPressure,
				lastTick: processedThrough,
				testRngSeed: rngSeed,
			})
			.where(eq(colonies._id, colony._id))
			.run();

		return {
			ok: true,
			colonyId: colony._id,
			resources: patchedResources,
			automationTier,
			globalUpgradePoints,
			policyTier,
			reset: false,
		};
	});
}
