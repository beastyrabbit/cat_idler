/**
 * Player presence and identity (ported from convex/players.ts).
 */

import { eq, gte } from "drizzle-orm";
import { nanoid } from "nanoid";

import type { GameDb } from "@/db/client";
import { type PlayerRow, players } from "@/db/schema";

export function upsertPlayer(
	db: GameDb,
	sessionId: string,
	nickname: string,
	now: number = Date.now(),
): PlayerRow {
	const existing = db
		.select()
		.from(players)
		.where(eq(players.sessionId, sessionId))
		.get();

	if (existing) {
		db.update(players)
			.set({ nickname, lastSeenAt: now })
			.where(eq(players._id, existing._id))
			.run();
		return { ...existing, nickname, lastSeenAt: now };
	}

	const row: PlayerRow = {
		_id: nanoid(),
		sessionId,
		nickname,
		lastSeenAt: now,
		clickWindowStart: now,
		clicksInWindow: 0,
		lifetimeClicks: 0,
		lifetimeContribution: {
			food: 0,
			water: 0,
			jobsRequested: 0,
			upgradesPurchased: 0,
		},
	};
	db.insert(players).values(row).run();
	return row;
}

export function countOnlinePlayers(
	db: GameDb,
	now: number = Date.now(),
	minutes = 5,
): number {
	const cutoff = now - minutes * 60 * 1000;
	return db.select().from(players).where(gte(players.lastSeenAt, cutoff)).all()
		.length;
}
