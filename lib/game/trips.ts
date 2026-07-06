/**
 * Multi-trip gathering (pure rules) — SC2 drones, idle-paced.
 *
 * A hunting cat doesn't sit at its site for the whole job: it hauls the
 * yield home in shares — collect, carry a share to the shrine, walk
 * back, repeat — with the final share loading at job completion.
 */

/** Total hauls per hunt (2 mid-job trips + the completion haul). */
export const HUNT_TRIP_COUNT = 3;

/**
 * Integer share for one trip; earlier trips carry the remainder so the
 * shares always sum exactly to the total.
 */
export function splitYield(
	total: number,
	tripCount: number,
	tripIndex: number,
): number {
	const whole = Math.floor(total);
	const count = Math.max(1, tripCount);
	const base = Math.floor(whole / count);
	const bonusTrips = whole % count;
	return base + (tripIndex < bonusTrips ? 1 : 0);
}

/** Yield still at the site after `tripsDone` shares have been hauled. */
export function remainingYield(
	total: number,
	tripCount: number,
	tripsDone: number,
): number {
	let hauled = 0;
	for (let i = 0; i < Math.min(tripsDone, tripCount); i++) {
		hauled += splitYield(total, tripCount, i);
	}
	return Math.max(0, Math.floor(total) - hauled);
}

/**
 * When mid-trip `tripIndex` (1-based, 1..tripCount-1) departs for the
 * shrine, spaced evenly across the job duration.
 */
export function tripDueAt(
	startedAt: number,
	endsAt: number,
	tripIndex: number,
	tripCount: number = HUNT_TRIP_COUNT,
): number {
	const duration = Math.max(1, endsAt - startedAt);
	return startedAt + (duration * tripIndex) / Math.max(1, tripCount);
}
