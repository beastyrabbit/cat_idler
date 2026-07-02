/**
 * Shrine deposit rules (pure).
 *
 * Hunt and ritual yields are carried back to the shrine and credited on
 * arrival. Cosmetics can never break the economy: if a carrier hasn't
 * arrived within a grace window after its job ended, the deposit is
 * force-credited anyway.
 */

export interface Carrying {
	kind: "food" | "blessings";
	amount: number;
	jobEndedAt: number;
}

/** Credit no later than this long after the producing job ended. */
export const DEPOSIT_GRACE_MS = 60_000;

/** A carrier within this Chebyshev distance of the shrine deposits. */
export const DEPOSIT_RADIUS = 1;

export function isAtShrine(
	pos: { x: number; y: number },
	shrine: { x: number; y: number },
): boolean {
	return (
		Math.max(Math.abs(pos.x - shrine.x), Math.abs(pos.y - shrine.y)) <=
		DEPOSIT_RADIUS
	);
}

/** True once the grace window has elapsed — credit regardless of position. */
export function shouldForceDeposit(
	carrying: Carrying,
	now: number,
	graceMs: number = DEPOSIT_GRACE_MS,
): boolean {
	return now >= carrying.jobEndedAt + graceMs;
}

/**
 * Whether a carrier deposits this tick: it either reached the shrine or
 * ran out the grace window.
 */
export function shouldDeposit(
	carrying: Carrying,
	pos: { x: number; y: number },
	shrine: { x: number; y: number },
	now: number,
	graceMs: number = DEPOSIT_GRACE_MS,
): boolean {
	return isAtShrine(pos, shrine) || shouldForceDeposit(carrying, now, graceMs);
}
