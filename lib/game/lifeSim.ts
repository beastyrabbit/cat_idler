/**
 * Life-simulation rules (pure, deterministic, no side effects).
 *
 * The colony is a living population: cats age through life stages, work with
 * stage- and experience-dependent effectiveness, pair off and breed when the
 * settlement is healthy, and eventually die of old age. This module owns the
 * *decisions* — the worker tick threads a seeded roll function through them and
 * applies the results to the DB.
 *
 * Ages are measured in game-hours (see `cats.ageHours`), so every span here is
 * expressed on the same accelerated clock the job system runs on. Life-stage
 * thresholds and old-age death chance are reused from lib/game/age.ts.
 */

import type { CatSpecialization, CatStats, LifeStage } from "@/types/game";
import { getDeathChance } from "./age";
import { calculateBreedingChance } from "./breeding";

export { getLifeStage } from "./age";

/**
 * How capable a cat is at real work by life stage. Kittens can't work at all;
 * young cats pull their weight but haven't grown into it; adults are the
 * backbone; elders slow down but stay useful. Applied to hunt yield and (as a
 * duration penalty) to how long a cat takes to finish a job.
 */
export function stageWorkEffectiveness(stage: LifeStage): number {
	switch (stage) {
		case "kitten":
			return 0;
		case "young":
			return 0.8;
		case "adult":
			return 1;
		case "elder":
			return 0.7;
		default:
			return 1;
	}
}

/** A cat can be dispatched to jobs once it is out of the nursery. */
export function canWork(stage: LifeStage): boolean {
	return stage !== "kitten";
}

/**
 * Fraction of a full worker this cat counts as for the leader's employment
 * budget — kittens don't count, elders count partially. Summed across the
 * colony this gives the stage-aware workforce.
 */
export function workforceWeight(stage: LifeStage): number {
	return stageWorkEffectiveness(stage);
}

/**
 * Probability a cat dies of old age over `elapsedGameHours`. The per-hour
 * hazard from age.ts is scaled by the elapsed game-hours so the death rate is
 * independent of tick cadence (a 1s tick and a skip-time jump reach the same
 * expected lifespan). Clamped to a valid probability.
 */
export function oldAgeDeathProbability(
	ageHours: number,
	isLeaderOrHealer: boolean,
	elapsedGameHours: number,
): number {
	if (elapsedGameHours <= 0) {
		return 0;
	}
	const perHour = getDeathChance(ageHours, isLeaderOrHealer);
	return Math.max(0, Math.min(1, perHour * elapsedGameHours));
}

// --- Breeding --------------------------------------------------------------

/** Food must sit above this fraction of capacity for the colony to breed. */
export const BREEDING_MIN_FOOD_RATIO = 0.35;
/** Water must sit above this fraction of capacity for the colony to breed. */
export const BREEDING_MIN_WATER_RATIO = 0.35;
/**
 * Capacity-independent fallback breeding gate: a colony may also breed once it
 * holds this many units of food (and water) per living cat, even if that is
 * below {@link BREEDING_MIN_FOOD_RATIO} of a large granary. The ratio gate reads
 * "fraction of a full storehouse", which a founding colony with a 600-food cap
 * can never reach on subsistence hunting (0.35 * 600 = 210 vs the ~50 it holds
 * once its starting buffer is drawn down), so it would stop breeding and age out.
 * This per-capita floor gives a small, self-limiting surplus the same meaning at
 * any storage size: a fed/late colony always clears the ratio gate and so is
 * unaffected, while an unaided early colony can keep replacing its founders.
 */
export const BREEDING_FOOD_PER_CAT = 2.5;
export const BREEDING_WATER_PER_CAT = 2.5;
/** Gestation length in game-hours (matches the kitten stage span). */
export const GESTATION_GAME_HOURS = 6;
/** Base per-game-hour conception chance for a plain, healthy adult. */
export const BASE_BREEDING_CHANCE_PER_HOUR = 0.06;
/** Extra per-game-hour conception chance for a specialized adult. */
export const SPECIALIST_BREEDING_BONUS = 0.1;

export interface ColonyBreedingState {
	foodRatio: number;
	waterRatio: number;
	population: number;
	housingCapacity: number;
	/** Absolute stored food, for the per-capita fallback gate. Optional so
	 * callers that only know ratios keep the pure ratio behaviour. */
	food?: number;
	/** Absolute stored water, for the per-capita fallback gate. */
	water?: number;
}

/**
 * Whether the colony is healthy and roomy enough to grow. Breeding needs food
 * and water above their thresholds and at least one bed of housing headroom —
 * this is the soft population cap that keeps growth tied to the village's
 * shelter rather than exploding.
 *
 * Food/water are "sufficient" when they clear EITHER the fraction-of-capacity
 * gate (what a fed/late colony always does) OR the per-capita floor (what lets
 * a subsistence early colony keep breeding despite a large, mostly-empty
 * granary). The ratio path alone is unchanged, so mid/late balance is untouched.
 */
