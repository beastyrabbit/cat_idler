/**
 * Elections & vote-kick — server orchestration over lib/game/elections.
 *
 * `runElectionLifecycle` is called from workerTick (single tick path);
 * `castVote` / `requestVoteKick` are player actions from the API route.
 *
 * Voter identity is a verified session plus a server-derived salted
 * subscriber/network hash. Either axis collapses to one ballot per poll.
 */

import { and, desc, eq, isNull, or } from "drizzle-orm";
import { nanoid } from "nanoid";

import type { GameDb } from "@/db/client";
import {
	type CatRow,
	type ColonyRow,
	cats,
	colonies,
	type ElectionRow,
	elections,
	events,
	type PlayerRow,
	votes,
} from "@/db/schema";
import {
	candidatesFor,
	ELECTION_WINDOW_MS,
	electionDue,
	electionWinner,
	KICK_THRESHOLD,
	KICK_WINDOW_MS,
	shouldTriggerKick,
	TERM_MS,
	tallyVotes,
} from "@/lib/game/elections";

import { upsertPlayer } from "./players";

export const VOTE_SESSION_MIN_AGE_MS = 2 * 60 * 1000;
export const VOTE_SESSION_MIN_PRESENCE_COUNT = 2;

interface VoteIdentity {
	subscriberHash?: string | null;
}

