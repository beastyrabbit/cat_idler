import {
	chunkWindow,
	DEFAULT_ISO_GEOMETRY,
	type IsoGeometry,
	isoContentSize,
} from "@/lib/game/isoProjection";

export const ISO: IsoGeometry = DEFAULT_ISO_GEOMETRY;
export const ISO_CONTENT = isoContentSize(ISO);

export const CHUNK_SIZE = ISO.chunkSize;

/**
 * Renderable chunk window (25x25 chunks, ±12 around the village). Derived from
 * the content-plane geometry so it can never drift from the drawable area.
 */
const CHUNK_WINDOW = chunkWindow(ISO);
export const CHUNK_MIN = CHUNK_WINDOW.min;
export const CHUNK_MAX = CHUNK_WINDOW.max;

/**
 * Terrain sprite per tile type (Kenney Isometric Miniature series,
 * 256x512 bottom-anchored). `filter` tints reused sprites for biomes
 * without dedicated art. Standalone tree sprites have no ground in the
 * source art, so they declare a grass `base` underlay. River tiles render
 * as a CSS water diamond.
 */
const GRASS = "/images/iso/tiles/grass.png";

export const TILE_SPRITES: Record<
	string,
	{ src: string; filter?: string; base?: string }
> = {
	field: { src: GRASS },
	meadow: { src: "/images/iso/tiles/grass-clearing.png" },
	forest: { src: "/images/iso/tiles/tree-pine-small.png", base: GRASS },
	oak_forest: { src: "/images/iso/tiles/tree-pine-large.png", base: GRASS },
	pine_forest: { src: "/images/iso/tiles/tree-pine-huge.png", base: GRASS },
	dense_woods: {
		src: "/images/iso/tiles/tree-pine-huge.png",
		filter: "brightness(0.75)",
		base: GRASS,
	},
	jungle: {
		src: "/images/iso/tiles/tree-pine-large.png",
		filter: "saturate(1.6) hue-rotate(15deg)",
		base: GRASS,
	},
	dead_forest: { src: "/images/iso/tiles/tree-dead-large.png", base: GRASS },
	mountains: { src: "/images/iso/tiles/grass-hill-high.png" },
	swamp: {
		src: "/images/iso/tiles/grass-tree-stump.png",
		filter: "saturate(0.7) hue-rotate(30deg)",
	},
	desert: { src: "/images/iso/tiles/dirt.png" },
	tundra: { src: "/images/iso/tiles/snow.png" },
	cave_entrance: { src: "/images/iso/tiles/grass-stone-large.png" },
	enemy_territory: {
		src: "/images/iso/tiles/tree-dead-small.png",
		filter: "sepia(0.3) hue-rotate(-20deg)",
		base: GRASS,
	},
	enemy_lair: {
		src: "/images/iso/tiles/grass-stone-large.png",
		filter: "sepia(0.5) hue-rotate(-30deg) brightness(0.8)",
	},
};

/** Palisade fence pieces; the village shape decides where each segment sits. */
export const VILLAGE_RING_RADIUS = 4;
export const FENCE_X_SPRITE = "/images/iso/tiles/fence-x.png";
export const FENCE_Y_SPRITE = "/images/iso/tiles/fence-y.png";
export const GATE_SPRITE = "/images/iso/tiles/gate.png";

/** Water terrain (Isometric Nature pack, remapped to our diamond). */
export const WATER_SPRITE = "/images/iso/tiles/water.png";

/** Chopped-forest stump (drawn where a felled forest tile became field). */
export const STUMP_SPRITE = "/images/iso/tiles/stump.png";

/**
 * Oriented road/path sprites (Kenney "Isometric Miniature Overworld", same
 * 256x512 canvas as the ground tiles). A road tile picks its sprite from which
 * of its four orthogonal neighbours are also roads: a straight run along the
 * x- or y-axis, an L-corner where the run turns, or a crossing at a junction.
 * Both player-paved roads and heavily-trodden trails use these; worn trails are
 * dimmed a touch (see `ROAD_WORN_FILTER`) so paved roads still read as brighter.
 */
export const ROAD_SPRITES = {
	/** Runs along the x-axis (connects the E/W tile neighbours). */
	straightX: "/images/iso/tiles/path-straight-e.png",
	/** Runs along the y-axis (connects the N/S tile neighbours). */
	straightY: "/images/iso/tiles/path-straight-n.png",
	/** L-corner turning between one x-neighbour and one y-neighbour. */
	cornerEN: "/images/iso/tiles/path-corner-e.png",
	cornerES: "/images/iso/tiles/path-corner-s.png",
	cornerWN: "/images/iso/tiles/path-corner-n.png",
	cornerWS: "/images/iso/tiles/path-corner-w.png",
	/**
	 * Dead-end: path enters from one side and stops, keyed by the tile-space
	 * neighbour it connects to (E=bottom-right, W=top-left, N=top-right,
	 * S=bottom-left on screen). The pack's End sprites label the N/S diagonal
	 * opposite to its Corner sprites, so end-N connects our S edge and vice
	 * versa — verified sprite-by-sprite on /dev/fit.
	 */
	endE: "/images/iso/tiles/path-end-e.png",
	endW: "/images/iso/tiles/path-end-w.png",
	endN: "/images/iso/tiles/path-end-s.png",
	endS: "/images/iso/tiles/path-end-n.png",
	/** Isolated stub (no road neighbours) — a path terminating in a clearing. */
	clearing: "/images/iso/tiles/path-clearing-s.png",
	/** 3- or 4-way junction. */
	crossing: "/images/iso/tiles/path-crossing.png",
} as const;

