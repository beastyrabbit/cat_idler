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
 * Actors (cats, buildings, fences, gates) still use the Kenney "Isometric
 * Miniature" sprites (256x512 canvas, 256x128 diamond). To seat them on the
 * Nature ground diamond (180 wide) we scale them uniformly by 180/256 rather
 * than stretching — their diamond becomes 180 wide, so they line up
 * horizontally with the ground while keeping their own aspect. Style mixing
 * (Miniature actors on Nature terrain) is accepted for this pass.
 */
const MINI_DIAMOND_WIDTH = 256;
const MINI_IMAGE_HEIGHT = 512;
const MINI_SURFACE_OFFSET = 368;
export const ACTOR_SCALE = ISO.tileWidth / MINI_DIAMOND_WIDTH;
export const ACTOR = {
	scale: ACTOR_SCALE,
	width: MINI_DIAMOND_WIDTH * ACTOR_SCALE,
	height: MINI_IMAGE_HEIGHT * ACTOR_SCALE,
	/** Y within the scaled canvas where the diamond top vertex sits. */
	surfaceOffset: MINI_SURFACE_OFFSET * ACTOR_SCALE,
};

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

/** Fence ring around the founding village, with a gate on the south side. */
export const VILLAGE_RING_RADIUS = 4;
export const FENCE_X_SPRITE = "/images/iso/tiles/fence-x.png";
export const FENCE_Y_SPRITE = "/images/iso/tiles/fence-y.png";
export const GATE_SPRITE = "/images/iso/tiles/gate.png";

/** Water terrain (Isometric Nature pack, remapped to our diamond). */
export const WATER_SPRITE = "/images/iso/tiles/water.png";

/** Worn road on heavily-trodden tiles. */
export const ROAD_SPRITE = "/images/iso/tiles/road.png";

/** Player-paved permanent road (fastest travel). */
export const BUILT_ROAD_SPRITE = "/images/iso/tiles/road-built.png";

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
};
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

/** CSS clip-path for an iso ground diamond (parameterized by the tile box). */
export const DIAMOND_CLIP = "polygon(50% 0%, 100% 50%, 50% 100%, 0% 50%)";

/** Worn-trail and paved-road recolors, drawn as a translucent diamond over the
 *  ground so roads read on the Nature terrain without a dedicated road pack. */
export const ROAD_FILL = "rgba(120, 94, 62, 0.55)";
export const BUILT_ROAD_FILL = "rgba(150, 120, 78, 0.8)";

/**
 * Fog-of-war shades by distance (in tiles) to the nearest explored tile.
 * Index 0 hugs the explored frontier (lightest, a hint of land beyond the
 * fence); each step darkens until the last shade equals the page backdrop
 * (`bg-[#141c12]`), so deep fog dissolves into "beyond the known world".
 * Tiles more than `FOG_SHADES.length - 1` away, and ungenerated chunks, use
 * the final (solid) shade.
 */
export const FOG_SHADES = ["#33422a", "#26321f", "#1b2416", "#141c12"];

/**
 * Fog is now a translucent overlay over the Nature terrain (so unexplored land
 * still reads as a dim silhouette of cliffs/rivers rather than a flat patch),
 * graded by distance to the explored frontier: a light haze at the frontier
 * deepening toward the near-opaque backdrop far out. Index maps 1:1 to
 * `FOG_SHADES`; ungenerated/far tiles use the last (deepest) value.
 */
export const FOG_OPACITIES = [0.5, 0.7, 0.85, 0.95];
