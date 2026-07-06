/**
 * Drizzle SQLite Schema
 *
 * Ported from the former convex/schema.ts. Tables keep their names and
 * camelCase field names; row objects keep the `_id` property (mapped to
 * the `id` column) so API payload shapes match what the frontend already
 * expects.
 *
 * The legacy `tasks` and `encounters` tables were not ported — their only
 * consumer was the retired /colony/[id] UI. Encounters return with the
 * map combat phase later.
 */

import {
	index,
	integer,
	real,
	sqliteTable,
	text,
	uniqueIndex,
} from "drizzle-orm/sqlite-core";

export interface ColonyResources {
	food: number;
	water: number;
	herbs: number;
	materials: number;
	blessings: number;
	/** Workshop output (Phase 7). Missing on older rows — read as 0. */
	refined?: number;
	/** Smithy-forged weapons in the armory. Missing on older rows — read as 0. */
	weapons?: number;
	/** Smithy-forged armor in the armory. Missing on older rows — read as 0. */
	armor?: number;
}

export interface CatStatsJson {
	attack: number;
	defense: number;
	hunting: number;
	medicine: number;
	cleaning: number;
	building: number;
	leadership: number;
	vision: number;
}

export interface CatNeedsJson {
	hunger: number;
	thirst: number;
	rest: number;
	health: number;
}

export interface PositionJson {
	map: "colony" | "world";
	x: number;
	y: number;
}

/** Yield a cat is hauling back to the shrine. */
export interface CarryingJson {
	kind: "food" | "blessings" | "materials" | "water";
	amount: number;
	/** When the producing job completed — drives the grace window. */
	jobEndedAt: number;
}

export interface RoleXpJson {
	hunter: number;
	architect: number;
	ritualist: number;
	/** Warrior trade experience. Missing on older rows — read as 0. */
	warrior?: number;
}

export interface LifetimeContributionJson {
	food: number;
	water: number;
	jobsRequested: number;
	upgradesPurchased: number;
}

/**
 * Serialized upgrade-tree progression (lib/game/upgradeTree.ts). The gods'
 * and cats' shared tech tree — owned nodes plus the cats' accrued research
 * points. Persisted on the colony because it represents civilization-level
 * progress that survives run resets, like {@link globalUpgrades}.
 */
export interface UpgradeTreeStateJson {
	ownedNodeIds: string[];
	researchPoints: number;
}

export const colonies = sqliteTable(
	"colonies",
	{
		_id: text("id").primaryKey(),
		name: text("name").notNull(),
		leaderId: text("leaderId"),
		status: text("status", {
			enum: ["starting", "thriving", "struggling", "dead"],
		}).notNull(),
		resources: text("resources", { mode: "json" })
			.$type<ColonyResources>()
			.notNull(),
		gridSize: integer("gridSize").notNull(),
		createdAt: integer("createdAt").notNull(),
		lastTick: integer("lastTick").notNull(),
		lastAttack: integer("lastAttack").notNull(),
		worldSeed: integer("worldSeed"),

		// Browser-idle v2
		isGlobal: integer("isGlobal", { mode: "boolean" }),
		runNumber: integer("runNumber"),
		runStartedAt: integer("runStartedAt"),
		lastPlayerActivityAt: integer("lastPlayerActivityAt"),
		lastResetAt: integer("lastResetAt"),
		automationTier: real("automationTier"),
		globalUpgradePoints: real("globalUpgradePoints"),
		/**
		 * God/cat upgrade-tree progress. Null on legacy rows — read through
		 * deserializeUpgradeTreeState, which fills a fresh empty tree.
		 */
		upgradeTree: text("upgradeTree", {
			mode: "json",
		}).$type<UpgradeTreeStateJson>(),
		ritualRequestedAt: integer("ritualRequestedAt"),
		criticalSince: integer("criticalSince"),
		/**
		 * The organic village footprint: the list of claimed world tiles the
		 * settlement occupies (lib/game/villageArea.ts). Grows one tile at a time;
		 * the auto-fence, clearing, building sites and walkability are all derived
		 * from it. Null on legacy rows — seeded from the founding square on open.
		 */
		claimedTiles: text("claimedTiles", { mode: "json" }).$type<
			Array<{ x: number; y: number }>
		>(),
		/**
		 * Accrued raid pressure (lib/game/threat.ts). Builds with wealth, size,
		 * warriors and colony age; a raid launches when it crosses the spawn
		 * threshold. Null on legacy rows — read as 0.
		 */
		threatPressure: real("threatPressure"),
		/** Wall-clock ms of the last raid launch, for cadence pacing. */
		lastRaidAt: integer("lastRaidAt"),
		/** The in-progress raid grouping the {@link raiders} rows, if any. */
		activeRaidId: text("activeRaidId"),
		/** Player defense clicks banked against the active raid. */
		raidClicks: real("raidClicks"),
		testTimeScale: real("testTimeScale"),
		testResourceDecayMultiplier: real("testResourceDecayMultiplier"),
		testResilienceHoursOverride: real("testResilienceHoursOverride"),
		testCriticalMsOverride: integer("testCriticalMsOverride"),
		testRngSeed: integer("testRngSeed"),
	},
	(table) => [
		index("colonies_by_status").on(table.status),
		uniqueIndex("colonies_by_is_global").on(table.isGlobal),
	],
);

