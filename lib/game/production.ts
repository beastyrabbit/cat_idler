/**
 * Production chains (pure rules) — Phase 7.
 *
 * Workshops convert materials into refined goods while a worker is
 * assigned; fields grow food passively. Both unlock as the village
 * levels up (see lib/game/housing.ts villageLevel).
 */

export const WORKSHOP_MATERIALS_PER_CYCLE = 5;
export const WORKSHOP_REFINED_PER_CYCLE = 1;
export const WORKSHOP_CYCLE_SEC = 600;
/** Architects run workshops at double speed. */
export const ARCHITECT_SPEED = 2;

export const FIELD_FOOD_PER_HOUR = 2;

export const WORKSHOP_UNLOCK_LEVEL = 2;
export const FIELD_UNLOCK_LEVEL = 4;

export function workshopUnlocked(villageLevel: number): boolean {
	return villageLevel >= WORKSHOP_UNLOCK_LEVEL;
}

export function fieldUnlocked(villageLevel: number): boolean {
	return villageLevel >= FIELD_UNLOCK_LEVEL;
}

export interface WorkshopStep {
	/** Carry-over cycle time (seconds) after this tick. */
	nextProgress: number;
	/** Materials consumed this tick. */
	materialsUsed: number;
	/** Refined goods produced this tick. */
	refinedProduced: number;
}

/**
 * Advance one workshop. Progress only accrues with a worker; completed
 * cycles are limited by available materials (unspent progress carries
 * over so short ticks still add up).
 */
export function advanceWorkshop(
	progressSec: number,
	elapsedSec: number,
	options: {
		hasWorker: boolean;
		workerIsArchitect?: boolean;
		materialsAvailable: number;
	},
): WorkshopStep {
	if (!options.hasWorker || elapsedSec <= 0) {
		return {
			nextProgress: progressSec,
			materialsUsed: 0,
			refinedProduced: 0,
		};
	}

	const speed = options.workerIsArchitect ? ARCHITECT_SPEED : 1;
	let progress = progressSec + elapsedSec * speed;

	const cyclesByTime = Math.floor(progress / WORKSHOP_CYCLE_SEC);
	const cyclesByMaterials = Math.floor(
		options.materialsAvailable / WORKSHOP_MATERIALS_PER_CYCLE,
	);
	const cycles = Math.max(0, Math.min(cyclesByTime, cyclesByMaterials));

	progress -= cycles * WORKSHOP_CYCLE_SEC;
	// Without materials, progress stalls at one full cycle rather than
	// banking unlimited time.
	progress = Math.min(progress, WORKSHOP_CYCLE_SEC);

	return {
		nextProgress: progress,
		materialsUsed: cycles * WORKSHOP_MATERIALS_PER_CYCLE,
		refinedProduced: cycles * WORKSHOP_REFINED_PER_CYCLE,
	};
}

/** Passive food from one field over an elapsed window. */
export function fieldYield(elapsedSec: number): number {
	return Math.max(0, (elapsedSec / 3600) * FIELD_FOOD_PER_HOUR);
}
