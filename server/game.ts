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

import { and, desc, eq, gt, gte, isNull, lte } from "drizzle-orm";
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
	runHistory,
	votes,
	type WorldTileRow,
	worldTiles,
} from "@/db/schema";
import { isForestType, regrowthAmount } from "@/lib/game/depletion";
import { KICK_THRESHOLD, tallyVotes } from "@/lib/game/elections";
import { inheritTraits, traitsToSpriteParams } from "@/lib/game/genetics";
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
import { type LeaderSnapshot, planLeaderActions } from "@/lib/game/leaderAI";
import {
	destinationForJob,
	EXPLORE_SPEED_FACTOR,
	MOVE_SPEED_TILES_PER_SEC,
	pickWanderTarget,
	type WorldPos,
	walkPath,
} from "@/lib/game/movement";
import { generateName } from "@/lib/game/naming";
import { addPathWear, getPathSpeedBonus } from "@/lib/game/paths";
import { configForTier, pickPolicyTier } from "@/lib/game/policy";
import {
	advanceWorkshop,
	fieldUnlocked,
	fieldYield,
	workshopUnlocked,
} from "@/lib/game/production";
import { rollSeeded } from "@/lib/game/seededRng";
import { shouldDeposit } from "@/lib/game/shrine";
import {
	countStorehouses,
	storageCapacities,
	storehouseCap,
} from "@/lib/game/storage";
import { applySurvivalTick } from "@/lib/game/survival";
import { configForPreset } from "@/lib/game/testAcceleration";
import {
	HUNT_TRIP_COUNT,
	remainingYield,
	splitYield,
	tripDueAt,
} from "@/lib/game/trips";
import {
	colonyToWorld,
	nextBuildingSite,
	ringCells,
	SHRINE_LOCAL,
	VILLAGE_ANCHOR,
	villageRingRadius,
} from "@/lib/game/villageLayout";
import { isInZone, pickTargetWithZones, type Zone } from "@/lib/game/zones";
import type { CatStats } from "@/types/game";

import { runElectionLifecycle } from "./elections";
import { countOnlinePlayers, upsertPlayer } from "./players";
import { initializeWorldMap } from "./worldMap";
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

/**
 * Wear a single traversal lays on a trodden tile. The first pass reveals the
 * tile (clamped to 64); a second pass over the same tile crosses the road
 * threshold (>=70), so shared corridors between shrine and work sites harden
 * into roads while a one-off scouting crossing stays bare explored ground.
 */
const WALK_WEAR = 8;
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

const DEFAULT_ROLE_XP = { hunter: 0, architect: 0, ritualist: 0 } as const;

/**
 * Stocked general storage for a fresh settlement — sized so 20 cats have
 * roughly 5 hours of food/water at base decay, plus starter materials.
 */
const STARTING_RESOURCES = {
	food: 100,
	water: 100,
	herbs: 16,
	materials: 24,
	blessings: 0,
	refined: 0,
} as const;

const STARTER_CAT_COUNT = 20;

function defaultRoleXp(cat: CatRow): {
	hunter: number;
	architect: number;
	ritualist: number;
} {
	return cat.roleXp ?? { ...DEFAULT_ROLE_XP };
}

function randomStat(min: number, max: number): number {
	return min + Math.floor(Math.random() * (max - min + 1));
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
				ritualRequestedAt: null,
				criticalSince: null,
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
	}

	ensureShrineAndWorld(db, colony._id);

	return getColony(db, colony._id);
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

	// Founding village around the shrine: dens housing the starter cats
	// (2 per den) plus a stocked general storage, all pre-built. Fixed
	// rolls keep the layout deterministic while looking organic.
	const starterBuildings: Array<{
		type: "den" | "food_storage";
		roll: number;
	}> = [
		{ type: "den", roll: 0.05 },
		{ type: "den", roll: 0.3 },
		{ type: "den", roll: 0.55 },
		{ type: "den", roll: 0.8 },
		{ type: "den", roll: 0.95 },
		{ type: "food_storage", roll: 0.4 },
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
				level: 1,
				position: site,
				constructionProgress: 100,
			})
			.run();
	}

	// First run for this colony: seed the starting 3x3 world chunks
	// (idempotent — skips chunks that already exist).
	initializeWorldMap(db, colonyId);
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
};