export const cats = sqliteTable(
	"cats",
	{
		_id: text("id").primaryKey(),
		colonyId: text("colonyId").notNull(),
		name: text("name").notNull(),
		parentIds: text("parentIds", { mode: "json" })
			.$type<Array<string | null>>()
			.notNull(),
		birthTime: integer("birthTime").notNull(),
		deathTime: integer("deathTime"),
		stats: text("stats", { mode: "json" }).$type<CatStatsJson>().notNull(),
		needs: text("needs", { mode: "json" }).$type<CatNeedsJson>().notNull(),
		currentTask: text("currentTask"),
		position: text("position", { mode: "json" })
			.$type<PositionJson>()
			.notNull(),
		destination: text("destination", {
			mode: "json",
		}).$type<PositionJson | null>(),
		carrying: text("carrying", { mode: "json" }).$type<CarryingJson | null>(),
		assignedBuildingId: text("assignedBuildingId"),
		activity: text("activity", {
			enum: ["idle", "traveling", "working", "returning"],
		})
			.notNull()
			.default("idle"),
		isPregnant: integer("isPregnant", { mode: "boolean" }).notNull(),
		pregnancyDueTime: integer("pregnancyDueTime"),
		/**
		 * Accumulated age in game-hours. Ticked up by
		 * elapsedSec * timeScale each tick, so aging tracks the same
		 * accelerated clock as jobs (and responds to advanceTime in tests)
		 * rather than wall-clock time. Life stage and old-age death read this.
		 */
		ageHours: real("ageHours").notNull().default(0),
		/**
		 * Target {@link ageHours} at which a pregnant cat gives birth. Null
		 * when not expecting. Comparing against the mother's own accumulated
		 * age keeps gestation on the accelerated clock.
		 */
		pregnancyDueAgeHours: real("pregnancyDueAgeHours"),
		/** The co-parent chosen at conception, for kitten trait inheritance. */
		pregnancyMateId: text("pregnancyMateId"),
		spriteParams: text("spriteParams", { mode: "json" }).$type<Record<
			string,
			unknown
		> | null>(),
		specialization: text("specialization", {
			enum: ["hunter", "architect", "ritualist", "warrior"],
		}),
		roleXp: text("roleXp", { mode: "json" }).$type<RoleXpJson | null>(),
	},
	(table) => [
		index("cats_by_colony").on(table.colonyId),
		index("cats_by_colony_alive").on(table.colonyId, table.deathTime),
	],
);

export const buildings = sqliteTable(
	"buildings",
	{
		_id: text("id").primaryKey(),
		colonyId: text("colonyId").notNull(),
		type: text("type", {
			enum: [
				"den",
				"food_storage",
				"water_bowl",
				"beds",
				"herb_garden",
				"nursery",
				"elder_corner",
				"walls",
				"mouse_farm",
				"shrine",
				"workshop",
				"field",
				"research_hut",
				"school",
				"smithy",
				"barracks",
				"bridge",
			],
		}).notNull(),
		level: integer("level").notNull(),
		position: text("position", { mode: "json" })
			.$type<{ x: number; y: number }>()
			.notNull(),
		constructionProgress: real("constructionProgress").notNull(),
		/** Accumulated workshop cycle time in seconds (Phase 7). */
		productionProgress: real("productionProgress").notNull().default(0),
	},
	(table) => [index("buildings_by_colony").on(table.colonyId)],
);