function logElectionEvent(
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

function getVotes(db: GameDb, electionId: string) {
	return db.select().from(votes).where(eq(votes.electionId, electionId)).all();
}

function normalizeSubscriberHash(
	hash: string | null | undefined,
): string | null {
	const trimmed = hash?.trim();
	return trimmed ? trimmed : null;
}

function assertEligibleVoter(player: PlayerRow, now: number) {
	if (
		player.presenceCount < VOTE_SESSION_MIN_PRESENCE_COUNT ||
		now - player.createdAt < VOTE_SESSION_MIN_AGE_MS
	) {
		throw new Error(
			"Stay in the colony a little longer before voting. New sessions need a short presence history.",
		);
	}
}

function matchingVoteRows(
	db: GameDb,
	electionId: string,
	playerId: string,
	subscriberHash: string | null,
) {
	const base = and(
		eq(votes.electionId, electionId),
		eq(votes.playerId, playerId),
	);
	const predicate = subscriberHash
		? and(
				eq(votes.electionId, electionId),
				or(
					eq(votes.playerId, playerId),
					eq(votes.subscriberHash, subscriberHash),
				),
			)
		: base;

	return db.select().from(votes).where(predicate).all();
}

function upsertVote(
	db: GameDb,
	election: ElectionRow,
	player: PlayerRow,
	catId: string,
	now: number,
	identity: VoteIdentity = {},
) {
	const subscriberHash = normalizeSubscriberHash(identity.subscriberHash);
	const matches = matchingVoteRows(
		db,
		election._id,
		player._id,
		subscriberHash,
	);
	const keeper =
		(subscriberHash
			? matches.find((vote) => vote.subscriberHash === subscriberHash)
			: undefined) ??
		matches.find((vote) => vote.playerId === player._id) ??
		matches[0];

	if (keeper) {
		for (const duplicate of matches) {
			if (duplicate._id !== keeper._id) {
				db.delete(votes).where(eq(votes._id, duplicate._id)).run();
			}
		}
		db.update(votes)
			.set({
				playerId: player._id,
				subscriberHash,
				catId,
				createdAt: now,
			})
			.where(eq(votes._id, keeper._id))
			.run();
		return;
	}

	db.insert(votes)
		.values({
			_id: nanoid(),
			electionId: election._id,
			playerId: player._id,
			subscriberHash,
			catId,
			createdAt: now,
		})
		.run();
}

function openElectionsFor(db: GameDb, colonyId: string): ElectionRow[] {
	return db
		.select()
		.from(elections)
		.where(and(eq(elections.colonyId, colonyId), eq(elections.status, "open")))
		.all();
}

/** Cats kicked this run cannot stand again. */
function barredCatIds(
	db: GameDb,
	colonyId: string,
	runNumber: number,
): Set<string> {
	const kicks = db
		.select()
		.from(elections)
		.where(
			and(
				eq(elections.colonyId, colonyId),
				eq(elections.kind, "vote_kick"),
				eq(elections.status, "resolved"),
				eq(elections.runNumber, runNumber),
			),
		)
		.all();
	const barred = new Set<string>();
	for (const kick of kicks) {
		// Convention: a successful kick records the target as winnerCatId.
		if (kick.winnerCatId && kick.winnerCatId === kick.targetCatId) {
			barred.add(kick.winnerCatId);
		}
	}
	return barred;
}

function lastResolvedElectionAt(db: GameDb, colonyId: string): number | null {
	const last = db
		.select()
		.from(elections)
		.where(
			and(
				eq(elections.colonyId, colonyId),
				eq(elections.kind, "election"),
				eq(elections.status, "resolved"),
			),
		)
		.orderBy(desc(elections.endsAt))
		.limit(1)
		.get();
	return last?.endsAt ?? null;
}

function openScheduledElection(
	db: GameDb,
	colony: ColonyRow,
	aliveCats: CatRow[],
	timeScale: number,
	now: number,
) {
	const barred = barredCatIds(db, colony._id, colony.runNumber ?? 1);
	const candidateIds = candidatesFor(
		aliveCats.map((cat) => ({
			_id: cat._id,
			leadership: cat.stats.leadership,
		})),
		barred,
	);
	if (candidateIds.length === 0) {
		return;
	}
	const windowMs = Math.max(5_000, ELECTION_WINDOW_MS / Math.max(1, timeScale));
	db.insert(elections)
		.values({
			_id: nanoid(),
			colonyId: colony._id,
			kind: "election",
			status: "open",
			candidateCatIds: candidateIds,
			targetCatId: null,
			startedAt: now,
			endsAt: now + windowMs,
			winnerCatId: null,
			runNumber: colony.runNumber ?? 1,
		})
		.run();
	logElectionEvent(
		db,
		colony._id,
		"election_started",
		"The colony is holding a leadership election — cast your vote!",
		candidateIds,
	);
}

/**
 * Resolve due polls and open the next scheduled election. Runs inside the
 * workerTick transaction; consumes no policy rolls.
 */
export function runElectionLifecycle(
	db: GameDb,
	colony: ColonyRow,
	aliveCats: CatRow[],
	runtime: { timeScale: number },
	now: number,
) {
	const open = openElectionsFor(db, colony._id);
	let openElection =
		open.find((election) => election.kind === "election") ?? null;
	const openKick =
		open.find((election) => election.kind === "vote_kick") ?? null;

	// Resolve a due leadership election.
	if (openElection && openElection.endsAt <= now) {
		const candidates = aliveCats
			.filter((cat) => openElection?.candidateCatIds.includes(cat._id))
			.map((cat) => ({ _id: cat._id, leadership: cat.stats.leadership }));
		const ballots = getVotes(db, openElection._id).map((vote) => ({
			playerId: vote.playerId,
			subscriberHash: vote.subscriberHash,
			catId: vote.catId,
		}));
		const winnerId = electionWinner(candidates, tallyVotes(ballots));

		db.update(elections)
			.set({ status: "resolved", winnerCatId: winnerId })
			.where(eq(elections._id, openElection._id))
			.run();
		openElection = null;

		if (winnerId) {
			const winner = aliveCats.find((cat) => cat._id === winnerId);
			if (winner && colony.leaderId !== winnerId) {
				db.update(colonies)
					.set({ leaderId: winnerId })
					.where(eq(colonies._id, colony._id))
					.run();
				colony.leaderId = winnerId;
			}
			if (winner) {
				logElectionEvent(
					db,
					colony._id,
					"election_won",
					`${winner.name} won the leadership election with ${ballots.length} ballot${ballots.length === 1 ? "" : "s"} cast.`,
					[winnerId],
				);
			}
		}
	}

	// Resolve a due vote-kick petition.
	if (openKick && openKick.endsAt <= now) {
		const ballots = getVotes(db, openKick._id).map((vote) => ({
			playerId: vote.playerId,
			subscriberHash: vote.subscriberHash,
			catId: vote.catId,
		}));
		const kicked =
			shouldTriggerKick(ballots) &&
			openKick.targetCatId !== null &&
			openKick.targetCatId === colony.leaderId;

		db.update(elections)
			.set({
				status: "resolved",
				winnerCatId: kicked ? openKick.targetCatId : null,
			})
			.where(eq(elections._id, openKick._id))
			.run();

		if (kicked && openKick.targetCatId) {
			const target = aliveCats.find((cat) => cat._id === openKick.targetCatId);
			logElectionEvent(
				db,
				colony._id,
				"leader_kicked",
				`${target?.name ?? "The leader"} was voted out by the players!`,
				openKick.targetCatId ? [openKick.targetCatId] : [],
			);

			// Interim replacement (kicked cat excluded), then a snap election.
			const barred = barredCatIds(db, colony._id, colony.runNumber ?? 1);
			barred.add(openKick.targetCatId);
			const interim = [...aliveCats]
				.filter((cat) => !barred.has(cat._id))
				.sort((a, b) => b.stats.leadership - a.stats.leadership)[0];
			db.update(colonies)
				.set({ leaderId: interim?._id ?? null })
				.where(eq(colonies._id, colony._id))
				.run();
			colony.leaderId = interim?._id ?? null;

			if (!openElection) {
				openScheduledElection(db, colony, aliveCats, runtime.timeScale, now);
				openElection =
					openElectionsFor(db, colony._id).find(
						(election) => election.kind === "election",
					) ?? null;
			}
		}
	}

	// Open the scheduled election when the term expires.
	if (!openElection) {
		const termMs = Math.max(10_000, TERM_MS / Math.max(1, runtime.timeScale));
		if (electionDue(lastResolvedElectionAt(db, colony._id), now, termMs)) {
			openScheduledElection(db, colony, aliveCats, runtime.timeScale, now);
		}
	}
}

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

export function castVote(
	db: GameDb,
	args: {
		sessionId: string;
		nickname: string;
		electionId: string;
		catId: string;
		subscriberHash?: string | null;
	},
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = getGlobalColonyOrThrow(tx);
		const now = Date.now();
		const player = upsertPlayer(tx, args.sessionId, args.nickname, now);
		assertEligibleVoter(player, now);

		const election = tx
			.select()
			.from(elections)
			.where(eq(elections._id, args.electionId))
			.get();
		if (
			!election ||
			election.colonyId !== colony._id ||
			election.status !== "open" ||
			election.endsAt <= now
		) {
			throw new Error("This poll is closed");
		}

		const validChoice =
			election.kind === "election"
				? election.candidateCatIds.includes(args.catId)
				: election.targetCatId === args.catId;
		if (!validChoice) {
			throw new Error("Not a valid candidate");
		}

		upsertVote(tx, election, player, args.catId, now, {
			subscriberHash: args.subscriberHash,
		});

		tx.update(colonies)
			.set({ lastPlayerActivityAt: now })
			.where(eq(colonies._id, colony._id))
			.run();

		return { ok: true, electionId: election._id, catId: args.catId };
	});
}

