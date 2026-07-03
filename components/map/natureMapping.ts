/**
 * Kenney "Isometric Nature" sprite mapping.
 *
 * Resolves the abstract terrain roles emitted by `@/lib/game/terrainGen` onto
 * concrete PNG sprites from the Isometric Nature pack. Classification was done
 * by visually inspecting montages of the pack's rotation-0 sprites (no human
 * annotations exist), so every data entry carries a confidence tag explaining
 * the basis for the mapping.
 *
 * Pack facts (measured with ImageMagick):
 *   - Every sprite is 220x379 px.
 *   - The ground diamond is 182x115 px, bottom-anchored; its top vertex sits at
 *     y=252 (surfaceOffset) and its left vertex 19 px in (horizontally centered).
 *   - Each numeric group `NNN` ships 4 rotations `_0`.._3` of the SAME object.
 *
 * Rotation -> compass analysis (task convention: diamond top vertex = N, right
 * = E, bottom = S, left = W; the renderer's `tileToIso` puts grid-East at the
 * lower-right screen edge and grid-South at the lower-left screen edge — the two
 * front-visible faces). For the straight cliff family (groups 097/116) the four
 * rotations present the vertical face clockwise: r0=South (lower-left), r1=West,
 * r2=North, r3=East (lower-right). This was confirmed on both the grey (097) and
 * tan (116) cliffs.
 */

import type {
	BiomeRole,
	CliffBase,
	Direction,
	RiverSegment,
} from "@/lib/game/terrainGen";

export interface NatureSprite {
	/** PNG basename, e.g. "naturePack_012_0.png". */
	file: string;
	/** Rotation index 0..3 baked into `file` (for the future /dev/tiles editor). */
	rotation: number;
	/**
	 * Canvas Y of this sprite's ground-diamond top vertex, when it differs from
	 * the standard flat-tile anchor (252). Full-tile raised blocks (cliffs) author
	 * their grass diamond higher in the canvas (y=187) with a one-floor face
	 * beneath it; anchoring by this value seats the grass at the tile's floor
	 * instead of double-raising it. Omit for flat/object sprites drawn on the
	 * standard diamond.
	 */
	surfaceOffset?: number;
}

// ---------------------------------------------------------------------------
// Low-level helpers
// ---------------------------------------------------------------------------

/** Path segments under `public/` that lead to the pack's PNG folder. */
const PACK_SEGMENTS: readonly string[] = [
	"Kenney Game Assets All-in-1 3.5.0",
	"2D assets",
	"Isometric Nature",
	"PNG",
];

/** Build a sprite record for a numeric group + rotation. */
function makeSprite(
	group: string,
	rotation: number,
	surfaceOffset?: number,
): NatureSprite {
	return {
		file: `naturePack_${group}_${rotation}.png`,
		rotation,
		surfaceOffset,
	};
}

/** Public URL of a Nature sprite (handles the spaces in the pack path). */
export function natureSpriteUrl(sprite: NatureSprite): string {
	const encoded = [...PACK_SEGMENTS, sprite.file].map(encodeURIComponent);
	return `/${encoded.join("/")}`;
}

// ---------------------------------------------------------------------------
// Rotation -> facing tables
// ---------------------------------------------------------------------------

/**
 * Facing -> rotation for stairs (group 124). Read off rot_120/rot_124: the
 * descending (open) end points lower-right at r2 (grid East) and lower-left at
 * r3 (grid South) — a clockwise sequence phase-shifted one step from the cliff
 * family (r0=West, r1=North, r2=East, r3=South).
 */
const STAIR_ROT: Record<Direction, number> = { W: 0, N: 1, E: 2, S: 3 };

/**
 * Facing -> rotation for river channel tiles. Reuses the cliff clockwise order
 * as a best guess; a symmetric straight channel makes the true flow direction
 * visually ambiguous, so this is low confidence.
 */
const RIVER_ROT: Record<Direction, number> = { S: 0, W: 1, N: 2, E: 3 };

