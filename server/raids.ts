/**
 * Raid director — the military threat loop (Roadmap 4).
 *
 * Runs inside {@link workerTick}'s transaction. With no raid in progress it
 * accrues threat pressure (lib/game/threat.ts) and, once pressure crosses the
 * spawn threshold, fields a warband at a map edge. While a raid is live it
 * marches the raiders toward the village gate and, when they reach it, resolves
 * the fight against the mustered warriors (lib/game/warriors.ts): gear is
 * consumed, the warband is driven off or the stores are looted, and events read
 * like dispatches from the front.
 *
 * All randomness comes from the injected `roll` (the tick's forked raid chain),
 * so raids are byte-stable under setTestRngSeed.
 */

import { and, eq, isNull } from "drizzle-orm";
import { nanoid } from "nanoid";

import type { GameDb } from "@/db/client";
import {
	type CatRow,
	type ColonyResources,
	cats,
	colonies,
	events,
	jobs,
	type RaiderRow,
	raiders,
} from "@/db/schema";
import { canWork, getLifeStage } from "@/lib/game/lifeSim";
import { type WorldPos, walkPath } from "@/lib/game/movement";
import { findPath, type WalkGrid } from "@/lib/game/pathfinding";
import {
	RAID_INTERCEPTION_WOUND_DAMAGE,
	resolveRaidInterception,
	selectRaidInterceptions,
} from "@/lib/game/raidInterception";
import {
	accrueThreat,
	colonyWealth,
	MAX_RAID_CASUALTIES,
	planRaid,
	type RaidPlan,
	resolveRaid,
	shouldSpawnRaid,
	type ThreatSnapshot,
} from "@/lib/game/threat";
import { VILLAGE_ANCHOR } from "@/lib/game/villageLayout";
import {
	canFight,
	type MusterCombatant,
	musterDefense,
	WARRIOR_XP_PER_RAID,
} from "@/lib/game/warriors";

/** Tiles a raider covers per game-second (a touch slower than a cat). */
export const RAIDER_SPEED_TILES_PER_SEC = 0.4;
/** How far outside the anchor a warband spawns. */
export const RAID_SPAWN_DISTANCE = 14;
/** A raider this close to the gate triggers the fight. */
export const ENGAGE_RANGE = 1.5;
/** Damage one player defense click deals to the frontmost raider. */
export const DEFEND_CLICK_DAMAGE = 6;
/** Stores a raid can carry off (keys mirror the lootable resource stores). */
const LOOTABLE: Array<keyof ColonyResources> = [
	"food",
	"water",
	"herbs",
	"materials",
	"refined",
];