export function requestVoteKick(
	db: GameDb,
	args: { sessionId: string; nickname: string; subscriberHash?: string | null },
) {
	return db.transaction((txRaw) => {
		const tx = txRaw as unknown as GameDb;
		const colony = getGlobalColonyOrThrow(tx);
		const now = Date.now();
		const player = upsertPlayer(tx, args.sessionId, args.nickname, now);
		assertEligibleVoter(player, now);

		if (!colony.leaderId) {
			throw new Error("There is no leader to vote out");
		}
		const leader = tx
			.select()
			.from(cats)
			.where(and(eq(cats._id, colony.leaderId), isNull(cats.deathTime)))
			.get();
		if (!leader) {
			throw new Error("There is no leader to vote out");
		}

		const alreadyOpen = openElectionsFor(tx, colony._id).find(
			(election) => election.kind === "vote_kick",
		);
		if (alreadyOpen) {
			// Petition exists — this call counts as the player's vote.
			return castVoteInline(tx, alreadyOpen, player, leader._id, now, {
				subscriberHash: args.subscriberHash,
			});
		}

		const timeScale = Math.max(1, colony.testTimeScale ?? 1);
		const windowMs = Math.max(5_000, KICK_WINDOW_MS / timeScale);
		const electionId = nanoid();
		tx.insert(elections)
			.values({
				_id: electionId,
				colonyId: colony._id,
				kind: "vote_kick",
				status: "open",
				candidateCatIds: [],
				targetCatId: leader._id,
				startedAt: now,
				endsAt: now + windowMs,
				winnerCatId: null,
				runNumber: colony.runNumber ?? 1,
			})
			.run();
		upsertVote(
			tx,
			{
				_id: electionId,
				colonyId: colony._id,
				kind: "vote_kick",
				status: "open",
				candidateCatIds: [],
				targetCatId: leader._id,
				startedAt: now,
				endsAt: now + windowMs,
				winnerCatId: null,
				runNumber: colony.runNumber ?? 1,
			},
			player,
			leader._id,
			now,
			{ subscriberHash: args.subscriberHash },
		);

		logElectionEvent(
			tx,
			colony._id,
			"vote_kick_started",
			`A petition to remove ${leader.name} is gathering signatures (${KICK_THRESHOLD} needed).`,
			[leader._id],
		);

		tx.update(colonies)
			.set({ lastPlayerActivityAt: now })
			.where(eq(colonies._id, colony._id))
			.run();

		return { ok: true, electionId };
	});
}

function castVoteInline(
	db: GameDb,
	election: ElectionRow,
	player: PlayerRow,
	catId: string,
	now: number,
	identity: VoteIdentity = {},
) {
	upsertVote(db, election, player, catId, now, identity);
	return { ok: true, electionId: election._id };
}