export const worldTiles = sqliteTable(
	"worldTiles",
	{
		_id: text("id").primaryKey(),
		colonyId: text("colonyId").notNull(),
		x: integer("x").notNull(),
		y: integer("y").notNull(),
		type: text("type").notNull(),
		resources: text("resources", { mode: "json" })
			.$type<{ food: number; herbs: number; water: number }>()
			.notNull(),
		maxResources: text("maxResources", { mode: "json" })
			.$type<{ food: number; herbs: number }>()
			.notNull(),
		dangerLevel: real("dangerLevel").notNull(),
		pathWear: real("pathWear").notNull(),
		lastDepleted: integer("lastDepleted").notNull(),
		overlayFeature: text("overlayFeature"),
	},
	(table) => [
		index("worldTiles_by_colony").on(table.colonyId),
		uniqueIndex("worldTiles_by_colony_position").on(
			table.colonyId,
			table.x,
			table.y,
		),
	],
);

export const events = sqliteTable(
	"events",
	{
		_id: text("id").primaryKey(),
		colonyId: text("colonyId").notNull(),
		catId: text("catId"),
		timestamp: integer("timestamp").notNull(),
		type: text("type").notNull(),
		message: text("message").notNull(),
		involvedCatIds: text("involvedCatIds", { mode: "json" })
			.$type<string[]>()
			.notNull(),
		metadata: text("metadata", { mode: "json" })
			.$type<Record<string, unknown>>()
			.notNull(),
	},
	(table) => [
		index("events_by_colony").on(table.colonyId),
		index("events_by_colony_time").on(table.colonyId, table.timestamp),
	],
);

export const players = sqliteTable(
	"players",
	{
		_id: text("id").primaryKey(),
		sessionId: text("sessionId").notNull(),
		nickname: text("nickname").notNull(),
		lastSeenAt: integer("lastSeenAt").notNull(),
		clickWindowStart: integer("clickWindowStart").notNull(),
		clicksInWindow: integer("clicksInWindow").notNull(),
		lifetimeClicks: integer("lifetimeClicks").notNull(),
		lifetimeContribution: text("lifetimeContribution", { mode: "json" })
			.$type<LifetimeContributionJson>()
			.notNull(),
	},
	(table) => [
		uniqueIndex("players_by_session").on(table.sessionId),
		index("players_by_last_seen").on(table.lastSeenAt),
	],
);

export const jobs = sqliteTable(
	"jobs",
	{
		_id: text("id").primaryKey(),
		colonyId: text("colonyId").notNull(),
		kind: text("kind", {
			enum: [
				"supply_food",
				"supply_water",
				"leader_plan_hunt",
				"hunt_expedition",
				"leader_plan_house",
				"build_house",
				"ritual",
				"quarry",
				"explore",
				"fetch_water",
				"train_warrior",
				"expand_village",
			],
		}).notNull(),
		status: text("status", {
			enum: ["queued", "active", "completed", "failed", "cancelled"],
		}).notNull(),
		requestedByType: text("requestedByType", {
			enum: ["player", "leader", "system"],
		}).notNull(),
		requestedByPlayerId: text("requestedByPlayerId"),
		assignedCatId: text("assignedCatId"),
		baseDurationSec: real("baseDurationSec").notNull(),
		speedMultiplier: real("speedMultiplier").notNull(),
		yieldMultiplier: real("yieldMultiplier").notNull(),
		clickTimeReducedSec: real("clickTimeReducedSec").notNull(),
		createdAt: integer("createdAt").notNull(),
		startedAt: integer("startedAt").notNull(),
		endsAt: integer("endsAt").notNull(),
		completedAt: integer("completedAt"),
		metadata: text("metadata", { mode: "json" }).$type<
			Record<string, unknown>
		>(),
	},
	(table) => [
		index("jobs_by_colony_status").on(table.colonyId, table.status),
		index("jobs_by_colony_end").on(table.colonyId, table.endsAt),
		index("jobs_by_player").on(table.requestedByPlayerId),
	],
);

export const elections = sqliteTable(
	"elections",
	{
		_id: text("id").primaryKey(),
		colonyId: text("colonyId").notNull(),
		kind: text("kind", { enum: ["election", "vote_kick"] }).notNull(),
		status: text("status", { enum: ["open", "resolved"] }).notNull(),
		candidateCatIds: text("candidateCatIds", { mode: "json" })
			.$type<string[]>()
			.notNull(),
		/** Leader on trial (vote_kick only). */
		targetCatId: text("targetCatId"),
		startedAt: integer("startedAt").notNull(),
		endsAt: integer("endsAt").notNull(),
		winnerCatId: text("winnerCatId"),
		runNumber: integer("runNumber").notNull(),
	},
	(table) => [
		index("elections_by_colony_status").on(table.colonyId, table.status),
	],
);

