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

/**
 * The food cap the chop path stamps onto a felled forest tile. A natural
 * grassland/meadow `field` tile caps far higher (40+), so this low cap is a
 * reliable render-only signature that a tile *used to be forest* and was
 * chopped — no schema field or overlay marker needed.
 */
export const CHOPPED_FOREST_FOOD_CAP = 5;

/**
 * Whether a tile is a chopped-forest stump: converted to `field`, drained, and
 * stamped with the chop's low food cap. Pure and render-only — lets the map
 * draw a tree stump where a forest once stood without touching the chop writer.
 */
export function isChoppedStumpTile(tile: {
	type: string;
	maxResources: { food: number };
	lastDepleted: number;
}): boolean {
	return (
		tile.type === "field" &&
		tile.lastDepleted > 0 &&
		tile.maxResources.food <= CHOPPED_FOREST_FOOD_CAP
	);
}