function selectBestCat(
	db: GameDb,
	colonyId: string,
	specialization: CatSpecialization,
): CatRow | null {
	const aliveCats = getAliveCats(db, colonyId);
	if (aliveCats.length === 0) {
		return null;
	}

	const preferred = aliveCats.filter(
		(cat) => (cat.specialization ?? null) === specialization,
	);
	const pool = preferred.length > 0 ? preferred : aliveCats;

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
	const duration = getScaledDurationSeconds(
		kind,
		specialization,
		upgrades,
		runtime.timeScale,
	);
	const now = Date.now();

	const jobId = nanoid();
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
		type: "workshop" | "field";
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
		const jobId = queueJob(
			tx,
			colony._id,
			"build_house",
			"player",
			upgrades,
			runtime,
			null,
			selectBestCat(tx, colony._id, "architect"),
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
			building.type !== "workshop" ||
			building.constructionProgress < 100
		) {
			throw new Error("That building cannot take a worker");
		}

		// One worker per workshop — displace any current occupant.
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

		const path: Array<{ x: number; y: number }> = [];
		const ax = Math.round(args.a.x);
		const ay = Math.round(args.a.y);
		const bx = Math.round(args.b.x);
		const by = Math.round(args.b.y);
		for (let x = ax; x !== bx; x += Math.sign(bx - ax)) {
			path.push({ x, y: ay });
		}
		for (let y = ay; y !== by + Math.sign(by - ay); y += Math.sign(by - ay)) {
			path.push({ x: bx, y });
		}
		if (path.length === 0) {
			path.push({ x: bx, y: by });
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
		villageRadius: villageRingRadius(colonyBuildings.length),
		buildings: colonyBuildings.map((building) => ({
			...building,
			worldPosition: colonyToWorld(building.position),
		})),
		storage: {
			// Per-resource caps derived from the finished storehouses, plus a
			// `foodCapacity` alias kept for the existing HUD.
			capacities: storageCapacities(colonyBuildings),
			foodCapacity: storageCapacities(colonyBuildings).food,
			titheRates: { food: 20, refined: 5 },
		},
		housing: {
			population: aliveCats.length,
			capacity: housingCapacity(colonyBuildings),
			pressure: housingPressure(
				aliveCats.length,
				housingCapacity(colonyBuildings),
			),
			villageLevel: villageLevel(colonyBuildings),
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
			.map((vote) => ({ playerId: vote.playerId, catId: vote.catId }));
		election = {
			_id: poll._id,
			endsAt: poll.endsAt,
			tally: tallyVotes(ballots),
			totalBallots: new Set(ballots.map((ballot) => ballot.playerId)).size,
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
				.map((vote) => vote.playerId),
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
				status: "active",
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

export function upsertPresence(
	db: GameDb,
	sessionId: string,
	nickname: string,
): string {
	return upsertPlayer(db, sessionId, nickname)._id;
}

export function workerTick(db: GameDb) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = ensureGlobalColony(tx);

		const now = Date.now();
		const elapsedSec = Math.max(0, Math.floor((now - colony.lastTick) / 1000));
		if (elapsedSec === 0) {
			// Sub-second tick — nothing to process yet
			tx.update(colonies)
				.set({ lastTick: now })
				.where(eq(colonies._id, colony._id))
				.run();
			return { ok: true, skipped: true };
		}

		const upgrades = upgradesToLevels(getUpgradeRows(tx, colony._id));
		const runtime = getRuntimeConfig(colony);

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

		const aliveCats = getAliveCats(tx, colony._id);

		// Storage: base stores plus each finished food storehouse.
		const colonyBuildingsEarly = tx
			.select()
			.from(buildings)
			.where(eq(buildings.colonyId, colony._id))
			.all();
		const caps = storageCapacities(colonyBuildingsEarly);
		const foodCapacity = caps.food;

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

		// A colony-local build cell sits on water when its world tile is a
		// river/pond — scaffolds must never rise there.
		const localCellIsWater = (local: WorldPos): boolean => {
			const world = colonyToWorld(local);
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
					requestedType === "food_storage"
						? (requestedType as "workshop" | "field" | "food_storage")
						: ("den" as const);
				const occupied = tx
					.select()
					.from(buildings)
					.where(eq(buildings.colonyId, colony._id))
					.all()
					.map((b) => b.position);
				const siteLocal = nextBuildingSite(
					occupied,
					nextMovementRoll(),
					undefined,
					localCellIsWater,
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
			const jobDestination = destinationForJob(job.kind, {
				anchor: VILLAGE_ANCHOR,
				shrine: VILLAGE_ANCHOR,
				foodTiles: huntTiles,
				roll: nextMovementRoll(),
				site: constructionSite ?? undefined,
				quarrySite,
				waterSite,
				exploreSite,
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
		const idleCatRows = aliveCats.filter(
			(cat) =>
				!busyIds.has(cat._id) &&
				!cat.assignedBuildingId &&
				(cat.activity ?? "idle") === "idle",
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

		const snapshot: LeaderSnapshot = {
			population: aliveCats.length,
			idleCats: idleCatRows.length,
			employedCats: aliveCats.length - idleCatRows.length,
			resources: {
				food: nextResources.food,
				refined: nextResources.refined ?? 0,
			},
			foodCapacity,
			materials: nextResources.materials,
			materialsCapacity: caps.materials,
			water: nextResources.water,
			waterCapacity: caps.water,
			housing: {
				capacity: housingCapacity(colonyBuildings),
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

		for (const decision of planLeaderActions(snapshot)) {
			switch (decision.kind) {
				case "hunt": {
					const hunters = [...availableIdle].sort(
						(a, b) => b.stats.hunting - a.stats.hunting,
					);
					let dispatched = 0;
					for (const hunter of hunters) {
						if (dispatched >= decision.count) {
							break;
						}
						if (!canTakePolicyAction()) {
							break;
						}
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
						claimIdle(hunter);
						dispatched += 1;
					}
					break;
				}
				case "cancel_hunts": {
					// Overflowing stores: call the hunts off; the cats walk home
					// and only pick up new work back at the shrine.
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
					break;
				}
				case "quarry": {
					// Best builders make the best miners.
					const miners = [...availableIdle].sort(
						(a, b) => b.stats.building - a.stats.building,
					);
					let dispatched = 0;
					for (const miner of miners) {
						if (dispatched >= decision.count) {
							break;
						}
						if (!canTakePolicyAction()) {
							break;
						}
						queueJob(
							tx,
							colony._id,
							"quarry",
							"leader",
							upgrades,
							runtime,
							null,
							miner,
						);
						claimIdle(miner);
						dispatched += 1;
					}
					break;
				}
				case "fetch_water": {
					// Any able-bodied cat can haul water; send the sturdiest idlers.
					const carriers = [...availableIdle].sort(
						(a, b) => b.stats.hunting - a.stats.hunting,
					);
					let dispatched = 0;
					for (const carrier of carriers) {
						if (dispatched >= decision.count) {
							break;
						}
						if (!canTakePolicyAction()) {
							break;
						}
						queueJob(
							tx,
							colony._id,
							"fetch_water",
							"leader",
							upgrades,
							runtime,
							null,
							carrier,
						);
						claimIdle(carrier);
						dispatched += 1;
					}
					break;
				}
				case "scout": {
					// Sharp-eyed cats scout best.
					const scouts = [...availableIdle].sort(
						(a, b) => b.stats.vision - a.stats.vision,
					);
					let dispatched = 0;
					for (const scout of scouts) {
						if (dispatched >= decision.count) {
							break;
						}
						if (!canTakePolicyAction()) {
							break;
						}
						queueJob(
							tx,
							colony._id,
							"explore",
							"leader",
							upgrades,
							runtime,
							null,
							scout,
						);
						claimIdle(scout);
						dispatched += 1;
					}
					break;
				}
				case "build_storage": {
					if (canTakePolicyAction()) {
						queueJob(
							tx,
							colony._id,
							"build_house",
							"leader",
							upgrades,
							runtime,
							null,
							selectBestCat(tx, colony._id, "architect"),
							{ phase: "construct_house", buildingType: "food_storage" },
						);
					}
					break;
				}
				case "build_den": {
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
					break;
				}
				case "assign_workshop": {
					let staffed = 0;
					for (const workshop of workshopsNeedingWorkers) {
						if (staffed >= decision.count) {
							break;
						}
						const idle = [...availableIdle].sort(
							(a, b) => b.stats.building - a.stats.building,
						)[0];
						if (!idle) {
							break;
						}
						if (!canTakePolicyAction()) {
							break;
						}
						tx.update(cats)
							.set({ assignedBuildingId: workshop._id })
							.where(eq(cats._id, idle._id))
							.run();
						workshopWorkers.set(workshop._id, idle);
						claimIdle(idle);
						logEvent(
							tx,
							colony._id,
							"worker_assigned",
							`The leader put ${idle.name} to work at the workshop.`,
							[idle._id],
						);
						staffed += 1;
					}
					break;
				}
				case "tithe": {
					// Surplus offering is capped to once a minute.
					if (!minuteRolled) {
						break;
					}
					nextResources.food -= decision.food;
					nextResources.refined =
						(nextResources.refined ?? 0) - decision.refined;
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
					break;
				}
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
				tx.update(cats)
					.set({ deathTime: now, currentTask: null, carrying: null })
					.where(eq(cats._id, cat._id))
					.run();
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
		const dueJobs = activeJobs.filter((job) => job.endsAt <= now);
		const activeOrQueuedJobs: Array<{
			kind: JobKind;
			metadata?: Record<string, unknown>;
		}> = [
			...activeJobs.map((job) => ({
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
						.where(eq(cats._id, job.assignedCatId))
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
						: getHuntReward(
								assignedCat.stats.hunting,
								assignedCat.specialization ?? null,
								roleXp.hunter,
								upgrades,
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
					job.kind === "fetch_water")
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
							: getHuntReward(
									worker.stats.hunting,
									worker.specialization ?? null,
									workerRoleXp.hunter,
									upgrades,
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
				exploreSlowdown;
			// The village fence blocks travel: crossing the ring means going
			// through the south gate first.
			const gate = { x: VILLAGE_ANCHOR.x, y: VILLAGE_ANCHOR.y + ringRadius };
			const ringDist = (p: WorldPos) =>
				Math.max(
					Math.abs(p.x - VILLAGE_ANCHOR.x),
					Math.abs(p.y - VILLAGE_ANCHOR.y),
				);
			const crossesFence =
				ringDist(worldPos) < ringRadius !== ringDist(destination) < ringRadius;
			const atGate =
				Math.abs(worldPos.x - gate.x) < 1 && Math.abs(worldPos.y - gate.y) < 1;
			// Walk the whole tick's budget tile-by-tile — through the south gate
			// when the route crosses the fence — so even a huge accelerated step
			// traverses (and wears) every tile instead of teleporting one leg.
			const waypoints = crossesFence && !atGate ? [gate] : [];
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
					const outsideVillage =
						Math.max(
							Math.abs(tile.x - VILLAGE_ANCHOR.x),
							Math.abs(tile.y - VILLAGE_ANCHOR.y),
						) > ringRadius;
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

		// Storage caps are the final word: deposits, field yields, and player
		// supplies this tick can push a store past its cap, so clamp every
		// resource down to what the buildings can actually hold.
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
				lastTick: now,
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
