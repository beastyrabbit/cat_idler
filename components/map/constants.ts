import {
	DEFAULT_ISO_GEOMETRY,
	type IsoGeometry,
	isoContentSize,
} from "@/lib/game/isoProjection";

export const ISO: IsoGeometry = DEFAULT_ISO_GEOMETRY;
export const ISO_CONTENT = isoContentSize(ISO);

export const CHUNK_SIZE = ISO.chunkSize;

/** Renderable chunk window (7x7 chunks around the village). */
export const CHUNK_MIN = -3;
export const CHUNK_MAX = 3;

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

/** CSS clip-path for a 2:1 iso ground diamond. */
export const DIAMOND_CLIP = "polygon(50% 0%, 100% 50%, 50% 100%, 0% 50%)";