export const votes = sqliteTable(
	"votes",
	{
		_id: text("id").primaryKey(),
		electionId: text("electionId").notNull(),
		playerId: text("playerId").notNull(),
		/** Chosen candidate (election) or the kick target (vote_kick). */
		catId: text("catId").notNull(),
		createdAt: integer("createdAt").notNull(),
	},
	(table) => [
		index("votes_by_election").on(table.electionId),
		uniqueIndex("votes_by_election_player").on(
			table.electionId,
			table.playerId,
		),
	],
);

export const zones = sqliteTable(
	"zones",
	{
		_id: text("id").primaryKey(),
		colonyId: text("colonyId").notNull(),
		kind: text("kind", { enum: ["avoid", "gather"] }).notNull(),
		x1: integer("x1").notNull(),
		y1: integer("y1").notNull(),
		x2: integer("x2").notNull(),
		y2: integer("y2").notNull(),
		playerId: text("playerId").notNull(),
		createdAt: integer("createdAt").notNull(),
		expiresAt: integer("expiresAt").notNull(),
	},
	(table) => [
		index("zones_by_colony").on(table.colonyId),
		index("zones_by_colony_expires").on(table.colonyId, table.expiresAt),
		index("zones_by_player").on(table.playerId),
	],
);

/**
 * Enemy raider units on the map (Roadmap 4, Military). Every row belongs to a
 * single active raid (grouped by {@link colonies.activeRaidId}); they spawn at
 * a map edge and path toward the village gate, where the warband is resolved
 * against the mustered warriors. `hp` starts at `strength` and drops as player
 * defense clicks and warriors wear it down.
 */
export const raiders = sqliteTable(
	"raiders",
	{
		_id: text("id").primaryKey(),
		colonyId: text("colonyId").notNull(),
		raidId: text("raidId").notNull(),
		position: text("position", { mode: "json" })
			.$type<{ x: number; y: number }>()
			.notNull(),
		target: text("target", { mode: "json" })
			.$type<{ x: number; y: number }>()
			.notNull(),
		strength: real("strength").notNull(),
		hp: real("hp").notNull(),
		status: text("status", {
			enum: ["advancing", "engaging", "retreating", "dead"],
		})
			.notNull()
			.default("advancing"),
		spawnedAt: integer("spawnedAt").notNull(),
	},
	(table) => [
		index("raiders_by_colony").on(table.colonyId),
		index("raiders_by_raid").on(table.raidId),
	],
);

export const globalUpgrades = sqliteTable(
	"globalUpgrades",
	{
		_id: text("id").primaryKey(),
		colonyId: text("colonyId").notNull(),
		key: text("key", {
			enum: [
				"click_power",
				"supply_speed",
				"hunt_mastery",
				"build_mastery",
				"ritual_mastery",
				"resilience",
			],
		}).notNull(),
		level: integer("level").notNull(),
		maxLevel: integer("maxLevel").notNull(),
		baseCost: integer("baseCost").notNull(),
		description: text("description").notNull(),
	},
	(table) => [
		index("globalUpgrades_by_colony").on(table.colonyId),
		uniqueIndex("globalUpgrades_by_colony_key").on(table.colonyId, table.key),
	],
);

export const runHistory = sqliteTable(
	"runHistory",
	{
		_id: text("id").primaryKey(),
		colonyId: text("colonyId").notNull(),
		runNumber: integer("runNumber").notNull(),
		startedAt: integer("startedAt").notNull(),
		endedAt: integer("endedAt").notNull(),
		durationSec: integer("durationSec").notNull(),
		reason: text("reason").notNull(),
		finalResources: text("finalResources", { mode: "json" })
			.$type<ColonyResources>()
			.notNull(),
		activePlayers: integer("activePlayers").notNull(),
	},
	(table) => [
		index("runHistory_by_colony_run").on(table.colonyId, table.runNumber),
	],
);

export type ColonyRow = typeof colonies.$inferSelect;
export type CatRow = typeof cats.$inferSelect;
export type BuildingRow = typeof buildings.$inferSelect;
export type WorldTileRow = typeof worldTiles.$inferSelect;
export type EventRow = typeof events.$inferSelect;
export type PlayerRow = typeof players.$inferSelect;
export type JobRow = typeof jobs.$inferSelect;
export type GlobalUpgradeRow = typeof globalUpgrades.$inferSelect;
export type RunHistoryRow = typeof runHistory.$inferSelect;
export type ElectionRow = typeof elections.$inferSelect;
export type VoteRow = typeof votes.$inferSelect;
export type ZoneRow = typeof zones.$inferSelect;
export type RaiderRow = typeof raiders.$inferSelect;