// ---------------------------------------------------------------------------
// Grounds
// ---------------------------------------------------------------------------
//
// The pack has no biome-tinted ground diamonds — all full flat diamonds share
// a green grass top and differ only in the side-band colour. Group 001 (grass
// top, tan/dirt side, 182x115 full diamond) is the universal grass tile; group
// 006 (grass top, pale grey/stone side) is the closest thing to a rocky base.
// The `flat_*` sprites are NOT grounds — they are tiny (~22 px) flowers/plants.

// confidence: high - 001 is a full 182x115 grass diamond with a dirt side band.
const GROUND_GRASS = "001";
// confidence: medium - 006 is a full flat diamond but the grey/stone band is
// the only "rocky" cue; the top is still grass (pack has no bare-rock ground).
const GROUND_ROCKY = "006";

const GROUND_BY_BIOME: Record<BiomeRole, string> = {
	// confidence: high - grass diamond, exact match for open lowland.
	lowland: GROUND_GRASS,
	// confidence: high - grass diamond, exact match for grassland.
	grassland: GROUND_GRASS,
	// confidence: medium - no distinct forest-floor ground; reuse grass (trees
	// are drawn as separate decoration sprites on top).
	forest: GROUND_GRASS,
	// confidence: medium - stone-banded flat diamond 006 as nearest rocky look.
	rocky: GROUND_ROCKY,
	// confidence: low - no dedicated highland ground; reuse the stone-band tile.
	highland: GROUND_ROCKY,
};

/** Ultimate fallback ground diamond. */
// confidence: high - 001 rotation 0 is a plain grass diamond, safe default.
export const FALLBACK_GROUND: NatureSprite = makeSprite(GROUND_GRASS, 0);

/** Flat ground diamond for a biome. Always defined (fallback = grass). */
export function groundSprite(biome: BiomeRole): NatureSprite {
	const group = GROUND_BY_BIOME[biome] ?? GROUND_GRASS;
	// Ground diamonds are rotationally symmetric, so rotation 0 always suffices.
	return makeSprite(group, 0);
}

// ---------------------------------------------------------------------------
// Cliffs
// ---------------------------------------------------------------------------
//
// The pack's oriented single-face cliffs (groups 116/097) are only ~114 px wide
// — half a tile — so a run of them reads as disconnected "torn strips" rather
// than a continuous cliff wall. Group 095 is a FULL-tile (182 px) grass-topped
// block: a 182x180 sprite whose grass diamond sits at canvas y=187 with a one-
// floor (~64 px) face beneath it, and whose four rotations are identical (both
// front faces are drawn, so the isometric camera always sees a solid S+E wall).
// It meshes with the flat ground diamond and reads as a coherent cliff. Group
// 107 is the equivalent rockier block. Anchoring by the sprite's own diamond-top
// (187) seats the grass at the tile's floor instead of stacking an extra block.
//
// measured with `magick -trim`:
//   095 -> 182x180 +19+187 (grass-topped full block)
//   107 -> 182x178 +19+189 (rockier full block)

/** Canvas Y of the raised-block sprites' grass-diamond top vertex. */
const CLIFF_BLOCK_SURFACE_OFFSET = 187;
// confidence: high - 095 is a full-width grass-topped one-floor block.
const CLIFF_BLOCK_GROUP = "095";
// confidence: medium - 107 is the rockier full-width block, for rocky/highland.
const CLIFF_BLOCK_ROCKY_GROUP = "107";

/**
 * Full-tile cliff block. `base`/`variant`/`facing` come from terrainGen's
 * CliffTerrainRole but the block is orientation-symmetric (it always presents
 * both visible front faces; front neighbours overdraw any spurious face), so we
 * only vary the grass vs. rocky top. Never throws.
 */
export function cliffSprite(
	base: CliffBase,
	_variant: string,
	_facing: Direction | null,
): NatureSprite {
	const group = base === "pillar" ? CLIFF_BLOCK_ROCKY_GROUP : CLIFF_BLOCK_GROUP;
	return makeSprite(group, 0, CLIFF_BLOCK_SURFACE_OFFSET);
}

// ---------------------------------------------------------------------------
// Stairs
// ---------------------------------------------------------------------------

