/**
 * Player zones — server orchestration over lib/game/zones.
 *
 * Zones are per-player, capped, short-lived rectangles. Expiry is swept
 * inside workerTick; creation/removal are player actions.
 */

import { and, eq, gt } from "drizzle-orm";
import { nanoid } from "nanoid";

import type { GameDb } from "@/db/client";
import { type ColonyRow, colonies, type ZoneRow, zones } from "@/db/schema";
import { normalizeRect, validateZone } from "@/lib/game/zones";

import { upsertPlayer } from "./players";

function getGlobalColonyOrThrow(db: GameDb): ColonyRow {
	const colony = db
		.select()
		.from(colonies)
		.where(eq(colonies.isGlobal, true))
		.get();
	if (!colony) {
		throw new Error("Colony not initialized");
	}
	return colony;
}

export function activeZones(
	db: GameDb,
	colonyId: string,
	now: number,
): ZoneRow[] {
	return db
		.select()
		.from(zones)
		.where(and(eq(zones.colonyId, colonyId), gt(zones.expiresAt, now)))
		.all();
}

/** Remove expired zones (called from workerTick). */
export function sweepExpiredZones(db: GameDb, colonyId: string, now: number) {
	const expired = db
		.select({ _id: zones._id, expiresAt: zones.expiresAt })
		.from(zones)
		.where(eq(zones.colonyId, colonyId))
		.all()
		.filter((zone) => zone.expiresAt <= now);
	for (const zone of expired) {
		db.delete(zones).where(eq(zones._id, zone._id)).run();
	}
}

export function createZone(
	db: GameDb,
	args: {
		sessionId: string;
		nickname: string;
		kind: "avoid" | "gather";
		a: { x: number; y: number };
		b: { x: number; y: number };
		durationMs: number;
	},
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = getGlobalColonyOrThrow(tx);
		const now = Date.now();
		const player = upsertPlayer(tx, args.sessionId, args.nickname, now);

		const rect = normalizeRect(args.a, args.b);
		const mine = activeZones(tx, colony._id, now).filter(
			(zone) => zone.playerId === player._id,
		);
		const error = validateZone(rect, args.durationMs, mine.length);
		if (error) {
			throw new Error(error);
		}

		const zoneId = nanoid();
		tx.insert(zones)
			.values({
				_id: zoneId,
				colonyId: colony._id,
				kind: args.kind,
				...rect,
				playerId: player._id,
				createdAt: now,
				expiresAt: now + args.durationMs,
			})
			.run();

		tx.update(colonies)
			.set({ lastPlayerActivityAt: now })
			.where(eq(colonies._id, colony._id))
			.run();

		return { ok: true, zoneId };
	});
}

export function removeZone(
	db: GameDb,
	args: { sessionId: string; nickname: string; zoneId: string },
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		getGlobalColonyOrThrow(tx);
		const now = Date.now();
		const player = upsertPlayer(tx, args.sessionId, args.nickname, now);

		const zone = tx
			.select()
			.from(zones)
			.where(eq(zones._id, args.zoneId))
			.get();
		if (!zone) {
			throw new Error("Zone not found");
		}
		if (zone.playerId !== player._id) {
			throw new Error("You can only remove your own zones");
		}
		tx.delete(zones).where(eq(zones._id, zone._id)).run();
		return { ok: true };
	});
}
