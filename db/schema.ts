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

export interface RoleXpJson {
	hunter: number;
	architect: number;
	ritualist: number;
}

export interface LifetimeContributionJson {
	food: number;
	water: number;
	jobsRequested: number;
	upgradesPurchased: number;
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
		ritualRequestedAt: integer("ritualRequestedAt"),
		criticalSince: integer("criticalSince"),
		testTimeScale: real("testTimeScale"),
		testResourceDecayMultiplier: real("testResourceDecayMultiplier"),
		testResilienceHoursOverride: real("testResilienceHoursOverride"),
		testCriticalMsOverride: integer("testCriticalMsOverride"),
		testRngSeed: integer("testRngSeed"),
	},
	(table) => [
		index("colonies_by_status").on(table.status),
		index("colonies_by_is_global").on(table.isGlobal),
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
		isPregnant: integer("isPregnant", { mode: "boolean" }).notNull(),
		pregnancyDueTime: integer("pregnancyDueTime"),
		spriteParams: text("spriteParams", { mode: "json" }).$type<Record<
			string,
			unknown
		> | null>(),
		specialization: text("specialization", {
			enum: ["hunter", "architect", "ritualist"],
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
			],
		}).notNull(),
		level: integer("level").notNull(),
		position: text("position", { mode: "json" })
			.$type<{ x: number; y: number }>()
			.notNull(),
		constructionProgress: real("constructionProgress").notNull(),
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