// confidence: medium - 124 is a tan staircase cut into a cliff (steps clearly
// visible in all four rotations of rot_124); descent direction is low
// confidence (STAIR_ROT is a best-guess reading of the open step end).
const STAIRS_GROUP = "124";

/** Staircase descending toward `facing`. */
export function stairsSprite(facing: Direction): NatureSprite {
	return makeSprite(STAIRS_GROUP, STAIR_ROT[facing]);
}

// ---------------------------------------------------------------------------
// Rivers
// ---------------------------------------------------------------------------
//
// River tiles are flat diamonds (182x115, same footprint as ground) with a
// teal water channel. Group 002 is the clean straight channel. Groups 145/146/
// 147 are water-pool tiles used as source/bend/mouth best guesses. All are low
// confidence on exact segment role and flow orientation.

const RIVER_BY_SEGMENT: Record<RiverSegment, string> = {
	// confidence: low - 145 is a water-pool tile; used as the source cap.
	start: "145",
	// confidence: medium - 002 is a clean straight water channel across the tile.
	straight: "002",
	// confidence: low - 146 is a water-pool tile; used as a bend (no clean
	// L-shaped channel sprite was found).
	bend: "146",
	// confidence: low - 147 is a water-pool tile; used as the mouth/end cap.
	end: "147",
};

/** Oriented river segment. `facing` is the flow (out) dir; inflow for `end`. */
export function riverSprite(
	segment: RiverSegment,
	facing: Direction,
): NatureSprite {
	const group = RIVER_BY_SEGMENT[segment] ?? "002";
	return makeSprite(group, RIVER_ROT[facing]);
}

// ---------------------------------------------------------------------------
// Trees
// ---------------------------------------------------------------------------
//
// Distinct species read off the montages (each ships 4 rotations; canopy is
// near rotation-invariant, so rotation 0 is used). Species is picked from a
// biome-appropriate palette so warm-climate trees (palms) stay out of conifer
// biomes and vice-versa; the terrain's abstract `species` index just selects
// within that biome's palette (wraps via modulo).
//   062 round green deciduous · 051 pointed conifer/pine · 066 autumn deciduous
//   061 palm · 148 tall cypress/columnar conifer · 079 bare stump
const TREES_BY_BIOME: Record<BiomeRole, readonly string[]> = {
	// Warm open lowland: leafy deciduous with the occasional palm.
	lowland: ["062", "066", "061"],
	// Temperate grassland: deciduous mix, no palms.
	grassland: ["062", "066", "148"],
	// Forest: conifer-dominant with some broadleaf.
	forest: ["051", "148", "062"],
	// Rocky slopes: sparse hardy conifers and dead stumps.
	rocky: ["051", "079"],
	// Cold highland: columnar conifers only.
	highland: ["148", "051"],
};

/** Fallback tree palette when a biome is unknown. */
const DEFAULT_TREES: readonly string[] = ["062", "051"];

/** Tree for a biome + species index (index wraps within the biome palette). */
export function treeSprite(species: number, biome?: BiomeRole): NatureSprite {
	const palette = (biome && TREES_BY_BIOME[biome]) || DEFAULT_TREES;
	const idx =
		((Math.trunc(species) % palette.length) + palette.length) % palette.length;
	return makeSprite(palette[idx], 0);
}

// ---------------------------------------------------------------------------
// Rocks
// ---------------------------------------------------------------------------
//
// Clean grey/white boulder set at the tail of the pack, graded by footprint.

const ROCK_BY_SIZE: Record<"small" | "medium" | "large", string> = {
	// confidence: high - 171 is a small pebble/stone (~48px).
	small: "171",
	// confidence: high - 172 is a medium boulder (~112px).
	medium: "172",
	// confidence: high - 173 is a large boulder (~123px).
	large: "173",
};

/** Rock by size. */
export function rockSprite(size: "small" | "medium" | "large"): NatureSprite {
	const group = ROCK_BY_SIZE[size] ?? ROCK_BY_SIZE.medium;
	return makeSprite(group, 0);
}
