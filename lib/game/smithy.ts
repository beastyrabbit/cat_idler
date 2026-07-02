/**
 * Smithy production chain (pure rules) — Roadmap 4, Military.
 *
 * A staffed smithy converts refined goods + raw materials into weapons and
 * armor that pile up in the colony stockpile (`weapons`, `armor` on
 * ColonyResources). It reuses the exact shape of {@link advanceWorkshop} in
 * lib/game/production.ts — accumulated `productionProgress`, one assigned
 * worker, whole cycles limited by both time and available inputs — so the
 * tick integrates it the same way it integrates workshops.
 */

/** Refined goods one smithy cycle consumes. */
export const SMITHY_REFINED_PER_CYCLE = 2;
/** Raw materials one smithy cycle consumes. */
export const SMITHY_MATERIALS_PER_CYCLE = 3;
/** Weapons one smithy cycle forges. */
export const SMITHY_WEAPONS_PER_CYCLE = 1;
/** Armor one smithy cycle forges. */
export const SMITHY_ARMOR_PER_CYCLE = 1;
/** Seconds of work one full smithy cycle takes (15 game-minutes). */
export const SMITHY_CYCLE_SEC = 900;
/** A smith with the architect trade works the forge at double speed. */
export const SMITH_FAST_SPEED = 2;

export interface SmithyStep {
	/** Carry-over cycle time (seconds) after this tick. */
	nextProgress: number;
	/** Refined goods consumed this tick. */
	refinedUsed: number;
	/** Raw materials consumed this tick. */
	materialsUsed: number;
	/** Weapons produced this tick. */
	weaponsProduced: number;
	/** Armor produced this tick. */
	armorProduced: number;
}

/**
 * Advance one smithy. Progress only accrues with a worker; completed cycles
 * are limited by whichever input runs out first (refined or materials).
 * Unspent progress carries over so short ticks still add up, and without
 * inputs the progress stalls at one full cycle rather than banking unbounded
 * time.
 */
export function advanceSmithy(
	progressSec: number,
	elapsedSec: number,
	options: {
		hasWorker: boolean;
		workerIsFast?: boolean;
		refinedAvailable: number;
		materialsAvailable: number;
	},
): SmithyStep {
	if (!options.hasWorker || elapsedSec <= 0) {
		return {
			nextProgress: progressSec,
			refinedUsed: 0,
			materialsUsed: 0,
			weaponsProduced: 0,
			armorProduced: 0,
		};
	}

	const speed = options.workerIsFast ? SMITH_FAST_SPEED : 1;
	let progress = progressSec + elapsedSec * speed;

	const cyclesByTime = Math.floor(progress / SMITHY_CYCLE_SEC);
	const cyclesByRefined = Math.floor(
		options.refinedAvailable / SMITHY_REFINED_PER_CYCLE,
	);
	const cyclesByMaterials = Math.floor(
		options.materialsAvailable / SMITHY_MATERIALS_PER_CYCLE,
	);
	const cycles = Math.max(
		0,
		Math.min(cyclesByTime, cyclesByRefined, cyclesByMaterials),
	);

	progress -= cycles * SMITHY_CYCLE_SEC;
	// Without inputs, progress stalls at one full cycle rather than banking
	// unlimited time.
	progress = Math.min(progress, SMITHY_CYCLE_SEC);

	return {
		nextProgress: progress,
		refinedUsed: cycles * SMITHY_REFINED_PER_CYCLE,
		materialsUsed: cycles * SMITHY_MATERIALS_PER_CYCLE,
		weaponsProduced: cycles * SMITHY_WEAPONS_PER_CYCLE,
		armorProduced: cycles * SMITHY_ARMOR_PER_CYCLE,
	};
}