/** Worn trails render dimmer than paved roads (same oriented sprites). */
export const ROAD_WORN_FILTER = "brightness(0.82) saturate(0.85)";

/** Road-neighbour direction bits, in tile space (x east, y south). */
export const ROAD_DIR = { E: 1, W: 2, N: 4, S: 8 } as const;

/**
 * Oriented road sprite for a tile, given which of its orthogonal neighbours are
 * also roads (a bitmask of {@link ROAD_DIR}). Pure so it can be unit-tested.
 *
 * Full autotile:
 * - no neighbours → an isolated clearing stub
 * - one neighbour → a dead-end oriented toward it
 * - opposite pair (E+W, or N+S) → a straight run along that axis
 * - one horizontal + one vertical → the matching L-corner
 * - three or more → a crossing (T or 4-way)
 */
export function roadSpriteFor(mask: number): string {
	const e = (mask & ROAD_DIR.E) !== 0;
	const w = (mask & ROAD_DIR.W) !== 0;
	const n = (mask & ROAD_DIR.N) !== 0;
	const s = (mask & ROAD_DIR.S) !== 0;
	const horizontal = Number(e) + Number(w);
	const vertical = Number(n) + Number(s);
	const total = horizontal + vertical;

	if (total >= 3) return ROAD_SPRITES.crossing;
	if (total === 0) return ROAD_SPRITES.clearing;
	if (total === 1) {
		if (e) return ROAD_SPRITES.endE;
		if (w) return ROAD_SPRITES.endW;
		if (n) return ROAD_SPRITES.endN;
		return ROAD_SPRITES.endS; // s
	}
	// total === 2
	if (e && w) return ROAD_SPRITES.straightX;
	if (n && s) return ROAD_SPRITES.straightY;
	if (e && n) return ROAD_SPRITES.cornerEN;
	if (e && s) return ROAD_SPRITES.cornerES;
	if (w && n) return ROAD_SPRITES.cornerWN;
	return ROAD_SPRITES.cornerWS; // w && s
}

export const BUILDING_SPRITES: Record<string, string> = {
	shrine: "/images/iso/buildings/shrine.png",
	den: "/images/iso/buildings/den.png",
	food_storage: "/images/iso/buildings/food-storage.png",
	water_bowl: "/images/iso/buildings/water-bowl.png",
	beds: "/images/iso/buildings/beds.png",
	herb_garden: "/images/iso/buildings/herb-garden.png",
	nursery: "/images/iso/buildings/nursery.png",
	elder_corner: "/images/iso/buildings/elder-corner.png",
	walls: "/images/iso/buildings/walls.png",
	mouse_farm: "/images/iso/buildings/mouse-farm.png",
	workshop: "/images/iso/buildings/workshop.png",
	field: "/images/iso/buildings/field.png",
	// Reuse curated pieces that read the part: elders at study for the
	// research hut, kittens at their books for the school.
	research_hut: "/images/iso/buildings/elder-corner.png",
	school: "/images/iso/buildings/nursery.png",
	// No dedicated 2D bridge art in the curated set; a road plank over water
	// reads correctly and keeps the 256x512 isometric canvas contract.
	bridge: "/images/iso/tiles/path-straight-e.png",
};
export const BRIDGE_SPRITES = {
	east_west: "/images/iso/tiles/path-straight-e.png",
	north_south: "/images/iso/tiles/path-straight-n.png",
} as const;
export const BUILDING_SPRITE_FALLBACK = "/images/iso/buildings/default.png";

/** Flat biome colors — used for the fog-of-war shimmer and loading states. */
export const TILE_COLORS: Record<string, string> = {
	field: "#b5cf8f",
	meadow: "#a9d18a",
	forest: "#6f9e58",
	oak_forest: "#7aa75e",
	pine_forest: "#527a5a",
	dense_woods: "#4c7a44",
	jungle: "#3f8f5f",
	dead_forest: "#8a8270",
	river: "#6aa9d8",
	mountains: "#9aa0a8",
	swamp: "#6e7d5a",
	desert: "#e0c98f",
	tundra: "#dfe7ea",
	cave_entrance: "#7d7468",
	enemy_territory: "#b07a7a",
	enemy_lair: "#9b5a5a",
};

/** CSS clip-path for a 2:1 iso ground diamond. */
export const DIAMOND_CLIP = "polygon(50% 0%, 100% 50%, 50% 100%, 0% 50%)";

/**
 * Fog-of-war shades by distance (in tiles) to the nearest explored tile.
 * Index 0 hugs the explored frontier (lightest, a hint of land beyond the
 * fence); each step darkens until the last shade equals the page backdrop
 * (`bg-[#141c12]`), so deep fog dissolves into "beyond the known world".
 * Tiles more than `FOG_SHADES.length - 1` away, and ungenerated chunks, use
 * the final (solid) shade.
 */
export const FOG_SHADES = ["#33422a", "#26321f", "#1b2416", "#141c12"];