function markCatDead(db: GameDb, catId: string, now: number): void {
	db.update(jobs)
		.set({ status: "cancelled", completedAt: now })
		.where(and(eq(jobs.assignedCatId, catId), eq(jobs.status, "active")))
		.run();
	db.update(jobs)
		.set({ status: "cancelled", completedAt: now })
		.where(and(eq(jobs.assignedCatId, catId), eq(jobs.status, "queued")))
		.run();
	db.update(cats)
		.set({
			deathTime: now,
			currentTask: null,
			carrying: null,
			assignedBuildingId: null,
			destination: null,
			activity: "idle",
		})
		.where(and(eq(cats._id, catId), isNull(cats.deathTime)))
		.run();
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

/** The village gate the raiders march on (south side of the fence ring). */
export function gatePosition(ringRadius: number): WorldPos {
	return { x: VILLAGE_ANCHOR.x, y: VILLAGE_ANCHOR.y + ringRadius };
}

/**
 * Cats eligible to defend: every able-bodied adult, young or elder cat turns
 * out (warriors and hunters at the front, the rest as militia). Kittens — the
 * only stage that `canWork` rejects — never fight.
 */
export function defenders(aliveCats: CatRow[]): MusterCombatant[] {
	return aliveCats
		.filter(
			(cat) =>
				canFight(cat.specialization ?? null) &&
				canWork(getLifeStage(cat.ageHours ?? 0)),
		)
		.map((cat) => ({
			id: cat._id,
			attack: cat.stats.attack,
			defense: cat.stats.defense,
			specialization: cat.specialization ?? null,
			warriorXp: cat.roleXp?.warrior ?? 0,
			lifeStage: getLifeStage(cat.ageHours ?? 0),
		}));
}

function getRaiders(db: GameDb, colonyId: string, raidId: string): RaiderRow[] {
	return db
		.select()
		.from(raiders)
		.where(and(eq(raiders.colonyId, colonyId), eq(raiders.raidId, raidId)))
		.all();
}

export interface SpawnOptions {
	/** Place the warband right at the gate so the next tick resolves it. */
	atGate?: boolean;
	/** Override the planned warband (tests / debug). */
	plan?: RaidPlan;
}

/**
 * Field a warband: create the raider rows at a map edge (or at the gate) and
 * mark the raid active on the colony. Returns the new raid id.
 */
export function spawnRaid(
	db: GameDb,
	colonyId: string,
	snapshot: ThreatSnapshot,
	ringRadius: number,
	roll: () => number,
	opts: SpawnOptions = {},
): string {
	const now = Date.now();
	const plan = opts.plan ?? planRaid(snapshot);
	const gate = gatePosition(ringRadius);

	// Approach from a roll-chosen compass direction, well outside the fence.
	const angle = roll() * Math.PI * 2;
	const origin = opts.atGate
		? gate
		: {
				x: Math.round(VILLAGE_ANCHOR.x + Math.cos(angle) * RAID_SPAWN_DISTANCE),
				y: Math.round(VILLAGE_ANCHOR.y + Math.sin(angle) * RAID_SPAWN_DISTANCE),
			};

	const raidId = nanoid();
	for (let i = 0; i < plan.count; i += 1) {
		// Fan the warband out a little so they don't stack on one tile.
		const jitterX = i % 3 === 0 ? 0 : i % 3 === 1 ? 1 : -1;
		const jitterY = Math.floor(i / 3) % 2 === 0 ? 0 : 1;
		db.insert(raiders)
			.values({
				_id: nanoid(),
				colonyId,
				raidId,
				position: opts.atGate
					? { x: gate.x, y: gate.y }
					: { x: origin.x + jitterX, y: origin.y + jitterY },
				target: { x: gate.x, y: gate.y },
				strength: plan.strengthEach,
				hp: plan.strengthEach,
				status: "advancing",
				spawnedAt: now,
			})
			.run();
	}

	db.update(colonies)
		.set({ activeRaidId: raidId, raidClicks: 0, lastRaidAt: now })
		.where(eq(colonies._id, colonyId))
		.run();

	logEvent(
		db,
		colonyId,
		"raid_incoming",
		`A warband of ${plan.count} raider${plan.count === 1 ? "" : "s"} was spotted advancing on the village!`,
		[],
		{ raidId, count: plan.count },
	);
	return raidId;
}

export interface RaidDirectorContext {
	now: number;
	/** elapsedSec * timeScale — the accelerated game-seconds this tick. */
	elapsedGameSec: number;
	ringRadius: number;
	aliveCats: CatRow[];
	effects: { combatPowerMult: number; defenseMult: number };
	/** Mutated in place when a raid loots the stores. */
	resources: ColonyResources;
	/** Forked seeded roll chain for the raid subsystem. */
	roll: () => number;
	/** Current accrued threat pressure. */
	pressure: number;
	/** Game-seconds since the run started (accelerated clock). */
	colonyAgeSec: number;
	/** Active raid id, or null when none in progress. */
	activeRaidId: string | null;
	/** Player defense clicks banked against the active raid. */
	raidClicks: number;
	/**
	 * Walkability for real pathing — raiders route around rivers and funnel to
	 * the gate through the fence just like cats. Omitted (tests / legacy) falls
	 * back to a straight march.
	 */
	walkGrid?: WalkGrid;
	/** Per-cat world tiles crossed during this tick's movement pass. */
	catMovementTrails?: Map<string, WorldPos[]>;
	/** Organic-fence test; defaults to the legacy square ring in older tests. */
	isInsideVillage?: (pos: WorldPos) => boolean;
}

export interface RaidDirectorResult {
	pressure: number;
	activeRaidId: string | null;
	/** Cats the raid killed this tick (already marked dead in the DB). */
	killedCatIds: string[];
}

/**
 * Advance the raid subsystem one tick. Either marches/resolves the active raid
 * or accrues pressure and possibly spawns a new warband.
 */
export function runRaidDirector(
	db: GameDb,
	colony: { _id: string },
	ctx: RaidDirectorContext,
): RaidDirectorResult {
	const colonyId = colony._id;
	const snapshot: ThreatSnapshot = {
		wealth: colonyWealth(ctx.resources),
		population: ctx.aliveCats.length,
		warriors: ctx.aliveCats.filter((c) => c.specialization === "warrior")
			.length,
		colonyAgeSec: ctx.colonyAgeSec,
	};

	if (ctx.activeRaidId) {
		return advanceActiveRaid(db, colonyId, ctx);
	}

	// No raid in flight — build pressure and maybe launch one.
	const pressure = accrueThreat(ctx.pressure, snapshot, ctx.elapsedGameSec);
	if (shouldSpawnRaid(pressure)) {
		spawnRaid(db, colonyId, snapshot, ctx.ringRadius, ctx.roll);
		return {
			pressure: 0,
			activeRaidId:
				db
					.select({ activeRaidId: colonies.activeRaidId })
					.from(colonies)
					.where(eq(colonies._id, colonyId))
					.get()?.activeRaidId ?? null,
			killedCatIds: [],
		};
	}
	return { pressure, activeRaidId: null, killedCatIds: [] };
}

function advanceActiveRaid(
	db: GameDb,
	colonyId: string,
	ctx: RaidDirectorContext,
): RaidDirectorResult {
	const raidId = ctx.activeRaidId as string;
	const units = getRaiders(db, colonyId, raidId).filter(
		(r) => r.status !== "dead" && r.hp > 0,
	);

	// Every raider already cut down (player clicks / prior fight): the raid is
	// broken before it lands.
	if (units.length === 0) {
		return endRaid(db, colonyId, raidId, ctx, "clicks");
	}

	const gate = gatePosition(ctx.ringRadius);
	const budget = ctx.elapsedGameSec * RAIDER_SPEED_TILES_PER_SEC;
	let anyAtGate = false;
	const movedUnits: Array<RaiderRow & { trail: WorldPos[] }> = [];
	for (const unit of units) {
		// Raiders path to the gate the same way cats do: A* around rivers and
		// the palisade, falling back to a straight march when no grid is given.
		const route = ctx.walkGrid
			? findPath(unit.position, gate, ctx.walkGrid)
			: null;
		const waypoints = route && route.length > 2 ? route.slice(1, -1) : [];
		const walk = walkPath(unit.position, gate, budget, waypoints);
		const atGate =
			Math.max(
				Math.abs(walk.position.x - gate.x),
				Math.abs(walk.position.y - gate.y),
			) <= ENGAGE_RANGE;
		if (atGate) {
			anyAtGate = true;
		}
		db.update(raiders)
			.set({
				position: walk.position,
				status: atGate ? "engaging" : "advancing",
			})
			.where(eq(raiders._id, unit._id))
			.run();
		movedUnits.push({
			...unit,
			position: walk.position,
			trail: walk.tiles,
			status: atGate ? "engaging" : "advancing",
		});
	}

	const ambushKills = resolveRaiderInterceptions(
		db,
		colonyId,
		raidId,
		ctx,
		movedUnits,
	);

	if (!anyAtGate) {
		return {
			pressure: ctx.pressure,
			activeRaidId: raidId,
			killedCatIds: ambushKills,
		};
	}

	return resolveActiveRaid(db, colonyId, raidId, ctx, units, ambushKills);
}

function defaultInsideVillage(ringRadius: number, pos: WorldPos): boolean {
	return (
		Math.max(
			Math.abs(Math.round(pos.x) - VILLAGE_ANCHOR.x),
			Math.abs(Math.round(pos.y) - VILLAGE_ANCHOR.y),
		) < ringRadius
	);
}

function raidCasualtiesSoFar(
	db: GameDb,
	colonyId: string,
	raidId: string,
): number {
	return db
		.select()
		.from(events)
		.where(eq(events.colonyId, colonyId))
		.all()
		.filter((event) => {
			const metadata = event.metadata as Record<string, unknown> | null;
			if (metadata?.raidId !== raidId) {
				return false;
			}
			return (
				event.type === "raid_casualty" ||
				(event.type === "raider_ambush" && metadata.outcome === "killed")
			);
		}).length;
}

function cancelCatFieldWork(db: GameDb, catId: string, now: number): void {
	db.update(jobs)
		.set({ status: "cancelled", completedAt: now })
		.where(and(eq(jobs.assignedCatId, catId), eq(jobs.status, "active")))
		.run();
	db.update(jobs)
		.set({ status: "cancelled", completedAt: now })
		.where(and(eq(jobs.assignedCatId, catId), eq(jobs.status, "queued")))
		.run();
}

function resolveRaiderInterceptions(
	db: GameDb,
	colonyId: string,
	raidId: string,
	ctx: RaidDirectorContext,
	units: Array<RaiderRow & { trail?: WorldPos[] }>,
): string[] {
	const inside =
		ctx.isInsideVillage ??
		((pos: WorldPos) => defaultInsideVillage(ctx.ringRadius, pos));
	const pairs = selectRaidInterceptions(
		units.map((unit) => ({
			id: unit._id,
			position: unit.position,
			trail: unit.trail,
			hp: unit.hp,
			strength: unit.strength,
			status: unit.status,
		})),
		ctx.aliveCats.map((cat) => ({
			id: cat._id,
			position:
				cat.position.map === "world"
					? { x: cat.position.x, y: cat.position.y }
					: {
							x: VILLAGE_ANCHOR.x + cat.position.x,
							y: VILLAGE_ANCHOR.y + cat.position.y,
						},
			trail: ctx.catMovementTrails?.get(cat._id),
			activity: cat.activity,
			currentTask: cat.currentTask,
			carrying: cat.carrying,
			deathTime: cat.deathTime,
			stats: cat.stats,
			specialization: cat.specialization ?? null,
			ageHours: cat.ageHours,
			roleXp: cat.roleXp,
		})),
		inside,
	);
	if (pairs.length === 0) {
		return [];
	}

	let casualties = raidCasualtiesSoFar(db, colonyId, raidId);
	const killedCatIds: string[] = [];
	for (const pair of pairs) {
		const cat = ctx.aliveCats.find((c) => c._id === pair.cat.id);
		const raider = units.find((r) => r._id === pair.raider.id);
		if (!cat || !raider || cat.deathTime != null) {
			continue;
		}
		const result = resolveRaidInterception(
			{
				stats: cat.stats,
				specialization: cat.specialization ?? null,
				ageHours: cat.ageHours,
				roleXp: cat.roleXp,
			},
			{ hp: raider.hp, strength: raider.strength },
			ctx.roll(),
			casualties,
		);
		const carried = cat.carrying
			? { kind: cat.carrying.kind, amount: cat.carrying.amount }
			: null;
		const metadata = {
			raidId,
			raiderId: raider._id,
			outcome: result.outcome,
			margin: result.margin,
			catPower: result.catPower,
			raiderPower: result.raiderPower,
			dropped: carried,
		};

		if (result.outcome === "escape") {
			logEvent(
				db,
				colonyId,
				"raider_ambush",
				`${cat.name} slipped past an advancing raider and kept going.`,
				[cat._id],
				metadata,
			);
			continue;
		}

		if (result.outcome === "killed") {
			markCatDead(db, cat._id, ctx.now);
			casualties += 1;
			killedCatIds.push(cat._id);
			logEvent(
				db,
				colonyId,
				"raider_ambush",
				`${cat.name} was caught beyond the fence by the raiders and did not return.`,
				[cat._id],
				metadata,
			);
			continue;
		}

		cancelCatFieldWork(db, cat._id, ctx.now);
		const health =
			result.outcome === "wounded"
				? Math.max(1, cat.needs.health - RAID_INTERCEPTION_WOUND_DAMAGE)
				: cat.needs.health;
		db.update(cats)
			.set({
				needs: { ...cat.needs, health },
				currentTask: null,
				carrying: null,
				destination: { map: "world", ...VILLAGE_ANCHOR },
				activity: "returning",
			})
			.where(eq(cats._id, cat._id))
			.run();
		logEvent(
			db,
			colonyId,
			"raider_ambush",
			result.outcome === "wounded"
				? `${cat.name} was wounded by raiders beyond the fence and fled home.`
				: `${cat.name} fled an advancing raider, dropping the carried yield.`,
			[cat._id],
			metadata,
		);
	}

	return killedCatIds;
}

function resolveActiveRaid(
	db: GameDb,
	colonyId: string,
	raidId: string,
	ctx: RaidDirectorContext,
	units: RaiderRow[],
	priorKilledCatIds: string[] = [],
): RaidDirectorResult {
	const aliveForGate = ctx.aliveCats.filter(
		(cat) => !priorKilledCatIds.includes(cat._id),
	);
	const muster = musterDefense(
		defenders(aliveForGate),
		{
			weapons: ctx.resources.weapons ?? 0,
			armor: ctx.resources.armor ?? 0,
		},
		ctx.effects,
	);

	const raiderPower = units.reduce((sum, r) => sum + Math.max(0, r.hp), 0);
	const outcome = resolveRaid(muster.totalPower, raiderPower, ctx.roll());

	// Gear is a consumable — the raid burns through whatever it mustered.
	ctx.resources.weapons = Math.max(
		0,
		(ctx.resources.weapons ?? 0) - muster.weaponsUsed,
	);
	ctx.resources.armor = Math.max(
		0,
		(ctx.resources.armor ?? 0) - muster.armorUsed,
	);

	const killedCatIds: string[] = [...priorKilledCatIds];

	if (outcome.defendersWin) {
		// Veterans of a won fight sharpen their trade.
		for (const m of muster.perCat) {
			const cat = aliveForGate.find((c) => c._id === m.id);
			if (!cat || cat.specialization !== "warrior") {
				continue;
			}
			const xp = (cat.roleXp?.warrior ?? 0) + WARRIOR_XP_PER_RAID;
			db.update(cats)
				.set({
					roleXp: {
						...(cat.roleXp ?? { hunter: 0, architect: 0, ritualist: 0 }),
						warrior: xp,
					},
				})
				.where(eq(cats._id, cat._id))
				.run();
		}
		logEvent(
			db,
			colonyId,
			"raid_repelled",
			muster.combatants > 0
				? `The village guard drove the raiders off at the gate — ${muster.combatants} defender${muster.combatants === 1 ? "" : "s"} held the line and the warband broke.`
				: "The raiders battered at the fence but found nothing worth the fight and melted back into the wilds.",
			[],
			{ raidId, defenders: muster.combatants, margin: outcome.margin },
		);
	} else {
		// Sack: raiders carry off a share of every dry store and the reservoir.
		const stolen: Record<string, number> = {};
		for (const key of LOOTABLE) {
			const have = (ctx.resources[key] as number | undefined) ?? 0;
			const take = Math.floor(have * outcome.lootFraction);
			if (take > 0) {
				(ctx.resources as unknown as Record<string, number>)[key] = have - take;
				stolen[key] = take;
			}
		}

		// A close, losing fight also costs a defender's life — the weakest
		// mustered cat falls; if none turned out, a random villager is taken.
		const casualtySlots = Math.max(
			0,
			MAX_RAID_CASUALTIES - raidCasualtiesSoFar(db, colonyId, raidId),
		);
		if (outcome.defenderCasualties > 0 && casualtySlots > 0) {
			const victimId =
				[...muster.perCat].sort((a, b) => a.power - b.power)[0]?.id ??
				pickRandomCat(aliveForGate, ctx.roll)?._id ??
				null;
			if (victimId) {
				markCatDead(db, victimId, ctx.now);
				const victim = aliveForGate.find((c) => c._id === victimId);
				killedCatIds.push(victimId);
				logEvent(
					db,
					colonyId,
					"raid_casualty",
					`${victim?.name ?? "A villager"} fell defending the gate as the raiders broke through.`,
					victimId ? [victimId] : [],
					{ raidId },
				);
			}
		}

		const lootLine =
			Object.entries(stolen)
				.map(([k, v]) => `${v} ${k}`)
				.join(", ") || "little of value";
		logEvent(
			db,
			colonyId,
			"raid_sacked",
			`Raiders overran the fence and made off with ${lootLine}. The village licks its wounds.`,
			[],
			{ raidId, stolen },
		);
	}

	return endRaid(db, colonyId, raidId, ctx, "resolved", killedCatIds);
}

function endRaid(
	db: GameDb,
	colonyId: string,
	raidId: string,
	_ctx: RaidDirectorContext,
	_reason: string,
	killedCatIds: string[] = [],
): RaidDirectorResult {
	db.delete(raiders).where(eq(raiders.raidId, raidId)).run();
	db.update(colonies)
		.set({ activeRaidId: null, raidClicks: 0 })
		.where(eq(colonies._id, colonyId))
		.run();
	return { pressure: 0, activeRaidId: null, killedCatIds };
}

function pickRandomCat(aliveCats: CatRow[], roll: () => number): CatRow | null {
	if (aliveCats.length === 0) {
		return null;
	}
	const idx = Math.min(
		aliveCats.length - 1,
		Math.floor(roll() * aliveCats.length),
	);
	return aliveCats[idx];
}