export function colonyCanBreed(state: ColonyBreedingState): boolean {
	const foodOk =
		state.foodRatio > BREEDING_MIN_FOOD_RATIO ||
		(state.food ?? 0) >= state.population * BREEDING_FOOD_PER_CAT;
	const waterOk =
		state.waterRatio > BREEDING_MIN_WATER_RATIO ||
		(state.water ?? 0) >= state.population * BREEDING_WATER_PER_CAT;
	return foodOk && waterOk && state.population < state.housingCapacity;
}

/**
 * Per-game-hour conception chance for one adult, before the elapsed-time
 * scaling. Specialists breed more readily (the roadmap's "specialized parents
 * beget gifted kittens"), and colony blessings raise fertility via the shared
 * breeding-chance curve.
 */
export function catBreedingChancePerHour(
	specialization: CatSpecialization,
	blessings: number,
): number {
	const base =
		BASE_BREEDING_CHANCE_PER_HOUR +
		(specialization ? SPECIALIST_BREEDING_BONUS : 0);
	return calculateBreedingChance(base, blessings);
}

/**
 * Probability this adult conceives this tick — the per-hour chance stretched
 * over the elapsed game-hours and clamped to a valid probability.
 */
export function conceptionProbability(
	specialization: CatSpecialization,
	blessings: number,
	elapsedGameHours: number,
): number {
	if (elapsedGameHours <= 0) {
		return 0;
	}
	const perHour = catBreedingChancePerHour(specialization, blessings);
	return Math.max(0, Math.min(1, perHour * elapsedGameHours));
}

// --- Genetic stat inheritance ---------------------------------------------

/**
 * Blend two parents' stats into a kitten's, biased toward the stronger parent
 * on each trait so a lineage's strengths compound: pair two born hunters and
 * their kittens start with high hunting and reach the hunter specialization
 * fast. A small mutation (±`STAT_MUTATION`) keeps siblings distinct.
 *
 * `roll` is a deterministic 0..1 source; one draw is consumed per stat.
 */
export const STAT_INHERIT_HIGH_WEIGHT = 0.6;
export const STAT_MUTATION = 8;

export function inheritStats(
	parent1: CatStats,
	parent2: CatStats | null,
	roll: () => number,
): CatStats {
	const keys: (keyof CatStats)[] = [
		"attack",
		"defense",
		"hunting",
		"medicine",
		"cleaning",
		"building",
		"leadership",
		"vision",
	];

	const out = {} as CatStats;
	for (const key of keys) {
		const a = parent1[key];
		const b = parent2 ? parent2[key] : parent1[key];
		const high = Math.max(a, b);
		const low = Math.min(a, b);
		const base =
			high * STAT_INHERIT_HIGH_WEIGHT + low * (1 - STAT_INHERIT_HIGH_WEIGHT);
		const mutation = (roll() * 2 - 1) * STAT_MUTATION;
		out[key] = Math.max(1, Math.min(100, Math.round(base + mutation)));
	}
	return out;
}

// --- Trade depth (specialization payoff) ----------------------------------

/**
 * Visible trade rank from role experience, on a diminishing curve so early
 * completions matter most. Shown in the cat card.
 */
export function tradeLevel(xp: number): number {
	if (xp <= 0) {
		return 0;
	}
	return Math.floor(Math.sqrt(xp));
}

/**
 * Yield bonus multiplier from working a trade — grows with experience but
 * flattens out (diminishing returns), capping around +40%.
 */
export function tradeYieldMultiplier(xp: number): number {
	if (xp <= 0) {
		return 1;
	}
	return 1 + 0.4 * (1 - 1 / (1 + xp / 30));
}

/**
 * Duration multiplier from working a trade — a trained cat finishes faster,
 * bottoming out around 25% quicker. Values <= 1.
 */
export function tradeSpeedMultiplier(xp: number): number {
	if (xp <= 0) {
		return 1;
	}
	return 1 - 0.25 * (1 - 1 / (1 + xp / 25));
}

// --- Leadership tenure -----------------------------------------------------

/** Leadership stat a sitting leader gains per game-hour in office. */
export const LEADERSHIP_GAIN_PER_HOUR = 0.35;

/** New leadership stat after `elapsedGameHours` in office, capped at 100. */
export function leadershipAfterTenure(
	leadership: number,
	elapsedGameHours: number,
): number {
	return Math.min(
		100,
		leadership + LEADERSHIP_GAIN_PER_HOUR * elapsedGameHours,
	);
}
