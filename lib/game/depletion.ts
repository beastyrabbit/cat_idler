/**
 * World-resource depletion & regrowth rules (pure).
 *
 * Hunt hauls drain a site tile's food; non-forest tiles slowly regrow it
 * back toward their cap; forests, once chopped for lumber, are converted
 * to field permanently (their tree type never comes back).
 */

/**
 * Tile `type` values that count as forest — choppable for lumber and
 * excluded from food regrowth. Worldgen currently emits "forest" and
 * "dense_woods", but the biome-named variants are listed too so the rule
 * stays correct if the generator starts surfacing them directly.
 */
export const FOREST_TYPES = [
	"forest",
	"oak_forest",
	"pine_forest",
	"dense_woods",
	"jungle",
	"dead_forest",
] as const;

const FOREST_TYPE_SET: ReadonlySet<string> = new Set(FOREST_TYPES);

/** Whether a tile of this `type` is forest (choppable, no food regrowth). */
export function isForestType(type: string): boolean {
	return FOREST_TYPE_SET.has(type);
}

/**
 * Food regrown over `elapsedSec` at +1 per hour. Callers pass game-seconds
 * (wall seconds already scaled by the colony's time scale). Never negative.
 */
export function regrowthAmount(elapsedSec: number): number {
	if (elapsedSec <= 0) {
		return 0;
	}
	return elapsedSec / 3600;
}
