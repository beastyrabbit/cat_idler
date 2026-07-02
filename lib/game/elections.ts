/**
 * Election & vote-kick rules (pure).
 *
 * Players elect the colony leader from the most leaderly cats. A bad
 * leader can be ousted early when enough distinct players demand it.
 * The server resolves both inside workerTick.
 */

export interface ElectionCandidate {
	_id: string;
	leadership: number;
}

export interface BallotVote {
	playerId: string;
	catId: string;
}

/** Distinct players required to kick a leader. */
export const KICK_THRESHOLD = 5;

/** How many cats stand for election. */
export const CANDIDATE_COUNT = 5;

/** Leader term length (real ms; scale by testTimeScale at usage). */
export const TERM_MS = 24 * 3600 * 1000;

/** How long the polls stay open (real ms; scaled at usage). */
export const ELECTION_WINDOW_MS = 30 * 60 * 1000;

/** How long a vote-kick petition stays open (real ms; scaled at usage). */
export const KICK_WINDOW_MS = 10 * 60 * 1000;

/** Top candidates by leadership, excluding barred cats. */
export function candidatesFor(
	cats: ElectionCandidate[],
	barred: ReadonlySet<string> = new Set(),
): string[] {
	return [...cats]
		.filter((cat) => !barred.has(cat._id))
		.sort((a, b) => b.leadership - a.leadership)
		.slice(0, CANDIDATE_COUNT)
		.map((cat) => cat._id);
}

/** One position per player (their latest vote wins), counted per cat. */
export function tallyVotes(votes: BallotVote[]): Record<string, number> {
	const latest = new Map<string, string>();
	for (const vote of votes) {
		latest.set(vote.playerId, vote.catId);
	}
	const tally: Record<string, number> = {};
	for (const catId of latest.values()) {
		tally[catId] = (tally[catId] ?? 0) + 1;
	}
	return tally;
}

/**
 * Winner = most votes among candidates; ties break toward higher
 * leadership; zero votes fall back to the most leaderly candidate.
 */
export function electionWinner(
	candidates: ElectionCandidate[],
	tally: Record<string, number>,
): string | null {
	if (candidates.length === 0) {
		return null;
	}
	let winner = candidates[0];
	let winnerVotes = tally[winner._id] ?? 0;
	for (const candidate of candidates.slice(1)) {
		const votes = tally[candidate._id] ?? 0;
		if (
			votes > winnerVotes ||
			(votes === winnerVotes && candidate.leadership > winner.leadership)
		) {
			winner = candidate;
			winnerVotes = votes;
		}
	}
	return winner._id;
}

/** A kick succeeds only with enough distinct players behind it. */
export function shouldTriggerKick(votes: BallotVote[]): boolean {
	return new Set(votes.map((vote) => vote.playerId)).size >= KICK_THRESHOLD;
}

/** A new election is due when none has happened or the term expired. */
export function electionDue(
	lastResolvedAt: number | null,
	now: number,
	termMs: number,
): boolean {
	if (lastResolvedAt === null) {
		return true;
	}
	return now - lastResolvedAt >= termMs;
}
